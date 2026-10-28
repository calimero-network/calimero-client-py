//! Python bindings for core's Calimero 1.0 client (`calimero-client`).
//!
//! Attestation, the sealed channel and passkey handling all run in the Rust
//! client, so Python never re-implements the trust check. Every method blocks
//! with the GIL released.

use std::sync::Arc;
use std::time::Duration;

use calimero_client::{Client, Consistency, Error, Measurements, Policy, Session};
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

/// Metadata to set: `None` clears it (JSON `null`), anything else as given.
fn metadata_json(py: Python<'_>, obj: Option<&Bound<'_, PyAny>>) -> PyResult<Value> {
    match obj {
        Some(o) if !o.is_none() => to_json(py, Some(o)),
        _ => Ok(Value::Null),
    }
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
enum Verb {
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

/// One path segment, percent-encoded, so an id can never reach another route.
fn seg(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}

fn field(v: &Value, name: &str) -> PyResult<String> {
    v[name]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| PyValueError::new_err(format!("the node's answer has no {name}")))
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

    /// Create an account with a new passkey and sign in. Returns the session.
    fn sign_up(&self, py: Python<'_>, passkey: &PySoftPasskey, name: &str) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let name = name.to_owned();
        let s = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.sign_up(&mut *auth, &name).await
        })?;
        self.forget_api_key();
        to_py(py, &s)
    }

    /// Sign in with an existing passkey. Returns the session.
    fn sign_in(&self, py: Python<'_>, passkey: &PySoftPasskey) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let s = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.sign_in(&mut *auth).await
        })?;
        self.forget_api_key();
        to_py(py, &s)
    }

    /// Add `passkey` (another device) to the signed-in account. Returns its
    /// credential id.
    fn add_passkey(&self, py: Python<'_>, passkey: &PySoftPasskey) -> PyResult<String> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        self.run(py, async move {
            let mut auth = pk.lock().await;
            c.add_passkey(&mut *auth).await
        })
    }

    /// Exchange the refresh token for a new session.
    fn refresh(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let s = self.run(py, async move { c.refresh().await })?;
        to_py(py, &s)
    }

    /// Authenticate every request with an API key (`cal_...`).
    fn use_api_key(&self, py: Python<'_>, key: &str) -> PyResult<()> {
        *self
            .api_key
            .lock()
            .map_err(|_| PyValueError::new_err("the API key lock is poisoned"))? =
            Some(key.to_owned());
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
        })?;
        self.forget_api_key();
        Ok(())
    }

    /// The session in use (`sign_up`, `sign_in`, `refresh`, `use_session`),
    /// or `None` (none yet, or an API key instead).
    fn session(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let s = self.run(py, async move { Ok(c.session().await) })?;
        to_py(py, &s)
    }

    fn me(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.me().await })?;
        to_py(py, &v)
    }

    /// Create an API key. Returns `(key_id, key)`; the key is shown once.
    /// `options` may set `label`, `apps` and `methods` (scopes), `readOnly`,
    /// `allowConfirm` and `ttlSecs` (at most a year).
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

    /// Upload a blob for `app`. Returns `{"blobId", "id", "size"}` (`blobId`
    /// base58, as apps print it; `id` hex).
    fn upload_blob(&self, py: Python<'_>, app: &str, data: Vec<u8>) -> PyResult<PyObject> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        let r = self.run(py, async move { c.upload_blob(&app, data).await })?;
        to_py(py, &r)
    }

    /// One of `app`'s blobs, by id (base58 or hex).
    fn download_blob(&self, py: Python<'_>, app: &str, id: &str) -> PyResult<PyObject> {
        let (c, app, id) = (Arc::clone(&self.inner), app.to_owned(), id.to_owned());
        let bytes = self.run(py, async move { c.download_blob(&app, &id).await })?;
        Ok(pyo3::types::PyBytes::new_bound(py, &bytes)
            .into_any()
            .unbind())
    }

    /// `app`'s blobs: `{"blobs": [{"blobId", "id", "size"}, ...]}`.
    fn list_blobs(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        let v = self.run(py, async move { c.list_blobs(&app).await })?;
        to_py(py, &v)
    }

    /// One blob's id and size.
    fn blob_info(&self, py: Python<'_>, app: &str, id: &str) -> PyResult<PyObject> {
        let (c, app, id) = (Arc::clone(&self.inner), app.to_owned(), id.to_owned());
        let v = self.run(py, async move { c.blob_info(&app, &id).await })?;
        to_py(py, &v)
    }

    /// Delete one of `app`'s blobs (its owner's or an org admin's).
    fn delete_blob(&self, py: Python<'_>, app: &str, id: &str) -> PyResult<PyObject> {
        let (c, app, id) = (Arc::clone(&self.inner), app.to_owned(), id.to_owned());
        let v = self.run(py, async move { c.delete_blob(&app, &id).await })?;
        to_py(py, &v)
    }

    /// Hand `app` to the account `to` (an org app only to one of its members),
    /// approved with the owner's `passkey`.
    fn transfer_app(
        &self,
        py: Python<'_>,
        app: &str,
        to: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let (app, to) = (app.to_owned(), to.to_owned());
        let v = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.transfer_app(&app, &to, &mut *auth).await
        })?;
        to_py(py, &v)
    }

    /// Publish this caller's presence in `app`: `state` (bytes, at most 4 KiB),
    /// or `None` to retract it. The newest `seq` wins.
    #[pyo3(signature = (app, seq, state = None))]
    fn publish_presence(
        &self,
        py: Python<'_>,
        app: &str,
        seq: u64,
        state: Option<Vec<u8>>,
    ) -> PyResult<PyObject> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        let v = self.run(py, async move {
            c.publish_presence(&app, seq, state.as_deref()).await
        })?;
        to_py(py, &v)
    }

    /// Who is present in `app` now.
    fn presence(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        let v = self.run(py, async move { c.presence(&app).await })?;
        to_py(py, &v)
    }

    /// Approve the mero-tee release `version` for `org`'s apps (or, without
    /// `org`, the account's own), with the owner's `passkey`.
    #[pyo3(signature = (version, passkey, org = None))]
    fn approve_release(
        &self,
        py: Python<'_>,
        version: &str,
        passkey: &PySoftPasskey,
        org: Option<String>,
    ) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let version = version.to_owned();
        let v = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.approve_release(org.as_deref(), &version, &mut *auth)
                .await
        })?;
        to_py(py, &v)
    }

    /// Approve URL prefixes `package`'s publisher may call out to, for `org`'s
    /// apps (or the account's own): every prefix its manifest declares when
    /// `prefixes` is `None`; `replace=True` sets exactly these (`[]` revokes).
    #[pyo3(signature = (package, passkey, org = None, prefixes = None, replace = false))]
    fn approve_outbound(
        &self,
        py: Python<'_>,
        package: &str,
        passkey: &PySoftPasskey,
        org: Option<String>,
        prefixes: Option<Vec<String>>,
        replace: bool,
    ) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let package = package.to_owned();
        let v = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.approve_outbound(
                org.as_deref(),
                &package,
                prefixes.as_deref(),
                replace,
                &mut *auth,
            )
            .await
        })?;
        to_py(py, &v)
    }

    /// Upgrade every app of `org` running `package` to `version` (optionally
    /// only `team`, with `cascade` its subtree; `migrate` names the migration
    /// export), approved with the owner's `passkey`. Returns the run
    /// (`{"runId", ...}`).
    #[pyo3(signature = (org, package, version, passkey, team = None, cascade = false, migrate = None))]
    #[allow(clippy::too_many_arguments)]
    fn upgrade_org(
        &self,
        py: Python<'_>,
        org: &str,
        package: &str,
        version: &str,
        passkey: &PySoftPasskey,
        team: Option<String>,
        cascade: bool,
        migrate: Option<String>,
    ) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let org = org.to_owned();
        let body = serde_json::json!({
            "package": package, "version": version, "team": team, "cascade": cascade,
            "migrate": migrate,
        });
        let v = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.upgrade_org(&org, body, &mut *auth).await
        })?;
        to_py(py, &v)
    }

    /// An upgrade run's status and per-app results.
    fn upgrade_run(&self, py: Python<'_>, org: &str, run: &str) -> PyResult<PyObject> {
        let (c, org, run) = (Arc::clone(&self.inner), org.to_owned(), run.to_owned());
        let v = self.run(py, async move { c.upgrade_run(&org, &run).await })?;
        to_py(py, &v)
    }

    /// Run the failed apps of an upgrade run again, as a new run.
    fn retry_upgrade_run(
        &self,
        py: Python<'_>,
        org: &str,
        run: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let (org, run) = (org.to_owned(), run.to_owned());
        let v = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.retry_upgrade_run(&org, &run, &mut *auth).await
        })?;
        to_py(py, &v)
    }

    /// Create an app from a package release (the newest without `version`),
    /// installed from the registry first if the cluster lacks it. `options`
    /// as `create_app`. Returns the app id.
    #[pyo3(signature = (name, package, version = None, service = None, options = None))]
    fn create_app_from_package(
        &self,
        py: Python<'_>,
        name: &str,
        package: &str,
        version: Option<String>,
        service: Option<String>,
        options: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<String> {
        let (c, o) = (Arc::clone(&self.inner), to_json(py, options)?);
        let (name, package) = (name.to_owned(), package.to_owned());
        self.run(py, async move {
            c.create_app_from_package(&name, &package, version.as_deref(), service.as_deref(), o)
                .await
        })
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

    /// Call a method. Returns `{"index", "result", "events"}`. While the node
    /// answers 503 or cannot be reached, the call is sent again with the same
    /// idempotency key (`idempotency_key`, else one the client makes), so it
    /// runs once.
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

    /// End this session on the node (its refresh token stops working).
    fn sign_out(&self, py: Python<'_>) -> PyResult<()> {
        self.send(py, Verb::Post, "auth/logout".into(), Value::Null)
            .map(drop)
    }

    /// End every session of this account, on every device.
    fn sign_out_everywhere(&self, py: Python<'_>) -> PyResult<()> {
        let c = Arc::clone(&self.inner);
        self.run(py, async move { c.sign_out_everywhere().await })
            .map(drop)
    }

    /// Remove a passkey (a device) from the signed-in account.
    fn remove_passkey(&self, py: Python<'_>, credential_id: &str) -> PyResult<()> {
        self.send(
            py,
            Verb::Delete,
            format!("auth/passkeys/{}", seg(credential_id)),
            Value::Null,
        )
        .map(drop)
    }

    /// The account's API keys (their records; the keys themselves are never shown again).
    fn list_api_keys(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, "auth/apikeys".into(), Value::Null)?;
        to_py(py, &v["keys"])
    }

    /// Revoke an API key by its id.
    fn revoke_api_key(&self, py: Python<'_>, key_id: &str) -> PyResult<()> {
        self.send(
            py,
            Verb::Delete,
            format!("auth/apikeys/{}", seg(key_id)),
            Value::Null,
        )
        .map(drop)
    }

    // ------------------------------------------------------------ apps

    /// The apps the caller can reach: `[{"appId", "name", "owner", "org", "bundle"}]`.
    fn list_apps(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, "apps".into(), Value::Null)?;
        to_py(py, &v["apps"])
    }

    /// One app: its directory record and state (`ready`, `initError`).
    fn get_app(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, format!("apps/{}", seg(app)), Value::Null)?;
        to_py(py, &v)
    }

    /// Who may reach an app: `{"kind": "owner" | "org" | "public"}`,
    /// `{"kind": "accounts", "accounts": [...]}` or `{"kind": "team", "team": id}`.
    fn set_access(&self, py: Python<'_>, app: &str, access: &Bound<'_, PyAny>) -> PyResult<()> {
        let body = serde_json::json!({ "access": to_json(py, Some(access))? });
        self.send(py, Verb::Post, format!("apps/{}/access", seg(app)), body)
            .map(drop)
    }

    /// Run `app` on another uploaded bundle (or `version` of its package).
    /// The node plans the migration from the two bundles' ABIs (`migrate`
    /// names an export to use instead). Where the app's owner approves
    /// upgrades, `passkey` is theirs: it answers the node's challenge over
    /// this exact upgrade. `package` and `service` move it to another package
    /// or service. A failed migration raises `CloudError` with code
    /// `UPGRADE_FAILED` (422).
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (app, bundle = None, migrate = None, version = None, passkey = None, package = None, service = None))]
    fn upgrade_app(
        &self,
        py: Python<'_>,
        app: &str,
        bundle: Option<&str>,
        migrate: Option<&str>,
        version: Option<&str>,
        passkey: Option<&PySoftPasskey>,
        package: Option<&str>,
        service: Option<&str>,
    ) -> PyResult<PyObject> {
        let body = serde_json::json!({
            "bundle": bundle,
            "version": version,
            "migrate": migrate,
            "package": package,
            "service": service,
        });
        let path = format!("apps/{}/upgrade", seg(app));
        let v = match passkey {
            Some(pk) => self.confirmed(py, path, body, pk)?,
            None => self.send(py, Verb::Post, path, body)?,
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

    // ------------------------------------------------------------ organizations

    /// Create an organization; the caller owns it. Returns its id.
    fn create_org(&self, py: Python<'_>, name: &str) -> PyResult<String> {
        let v = self.send(
            py,
            Verb::Post,
            "orgs".into(),
            serde_json::json!({ "name": name }),
        )?;
        field(&v, "orgId")
    }

    /// An organization: members, roles and the teams the caller can see.
    fn get_org(&self, py: Python<'_>, org: &str) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, format!("orgs/{}", seg(org)), Value::Null)?;
        to_py(py, &v)
    }

    /// Add `account` to the organization, or change its role
    /// (`guest`, `member`, `admin`, `owner`).
    fn set_member(&self, py: Python<'_>, org: &str, account: &str, role: &str) -> PyResult<()> {
        let body = serde_json::json!({ "account": account, "role": role });
        self.send(py, Verb::Post, format!("orgs/{}/members", seg(org)), body)
            .map(drop)
    }

    /// Remove `account` from the organization (yourself, to leave it).
    fn remove_member(&self, py: Python<'_>, org: &str, account: &str) -> PyResult<()> {
        self.send(
            py,
            Verb::Delete,
            format!("orgs/{}/members/{}", seg(org), seg(account)),
            Value::Null,
        )
        .map(drop)
    }

    /// Set capability bits (v1's `MemberCapabilities`): for `account` (in
    /// `team`, if given), or, with no account, the organization's default.
    #[pyo3(signature = (org, capabilities, account = None, team = None))]
    fn set_capabilities(
        &self,
        py: Python<'_>,
        org: &str,
        capabilities: u32,
        account: Option<&str>,
        team: Option<&str>,
    ) -> PyResult<()> {
        let body =
            serde_json::json!({ "capabilities": capabilities, "account": account, "team": team });
        self.send(
            py,
            Verb::Post,
            format!("orgs/{}/capabilities", seg(org)),
            body,
        )
        .map(drop)
    }

    /// Create a team: `join="everyone"` (open: whoever reaches the parent is a
    /// member) or `"invite"` (restricted), under `parent` or at the top.
    /// Returns its id.
    #[pyo3(signature = (org, name, join = "invite", parent = None))]
    fn create_team(
        &self,
        py: Python<'_>,
        org: &str,
        name: &str,
        join: &str,
        parent: Option<&str>,
    ) -> PyResult<String> {
        let body = serde_json::json!({ "name": name, "join": join, "parent": parent });
        let v = self.send(py, Verb::Post, format!("orgs/{}/teams", seg(org)), body)?;
        field(&v, "teamId")
    }

    /// A team: its members, sub-teams and apps.
    fn get_team(&self, py: Python<'_>, org: &str, team: &str) -> PyResult<PyObject> {
        let v = self.send(
            py,
            Verb::Get,
            format!("orgs/{}/teams/{}", seg(org), seg(team)),
            Value::Null,
        )?;
        to_py(py, &v)
    }

    /// Rename, change how it is joined, set its metadata, or move it: under
    /// `parent`, or to the top with `to_top=True`. Unset arguments stay as they are.
    #[pyo3(signature = (org, team, name = None, join = None, metadata = None, parent = None, to_top = false))]
    #[allow(clippy::too_many_arguments)]
    fn update_team(
        &self,
        py: Python<'_>,
        org: &str,
        team: &str,
        name: Option<&str>,
        join: Option<&str>,
        metadata: Option<&Bound<'_, PyAny>>,
        parent: Option<&str>,
        to_top: bool,
    ) -> PyResult<()> {
        if to_top && parent.is_some() {
            return Err(PyValueError::new_err("pass parent or to_top, not both"));
        }
        let mut body = serde_json::Map::new();
        if let Some(name) = name {
            let _prev = body.insert("name".into(), name.into());
        }
        if let Some(join) = join {
            let _prev = body.insert("join".into(), join.into());
        }
        if let Some(metadata) = metadata {
            let _prev = body.insert("metadata".into(), to_json(py, Some(metadata))?);
        }
        if let Some(parent) = parent {
            let _prev = body.insert("parent".into(), parent.into());
        } else if to_top {
            let _prev = body.insert("parent".into(), Value::Null);
        }
        self.send(
            py,
            Verb::Post,
            format!("orgs/{}/teams/{}", seg(org), seg(team)),
            Value::Object(body),
        )
        .map(drop)
    }

    /// Delete a team, the teams below it and their apps.
    fn delete_team(&self, py: Python<'_>, org: &str, team: &str) -> PyResult<()> {
        self.send(
            py,
            Verb::Delete,
            format!("orgs/{}/teams/{}", seg(org), seg(team)),
            Value::Null,
        )
        .map(drop)
    }

    /// Add `account` to a team, or change its row: `role` is `lead`, `member`
    /// or `readOnly`; `capabilities` overrides the organization's default.
    #[pyo3(signature = (org, team, account, role = None, capabilities = None))]
    fn set_team_member(
        &self,
        py: Python<'_>,
        org: &str,
        team: &str,
        account: &str,
        role: Option<&str>,
        capabilities: Option<u32>,
    ) -> PyResult<()> {
        let body = serde_json::json!({
            "account": account,
            "role": role,
            "capabilities": capabilities,
        });
        self.send(
            py,
            Verb::Post,
            format!("orgs/{}/teams/{}/members", seg(org), seg(team)),
            body,
        )
        .map(drop)
    }

    /// Remove `account` from a team. It does not come back through an open
    /// parent until it is added again.
    fn remove_team_member(
        &self,
        py: Python<'_>,
        org: &str,
        team: &str,
        account: &str,
    ) -> PyResult<()> {
        self.send(
            py,
            Verb::Delete,
            format!(
                "orgs/{}/teams/{}/members/{}",
                seg(org),
                seg(team),
                seg(account)
            ),
            Value::Null,
        )
        .map(drop)
    }

    /// Invite someone into the organization with `role`, and into `teams`.
    /// Returns the token to hand them (valid `ttl_secs`, default a week, for
    /// `uses` redemptions, default one).
    #[pyo3(signature = (org, role, teams = Vec::new(), ttl_secs = None, uses = None))]
    fn invite(
        &self,
        py: Python<'_>,
        org: &str,
        role: &str,
        teams: Vec<String>,
        ttl_secs: Option<u64>,
        uses: Option<u32>,
    ) -> PyResult<String> {
        let body =
            serde_json::json!({ "role": role, "teams": teams, "ttlSecs": ttl_secs, "uses": uses });
        let v = self.send(py, Verb::Post, format!("orgs/{}/invites", seg(org)), body)?;
        field(&v, "token")
    }

    /// Join an organization (and its invited teams) with an invite token.
    fn redeem_invite(&self, py: Python<'_>, token: &str) -> PyResult<PyObject> {
        let v = self.send(
            py,
            Verb::Post,
            format!("invites/{}/redeem", seg(token)),
            Value::Null,
        )?;
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

    // ------------------------------------------------------------ account

    /// What this account uses, with the cluster's quotas.
    fn usage(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.usage().await })?;
        to_py(py, &v)
    }

    /// Name one of this account's passkeys; `""` clears the name.
    fn label_passkey(&self, py: Python<'_>, credential_id: &str, label: &str) -> PyResult<()> {
        let (c, cred, label) = (
            Arc::clone(&self.inner),
            credential_id.to_owned(),
            label.to_owned(),
        );
        self.run(py, async move { c.label_passkey(&cred, &label).await })
            .map(drop)
    }

    /// Name `app` for this account (`alias` then reaches it wherever an app id does).
    fn set_alias(&self, py: Python<'_>, alias: &str, app: &str) -> PyResult<()> {
        let (c, alias, app) = (Arc::clone(&self.inner), alias.to_owned(), app.to_owned());
        self.run(py, async move { c.set_alias(&alias, &app).await })
            .map(drop)
    }

    /// The app an alias names: `{"alias", "appId"}`.
    fn alias(&self, py: Python<'_>, alias: &str) -> PyResult<PyObject> {
        let (c, alias) = (Arc::clone(&self.inner), alias.to_owned());
        let v = self.run(py, async move { c.alias(&alias).await })?;
        to_py(py, &v)
    }

    /// Every alias of this account: `{alias: app_id}`.
    fn aliases(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.aliases().await })?;
        to_py(py, &v["aliases"])
    }

    fn delete_alias(&self, py: Python<'_>, alias: &str) -> PyResult<()> {
        let (c, alias) = (Arc::clone(&self.inner), alias.to_owned());
        self.run(py, async move { c.delete_alias(&alias).await })
            .map(drop)
    }

    // ------------------------------------------------------------ packages

    /// Install a signed `.mpk` package. Returns what the node installed.
    fn upload_package(&self, py: Python<'_>, mpk: Vec<u8>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.upload_package(mpk).await })?;
        to_py(py, &v)
    }

    /// Install `package@version` from the node's package registry.
    fn install_package(&self, py: Python<'_>, package: &str, version: &str) -> PyResult<PyObject> {
        let (c, package, version) = (
            Arc::clone(&self.inner),
            package.to_owned(),
            version.to_owned(),
        );
        let v = self.run(
            py,
            async move { c.install_package(&package, &version).await },
        )?;
        to_py(py, &v)
    }

    /// Installed packages and their versions.
    fn packages(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.packages().await })?;
        to_py(py, &v)
    }

    /// One installed package and its versions.
    fn package(&self, py: Python<'_>, package: &str) -> PyResult<PyObject> {
        let (c, package) = (Arc::clone(&self.inner), package.to_owned());
        let v = self.run(py, async move { c.package(&package).await })?;
        to_py(py, &v)
    }

    /// Uninstall one version of a package, or (without `version`) all of it.
    #[pyo3(signature = (package, version = None))]
    fn uninstall_package(
        &self,
        py: Python<'_>,
        package: &str,
        version: Option<String>,
    ) -> PyResult<PyObject> {
        let (c, package) = (Arc::clone(&self.inner), package.to_owned());
        let v = self.run(py, async move {
            c.uninstall_package(&package, version.as_deref()).await
        })?;
        to_py(py, &v)
    }

    // ------------------------------------------------------------ apps (more)

    /// Delete an app (its owner's, or an org admin's).
    fn delete_app(&self, py: Python<'_>, app: &str) -> PyResult<()> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        self.run(py, async move { c.delete_app(&app).await })
            .map(drop)
    }

    /// What the app's storage holds: `{"keys", "bytes", "blobs", "blobBytes"}`.
    fn app_storage(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        let v = self.run(py, async move { c.app_storage(&app).await })?;
        to_py(py, &v)
    }

    /// The ABI the app's bundle embeds: `{"bundle", "abi"}`.
    fn app_abi(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        let v = self.run(py, async move { c.app_abi(&app).await })?;
        to_py(py, &v)
    }

    /// Set an app's metadata (its owner's, or an org admin's); `None` clears it.
    #[pyo3(signature = (app, metadata))]
    fn set_app_metadata(
        &self,
        py: Python<'_>,
        app: &str,
        metadata: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let (c, app, m) = (
            Arc::clone(&self.inner),
            app.to_owned(),
            metadata_json(py, metadata)?,
        );
        self.run(py, async move { c.set_app_metadata(&app, m).await })
            .map(drop)
    }

    /// Verify every replica of `app`: each one's quote over a fresh nonce,
    /// held to this client's policy, binding its receipt key and the
    /// cluster's governance. Returns `{"keys": {node_id: receipt_key_hex},
    /// "governance": {"releases", "appUpgrades", "outbound"}}`; a call's or
    /// view's `receipt` is to be trusted only from one of those keys.
    fn verify_app_cluster(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        let cluster = self.run(py, async move { c.verify_app_cluster(&app).await })?;
        let keys = pyo3::types::PyDict::new_bound(py);
        for (node, key) in &cluster.keys {
            keys.set_item(node, hex::encode(key))?;
        }
        let out = pyo3::types::PyDict::new_bound(py);
        out.set_item("keys", keys)?;
        out.set_item("governance", to_py(py, &cluster.governance)?)?;
        Ok(out.into_any().unbind())
    }

    /// Presence as a stream: the snapshot first (`{"kind": "snapshot",
    /// "data": [...]}`), then one `{"kind": "presence", "data": {...}}` per
    /// change; up to `limit` of them, or as many as arrive within `timeout`
    /// seconds.
    #[pyo3(signature = (app, limit = 100, timeout = 5.0))]
    fn presence_stream(
        &self,
        py: Python<'_>,
        app: &str,
        limit: usize,
        timeout: f64,
    ) -> PyResult<PyObject> {
        let (c, app) = (Arc::clone(&self.inner), app.to_owned());
        let wait = Duration::from_secs_f64(timeout.max(0.0));
        let out = self.run(py, async move {
            let stream = c.presence_stream(&app).await?;
            let mut stream = Box::pin(stream.take(limit));
            let mut out = Vec::new();
            let _elapsed = tokio::time::timeout(wait, async {
                while let Some((_id, kind, data)) = stream.next().await {
                    out.push(serde_json::json!({ "kind": kind, "data": data }));
                }
            })
            .await;
            Ok(out)
        })?;
        to_py(py, &out)
    }

    // ------------------------------------------------------------ organizations (more)

    /// Delete an organization with its teams, invites and apps: the owner's,
    /// approved with their `passkey`.
    fn delete_org(&self, py: Python<'_>, org: &str, passkey: &PySoftPasskey) -> PyResult<()> {
        let (c, pk, org) = (
            Arc::clone(&self.inner),
            Arc::clone(&passkey.inner),
            org.to_owned(),
        );
        self.run(py, async move {
            let mut auth = pk.lock().await;
            c.delete_org(&org, &mut *auth).await
        })
        .map(drop)
    }

    /// Set an organization's metadata (an admin's, or a holder of
    /// `MANAGE_METADATA`); `None` clears it.
    #[pyo3(signature = (org, metadata))]
    fn set_org_metadata(
        &self,
        py: Python<'_>,
        org: &str,
        metadata: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let (c, org, m) = (
            Arc::clone(&self.inner),
            org.to_owned(),
            metadata_json(py, metadata)?,
        );
        self.run(py, async move { c.set_org_metadata(&org, m).await })
            .map(drop)
    }

    /// Set a member's metadata (their own, or an admin's); `None` clears it.
    #[pyo3(signature = (org, account, metadata))]
    fn set_member_metadata(
        &self,
        py: Python<'_>,
        org: &str,
        account: &str,
        metadata: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let (c, org, account, m) = (
            Arc::clone(&self.inner),
            org.to_owned(),
            account.to_owned(),
            metadata_json(py, metadata)?,
        );
        self.run(
            py,
            async move { c.set_member_metadata(&org, &account, m).await },
        )
        .map(drop)
    }

    // ------------------------------------------------------------ Ops API
    //
    // The operator's calls, authenticated with the node's ops token:
    // `client.use_api_key(ops_token)` first. The Ops API has no read path
    // into apps' data: counts, placement, releases, packages, quotas,
    // backups and metrics only.

    /// Node, group and directory counts: `{"node", "system", "governance",
    /// "groups", "nodes", "apps", "accounts"}`.
    fn ops_status(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, "ops/status".into(), Value::Null)?;
        to_py(py, &v)
    }

    /// Prometheus metrics, as text.
    fn ops_metrics(&self, py: Python<'_>) -> PyResult<String> {
        let c = Arc::clone(&self.inner);
        self.run(py, async move { c.get_text("ops/metrics").await })
    }

    /// Register node `node_id` at `addr` (its URL) in the directory. A node
    /// started without `--bootstrap` then joins the system group as a
    /// learner, and replica moves may place apps on it.
    #[pyo3(signature = (node_id, addr, zone = ""))]
    fn ops_register_node(
        &self,
        py: Python<'_>,
        node_id: u64,
        addr: &str,
        zone: &str,
    ) -> PyResult<()> {
        let body = serde_json::json!({ "id": node_id, "addr": addr, "zone": zone });
        self.send(py, Verb::Post, "ops/nodes".into(), body)
            .map(drop)
    }

    /// Remove node `node_id` from the cluster once no app keeps a replica on
    /// it. Runs on the system leader; a `NOT_LEADER` answer is followed there.
    /// Returns the system group's status.
    fn ops_remove_node(&self, py: Python<'_>, node_id: u64) -> PyResult<PyObject> {
        let v = self.send_to_leader(
            py,
            Verb::Delete,
            format!("ops/nodes/{node_id}"),
            Value::Null,
        )?;
        to_py(py, &v)
    }

    /// Move `app` to the replica node ids `replicas` (at least 3). Runs on
    /// the app group's leader; a `NOT_LEADER` answer is followed there.
    /// Returns the group's status.
    fn ops_set_replicas(
        &self,
        py: Python<'_>,
        app: &str,
        replicas: Vec<u64>,
    ) -> PyResult<PyObject> {
        let v = self.send_to_leader(
            py,
            Verb::Post,
            format!("ops/apps/{}/replicas", seg(app)),
            serde_json::json!({ "replicas": replicas }),
        )?;
        to_py(py, &v)
    }

    /// Write encrypted snapshots of the apps this node leads (to the node's
    /// `--backup-dir`). Returns how many it wrote.
    fn ops_backup(&self, py: Python<'_>) -> PyResult<u64> {
        let v = self.send(py, Verb::Post, "ops/backup".into(), Value::Null)?;
        Ok(v["written"].as_u64().unwrap_or_default())
    }

    /// The per-account quotas.
    fn ops_quotas(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, "ops/quotas".into(), Value::Null)?;
        to_py(py, &v)
    }

    /// Change the per-account limits `quotas` names; the others stay as they
    /// are. A key that names no limit is refused (`UNKNOWN_QUOTA`).
    fn ops_set_quotas(&self, py: Python<'_>, quotas: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let body = to_json(py, Some(quotas))?;
        let v = self.send(py, Verb::Post, "ops/quotas".into(), body)?;
        to_py(py, &v)
    }

    /// Approve (or, `withdraw=True`, withdraw) the mero-tee release `version`
    /// for every app, where the deployment gave the operator release
    /// approvals (`merod --release-approvals operator`).
    #[pyo3(signature = (version, withdraw = false))]
    fn ops_approve_release(
        &self,
        py: Python<'_>,
        version: &str,
        withdraw: bool,
    ) -> PyResult<PyObject> {
        let body = serde_json::json!({ "version": version, "approved": !withdraw });
        let v = self.send(py, Verb::Post, "ops/releases".into(), body)?;
        to_py(py, &v)
    }

    /// Approve URL prefixes `package`'s publisher may call out to, for every
    /// app, where the operator holds outbound approvals: every prefix its
    /// manifest declares when `prefixes` is `None`; `replace=True` sets
    /// exactly these (`[]` revokes).
    #[pyo3(signature = (package, prefixes = None, replace = false))]
    fn ops_approve_outbound(
        &self,
        py: Python<'_>,
        package: &str,
        prefixes: Option<Vec<String>>,
        replace: bool,
    ) -> PyResult<PyObject> {
        let prefixes =
            prefixes.map_or_else(|| serde_json::json!("manifest"), |p| serde_json::json!(p));
        let body =
            serde_json::json!({ "package": package, "prefixes": prefixes, "replace": replace });
        let v = self.send(py, Verb::Post, "ops/outbound".into(), body)?;
        to_py(py, &v)
    }

    /// Upgrade `app` (to an uploaded `bundle`, or `version` of its package,
    /// optionally moving it to `package`/`service`; `migrate` names the
    /// migration export), where the operator holds app upgrades.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (app, bundle = None, version = None, package = None, service = None, migrate = None))]
    fn ops_upgrade_app(
        &self,
        py: Python<'_>,
        app: &str,
        bundle: Option<&str>,
        version: Option<&str>,
        package: Option<&str>,
        service: Option<&str>,
        migrate: Option<&str>,
    ) -> PyResult<PyObject> {
        let body = serde_json::json!({
            "bundle": bundle, "version": version, "package": package,
            "service": service, "migrate": migrate,
        });
        let v = self.send(
            py,
            Verb::Post,
            format!("ops/apps/{}/upgrade", seg(app)),
            body,
        )?;
        to_py(py, &v)
    }

    /// Upgrade every app of `org` running `package` to `version` (optionally
    /// only `team`, with `cascade` its subtree), where the operator holds app
    /// upgrades. Returns the run, which `upgrade_run` follows.
    #[pyo3(signature = (org, package, version, team = None, cascade = false, migrate = None))]
    #[allow(clippy::too_many_arguments)]
    fn ops_upgrade_org(
        &self,
        py: Python<'_>,
        org: &str,
        package: &str,
        version: &str,
        team: Option<&str>,
        cascade: bool,
        migrate: Option<&str>,
    ) -> PyResult<PyObject> {
        let body = serde_json::json!({
            "package": package, "version": version, "team": team,
            "cascade": cascade, "migrate": migrate,
        });
        let v = self.send(
            py,
            Verb::Post,
            format!("ops/orgs/{}/upgrade", seg(org)),
            body,
        )?;
        to_py(py, &v)
    }

    /// The packages the operator installed.
    fn ops_packages(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, "ops/packages".into(), Value::Null)?;
        to_py(py, &v)
    }

    /// Install a signed `.mpk` for the operator's upgrades.
    fn ops_upload_package(&self, py: Python<'_>, mpk: Vec<u8>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.ops_upload_package(mpk).await })?;
        to_py(py, &v)
    }

    /// Install `package@version` from the registry for the operator's upgrades.
    fn ops_install_package(
        &self,
        py: Python<'_>,
        package: &str,
        version: &str,
    ) -> PyResult<PyObject> {
        let (c, package, version) = (
            Arc::clone(&self.inner),
            package.to_owned(),
            version.to_owned(),
        );
        let v = self.run(py, async move {
            c.ops_install_package(&package, &version).await
        })?;
        to_py(py, &v)
    }

    /// Uninstall a package the operator installed (or one `version` of it).
    #[pyo3(signature = (package, version = None))]
    fn ops_uninstall_package(
        &self,
        py: Python<'_>,
        package: &str,
        version: Option<String>,
    ) -> PyResult<PyObject> {
        let (c, package) = (Arc::clone(&self.inner), package.to_owned());
        let v = self.run(py, async move {
            c.ops_uninstall_package(&package, version.as_deref()).await
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

#[cfg(test)]
mod tests {
    use super::seg;

    #[test]
    fn a_path_segment_cannot_reach_another_route() {
        assert_eq!(seg("39VM3IF4orLKKYdSYpZgqg"), "39VM3IF4orLKKYdSYpZgqg");
        assert_eq!(seg("a/../ops/status"), "a%2F..%2Fops%2Fstatus");
        assert_eq!(seg("x?y=1#z"), "x%3Fy%3D1%23z");
        assert_eq!(seg("a b"), "a%20b");
    }
}
