//! `CloudError`, the one exception the bindings raise for a node's answer.

use calimero_client::Error;
use pyo3::PyErr;

// pyo3 0.22's macro checks a `gil-refs` feature this crate does not declare.
#[allow(unexpected_cfgs)]
mod exception {
    use pyo3::create_exception;
    use pyo3::exceptions::PyException;

    create_exception!(
        calimero_client_py,
        CloudError,
        PyException,
        "An error from a cloud node: args are (message, status, code, details); status is 0 when the node could not be reached or attested."
    );
}
pub use exception::CloudError;

pub(crate) fn cloud_err(e: Error) -> PyErr {
    match e {
        Error::Api {
            status,
            code,
            message,
            details,
        } => CloudError::new_err((
            format!("{status} {code}: {message}"),
            status,
            code,
            details.to_string(),
        )),
        Error::Transport(r) => CloudError::new_err((format!("{r:#}"), 0_u16, "TRANSPORT", "null")),
    }
}
