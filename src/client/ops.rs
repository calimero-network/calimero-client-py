//! The Ops API, for operators: authenticate with `use_api_key(ops_token)`.

use super::prelude::*;

#[pymethods]
impl PyCloudClient {
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
}
