//! Python wrapper for ConnectionInfo

use std::sync::Arc;

use calimero_client::connection::ConnectionInfo;
use calimero_client::proof::RequestProofSigner;
use calimero_client::tee::sealed::SealedTransport;
use calimero_client::CliAuthenticator;
use calimero_primitives::identity::PrivateKey;
use pyo3::prelude::*;
use tokio::runtime::Runtime;
use url::Url;

use crate::auth::PyAuthMode;
use crate::storage::MeroboxFileStorage;
use crate::tee::{decode_32, PyTeePolicy};
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

/// Seal `connection` to a TEE node's transport key, as asked.
///
/// Every combination that cannot do what it says is refused: a key given
/// without `sealed` would be ignored, `sealed` with no source for its key
/// cannot run, and a policy nothing uses verifies nothing. Each would leave the
/// caller believing traffic is protected when it is not.
fn sealed_transport(
    connection: Connection,
    tee: Option<&PyTeePolicy>,
    sealed: bool,
    transport_public_key: Option<&str>,
) -> PyResult<Connection> {
    let value_error =
        |message: &str| PyErr::new::<pyo3::exceptions::PyValueError, _>(message.to_owned());
    if !sealed {
        if transport_public_key.is_some() {
            return Err(value_error(
                "transport_public_key is the key requests are sealed to, so it needs sealed=True",
            ));
        }
        if tee.is_some() {
            return Err(value_error(
                "tee decides which node requests are sealed to, so it needs sealed=True",
            ));
        }
        return Ok(connection);
    }

    let api_url = connection.api_url().clone();
    let http = reqwest::Client::new();
    let transport =
        match (transport_public_key, tee) {
            (Some(_), Some(_)) => return Err(value_error(
                "give tee to attest the node's transport key, or transport_public_key, not both",
            )),
            (Some(key), None) => {
                SealedTransport::with_key(api_url, http, decode_32(key, "transport_public_key")?)
            }
            (None, Some(tee)) => SealedTransport::attested(api_url, http, tee.attestor()),
            (None, None) => return Err(value_error(
                "sealed=True needs tee to attest the node's transport key, or transport_public_key",
            )),
        };
    Ok(connection.with_sealed_transport(transport))
}

type Connection = ConnectionInfo<CliAuthenticator, MeroboxFileStorage>;

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
    ///
    /// For a TEE node, `sealed=True` encrypts every request, token refreshes
    /// included, to the node's attested transport key, so only its TD reads
    /// them, whatever proxy, load balancer or relay sits in front. The key
    /// comes from the attestation `tee` (a `TeePolicy`) verifies, on the first
    /// request and again whenever the node restarts, or from
    /// `transport_public_key` (hex) verified some other way. A key or a policy
    /// without `sealed`, or `sealed` with neither, is refused rather than
    /// ignored.
    #[new]
    #[pyo3(signature = (
        api_url,
        node_name=None,
        device_credential=None,
        device_secret=None,
        device_session=None,
        *,
        tee=None,
        sealed=false,
        transport_public_key=None,
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "the Python constructor's keyword arguments, one each"
    )]
    pub fn new(
        api_url: &str,
        node_name: Option<&str>,
        device_credential: Option<&str>,
        device_secret: Option<&str>,
        device_session: Option<&str>,
        tee: Option<PyTeePolicy>,
        sealed: bool,
        transport_public_key: Option<&str>,
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

        let connection = sealed_transport(connection, tee.as_ref(), sealed, transport_public_key)?;

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
    *,
    tee=None,
    sealed=false,
    transport_public_key=None,
))]
#[expect(
    clippy::too_many_arguments,
    reason = "mirrors ConnectionInfo's keyword arguments"
)]
pub fn create_connection(
    api_url: &str,
    node_name: Option<&str>,
    device_credential: Option<&str>,
    device_secret: Option<&str>,
    device_session: Option<&str>,
    tee: Option<PyTeePolicy>,
    sealed: bool,
    transport_public_key: Option<&str>,
) -> PyResult<PyConnectionInfo> {
    PyConnectionInfo::new(
        api_url,
        node_name,
        device_credential,
        device_secret,
        device_session,
        tee,
        sealed,
        transport_public_key,
    )
}
