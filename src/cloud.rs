//! Python bindings for the release-v2 cloud client (`calimero-cloud-client`).
//!
//! Attestation, the sealed channel and passkey handling all run in the Rust
//! client, so Python never re-implements the trust check. Every method blocks
//! with the GIL released.

use std::sync::Arc;
use std::time::Duration;

use calimero_cloud_client::{Client, Consistency, Error, Measurements, Policy, Session};
use calimero_identity::testing::SoftAuthenticator;
use futures_util::StreamExt;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde_json::Value;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

// pyo3 0.22's macro checks a `gil-refs` feature this crate does not declare.
#[allow(unexpected_cfgs)]
mod exception {
    use pyo3::create_exception;
    use pyo3::exceptions::PyException;

    create_exception!(
        calimero_client_py,
        CloudError,
        PyException,
        "An error from a cloud node: args are (message, status, code, details); status is 0 when the node could not be reached or attested."
    );
}
pub use exception::CloudError;

fn cloud_err(e: Error) -> PyErr {
    match e {
        Error::Api {
            status,
            code,
            message,
            details,
        } => CloudError::new_err((
            format!("{status} {code}: {message}"),
            status,
            code,
            details.to_string(),
        )),
        Error::Transport(r) => CloudError::new_err((format!("{r:#}"), 0_u16, "TRANSPORT", "null")),
    }
}

/// Python object -> JSON, through Python's own `json` module.
fn to_json(py: Python<'_>, obj: Option<&Bound<'_, PyAny>>) -> PyResult<Value> {
    let Some(obj) = obj else {
        return Ok(Value::Object(serde_json::Map::new()));
    };
    if obj.is_none() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    let s: String = py
        .import_bound("json")?
        .call_method1("dumps", (obj,))?
        .extract()?;
    serde_json::from_str(&s).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// JSON -> Python object.
fn to_py(py: Python<'_>, v: &impl serde::Serialize) -> PyResult<PyObject> {
    let s = serde_json::to_string(v).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(py
        .import_bound("json")?
        .call_method1("loads", (s,))?
        .unbind())
}

/// An in-memory ES256 passkey, for tests and scripts against development
/// clusters. Real users sign in from a browser; services use API keys.
#[pyclass(name = "SoftPasskey")]
pub struct PySoftPasskey {
    inner: Arc<Mutex<SoftAuthenticator>>,
}

#[pymethods]
impl PySoftPasskey {
    #[new]
    #[pyo3(signature = (rp_id = "localhost", origin = "http://localhost:3000"))]
    fn new(rp_id: &str, origin: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SoftAuthenticator::new(rp_id, origin))),
        }
    }

    #[getter]
    fn credential_id(&self) -> String {
        self.inner.blocking_lock().credential_id()
    }
}

/// A connection to one node of a Calimero cloud cluster.
#[pyclass(name = "CloudClient")]
pub struct PyCloudClient {
    inner: Arc<Client>,
    runtime: Arc<Runtime>,
}

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

#[pymethods]
impl PyCloudClient {
    /// Connect to `base_url`. The node is trusted only once its attestation
    /// verifies; `allow_mock` accepts development (mock) quotes, `app_hash`
    /// (hex) pins the bundle the quote must bind.
    ///
    /// A real TDX quote is accepted only from an image the policy names:
    /// `tee_profiles` admits nodes whose signed mero-tee release lists their
    /// measurements under one of those profiles (`min_release` is a floor).
    /// Without it, only mock quotes (`allow_mock`) can be accepted.
    #[new]
    #[pyo3(signature = (base_url, allow_mock = false, app_hash = None, tee_profiles = None, min_release = None))]
    fn new(
        base_url: &str,
        allow_mock: bool,
        app_hash: Option<&str>,
        tee_profiles: Option<Vec<String>>,
        min_release: Option<String>,
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
        let inner = {
            let _enter = runtime.enter();
            Client::connect(
                base_url,
                Policy {
                    allow_mock,
                    app_hash,
                    measurements: match tee_profiles {
                        Some(allowed_profiles) if !allowed_profiles.is_empty() => {
                            Measurements::Release {
                                allowed_profiles,
                                min_version: min_release,
                            }
                        }
                        _ => Measurements::None,
                    },
                    allowed_tcb_statuses: Vec::new(),
                },
            )
            .map_err(|e| PyValueError::new_err(format!("{e:#}")))?
        };
        Ok(Self {
            inner: Arc::new(inner),
            runtime: Arc::new(runtime),
        })
    }

    /// Create an account with a new passkey and sign in. Returns the session.
    fn sign_up(&self, py: Python<'_>, passkey: &PySoftPasskey, name: &str) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let name = name.to_owned();
        let s = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.sign_up(&mut *auth, &name).await
        })?;
        to_py(py, &s)
    }

    /// Sign in with an existing passkey. Returns the session.
    fn sign_in(&self, py: Python<'_>, passkey: &PySoftPasskey) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let s = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.sign_in(&mut *auth).await
        })?;
        to_py(py, &s)
    }

    /// Exchange the refresh token for a new session.
    fn refresh(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let s = self.run(py, async move { c.refresh().await })?;
        to_py(py, &s)
    }

    /// Authenticate every request with an API key (`cal_...`).
    fn use_api_key(&self, py: Python<'_>, key: &str) -> PyResult<()> {
        let (c, key) = (Arc::clone(&self.inner), key.to_owned());
        self.run(py, async move {
            c.use_api_key(&key).await;
            Ok(())
        })
    }

    /// Authenticate with a session returned by `sign_up`/`sign_in`.
    fn use_session(&self, py: Python<'_>, session: &Bound<'_, PyAny>) -> PyResult<()> {
        let s: Session = serde_json::from_value(to_json(py, Some(session))?)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let c = Arc::clone(&self.inner);
        self.run(py, async move {
            c.use_session(s).await;
            Ok(())
        })
    }

    fn me(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.me().await })?;
        to_py(py, &v)
    }

    /// Create an API key. Returns `(key_id, key)`; the key is shown once.
    /// `options` may set `name`, `apps`, `org` and `expiresAt`.
    #[pyo3(signature = (options = None))]
    fn create_api_key(
        &self,
        py: Python<'_>,
        options: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<(String, String)> {
        let (c, o) = (Arc::clone(&self.inner), to_json(py, options)?);
        self.run(py, async move { c.create_api_key(o).await })
    }

    /// Upload a WASM bundle. Returns its hash.
    fn upload_bundle(&self, py: Python<'_>, wasm: Vec<u8>) -> PyResult<String> {
        let c = Arc::clone(&self.inner);
        self.run(py, async move { c.upload_bundle(wasm).await })
    }

    /// Create an app from a bundle. `options` may set `org`, `access`, `confirmMethods`.
    #[pyo3(signature = (name, bundle, options = None))]
    fn create_app(
        &self,
        py: Python<'_>,
        name: &str,
        bundle: &str,
        options: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<String> {
        let (c, o) = (Arc::clone(&self.inner), to_json(py, options)?);
        let (name, bundle) = (name.to_owned(), bundle.to_owned());
        self.run(py, async move { c.create_app(&name, &bundle, o).await })
    }

    /// Call a method. Returns `{"index", "result", "events"}`.
    #[pyo3(signature = (app, method, args = None, idempotency_key = None))]
    fn call(
        &self,
        py: Python<'_>,
        app: &str,
        method: &str,
        args: Option<&Bound<'_, PyAny>>,
        idempotency_key: Option<String>,
    ) -> PyResult<PyObject> {
        let (c, a) = (Arc::clone(&self.inner), to_json(py, args)?);
        let (app, method) = (app.to_owned(), method.to_owned());
        let r = self.run(py, async move {
            c.call(&app, &method, a, idempotency_key.as_deref()).await
        })?;
        to_py(py, &r)
    }

    /// Call a `confirm` method; `passkey` approves this exact call.
    #[pyo3(signature = (app, method, args, passkey))]
    fn call_confirmed(
        &self,
        py: Python<'_>,
        app: &str,
        method: &str,
        args: Option<&Bound<'_, PyAny>>,
        passkey: &PySoftPasskey,
    ) -> PyResult<PyObject> {
        let (c, a, pk) = (
            Arc::clone(&self.inner),
            to_json(py, args)?,
            Arc::clone(&passkey.inner),
        );
        let (app, method) = (app.to_owned(), method.to_owned());
        let r = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.call_confirmed(&app, &method, a, &mut *auth).await
        })?;
        to_py(py, &r)
    }

    /// Run a read-only method. Strong by default; `stale=True` reads any
    /// replica at least as new as `min_index`.
    #[pyo3(signature = (app, method, args = None, stale = false, min_index = 0))]
    fn view(
        &self,
        py: Python<'_>,
        app: &str,
        method: &str,
        args: Option<&Bound<'_, PyAny>>,
        stale: bool,
        min_index: u64,
    ) -> PyResult<PyObject> {
        let (c, a) = (Arc::clone(&self.inner), to_json(py, args)?);
        let (app, method) = (app.to_owned(), method.to_owned());
        let consistency = if stale {
            Consistency::Stale { min_index }
        } else {
            Consistency::Strong
        };
        let r = self.run(
            py,
            async move { c.view(&app, &method, a, consistency).await },
        )?;
        to_py(py, &r)
    }

    /// Committed events from index `start`: up to `limit` of them, or as many
    /// as arrive within `timeout` seconds. Each is `{"id", "kind", "data"}`.
    #[pyo3(signature = (app, start = 0, limit = 100, timeout = 5.0))]
    fn events(
        &self,
        py: Python<'_>,
        app: &str,
        start: u64,
        limit: usize,
        timeout: f64,
    ) -> PyResult<PyObject> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        let wait = Duration::from_secs_f64(timeout.max(0.0));
        let out = self.run(py, async move {
            let stream = c.events(&app, start).await?;
            let mut stream = Box::pin(stream.take(limit));
            let mut out = Vec::new();
            let _elapsed = tokio::time::timeout(wait, async {
                while let Some((id, kind, data)) = stream.next().await {
                    out.push(serde_json::json!({ "id": id, "kind": kind, "data": data }));
                }
            })
            .await;
            Ok(out)
        })?;
        to_py(py, &out)
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

    /// The node's attestation document (unverified, for display).
    fn attestation(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.attestation().await })?;
        to_py(py, &v)
    }
}
