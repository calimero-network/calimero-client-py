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

use prelude::*;

/// A connection to one node of a Calimero 1.0 cluster. Also exported as
/// `Client`, the preferred name.
#[pyclass(name = "CloudClient")]
pub struct PyCloudClient {
    inner: Arc<Client>,
    runtime: Arc<Runtime>,
    /// The trust policy this client was made with, so a request that must
    /// run on a leader (`NOT_LEADER`) is sent to a node attested the same way.
    policy: Policy,
    /// The API key last given to `use_api_key` (the ops token, for the Ops
    /// API), which a request sent on to the leader carries too.
    api_key: std::sync::Mutex<Option<String>>,
}

/// How many `421 NOT_LEADER` redirects an Ops request follows. A node that
/// holds no replica names one that may itself redirect once more.
const LEADER_HOPS: usize = 3;

impl PyCloudClient {
    fn run<T: Send>(
        &self,
        py: Python<'_>,
        f: impl std::future::Future<Output = Result<T, Error>> + Send,
    ) -> PyResult<T> {
        let rt = Arc::clone(&self.runtime);
        py.allow_threads(move || rt.block_on(f).map_err(cloud_err))
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

impl PyCloudClient {
    /// A session replaced the API key as the client's credential.
    fn forget_api_key(&self) {
        if let Ok(mut k) = self.api_key.lock() {
            *k = None;
        }
    }

    fn confirmed(
        &self,
        py: Python<'_>,
        path: String,
        body: Value,
        passkey: &PySoftPasskey,
    ) -> PyResult<Value> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        self.run(py, async move {
            let mut auth = pk.lock().await;
            c.post_confirmed(&path, body, &mut *auth).await
        })
    }

    fn send(&self, py: Python<'_>, verb: Verb, path: String, body: Value) -> PyResult<Value> {
        let c = Arc::clone(&self.inner);
        let body = if body.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            body
        };
        self.run(py, async move { send_verb(&c, verb, &path, body).await })
    }

    /// `send`, following `421 NOT_LEADER` to the leader the node names (an
    /// app group's for replica moves, the system group's for node removal).
    /// The leader is attested under this client's policy and gets the same
    /// API key; without one the redirect is raised as it came.
    fn send_to_leader(
        &self,
        py: Python<'_>,
        verb: Verb,
        path: String,
        body: Value,
    ) -> PyResult<Value> {
        let key = self
            .api_key
            .lock()
            .map_err(|_| PyValueError::new_err("the API key lock is poisoned"))?
            .clone();
        let (policy, first) = (self.policy.clone(), Arc::clone(&self.inner));
        self.run(py, async move {
            let mut c = first;
            for _ in 0..LEADER_HOPS {
                match send_verb(&c, verb, &path, body.clone()).await {
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
            send_verb(&c, verb, &path, body).await
        })
    }
}

#[pymethods]
impl PyCloudClient {
    /// Connect to `base_url`. The node is trusted only once its attestation
    /// verifies; `allow_mock` accepts development (mock) quotes, `app_hash`
    /// (hex) pins the bundle the quote must bind.
    ///
    /// A real TDX quote is accepted only from an image the policy names:
    /// `tee_profiles` admits nodes whose signed mero-tee release lists their
    /// measurements under one of those profiles (`min_release` is a floor).
    /// Without it, only mock quotes (`allow_mock`) can be accepted. A
    /// release's debug profiles (whose memory the host can read) are refused
    /// unless `allow_debug_profiles`.
    #[new]
    #[pyo3(signature = (base_url, allow_mock = false, app_hash = None, tee_profiles = None, min_release = None, allow_debug_profiles = false))]
    fn new(
        base_url: &str,
        allow_mock: bool,
        app_hash: Option<&str>,
        tee_profiles: Option<Vec<String>>,
        min_release: Option<String>,
        allow_debug_profiles: bool,
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
        let inner = {
            let _enter = runtime.enter();
            Client::connect(base_url, policy.clone())
                .map_err(|e| PyValueError::new_err(format!("{e:#}")))?
        };
        Ok(Self {
            inner: Arc::new(inner),
            runtime: Arc::new(runtime),
            policy,
            api_key: std::sync::Mutex::new(None),
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
        let (c, b, path) = (Arc::clone(&self.inner), to_json(py, body)?, path.to_owned());
        let v = self.run(py, async move { c.post(&path, b).await })?;
        to_py(py, &v)
    }

    /// GET any API path.
    fn get(&self, py: Python<'_>, path: &str) -> PyResult<PyObject> {
        let (c, path) = (Arc::clone(&self.inner), path.to_owned());
        let v = self.run(py, async move { c.get(&path).await })?;
        to_py(py, &v)
    }

    /// DELETE any API path.
    fn delete(&self, py: Python<'_>, path: &str) -> PyResult<PyObject> {
        let (c, path) = (Arc::clone(&self.inner), path.to_owned());
        let v = self.run(py, async move { c.delete(&path).await })?;
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
        let (c, b, path) = (Arc::clone(&self.inner), to_json(py, body)?, path.to_owned());
        let v = self.run(py, async move { c.put(&path, b).await })?;
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
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let path = path.trim_start_matches('/').to_owned();
        let v = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.delete_confirmed(&path, &mut *auth).await
        })?;
        to_py(py, &v)
    }

    /// GET any API path whose answer is text (`ops/metrics`).
    fn get_text(&self, py: Python<'_>, path: &str) -> PyResult<String> {
        let (c, path) = (Arc::clone(&self.inner), path.to_owned());
        self.run(py, async move { c.get_text(&path).await })
    }

    /// The node's attestation document (unverified, for display).
    fn attestation(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.attestation().await })?;
        to_py(py, &v)
    }
}
