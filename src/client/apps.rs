//! Apps: creating and upgrading them, calls, views, events, presence, bundles and transfers.

use super::prelude::*;

/// A new idempotency key for a call the caller gave none, as core's client
/// makes one: a `node:` key is kept 10 minutes, long enough for the client's
/// own retries and failover.
fn new_call_key() -> String {
    format!(
        "node:c-{}",
        calimero_identity::b64url(&calimero_identity::random::<16>())
    )
}

#[pymethods]
impl PyCloudClient {
    /// Upload a WASM bundle. Returns its hash.
    fn upload_bundle(&self, py: Python<'_>, wasm: Vec<u8>) -> PyResult<String> {
        self.run(py, async |c| c.upload_bundle(wasm.clone()).await)
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
        let pk = Arc::clone(&passkey.inner);
        let (app, to) = (app.to_owned(), to.to_owned());
        let v = self.run(py, async |c| {
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
        let app = app.to_owned();
        let v = self.run(py, async |c| {
            c.publish_presence(&app, seq, state.as_deref()).await
        })?;
        to_py(py, &v)
    }

    /// Who is present in `app` now.
    fn presence(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let app = app.to_owned();
        let v = self.run(py, async |c| c.presence(&app).await)?;
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
        let o = to_json(py, options)?;
        let (name, package) = (name.to_owned(), package.to_owned());
        self.run(py, async |c| {
            c.create_app_from_package(
                &name,
                &package,
                version.as_deref(),
                service.as_deref(),
                o.clone(),
            )
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
        let o = to_json(py, options)?;
        let (name, bundle) = (name.to_owned(), bundle.to_owned());
        self.run(py, async |c| c.create_app(&name, &bundle, o.clone()).await)
    }

    /// Call a method. Returns `{"index", "result", "events"}`. While the node
    /// answers 503 or cannot be reached, the call is sent again with the same
    /// idempotency key (`idempotency_key`, else one the client makes), so it
    /// runs once; when the node keeps failing, the call goes with that key to
    /// the client's next node (the key is the app's, cluster-wide).
    #[pyo3(signature = (app, method, args = None, idempotency_key = None))]
    fn call(
        &self,
        py: Python<'_>,
        app: &str,
        method: &str,
        args: Option<&Bound<'_, PyAny>>,
        idempotency_key: Option<String>,
    ) -> PyResult<PyObject> {
        let a = to_json(py, args)?;
        let (app, method) = (app.to_owned(), method.to_owned());
        // Made here, not by core's client, so each node is sent the same key.
        let key = idempotency_key.unwrap_or_else(new_call_key);
        let r = self.run_repeatable(py, async |c| {
            c.call(&app, &method, a.clone(), Some(&key)).await
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
        let (a, pk) = (to_json(py, args)?, Arc::clone(&passkey.inner));
        let (app, method) = (app.to_owned(), method.to_owned());
        let r = self.run(py, async |c| {
            let mut auth = pk.lock().await;
            c.call_confirmed(&app, &method, a.clone(), &mut *auth).await
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
        let a = to_json(py, args)?;
        let (app, method) = (app.to_owned(), method.to_owned());
        let consistency = if stale {
            Consistency::Stale { min_index }
        } else {
            Consistency::Strong
        };
        let r = self.run_repeatable(py, async |c| {
            c.view(&app, &method, a.clone(), consistency).await
        })?;
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
        let app = app.to_owned();
        let wait = Duration::from_secs_f64(timeout.max(0.0));
        let out = self.run(py, async |c| {
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

    /// Delete an app (its owner's, or an org admin's).
    fn delete_app(&self, py: Python<'_>, app: &str) -> PyResult<()> {
        let app = app.to_owned();
        self.run(py, async |c| c.delete_app(&app).await).map(drop)
    }

    /// What the app's storage holds: `{"keys", "bytes", "blobs", "blobBytes"}`.
    fn app_storage(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let app = app.to_owned();
        let v = self.run(py, async |c| c.app_storage(&app).await)?;
        to_py(py, &v)
    }

    /// The ABI the app's bundle embeds: `{"bundle", "abi"}`.
    fn app_abi(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let app = app.to_owned();
        let v = self.run(py, async |c| c.app_abi(&app).await)?;
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
        let (app, m) = (app.to_owned(), metadata_json(py, metadata)?);
        self.run(py, async |c| c.set_app_metadata(&app, m.clone()).await)
            .map(drop)
    }

    /// Verify every replica of `app`: each one's quote over a fresh nonce,
    /// held to this client's policy, binding its receipt key and the
    /// cluster's governance. Returns `{"keys": {node_id: receipt_key_hex},
    /// "governance": {"releases", "appUpgrades", "outbound"}}`; a call's or
    /// view's `receipt` is to be trusted only from one of those keys.
    fn verify_app_cluster(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let app = app.to_owned();
        let cluster = self.run(py, async |c| c.verify_app_cluster(&app).await)?;
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
        let app = app.to_owned();
        let wait = Duration::from_secs_f64(timeout.max(0.0));
        let out = self.run(py, async |c| {
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
}
