//! An app's blobs and sealed files.

use std::path::PathBuf;

use super::prelude::*;

/// What `upload_file` reads: a file on disk, streamed, or bytes already in hand.
#[derive(FromPyObject)]
enum FileSource {
    Bytes(Vec<u8>),
    Path(PathBuf),
}

#[pymethods]
impl PyCloudClient {
    /// Upload a blob for `app`. Returns `{"blobId", "id", "size"}` (`blobId`
    /// base58, as apps print it; `id` hex).
    fn upload_blob(&self, py: Python<'_>, app: &str, data: Vec<u8>) -> PyResult<PyObject> {
        let app = app.to_owned();
        let r = self.run(py, async |c| c.upload_blob(&app, data.clone()).await)?;
        to_py(py, &r)
    }

    /// One of `app`'s blobs, by id (base58 or hex).
    fn download_blob(&self, py: Python<'_>, app: &str, id: &str) -> PyResult<PyObject> {
        let (app, id) = (app.to_owned(), id.to_owned());
        let bytes = self.run(py, async |c| c.download_blob(&app, &id).await)?;
        Ok(pyo3::types::PyBytes::new_bound(py, &bytes)
            .into_any()
            .unbind())
    }

    /// `app`'s blobs: `{"blobs": [{"blobId", "id", "size"}, ...]}`.
    fn list_blobs(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let app = app.to_owned();
        let v = self.run(py, async |c| c.list_blobs(&app).await)?;
        to_py(py, &v)
    }

    /// One blob's id and size.
    fn blob_info(&self, py: Python<'_>, app: &str, id: &str) -> PyResult<PyObject> {
        let (app, id) = (app.to_owned(), id.to_owned());
        let v = self.run(py, async |c| c.blob_info(&app, &id).await)?;
        to_py(py, &v)
    }

    /// Delete one of `app`'s blobs (its owner's or an org admin's).
    fn delete_blob(&self, py: Python<'_>, app: &str, id: &str) -> PyResult<PyObject> {
        let (app, id) = (app.to_owned(), id.to_owned());
        let v = self.run(py, async |c| c.delete_blob(&app, &id).await)?;
        to_py(py, &v)
    }

    /// Upload a sealed file for component app `app`: `file` is a path
    /// (streamed, never read whole) or `bytes`. Up to 16 MiB it is one
    /// request, else a resumable session. Returns `{"id", "blobId", "size",
    /// "sha256", "index"}`.
    fn upload_file(&self, py: Python<'_>, app: &str, file: FileSource) -> PyResult<PyObject> {
        let app = app.to_owned();
        let v = match file {
            FileSource::Bytes(data) => {
                let size = data.len() as u64;
                self.run(py, async |c| {
                    c.upload_file(&app, std::io::Cursor::new(data.as_slice()), size)
                        .await
                })?
            }
            FileSource::Path(path) => {
                let size = std::fs::metadata(&path)?.len();
                self.run(py, async |c| {
                    let f = tokio::fs::File::open(&path)
                        .await
                        .map_err(eyre::Report::from)?;
                    c.upload_file(&app, f, size).await
                })?
            }
        };
        to_py(py, &v)
    }

    /// `app`'s sealed files: `{"files": [{"id", "size", "sha256", ...}, ...]}`.
    fn list_files(&self, py: Python<'_>, app: &str) -> PyResult<PyObject> {
        let v = self.send(
            py,
            Verb::Get,
            format!("apps/{}/files", seg(app)),
            Value::Null,
        )?;
        to_py(py, &v)
    }

    /// One sealed file's size and plaintext SHA-256, by id.
    fn get_file(&self, py: Python<'_>, app: &str, id: &str) -> PyResult<PyObject> {
        let path = format!("apps/{}/files/{}", seg(app), seg(id));
        let v = self.send(py, Verb::Get, path, Value::Null)?;
        to_py(py, &v)
    }
}
