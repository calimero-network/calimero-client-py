//! What losing the node that issued a session, or a node on the wrong
//! release, does to a session.
//!
//! Since core #4774 a session token is signed by the node that issued it, and
//! the cluster's other nodes take it only while they can vouch for that node's
//! signing key. When the issuer is replaced, removed or out of reach (a rolling
//! deploy, an image rollout), they answer its tokens `401 UNAUTHENTICATED`:
//! the user did not sign out, and `POST /auth/refresh` on any node gives a
//! fresh session. A node whose mero-tee release the account's owners have not
//! approved answers `421 WRONG_NODE` instead: it neither issues nor takes the
//! account's sessions, so the client moves to another node rather than
//! refreshing there.
//!
//! [`Nodes::run`] answers both, once each per request:
//!
//! - a `401` to a request sent with a session: refresh once and repeat it. A
//!   refresh token is used once (a second use revokes the session), so the
//!   requests refused at the same time share one refresh ([`Nodes::renew`]).
//! - a `421 WRONG_NODE`: move to the next node the client was given and repeat
//!   the request there.
//!
//! Repeating is safe for any request, a call too: the node answers `401` and
//! `421` while judging the credential, before it runs anything, and answers
//! the ceremonies that mint a session `421` before it spends their challenge
//! or refresh token. A sign-in or sign-up goes through [`Nodes::run_once`]: it
//! needs the passkey again, so its caller repeats it. A refresh answered `421`
//! goes on to each next node with the same token, once per node.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use calimero_client::{Client, Error, Session};

/// What the recovery needs of one node's client. [`Client`] in the bindings; a
/// stand-in in the tests below.
pub(crate) trait Node: Send + Sync {
    async fn session(&self) -> Option<Session>;
    async fn use_session(&self, session: Session);
    async fn use_api_key(&self, key: &str);
    async fn refresh(&self) -> Result<Session, Error>;
}

impl Node for Client {
    async fn session(&self) -> Option<Session> {
        Client::session(self).await
    }
    async fn use_session(&self, session: Session) {
        Client::use_session(self, session).await;
    }
    async fn use_api_key(&self, key: &str) {
        Client::use_api_key(self, key).await;
    }
    async fn refresh(&self) -> Result<Session, Error> {
        Client::refresh(self).await
    }
}

/// `401 UNAUTHENTICATED`.
pub(crate) fn unauthenticated(e: &Error) -> bool {
    matches!(e, Error::Api { status: 401, code, .. } if code == "UNAUTHENTICATED")
}

/// `421 WRONG_NODE`, and not another 421 (`NOT_LEADER` names a leader to
/// follow). Core's client reports it as its own variant.
pub(crate) fn wrong_node(e: &Error) -> bool {
    matches!(e, Error::WrongNode { .. })
        || matches!(e, Error::Api { status: 421, code, .. } if code == "WRONG_NODE")
}

/// The nodes of one cluster a client may use, the one in use, and its credential.
pub(crate) struct Nodes<N> {
    urls: Vec<String>,
    nodes: Vec<Arc<N>>,
    at: AtomicUsize,
    /// The API key last given to `use_api_key`, or `None` while a session is
    /// the credential. A request made with an API key is never refreshed.
    api_key: std::sync::Mutex<Option<String>>,
    /// Held for a refresh: whoever comes next sees the session it made.
    refreshing: tokio::sync::Mutex<()>,
}

impl<N: Node> Nodes<N> {
    /// `nodes` in the order to use them (`(url, its client)`), the first in use.
    pub(crate) fn new(nodes: Vec<(String, N)>) -> Self {
        let (urls, nodes) = nodes.into_iter().map(|(u, n)| (u, Arc::new(n))).unzip();
        Self {
            urls,
            nodes,
            at: AtomicUsize::new(0),
            api_key: std::sync::Mutex::new(None),
            refreshing: tokio::sync::Mutex::new(()),
        }
    }

    /// The node in use and its index.
    pub(crate) fn current(&self) -> (usize, Arc<N>) {
        let at = self.at.load(Ordering::SeqCst);
        (at, Arc::clone(&self.nodes[at]))
    }

    /// The URL of the node in use.
    pub(crate) fn url(&self) -> &str {
        &self.urls[self.at.load(Ordering::SeqCst)]
    }

    pub(crate) fn api_key(&self) -> Option<String> {
        self.api_key.lock().ok().and_then(|k| k.clone())
    }

    /// Record the credential: `Some(key)` for an API key, `None` for a session.
    pub(crate) fn set_api_key(&self, key: Option<String>) {
        if let Ok(mut k) = self.api_key.lock() {
            *k = key;
        }
    }

    /// The session token a request sent to `node` now carries, if it carries one.
    async fn session_token(&self, node: &N) -> Option<String> {
        if self.api_key().is_some() {
            return None;
        }
        node.session().await.map(|s| s.session_token)
    }

    /// Move on from node `from` (the one that answered `421 WRONG_NODE`) to the
    /// next, handing it the credential. False when there is no other node. A
    /// request that saw `from` refuse after another moved on already does not
    /// move again.
    async fn move_on(&self, from: usize) -> bool {
        if self.nodes.len() < 2 {
            return false;
        }
        let next = (from + 1) % self.nodes.len();
        let (old, new) = (&self.nodes[from], &self.nodes[next]);
        if let Some(key) = self.api_key() {
            new.use_api_key(&key).await;
        } else if let Some(session) = old.session().await {
            new.use_session(session).await;
        }
        let _moved = self
            .at
            .compare_exchange(from, next, Ordering::SeqCst, Ordering::SeqCst);
        true
    }

    /// Run `op` on the node in use, answering one `401` with a refresh and one
    /// `421 WRONG_NODE` with the next node, each by repeating `op` once (see
    /// the module's documentation for why that is safe).
    pub(crate) async fn run<T>(
        &self,
        op: impl AsyncFn(Arc<N>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let (mut refreshed, mut moved) = (false, false);
        loop {
            let (at, node) = self.current();
            let token = self.session_token(&node).await;
            let e = match op(Arc::clone(&node)).await {
                Ok(v) => return Ok(v),
                Err(e) => e,
            };
            if wrong_node(&e) && !moved {
                moved = true;
                if self.move_on(at).await {
                    continue;
                }
            } else if unauthenticated(&e) && !refreshed {
                if let Some(token) = token {
                    refreshed = true;
                    self.renew(&token).await?;
                    continue;
                }
            }
            return Err(e);
        }
    }

    /// Run `op` once: a sign-in or sign-up. A `421 WRONG_NODE` still moves the
    /// client on, so the next attempt goes to the next node.
    pub(crate) async fn run_once<T>(
        &self,
        op: impl AsyncFnOnce(Arc<N>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let (at, node) = self.current();
        let answer = op(node).await;
        if matches!(&answer, Err(e) if wrong_node(e)) {
            let _moved = self.move_on(at).await;
        }
        answer
    }

    /// A fresh session in place of the one `token` belonged to: the one a
    /// refresh by another request made meanwhile, or a refresh now.
    async fn renew(&self, token: &str) -> Result<(), Error> {
        let _one = self.refreshing.lock().await;
        // The node in use, not the one that refused: a refresh that moved on
        // left the new session on the node it moved to.
        let (_, node) = self.current();
        if self.session_token(&node).await.as_deref() != Some(token) {
            return Ok(());
        }
        self.exchange().await.map(drop)
    }

    /// Exchange the refresh token for a new session on the node in use, one at
    /// a time with every other refresh of this client.
    pub(crate) async fn refresh(&self) -> Result<Session, Error> {
        let _one = self.refreshing.lock().await;
        self.exchange().await
    }

    /// A node that may not serve the account answers `421 WRONG_NODE` before
    /// it spends the refresh token (core #4774), so the same token goes on to
    /// the next node (which [`move_on`](Self::move_on) hands the session),
    /// once per node at most. Every node refusing is the caller's `WrongNode`.
    async fn exchange(&self) -> Result<Session, Error> {
        let mut tried = 1;
        loop {
            let (at, node) = self.current();
            match node.refresh().await {
                Err(e) if wrong_node(&e) && tried < self.nodes.len() => {
                    tried += 1;
                    let _moved = self.move_on(at).await;
                }
                answer => return answer,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU32;
    use std::time::Duration;

    use serde_json::json;
    use tokio::sync::RwLock;

    use super::*;

    fn session(token: &str, refresh: &str) -> Session {
        Session {
            account_id: "acc-1".to_owned(),
            session_token: token.to_owned(),
            expires_at: 0,
            refresh_token: refresh.to_owned(),
        }
    }

    fn refusal(status: u16, code: &str) -> Error {
        Error::Api {
            status,
            code: code.to_owned(),
            message: format!("{} (stand-in)", code.to_lowercase()),
            details: json!({ "node": 1 }),
        }
    }

    /// A stand-in node: takes the session tokens in `valid`, exchanges
    /// `rt_old` for `v2.new` once (slowly, so refreshes can overlap), or
    /// answers every request with `refuse`.
    #[derive(Default)]
    struct Stand {
        session: RwLock<Option<Session>>,
        api_key: RwLock<Option<String>>,
        valid: Vec<&'static str>,
        refuse: Option<(u16, &'static str)>,
        refuse_refresh: Option<(u16, &'static str)>,
        refreshes: AtomicU32,
        requests: AtomicU32,
    }

    impl Node for Stand {
        async fn session(&self) -> Option<Session> {
            self.session.read().await.clone()
        }
        async fn use_session(&self, s: Session) {
            *self.session.write().await = Some(s);
        }
        async fn use_api_key(&self, key: &str) {
            *self.api_key.write().await = Some(key.to_owned());
        }
        async fn refresh(&self) -> Result<Session, Error> {
            let _ = self.refreshes.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            if let Some((status, code)) = self.refuse_refresh {
                return Err(refusal(status, code));
            }
            let current = self.session().await.expect("a session");
            if current.refresh_token != "rt_old" {
                return Err(refusal(401, "UNAUTHENTICATED"));
            }
            let new = session("v2.new", "rt_new");
            self.use_session(new.clone()).await;
            Ok(new)
        }
    }

    impl Stand {
        /// A request: answered by this node's own rules.
        async fn ask(&self) -> Result<&'static str, Error> {
            let _ = self.requests.fetch_add(1, Ordering::SeqCst);
            if let Some((status, code)) = self.refuse {
                return Err(refusal(status, code));
            }
            if self.api_key.read().await.is_some() {
                return Err(refusal(401, "UNAUTHENTICATED"));
            }
            let token = self.session().await.map(|s| s.session_token);
            match token {
                Some(t) if self.valid.contains(&t.as_str()) => Ok("v1"),
                _ => Err(refusal(401, "UNAUTHENTICATED")),
            }
        }
    }

    /// A node that can no longer vouch for `v2.old`'s issuer: it takes only `v2.new`.
    fn issuer_lost() -> Stand {
        Stand {
            valid: vec!["v2.new"],
            ..Stand::default()
        }
    }

    async fn signed_in(nodes: Vec<Stand>) -> Nodes<Stand> {
        let nodes = Nodes::new(
            nodes
                .into_iter()
                .enumerate()
                .map(|(i, n)| (format!("http://node-{i}"), n))
                .collect(),
        );
        nodes
            .current()
            .1
            .use_session(session("v2.old", "rt_old"))
            .await;
        nodes
    }

    #[tokio::test]
    async fn a_401_for_a_lost_issuer_refreshes_once_and_repeats_the_request() {
        let nodes = signed_in(vec![issuer_lost()]).await;
        let answer = nodes.run(async |n| n.ask().await).await;
        assert_eq!(answer.unwrap(), "v1");
        let (_, node) = nodes.current();
        assert_eq!(node.refreshes.load(Ordering::SeqCst), 1);
        assert_eq!(node.requests.load(Ordering::SeqCst), 2);
        assert_eq!(node.session().await.unwrap().session_token, "v2.new");
    }

    #[tokio::test]
    async fn a_refused_refresh_is_login_required_and_the_request_is_not_repeated() {
        let nodes = signed_in(vec![Stand {
            refuse_refresh: Some((401, "UNAUTHENTICATED")),
            ..issuer_lost()
        }])
        .await;
        let e = nodes.run(async |n| n.ask().await).await.unwrap_err();
        assert!(unauthenticated(&e), "{e}");
        let (_, node) = nodes.current();
        assert_eq!(node.refreshes.load(Ordering::SeqCst), 1);
        assert_eq!(node.requests.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn requests_refused_at_the_same_time_share_one_refresh() {
        let nodes = Arc::new(signed_in(vec![issuer_lost()]).await);
        let tasks: Vec<_> = (0..8)
            .map(|_| {
                let nodes = Arc::clone(&nodes);
                tokio::spawn(async move { nodes.run(async |n| n.ask().await).await })
            })
            .collect();
        for task in tasks {
            assert_eq!(task.await.unwrap().unwrap(), "v1");
        }
        // A refresh token is single use: sent twice, the second would revoke the session.
        assert_eq!(nodes.current().1.refreshes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_second_401_after_the_refresh_is_final() {
        let nodes = signed_in(vec![Stand::default()]).await;
        let e = nodes.run(async |n| n.ask().await).await.unwrap_err();
        assert!(unauthenticated(&e), "{e}");
        let (_, node) = nodes.current();
        assert_eq!(node.refreshes.load(Ordering::SeqCst), 1);
        assert_eq!(node.requests.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn an_api_key_is_not_refreshed() {
        let nodes = signed_in(vec![issuer_lost()]).await;
        nodes.set_api_key(Some("cal_key".to_owned()));
        nodes.current().1.use_api_key("cal_key").await;
        let e = nodes.run(async |n| n.ask().await).await.unwrap_err();
        assert!(unauthenticated(&e), "{e}");
        assert_eq!(nodes.current().1.refreshes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn wrong_node_moves_to_the_next_node_with_the_session_and_repeats_there() {
        let nodes = signed_in(vec![
            Stand {
                refuse: Some((421, "WRONG_NODE")),
                ..Stand::default()
            },
            Stand {
                valid: vec!["v2.old"],
                ..Stand::default()
            },
        ])
        .await;
        assert_eq!(nodes.run(async |n| n.ask().await).await.unwrap(), "v1");
        assert_eq!(nodes.url(), "http://node-1");
        assert_eq!(nodes.nodes[0].requests.load(Ordering::SeqCst), 1);
        assert_eq!(nodes.nodes[1].requests.load(Ordering::SeqCst), 1);
        // Later requests stay on the new node.
        assert_eq!(nodes.run(async |n| n.ask().await).await.unwrap(), "v1");
        assert_eq!(nodes.nodes[0].requests.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn wrong_node_with_no_other_node_is_the_callers() {
        let nodes = signed_in(vec![Stand {
            refuse: Some((421, "WRONG_NODE")),
            ..Stand::default()
        }])
        .await;
        let e = nodes.run(async |n| n.ask().await).await.unwrap_err();
        assert!(wrong_node(&e), "{e}");
        assert_eq!(e.details(), Some(&json!({ "node": 1 })));
        assert_eq!(nodes.current().1.requests.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn wrong_node_twice_is_the_callers_after_one_move() {
        let refusing = || Stand {
            refuse: Some((421, "WRONG_NODE")),
            ..Stand::default()
        };
        let nodes = signed_in(vec![refusing(), refusing(), refusing()]).await;
        let e = nodes.run(async |n| n.ask().await).await.unwrap_err();
        assert!(wrong_node(&e), "{e}");
        let asked: u32 = nodes
            .nodes
            .iter()
            .map(|n| n.requests.load(Ordering::SeqCst))
            .sum();
        assert_eq!(asked, 2);
    }

    #[tokio::test]
    async fn not_leader_is_not_a_move() {
        let nodes = signed_in(vec![
            Stand {
                refuse: Some((421, "NOT_LEADER")),
                ..Stand::default()
            },
            Stand::default(),
        ])
        .await;
        let e = nodes.run(async |n| n.ask().await).await.unwrap_err();
        assert_eq!(e.code(), Some("NOT_LEADER"));
        assert_eq!(nodes.url(), "http://node-0");
        assert_eq!(nodes.nodes[1].requests.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_sign_in_refused_wrong_node_is_not_repeated_but_the_next_goes_elsewhere() {
        let nodes = signed_in(vec![
            Stand {
                refuse: Some((421, "WRONG_NODE")),
                ..Stand::default()
            },
            Stand::default(),
        ])
        .await;
        let e = nodes.run_once(async |n| n.ask().await).await.unwrap_err();
        assert!(wrong_node(&e), "{e}");
        assert_eq!(nodes.nodes[1].requests.load(Ordering::SeqCst), 0);
        assert_eq!(nodes.url(), "http://node-1");
    }

    #[tokio::test]
    async fn a_refresh_refused_wrong_node_goes_to_the_next_node_with_the_same_token() {
        let nodes = signed_in(vec![
            Stand {
                refuse_refresh: Some((421, "WRONG_NODE")),
                ..issuer_lost()
            },
            issuer_lost(),
        ])
        .await;
        assert_eq!(nodes.run(async |n| n.ask().await).await.unwrap(), "v1");
        assert_eq!(nodes.url(), "http://node-1");
        assert_eq!(nodes.nodes[0].refreshes.load(Ordering::SeqCst), 1);
        // Node 1's stand-in exchanges only `rt_old`: the token was not spent.
        assert_eq!(nodes.nodes[1].refreshes.load(Ordering::SeqCst), 1);
        let (_, node) = nodes.current();
        assert_eq!(node.session().await.unwrap().session_token, "v2.new");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_requests_share_a_refresh_that_moved_on() {
        let nodes = Arc::new(
            signed_in(vec![
                Stand {
                    refuse_refresh: Some((421, "WRONG_NODE")),
                    ..issuer_lost()
                },
                issuer_lost(),
            ])
            .await,
        );
        let tasks: Vec<_> = (0..8)
            .map(|_| {
                let nodes = Arc::clone(&nodes);
                tokio::spawn(async move { nodes.run(async |n| n.ask().await).await })
            })
            .collect();
        for task in tasks {
            assert_eq!(task.await.unwrap().unwrap(), "v1");
        }
        assert_eq!(nodes.nodes[1].refreshes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_refresh_every_node_refuses_wrong_node_is_tried_once_per_node() {
        let refusing = || Stand {
            refuse_refresh: Some((421, "WRONG_NODE")),
            ..issuer_lost()
        };
        let nodes = signed_in(vec![refusing(), refusing(), refusing()]).await;
        let e = nodes.refresh().await.unwrap_err();
        assert!(wrong_node(&e), "{e}");
        for node in &nodes.nodes {
            assert_eq!(node.refreshes.load(Ordering::SeqCst), 1);
        }
    }
}
