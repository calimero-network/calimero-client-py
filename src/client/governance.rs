//! Deployment governance: release and outbound approvals, org upgrades and upgrade runs.

use super::prelude::*;

#[pymethods]
impl PyCloudClient {
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
}
