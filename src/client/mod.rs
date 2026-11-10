//! `CloudClient` (also `Client`): a connection to one node of a Calimero 1.0
//! cluster. Attestation, the sealed channel and passkey handling run in core's
//! Rust client, so Python never re-implements the trust check. Every method
//! blocks with the GIL released.
//!
//! Two layers. The **runtime** is everything that is not a route: core's
//! `calimero-client` under it (attestation, the sealed channel, the session's
//! refresh), [`runtime`] (blocking with the GIL released, the `412` step-up,
//! operator signatures, the generic `request` / `get` / `post` / `put` /
//! `delete` a route layer sends through), [`recover`] (moving between nodes)
//! and [`directory`]. The **routes** are split by area, each file its own
//! `#[pymethods]` block: [`account`], [`apps`], [`blobs`], [`orgs`],
//! [`governance`], [`packages`] and [`ops`]. This file holds the connection.

mod account;
mod account_recovery;
mod apps;
mod blobs;
mod directory;
mod governance;
mod ops;
mod org_recovery;
mod orgs;
mod packages;
mod prelude;
mod recover;
mod runtime;

pub(crate) use directory::DirectoryError;
pub(crate) use runtime::{OperatorKey, Verb};

use crate::error::directory_err;
use prelude::*;
use recover::Nodes;
use runtime::{connect_all, policy};

/// A connection to one node of a Calimero 1.0 cluster (and, given `nodes`,
/// the others it may move to). Also exported as `Client`, the preferred name.
#[pyclass(name = "CloudClient")]
pub struct PyCloudClient {
    /// The nodes this client may use, the one in use, and the credential
    /// (the API key, the ops token for the Ops API, which a request sent on
    /// to the leader carries too). See [`recover`].
    nodes: Arc<Nodes<Client>>,
    runtime: Arc<Runtime>,
    /// The trust policy this client was made with, so a request that must
    /// run on a leader (`NOT_LEADER`) is sent to a node attested the same way.
    policy: Policy,
    /// The operator's Ed25519 key (`use_operator_key`): the Ops API's
    /// decisions answer `428 OPERATOR_SIGNATURE_REQUIRED` with it.
    operator_key: Arc<std::sync::Mutex<Option<OperatorKey>>>,
}

#[pymethods]
impl PyCloudClient {
    /// Connect to `base_url`. The node is trusted only once its attestation
    /// verifies; `allow_mock` accepts development (mock) quotes, `app_hash`
    /// (hex) pins the bundle the quote must bind.
    ///
    /// `nodes` are other nodes of the same cluster, in the order to move to
    /// them when a node answers `421 WRONG_NODE` (its release may not serve
    /// the account); each is attested under the same policy before anything
    /// is sent to it. Without them that answer is raised as `WrongNode`.
    /// A node that cannot be reached, or that keeps answering `503` to a read
    /// or a call, is left for the next too (each node once per request).
    /// A `401` to a request made with a session is answered by one refresh
    /// (any node takes it) and the request sent again; `LoginRequired` is
    /// raised only when that does not help.
    ///
    /// A real TDX quote is accepted only from an image the policy names:
    /// `tee_profiles` admits nodes whose signed mero-tee release lists their
    /// measurements under one of those profiles (`min_release` is a floor).
    /// Without it, only mock quotes (`allow_mock`) can be accepted. A
    /// release's debug profiles (whose memory the host can read) are refused
    /// unless `allow_debug_profiles`.
    #[new]
    #[pyo3(signature = (base_url, allow_mock = false, app_hash = None, tee_profiles = None, min_release = None, allow_debug_profiles = false, nodes = None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        base_url: &str,
        allow_mock: bool,
        app_hash: Option<&str>,
        tee_profiles: Option<Vec<String>>,
        min_release: Option<String>,
        allow_debug_profiles: bool,
        nodes: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let policy = policy(
            allow_mock,
            app_hash,
            tee_profiles,
            min_release,
            allow_debug_profiles,
        )?;
        let runtime = Runtime::new().map_err(|e| PyValueError::new_err(e.to_string()))?;
        let urls = std::iter::once(base_url.to_owned()).chain(nodes.unwrap_or_default());
        let nodes = connect_all(&runtime, urls, &policy)?;
        Ok(Self {
            nodes: Arc::new(Nodes::new(nodes)),
            runtime: Arc::new(runtime),
            policy,
            operator_key: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    /// Connect to the cluster named `cluster`, by the nodes the Calimero
    /// Cloud node directory lists for it (`GET {directory}/{cluster}`).
    ///
    /// The nodes in `zone` come first, then the rest, each group in a
    /// weighted random order; the first is the node in use and the others
    /// are `nodes`. The directory is not trusted: every node is attested
    /// under this policy (the arguments mean what they do for the
    /// constructor) before anything is sent to it. The listing is fetched
    /// again once it is due and after the client moved off an unavailable
    /// node. Raises `CloudError` (code `DIRECTORY`; the status is the
    /// directory's, `404` for an unknown cluster, 0 when it could not be
    /// reached) when the directory lists no usable node.
    #[staticmethod]
    #[pyo3(signature = (cluster, *, tee_profiles = None, min_release = None, allow_mock = false, app_hash = None, allow_debug_profiles = false, directory = None, zone = None))]
    #[allow(clippy::too_many_arguments)]
    fn connect(
        py: Python<'_>,
        cluster: &str,
        tee_profiles: Option<Vec<String>>,
        min_release: Option<String>,
        allow_mock: bool,
        app_hash: Option<&str>,
        allow_debug_profiles: bool,
        directory: Option<String>,
        zone: Option<String>,
    ) -> PyResult<Self> {
        if cluster.is_empty() {
            return Err(PyValueError::new_err("cluster must not be empty"));
        }
        let policy = policy(
            allow_mock,
            app_hash,
            tee_profiles,
            min_release,
            allow_debug_profiles,
        )?;
        let runtime = Runtime::new().map_err(|e| PyValueError::new_err(e.to_string()))?;
        let source = directory::Remote {
            directory: directory.unwrap_or_else(|| directory::DEFAULT_DIRECTORY.to_owned()),
            cluster: cluster.to_owned(),
            zone,
            policy: policy.clone(),
        };
        let (urls, refresh_after) = py
            .allow_threads(|| runtime.block_on(source.listing()))
            .map_err(directory_err)?;
        let nodes = connect_all(&runtime, urls, &policy)?;
        Ok(Self {
            nodes: Arc::new(Nodes::new(nodes).with_directory(Box::new(source), refresh_after)),
            runtime: Arc::new(runtime),
            policy,
            operator_key: Arc::new(std::sync::Mutex::new(None)),
        })
    }
}
