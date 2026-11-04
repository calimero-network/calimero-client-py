//! Signed packages: upload, install, list and uninstall, and the web origins
//! a package's frontends sign in from. Adding or removing an origin is
//! approved with the acting account's `passkey`: the node answers `412
//! CONFIRM_REQUIRED` first; the passkey signs `package:{package}`
//! / `__add_origin` or `__remove_origin`, bound in its challenge.

use super::prelude::*;

#[pymethods]
impl PyCloudClient {
    /// Install a signed `.mpk` package. Returns what the node installed.
    fn upload_package(&self, py: Python<'_>, mpk: Vec<u8>) -> PyResult<PyObject> {
        let v = self.run(py, async |c| c.upload_package(mpk.clone()).await)?;
        to_py(py, &v)
    }

    /// Install `package@version` from the node's package registry.
    fn install_package(&self, py: Python<'_>, package: &str, version: &str) -> PyResult<PyObject> {
        let (package, version) = (package.to_owned(), version.to_owned());
        let v = self.run(py, async |c| c.install_package(&package, &version).await)?;
        to_py(py, &v)
    }

    /// Installed packages and their versions.
    fn packages(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.run(py, async |c| c.packages().await)?;
        to_py(py, &v)
    }

    /// One installed package and its versions.
    fn package(&self, py: Python<'_>, package: &str) -> PyResult<PyObject> {
        let package = package.to_owned();
        let v = self.run(py, async |c| c.package(&package).await)?;
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
        let package = package.to_owned();
        let v = self.run(py, async |c| {
            c.uninstall_package(&package, version.as_deref()).await
        })?;
        to_py(py, &v)
    }

    /// The web origins registered for `package`: `{"package": ...,
    /// "origins": [{"origin", "rpId", "addedBy", "addedAt"}, ...]}`.
    fn list_package_origins(&self, py: Python<'_>, package: &str) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, origins_path(package), Value::Null)?;
        to_py(py, &v)
    }

    /// Register `origin` (`https://app.example`) for `package`'s frontends,
    /// approved with `passkey`. `rp_id` is the WebAuthn RP ID passkeys from
    /// that origin are made for; without it the node derives one from the
    /// origin. Returns `{"package", "origin", "rpId", "addedBy", "addedAt"}`.
    #[pyo3(signature = (package, origin, passkey, rp_id = None))]
    fn add_package_origin(
        &self,
        py: Python<'_>,
        package: &str,
        origin: &str,
        passkey: &PySoftPasskey,
        rp_id: Option<&str>,
    ) -> PyResult<PyObject> {
        let v = self.confirmed(
            py,
            origins_path(package),
            add_origin_body(origin, rp_id),
            passkey,
        )?;
        to_py(py, &v)
    }

    /// Remove `origin` from `package`, approved with `passkey`. Returns
    /// `{"package", "origin", "removed": true}`.
    fn remove_package_origin(
        &self,
        py: Python<'_>,
        package: &str,
        origin: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<PyObject> {
        let body = serde_json::json!({ "origin": origin });
        let v = self.confirmed_delete_with_body(py, origins_path(package), body, passkey)?;
        to_py(py, &v)
    }
}

/// `package-origins/{package}`, the package one segment.
fn origins_path(package: &str) -> String {
    format!("package-origins/{}", seg(package))
}

/// `{"origin": ...}`, with `rpId` only when the caller names one (the node
/// resolves it otherwise, and signs over the one it resolved).
fn add_origin_body(origin: &str, rp_id: Option<&str>) -> Value {
    let mut body = serde_json::json!({ "origin": origin });
    if let (Some(b), Some(r)) = (body.as_object_mut(), rp_id) {
        let _prev = b.insert("rpId".into(), Value::from(r));
    }
    body
}

#[cfg(test)]
mod tests {
    use super::{add_origin_body, origins_path};

    #[test]
    fn the_package_is_one_path_segment() {
        assert_eq!(
            origins_path("com.example.app"),
            "package-origins/com.example.app"
        );
        assert_eq!(origins_path("a/../ops"), "package-origins/a%2F..%2Fops");
    }

    #[test]
    fn rp_id_is_sent_only_when_given() {
        assert_eq!(
            add_origin_body("https://app.example", None),
            serde_json::json!({ "origin": "https://app.example" })
        );
        assert_eq!(
            add_origin_body("https://app.example", Some("example")),
            serde_json::json!({ "origin": "https://app.example", "rpId": "example" })
        );
    }
}
