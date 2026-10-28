//! An app's blobs.

use super::prelude::*;

#[pymethods]
impl PyCloudClient {
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
}
