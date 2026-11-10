//! The runtime the route methods send through: everything of the client that
//! is not a route. Blocking on core's client with the GIL released, moving
//! between nodes ([`super::recover`]), the `412 CONFIRM_REQUIRED` passkey
//! step-up ([`confirm_step_up`]), operator signatures ([`send_signed`]) and the
//! generic methods: `request(method, path, *, query, body, passkey,
//! repeatable)`, the one call a route layer generated from the node's OpenAPI
//! document needs, and `get` / `post` / `put` / `delete` /
//! `post_confirmed` / `delete_confirmed` / `get_text`.

use super::prelude::*;

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
    /// WRONG_NODE` (the next node, repeat) as `Nodes::run` does. The future
    /// runs on this thread (`block_on`), so it need not be `Send`.
    pub(super) fn run<T: Send>(
        &self,
        py: Python<'_>,
        op: impl AsyncFn(Arc<Client>) -> Result<T, Error> + Send,
    ) -> PyResult<T> {
        let (rt, nodes) = (Arc::clone(&self.runtime), Arc::clone(&self.nodes));
        py.allow_threads(move || rt.block_on(nodes.run(op)).map_err(cloud_err))
    }

    /// [`run`](Self::run) for a request that is safe to send twice (a read, a
    /// call with its idempotency key): a node that answers `503` or cannot be
    /// reached is also left for the next, as `Nodes::run_repeatable` does.
    pub(super) fn run_repeatable<T: Send>(
        &self,
        py: Python<'_>,
        op: impl AsyncFn(Arc<Client>) -> Result<T, Error> + Send,
    ) -> PyResult<T> {
        let (rt, nodes) = (Arc::clone(&self.runtime), Arc::clone(&self.nodes));
        py.allow_threads(move || rt.block_on(nodes.run_repeatable(op)).map_err(cloud_err))
    }

    /// [`run`](Self::run) for a sign-in or sign-up, sent once: a node that
    /// answers it `421` moves the client on, and the caller signs in again
    /// with the passkey.
    pub(super) fn run_once<T: Send>(
        &self,
        py: Python<'_>,
        op: impl AsyncFnOnce(Arc<Client>) -> Result<T, Error> + Send,
    ) -> PyResult<T> {
        let (rt, nodes) = (Arc::clone(&self.runtime), Arc::clone(&self.nodes));
        py.allow_threads(move || rt.block_on(nodes.run_once(op)).map_err(cloud_err))
    }

    /// Block on `f` with the GIL released, as it is: no recovery.
    pub(super) fn block<T: Send>(
        &self,
        py: Python<'_>,
        f: impl std::future::Future<Output = Result<T, Error>> + Send,
    ) -> PyResult<T> {
        let rt = Arc::clone(&self.runtime);
        py.allow_threads(move || rt.block_on(f).map_err(cloud_err))
    }

    /// The node in use.
    pub(super) fn client(&self) -> Arc<Client> {
        self.nodes.current().1
    }
}

/// Send `body` once; a `412 CONFIRM_REQUIRED` is answered with `passkey`'s
/// assertion over the node's challenge, added to `body` as `confirm:
/// {challengeId, assertion}`, and the request is sent again. What core's
/// `post_confirmed` / `delete_confirmed` do, for a request core has no
/// confirmed form of (a DELETE whose body names what goes).
pub(crate) async fn confirm_step_up(
    body: Value,
    passkey: &mut dyn Passkey,
    send: impl AsyncFn(Value) -> Result<Value, Error>,
) -> Result<Value, Error> {
    let details = match send(body.clone()).await {
        Err(e) if e.code() == Some("CONFIRM_REQUIRED") => {
            e.details().cloned().unwrap_or(Value::Null)
        }
        other => return other,
    };
    let challenge =
        calimero_identity::from_b64url(details["challenge"].as_str().unwrap_or_default())
            .map_err(|e| Error::Transport(eyre::eyre!("the node's challenge: {e}")))?;
    let assertion = passkey.get(&challenge);
    let mut body = body;
    if let Some(b) = body.as_object_mut() {
        let _prev = b.insert(
            "confirm".into(),
            serde_json::json!({ "challengeId": details["challengeId"], "assertion": assertion }),
        );
    }
    send(body).await
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
    pub(super) fn forget_api_key(&self) {
        self.nodes.set_api_key(None);
    }

    pub(super) fn confirmed(
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
    pub(super) fn confirmed_delete(
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

    /// DELETE `path` with `body` as JSON, answering `412 CONFIRM_REQUIRED`
    /// with `passkey` (the node reads what is removed from the body: a
    /// package's web origin). Core's `delete_confirmed` sends `{}`, so the
    /// step-up is made here, as [`confirm_step_up`] does it.
    pub(super) fn confirmed_delete_with_body(
        &self,
        py: Python<'_>,
        path: String,
        body: Value,
        passkey: &PySoftPasskey,
    ) -> PyResult<Value> {
        let pk = Arc::clone(&passkey.inner);
        self.run(py, async |c| {
            let mut auth = pk.lock().await;
            confirm_step_up(body.clone(), &mut *auth, async |b| {
                c.delete_with_body(&path, b).await
            })
            .await
        })
    }

    pub(super) fn send(
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
        let op = async |c: Arc<Client>| send_verb(&c, verb, &path, body.clone()).await;
        match verb {
            Verb::Get => self.run_repeatable(py, op),
            Verb::Post | Verb::Delete => self.run(py, op),
        }
    }

    /// The operator key `use_operator_key` set, if any.
    pub(super) fn operator_key(&self) -> Option<OperatorKey> {
        self.operator_key
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// `send` for an operator decision, signed with the operator key (see
    /// [`send_signed`]).
    pub(super) fn send_operator(
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
    pub(super) fn send_to_leader(
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
    pub(super) fn operator_signed(
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

/// The trust policy every node of the client is attested under.
pub(super) fn policy(
    allow_mock: bool,
    app_hash: Option<&str>,
    tee_profiles: Option<Vec<String>>,
    min_release: Option<String>,
    allow_debug_profiles: bool,
) -> PyResult<Policy> {
    let app_hash = app_hash
        .map(|h| {
            hex::decode(h)
                .ok()
                .and_then(|v| <[u8; 32]>::try_from(v).ok())
                .ok_or_else(|| PyValueError::new_err("app_hash must be 32 bytes of hex"))
        })
        .transpose()?;
    Ok(Policy {
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
    })
}

/// A client for each of `urls` (each once, in order), attested under `policy`
/// before anything is sent to it.
pub(super) fn connect_all(
    runtime: &Runtime,
    urls: impl IntoIterator<Item = String>,
    policy: &Policy,
) -> PyResult<Vec<(String, Client)>> {
    let mut unique: Vec<String> = Vec::new();
    for url in urls {
        let url = url.trim_end_matches('/').to_owned();
        if !unique.contains(&url) {
            unique.push(url);
        }
    }
    let _enter = runtime.enter();
    unique
        .into_iter()
        .map(|url| {
            let client = Client::connect(&url, policy.clone())
                .map_err(|e| PyValueError::new_err(format!("{e:#}")))?;
            Ok((url, client))
        })
        .collect()
}

#[pymethods]
impl PyCloudClient {
    /// Send `method path` through the runtime and answer the node's JSON: the
    /// one call a route layer needs (`calimero.transport.Transport`), so one
    /// generated from the node's OpenAPI document sends through the same
    /// attested, sealed channel, credential and recovery as every method here.
    ///
    /// `method` is `GET`, `POST`, `PUT` or `DELETE`. `path` is relative to the
    /// node (a leading `/` is dropped; `calimero.transport.route_path` fills a
    /// template with encoded segments). `query` (a dict; `None` values are left
    /// out, a list repeats its name) is appended, encoded. `body` goes as JSON
    /// (`{}` for a POST or PUT without one). `passkey` answers a `412
    /// CONFIRM_REQUIRED` (POST or DELETE) with its assertion over the node's
    /// challenge, and the same body is sent again with `confirm`. `repeatable`
    /// says whether the request may be sent again on a `503` or to the next
    /// node (default: only for a GET); give it for a call that carries its
    /// own idempotency key. A refusal is raised as `CloudError` (`LoginRequired`,
    /// `WrongNode`), as from any method.
    #[pyo3(signature = (method, path, *, query = None, body = None, passkey = None, repeatable = None))]
    #[allow(clippy::too_many_arguments)]
    fn request(
        &self,
        py: Python<'_>,
        method: &str,
        path: &str,
        query: Option<&Bound<'_, pyo3::types::PyDict>>,
        body: Option<&Bound<'_, PyAny>>,
        passkey: Option<&PySoftPasskey>,
        repeatable: Option<bool>,
    ) -> PyResult<PyObject> {
        let method = method.to_ascii_uppercase();
        let path = with_query(path.trim_start_matches('/'), query)?;
        let body = match body {
            Some(b) if !b.is_none() => Some(to_json(py, Some(b))?),
            _ => None,
        };
        let repeatable = repeatable.unwrap_or(method == "GET");
        let v = match (method.as_str(), passkey) {
            ("GET", None) if repeatable => self.run_repeatable(py, async |c| c.get(&path).await)?,
            ("GET", None) => self.run(py, async |c| c.get(&path).await)?,
            ("POST" | "PUT", None) => {
                let body = body.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
                let put = method == "PUT";
                let op = async |c: Arc<Client>| {
                    if put {
                        c.put(&path, body.clone()).await
                    } else {
                        c.post(&path, body.clone()).await
                    }
                };
                if repeatable {
                    self.run_repeatable(py, op)?
                } else {
                    self.run(py, op)?
                }
            }
            ("DELETE", None) => self.run(py, async |c| match &body {
                None => c.delete(&path).await,
                Some(b) => c.delete_with_body(&path, b.clone()).await,
            })?,
            ("POST", Some(passkey)) => {
                let body = body.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
                self.confirmed(py, path, body, passkey)?
            }
            ("DELETE", Some(passkey)) => match body {
                Some(body) => self.confirmed_delete_with_body(py, path, body, passkey)?,
                None => self.confirmed_delete(py, path, passkey)?,
            },
            (other, Some(_)) => {
                return Err(PyValueError::new_err(format!(
                    "a passkey answers a POST or a DELETE, not a {other}"
                )))
            }
            (other, None) => {
                return Err(PyValueError::new_err(format!(
                    "method must be GET, POST, PUT or DELETE, not {other}"
                )))
            }
        };
        to_py(py, &v)
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
        let v = self.run_repeatable(py, async |c| c.get(&path).await)?;
        to_py(py, &v)
    }

    /// DELETE any API path; `body`, when given, goes as JSON (a proof the
    /// caller made itself: `{"confirm": ...}` for a passkey, a member or a
    /// team row, `{"operatorProof": ...}` for `ops/nodes/{id}`). Without one
    /// the DELETE carries no body, as before.
    #[pyo3(signature = (path, body = None))]
    fn delete(
        &self,
        py: Python<'_>,
        path: &str,
        body: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let path = path.to_owned();
        let body = match body {
            Some(b) if !b.is_none() => Some(to_json(py, Some(b))?),
            _ => None,
        };
        let v = self.run(py, async |c| match &body {
            None => c.delete(&path).await,
            Some(b) => c.delete_with_body(&path, b.clone()).await,
        })?;
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
    /// organization) with `passkey`, as `post_confirmed` does. `body`, when
    /// given, goes as JSON, and the approval is added to it (`{"origin": ...}`
    /// on `package-origins/{package}`).
    #[pyo3(signature = (path, passkey, body = None))]
    fn delete_confirmed(
        &self,
        py: Python<'_>,
        path: &str,
        passkey: &PySoftPasskey,
        body: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let path = path.trim_start_matches('/').to_owned();
        let v = match body {
            Some(b) if !b.is_none() => {
                let body = to_json(py, Some(b))?;
                self.confirmed_delete_with_body(py, path, body, passkey)?
            }
            _ => self.confirmed_delete(py, path, passkey)?,
        };
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
        self.run_repeatable(py, async |c| c.get_text(&path).await)
    }

    /// The URL of the node in use: `base_url`, until a `421 WRONG_NODE` or
    /// an unavailable node moved the client to the next of `nodes`.
    #[getter]
    fn node_url(&self) -> String {
        self.nodes.url()
    }

    /// Every node this client may use, in the order it moves through them
    /// (the node in use among them).
    #[getter]
    fn node_urls(&self) -> Vec<String> {
        self.nodes.urls()
    }

    /// The node's attestation document (unverified, for display).
    fn attestation(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.run_repeatable(py, async |c| c.attestation().await)?;
        to_py(py, &v)
    }
}

/// `path` with `query` appended, encoded: `None` values are left out, a list
/// repeats its name, anything else is its `str()`.
pub(crate) fn with_query(
    path: &str,
    query: Option<&Bound<'_, pyo3::types::PyDict>>,
) -> PyResult<String> {
    let Some(query) = query else {
        return Ok(path.to_owned());
    };
    let mut pairs = url::form_urlencoded::Serializer::new(String::new());
    let mut any = false;
    for (name, value) in query.iter() {
        let name: String = name.str()?.extract()?;
        let values: Vec<Bound<'_, PyAny>> = match value.downcast::<pyo3::types::PyList>() {
            Ok(list) => list.iter().collect(),
            Err(_) => vec![value],
        };
        for one in values.into_iter().filter(|v| !v.is_none()) {
            let one: String = match one.downcast::<pyo3::types::PyBool>() {
                Ok(b) => (if b.is_true() { "true" } else { "false" }).to_owned(),
                Err(_) => one.str()?.extract()?,
            };
            let _ = pairs.append_pair(&name, &one);
            any = true;
        }
    }
    if !any {
        return Ok(path.to_owned());
    }
    let sep = if path.contains('?') { '&' } else { '?' };
    Ok(format!("{path}{sep}{}", pairs.finish()))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use calimero_identity::testing::SoftAuthenticator;
    use serde_json::json;

    use super::*;

    fn confirm_required(challenge_id: &str, challenge: &[u8]) -> Error {
        Error::Api {
            status: 412,
            code: "CONFIRM_REQUIRED".into(),
            message: "confirm with a passkey".into(),
            details: json!({
                "challengeId": challenge_id,
                "challenge": calimero_identity::b64url(challenge),
                "rpId": "localhost",
                "allowCredentials": [],
            }),
        }
    }

    #[tokio::test]
    async fn a_confirm_required_is_answered_once_with_the_body_kept() {
        let mut pk = SoftAuthenticator::new("localhost", "http://localhost:3000");
        let sent = RefCell::new(Vec::new());
        let v = confirm_step_up(
            json!({ "origin": "https://app.example" }),
            &mut pk,
            async |b| {
                sent.borrow_mut().push(b.clone());
                if b.get("confirm").is_some() {
                    Ok(json!({ "removed": true }))
                } else {
                    Err(confirm_required("ch-1", b"challenge bytes"))
                }
            },
        )
        .await
        .unwrap();
        assert_eq!(v, json!({ "removed": true }));
        let sent = sent.into_inner();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0], json!({ "origin": "https://app.example" }));
        assert_eq!(sent[1]["origin"], "https://app.example");
        assert_eq!(sent[1]["confirm"]["challengeId"], "ch-1");
        // The assertion signs the node's challenge, as core's step-up does.
        let expected = serde_json::to_value(pk.assert(b"challenge bytes")).unwrap();
        assert_eq!(
            sent[1]["confirm"]["assertion"]["clientDataJson"],
            expected["clientDataJson"]
        );
    }

    #[tokio::test]
    async fn any_other_answer_is_returned_as_it_came() {
        let mut pk = SoftAuthenticator::new("localhost", "http://localhost:3000");
        let calls = RefCell::new(0);
        let err = confirm_step_up(json!({ "origin": "x" }), &mut pk, async |_b| {
            *calls.borrow_mut() += 1;
            Err::<Value, _>(Error::Api {
                status: 404,
                code: "NOT_FOUND".into(),
                message: "no such origin".into(),
                details: Value::Null,
            })
        })
        .await
        .unwrap_err();
        assert_eq!(err.code(), Some("NOT_FOUND"));
        assert_eq!(*calls.borrow(), 1);
    }
}
