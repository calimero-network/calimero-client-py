//! `CloudClient` (also `Client`): a connection to one node of a Calimero 1.0
//! cluster. Attestation, the sealed channel and passkey handling run in core's
//! Rust client, so Python never re-implements the trust check. Every method
//! blocks with the GIL released.
//!
//! The methods are split by area, each file its own `#[pymethods]` block:
//! [`account`], [`apps`], [`blobs`], [`orgs`], [`governance`], [`packages`]
//! and [`ops`]. This file holds the connection and the generic transport.

mod account;
mod apps;
mod blobs;
mod governance;
mod ops;
mod orgs;
mod packages;
mod prelude;
mod recover;

use prelude::*;
use recover::Nodes;

/// A connection to one node of a Calimero 1.0 cluster (and, given `nodes`,
/// the others it may move to). Also exported as `Client`, the preferred name.
#[pyclass(name = "CloudClient")]
pub struct PyCloudClient {
    /// The nodes this client may use, the one in use, and the credential
    /// (the API key, the ops token for the Ops API, which a request sent on
    /// to the leader carries too). See [`recover`].
    nodes: Arc<Nodes<Client>>,
    runtime: Arc<Runtime>,
    /// The trust policy this client was made with, so a request that must
    /// run on a leader (`NOT_LEADER`) is sent to a node attested the same way.
    policy: Policy,
    /// The operator's Ed25519 key (`use_operator_key`): the Ops API's
    /// decisions answer `428 OPERATOR_SIGNATURE_REQUIRED` with it.
    operator_key: Arc<std::sync::Mutex<Option<OperatorKey>>>,
}

/// The operator's signing key, as the node's `--operator-key` names its
/// verify half.
pub(crate) type OperatorKey = ed25519_dalek::SigningKey;

/// An operator key from its 32-byte seed, hex.
pub(crate) fn operator_key_from_hex(seed: &str) -> PyResult<OperatorKey> {
    hex::decode(seed.trim())
        .ok()
        .and_then(|v| <[u8; 32]>::try_from(v).ok())
        .map(|seed| OperatorKey::from_bytes(&seed))
        .ok_or_else(|| PyValueError::new_err("operator_key must be a 32-byte Ed25519 seed, hex"))
}

/// How many `421 NOT_LEADER` redirects an Ops request follows. A node that
/// holds no replica names one that may itself redirect once more.
const LEADER_HOPS: usize = 3;

impl PyCloudClient {
    /// Run `op` on the node in use with the GIL released, answering a `401`
    /// once the issuing node is lost (refresh once, repeat) and a `421
    /// WRONG_NODE` (the next node, repeat) as [`Nodes::run`] does. The future
    /// runs on this thread (`block_on`), so it need not be `Send`.
    fn run<T: Send>(
        &self,
        py: Python<'_>,
        op: impl AsyncFn(Arc<Client>) -> Result<T, Error> + Send,
    ) -> PyResult<T> {
        let (rt, nodes) = (Arc::clone(&self.runtime), Arc::clone(&self.nodes));
        py.allow_threads(move || rt.block_on(nodes.run(op)).map_err(cloud_err))
    }

    /// [`run`](Self::run) for a sign-in or sign-up, sent once: a node that
    /// answers it `421` moves the client on, and the caller signs in again
    /// with the passkey.
    fn run_once<T: Send>(
        &self,
        py: Python<'_>,
        op: impl AsyncFnOnce(Arc<Client>) -> Result<T, Error> + Send,
    ) -> PyResult<T> {
        let (rt, nodes) = (Arc::clone(&self.runtime), Arc::clone(&self.nodes));
        py.allow_threads(move || rt.block_on(nodes.run_once(op)).map_err(cloud_err))
    }

    /// Block on `f` with the GIL released, as it is: no recovery.
    fn block<T: Send>(
        &self,
        py: Python<'_>,
        f: impl std::future::Future<Output = Result<T, Error>> + Send,
    ) -> PyResult<T> {
        let rt = Arc::clone(&self.runtime);
        py.allow_threads(move || rt.block_on(f).map_err(cloud_err))
    }

    /// The node in use.
    fn client(&self) -> Arc<Client> {
        self.nodes.current().1
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Verb {
    Get,
    Post,
    Delete,
}

async fn send_verb(c: &Client, verb: Verb, path: &str, body: Value) -> Result<Value, Error> {
    match verb {
        Verb::Get => c.get(path).await,
        Verb::Post => c.post(path, body).await,
        Verb::Delete => c.delete(path).await,
    }
}

/// [`send_verb`] for an operator decision: with `key`, a `428
/// OPERATOR_SIGNATURE_REQUIRED` is answered with its signature over the bytes
/// the node names (the Rust client's `post_operator_signed` /
/// `delete_operator_signed`); without one the `428` is raised as it came.
async fn send_signed(
    c: &Client,
    verb: Verb,
    path: &str,
    body: Value,
    key: Option<&OperatorKey>,
) -> Result<Value, Error> {
    use ed25519_dalek::Signer;
    let Some(key) = key else {
        return send_verb(c, verb, path, body).await;
    };
    let sign = |message: &[u8]| key.sign(message).to_bytes().to_vec();
    match verb {
        Verb::Get => c.get(path).await,
        Verb::Post => c.post_operator_signed(path, body, &sign).await,
        Verb::Delete => c.delete_operator_signed(path, &sign).await,
    }
}

impl PyCloudClient {
    /// A session replaced the API key as the client's credential.
    fn forget_api_key(&self) {
        self.nodes.set_api_key(None);
    }

    fn confirmed(
        &self,
        py: Python<'_>,
        path: String,
        body: Value,
        passkey: &PySoftPasskey,
    ) -> PyResult<Value> {
        let pk = Arc::clone(&passkey.inner);
        self.run(py, async |c| {
            let mut auth = pk.lock().await;
            c.post_confirmed(&path, body.clone(), &mut *auth).await
        })
    }

    /// DELETE `path`, answering `412 CONFIRM_REQUIRED` with `passkey`.
    fn confirmed_delete(
        &self,
        py: Python<'_>,
        path: String,
        passkey: &PySoftPasskey,
    ) -> PyResult<Value> {
        let pk = Arc::clone(&passkey.inner);
        self.run(py, async |c| {
            let mut auth = pk.lock().await;
            c.delete_confirmed(&path, &mut *auth).await
        })
    }

    fn send(&self, py: Python<'_>, verb: Verb, path: String, body: Value) -> PyResult<Value> {
        let body = if body.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            body
        };
        self.run(py, async |c| send_verb(&c, verb, &path, body.clone()).await)
    }

    /// The operator key `use_operator_key` set, if any.
    fn operator_key(&self) -> Option<OperatorKey> {
        self.operator_key
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// `send` for an operator decision, signed with the operator key (see
    /// [`send_signed`]).
    fn send_operator(
        &self,
        py: Python<'_>,
        verb: Verb,
        path: String,
        body: Value,
    ) -> PyResult<Value> {
        let body = if body.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            body
        };
        let key = self.operator_key();
        self.run(py, async |c| {
            send_signed(&c, verb, &path, body.clone(), key.as_ref()).await
        })
    }

    /// `send_operator`, following `421 NOT_LEADER` to the leader the node
    /// names (an app group's for replica moves, the system group's for node
    /// removal). The leader is attested under this client's policy and gets
    /// the same API key, and the decision is signed again there; without a
    /// key the redirect is raised as it came.
    fn send_to_leader(
        &self,
        py: Python<'_>,
        verb: Verb,
        path: String,
        body: Value,
    ) -> PyResult<Value> {
        let key = self.nodes.api_key();
        let operator = self.operator_key();
        let (policy, first) = (self.policy.clone(), self.client());
        self.block(py, async move {
            let mut c = first;
            for _ in 0..LEADER_HOPS {
                match send_signed(&c, verb, &path, body.clone(), operator.as_ref()).await {
                    Err(e) if e.code() == Some("NOT_LEADER") => {
                        let leader = e
                            .details()
                            .and_then(|d| d["leaderUrl"].as_str())
                            .map(str::to_owned);
                        let (Some(url), Some(key)) = (leader, key.as_deref()) else {
                            return Err(e);
                        };
                        let next =
                            Client::connect(&url, policy.clone()).map_err(Error::Transport)?;
                        next.use_api_key(key).await;
                        c = Arc::new(next);
                    }
                    done => return done,
                }
            }
            send_signed(&c, verb, &path, body, operator.as_ref()).await
        })
    }
}

impl PyCloudClient {
    fn operator_signed(
        &self,
        py: Python<'_>,
        verb: Verb,
        path: &str,
        body: Value,
        operator_key: Option<&str>,
    ) -> PyResult<PyObject> {
        let key = match operator_key {
            Some(seed) => Some(operator_key_from_hex(seed)?),
            None => self.operator_key(),
        };
        let path = path.trim_start_matches('/').to_owned();
        let body = if body.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            body
        };
        let v = self.run(py, async |c| {
            send_signed(&c, verb, &path, body.clone(), key.as_ref()).await
        })?;
        to_py(py, &v)
    }
}

#[pymethods]
impl PyCloudClient {
    /// Connect to `base_url`. The node is trusted only once its attestation
    /// verifies; `allow_mock` accepts development (mock) quotes, `app_hash`
    /// (hex) pins the bundle the quote must bind.
    ///
    /// `nodes` are other nodes of the same cluster, in the order to move to
    /// them when a node answers `421 WRONG_NODE` (its release may not serve
    /// the account); each is attested under the same policy before anything
    /// is sent to it. Without them that answer is raised as `WrongNode`.
    /// A `401` to a request made with a session is answered by one refresh
    /// (any node takes it) and the request sent again; `LoginRequired` is
    /// raised only when that does not help.
    ///
    /// A real TDX quote is accepted only from an image the policy names:
    /// `tee_profiles` admits nodes whose signed mero-tee release lists their
    /// measurements under one of those profiles (`min_release` is a floor).
    /// Without it, only mock quotes (`allow_mock`) can be accepted. A
    /// release's debug profiles (whose memory the host can read) are refused
    /// unless `allow_debug_profiles`.
    #[new]
    #[pyo3(signature = (base_url, allow_mock = false, app_hash = None, tee_profiles = None, min_release = None, allow_debug_profiles = false, nodes = None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        base_url: &str,
        allow_mock: bool,
        app_hash: Option<&str>,
        tee_profiles: Option<Vec<String>>,
        min_release: Option<String>,
        allow_debug_profiles: bool,
        nodes: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let app_hash = app_hash
            .map(|h| {
                hex::decode(h)
                    .ok()
                    .and_then(|v| <[u8; 32]>::try_from(v).ok())
                    .ok_or_else(|| PyValueError::new_err("app_hash must be 32 bytes of hex"))
            })
            .transpose()?;
        let runtime = Runtime::new().map_err(|e| PyValueError::new_err(e.to_string()))?;
        let policy = Policy {
            allow_mock,
            app_hash,
            measurements: match tee_profiles {
                Some(allowed_profiles) if !allowed_profiles.is_empty() => Measurements::Release {
                    allowed_profiles,
                    min_version: min_release,
                },
                _ => Measurements::None,
            },
            allowed_tcb_statuses: Vec::new(),
            allow_debug_profiles,
        };
        let mut urls: Vec<String> = Vec::new();
        for url in std::iter::once(base_url.to_owned()).chain(nodes.unwrap_or_default()) {
            let url = url.trim_end_matches('/').to_owned();
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
        let clients = {
            let _enter = runtime.enter();
            urls.into_iter()
                .map(|url| {
                    let client = Client::connect(&url, policy.clone())
                        .map_err(|e| PyValueError::new_err(format!("{e:#}")))?;
                    Ok((url, client))
                })
                .collect::<PyResult<Vec<_>>>()?
        };
        Ok(Self {
            nodes: Arc::new(Nodes::new(clients)),
            runtime: Arc::new(runtime),
            policy,
            operator_key: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    /// POST `body` to `path`, answering a `412 CONFIRM_REQUIRED` with
    /// `passkey`'s assertion over the node's challenge: what an owner
    /// approves (upgrades, organization upgrades, release and outbound
    /// approvals, deleting an organization's...).
    #[pyo3(signature = (path, body, passkey))]
    fn post_confirmed(
        &self,
        py: Python<'_>,
        path: &str,
        body: Option<&Bound<'_, PyAny>>,
        passkey: &PySoftPasskey,
    ) -> PyResult<PyObject> {
        let body = to_json(py, body)?;
        let body = if body.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            body
        };
        let v = self.confirmed(py, path.trim_start_matches('/').to_owned(), body, passkey)?;
        to_py(py, &v)
    }

    /// POST any API path (organizations, teams, invites, ...).
    #[pyo3(signature = (path, body = None))]
    fn post(
        &self,
        py: Python<'_>,
        path: &str,
        body: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let (b, path) = (to_json(py, body)?, path.to_owned());
        let v = self.run(py, async |c| c.post(&path, b.clone()).await)?;
        to_py(py, &v)
    }

    /// GET any API path.
    fn get(&self, py: Python<'_>, path: &str) -> PyResult<PyObject> {
        let path = path.to_owned();
        let v = self.run(py, async |c| c.get(&path).await)?;
        to_py(py, &v)
    }

    /// DELETE any API path.
    fn delete(&self, py: Python<'_>, path: &str) -> PyResult<PyObject> {
        let path = path.to_owned();
        let v = self.run(py, async |c| c.delete(&path).await)?;
        to_py(py, &v)
    }

    /// PUT any API path (an alias: `auth/aliases/{alias}` with `{"app": ...}`).
    #[pyo3(signature = (path, body = None))]
    fn put(
        &self,
        py: Python<'_>,
        path: &str,
        body: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let (b, path) = (to_json(py, body)?, path.to_owned());
        let v = self.run(py, async |c| c.put(&path, b.clone()).await)?;
        to_py(py, &v)
    }

    /// DELETE `path`, answering `412 CONFIRM_REQUIRED` (deleting an
    /// organization) with `passkey`, as `post_confirmed` does.
    fn delete_confirmed(
        &self,
        py: Python<'_>,
        path: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<PyObject> {
        let pk = Arc::clone(&passkey.inner);
        let path = path.trim_start_matches('/').to_owned();
        let v = self.run(py, async |c| {
            let mut auth = pk.lock().await;
            c.delete_confirmed(&path, &mut *auth).await
        })?;
        to_py(py, &v)
    }

    /// Sign the Ops API's decisions with the operator's Ed25519 key (its
    /// 32-byte seed, hex; the nodes were started with `--operator-key` set to
    /// its verify key): a `428 OPERATOR_SIGNATURE_REQUIRED` from `ops_*` is
    /// then answered with the signature. `None` forgets it.
    #[pyo3(signature = (seed))]
    fn use_operator_key(&self, seed: Option<&str>) -> PyResult<()> {
        let key = seed.map(operator_key_from_hex).transpose()?;
        *self
            .operator_key
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = key;
        Ok(())
    }

    /// The operator key's verify half (hex), what the nodes take as
    /// `--operator-key`, from its 32-byte seed (hex; `secrets.token_hex(32)`
    /// makes one).
    #[staticmethod]
    fn operator_public_key(seed: &str) -> PyResult<String> {
        Ok(hex::encode(
            operator_key_from_hex(seed)?.verifying_key().to_bytes(),
        ))
    }

    /// POST an operator decision to `path`, answering `428
    /// OPERATOR_SIGNATURE_REQUIRED` with `operator_key` (a seed, hex), else
    /// the key `use_operator_key` set.
    #[pyo3(signature = (path, body = None, operator_key = None))]
    fn post_operator_signed(
        &self,
        py: Python<'_>,
        path: &str,
        body: Option<&Bound<'_, PyAny>>,
        operator_key: Option<&str>,
    ) -> PyResult<PyObject> {
        self.operator_signed(py, Verb::Post, path, to_json(py, body)?, operator_key)
    }

    /// DELETE an operator decision (`ops/nodes/{id}`), signed as
    /// `post_operator_signed`.
    #[pyo3(signature = (path, operator_key = None))]
    fn delete_operator_signed(
        &self,
        py: Python<'_>,
        path: &str,
        operator_key: Option<&str>,
    ) -> PyResult<PyObject> {
        self.operator_signed(py, Verb::Delete, path, Value::Null, operator_key)
    }

    /// GET any API path whose answer is text (`ops/metrics`).
    fn get_text(&self, py: Python<'_>, path: &str) -> PyResult<String> {
        let path = path.to_owned();
        self.run(py, async |c| c.get_text(&path).await)
    }

    /// The URL of the node in use: `base_url`, until a `421 WRONG_NODE`
    /// moved the client to the next of `nodes`.
    #[getter]
    fn node_url(&self) -> String {
        self.nodes.url().to_owned()
    }

    /// The node's attestation document (unverified, for display).
    fn attestation(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.run(py, async |c| c.attestation().await)?;
        to_py(py, &v)
    }
}
