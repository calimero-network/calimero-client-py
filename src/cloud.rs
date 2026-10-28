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

#[derive(Clone, Copy)]
enum Verb {
    Get,
    Post,
    Delete,
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
        self.run(py, async move {
            match verb {
                Verb::Get => c.get(&path).await,
                Verb::Post => c.post(&path, body).await,
                Verb::Delete => c.delete(&path).await,
            }
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
                    allow_debug_profiles,
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
