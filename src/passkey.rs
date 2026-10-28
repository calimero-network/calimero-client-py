//! `SoftPasskey`: a software passkey for tests and scripts.

use std::sync::Arc;

use calimero_identity::testing::SoftAuthenticator;
use pyo3::prelude::*;
use tokio::sync::Mutex;

/// clusters. Real users sign in from a browser; services use API keys.
#[pyclass(name = "SoftPasskey")]
pub struct PySoftPasskey {
    pub(crate) inner: Arc<Mutex<SoftAuthenticator>>,
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
