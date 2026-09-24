//! Python wrapper for ConnectionInfo

use std::sync::Arc;

use calimero_client::connection::ConnectionInfo;
use calimero_client::proof::RequestProofSigner;
use calimero_client::CliAuthenticator;
use calimero_primitives::identity::PrivateKey;
use pyo3::prelude::*;
use tokio::runtime::Runtime;
use url::Url;

use crate::auth::PyAuthMode;
use crate::storage::MeroboxFileStorage;
use crate::utils::json_to_python;

/// Decode one hex, borsh-encoded link of a proof chain.
///
/// Named in the error because the links are indistinguishable as hex, and
/// passing them in the wrong order is the likely mistake.
fn decode_link<T: borsh::BorshDeserialize>(raw: &str, what: &str) -> PyResult<T> {
    let bytes = hex::decode(raw.trim()).map_err(|err| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{what} is not valid hex: {err}"))
    })?;
    borsh::from_slice(&bytes).map_err(|err| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "{what} is not a valid encoding: {err}"
        ))
    })
}

/// Build the signer, or nothing when no keys were given.
///
/// The credential and the secret require each other. Either alone is inert — a
/// credential with no key signs nothing, a key with no credential produces a
/// signature the node cannot attribute — so this refuses rather than handing
/// back a connection the caller believes signs and which quietly does not.
fn request_proof_signer(
    credential: Option<&str>,
    secret: Option<&str>,
    session: Option<&str>,
) -> PyResult<Option<RequestProofSigner>> {
    let (credential, secret) = match (credential, secret) {
        (None, None) => {
            if session.is_some() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "device_session names a device, so it needs device_credential and \
                     device_secret too",
                ));
            }
            return Ok(None);
        }
        (Some(credential), Some(secret)) => (credential, secret),
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "device_credential and device_secret must be given together: either alone \
                 signs nothing a node can attribute",
            ))
        }
    };

    let account_proof = decode_link(credential, "device_credential")?;
    let raw = hex::decode(secret.trim()).map_err(|err| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "device_secret is not valid hex: {err}"
        ))
    })?;
    let key = PrivateKey::from(<[u8; 32]>::try_from(raw.as_slice()).map_err(|_ignored| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "device_secret must be 32 bytes, i.e. 64 hex characters",
        )
    })?);

    Ok(Some(match session {
        Some(raw) => RequestProofSigner::with_session(
            account_proof,
            decode_link(raw, "device_session")?,
            key,
        ),
        None => RequestProofSigner::with_device_key(account_proof, key),
    }))
}

/// Python wrapper for ConnectionInfo
#[pyclass(name = "ConnectionInfo")]
pub struct PyConnectionInfo {
    pub(crate) inner: Arc<ConnectionInfo<CliAuthenticator, MeroboxFileStorage>>,
    pub(crate) runtime: Arc<Runtime>,
}

#[pymethods]
impl PyConnectionInfo {
    /// Build a connection, optionally signing every request with a device key.
    ///
    /// `device_credential` and `device_secret` go together: the credential says
    /// which account a key belongs to, the key signs each call. Supplying them
    /// is what lets this client drive a relay it has no account on — nothing was
    /// ever issued to it, so there is no token to present.
    ///
    /// `device_session` is optional and decides which key `device_secret` is:
    /// with it, a session key; without it, the device key itself. Worth
    /// supplying for anything long-running — only the session link names a node,
    /// so without one a captured proof is replayable at any node serving
    /// delegated access until it expires.
    ///
    /// The node must be running with `--delegated-access`. One that is not
    /// answers 403, which is distinct from the 401 a bad proof gets.
    #[new]
    #[pyo3(signature = (
        api_url,
        node_name=None,
        device_credential=None,
        device_secret=None,
        device_session=None,
    ))]
    pub fn new(
        api_url: &str,
        node_name: Option<&str>,
        device_credential: Option<&str>,
        device_secret: Option<&str>,
        device_session: Option<&str>,
    ) -> PyResult<Self> {
        let runtime = Arc::new(
            Runtime::new()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        );

        let url = Url::parse(api_url).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid URL: {}", e))
        })?;

        let authenticator = CliAuthenticator::new();
        let storage = MeroboxFileStorage::new();

        let connection = ConnectionInfo::new(
            url,
            node_name.map(|s| s.to_string()),
            authenticator,
            storage,
        );

        let connection =
            match request_proof_signer(device_credential, device_secret, device_session)? {
                Some(signer) => connection.with_request_proof(signer),
                None => connection,
            };

        Ok(Self {
            inner: Arc::new(connection),
            runtime,
        })
    }

    #[getter]
    pub fn api_url(&self) -> String {
        self.inner.api_url().to_string()
    }

    #[getter]
    pub fn node_name(&self) -> Option<String> {
        self.inner.node_name().map(str::to_owned)
    }

    /// Make a GET request
    pub fn get(&self, path: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let path = path.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get::<serde_json::Value>(&path).await });

            match result {
                Ok(data) => Ok(json_to_python(py, &data)),
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Check if authentication is required
    pub fn detect_auth_mode(&self) -> PyResult<PyAuthMode> {
        let inner = self.inner.clone();

        let result = self
            .runtime
            .block_on(async move { inner.detect_auth_mode().await });

        match result {
            Ok(mode) => Ok(PyAuthMode { mode }),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Client error: {}",
                e
            ))),
        }
    }
}

/// Create a new connection
#[pyfunction]
#[pyo3(signature = (
    api_url,
    node_name=None,
    device_credential=None,
    device_secret=None,
    device_session=None,
))]
pub fn create_connection(
    api_url: &str,
    node_name: Option<&str>,
    device_credential: Option<&str>,
    device_secret: Option<&str>,
    device_session: Option<&str>,
) -> PyResult<PyConnectionInfo> {
    PyConnectionInfo::new(
        api_url,
        node_name,
        device_credential,
        device_secret,
        device_session,
    )
}
