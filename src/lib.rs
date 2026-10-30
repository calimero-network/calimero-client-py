//! Python bindings for the Calimero 1.0 client, using PyO3.
//!
//! This crate provides Python bindings that can be built independently
//! without requiring the full Calimero workspace.
//!
//! ## Module Structure
//!
//! - [`client`] - `Client` (also `CloudClient`) over core's `calimero-client`,
//!   one file per area (account, apps, blobs, orgs, governance, packages, ops)
//! - [`passkey`] - `SoftPasskey`, a software passkey for tests and scripts
//! - [`error`] - `CloudError`
//! - [`convert`] - Python <-> JSON and path segments

// pyo3 0.22's `#[pymethods]` expands each `PyResult` return through an
// `Into<PyErr>` that newer clippy reports as `useless_conversion`; the lint
// points at macro-generated code, not ours. pyo3 0.23 fixed it upstream.
#[allow(clippy::useless_conversion)]
pub mod client;
pub mod convert;
pub mod error;
#[allow(clippy::useless_conversion)]
pub mod passkey;

use pyo3::prelude::*;

/// Python module for Calimero client
#[pymodule]
fn calimero_client_py(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<client::PyCloudClient>()?;
    // `Client` is the preferred name; `CloudClient` stays for the code that
    // already imports it. One class, two names.
    m.add("Client", m.getattr("CloudClient")?)?;
    m.add_class::<passkey::PySoftPasskey>()?;
    m.add("CloudError", m.py().get_type_bound::<error::CloudError>())?;
    m.add(
        "LoginRequired",
        m.py().get_type_bound::<error::LoginRequired>(),
    )?;
    m.add("WrongNode", m.py().get_type_bound::<error::WrongNode>())?;

    m.add("VERSION", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
