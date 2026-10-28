//! Signed packages: upload, install, list and uninstall.

use super::prelude::*;

#[pymethods]
impl PyCloudClient {
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
}
