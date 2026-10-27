//! Python bindings for the Calimero Cloud (release-v2) client, using PyO3.
//!
//! This crate provides Python bindings that can be built independently
//! without requiring the full Calimero workspace.
//!
//! ## Module Structure
//!
//! - `cloud` - CloudClient, SoftPasskey, CloudError for release-v2 clusters

// pyo3 0.22's `#[pymethods]` expands each `PyResult` return through an
// `Into<PyErr>` that newer clippy reports as `useless_conversion`; the lint
// points at macro-generated code, not ours. pyo3 0.23 fixed it upstream.
#[allow(clippy::useless_conversion)]
pub mod cloud;

use pyo3::prelude::*;

/// Python module for Calimero client
#[pymodule]
fn calimero_client_py(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<cloud::PyCloudClient>()?;
    m.add_class::<cloud::PySoftPasskey>()?;
    m.add("CloudError", m.py().get_type_bound::<cloud::CloudError>())?;

    m.add("VERSION", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
