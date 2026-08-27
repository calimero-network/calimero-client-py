//! Python wrapper for Client

use std::str::FromStr;
use std::sync::Arc;

use calimero_client::client::Client;
use calimero_client::CliAuthenticator;
use calimero_primitives::alias::Alias;
use calimero_primitives::application::ApplicationId;
use calimero_primitives::blobs;
use calimero_primitives::context::{ContextId, GroupMemberRole};
use calimero_primitives::hash::Hash;
use calimero_primitives::identity;
use calimero_primitives::identity::AccountId;
use calimero_server_primitives::admin;
use calimero_server_primitives::jsonrpc;
use pyo3::prelude::*;
use tokio::runtime::Runtime;

use crate::connection::PyConnectionInfo;
use crate::storage::MeroboxFileStorage;
use crate::utils::json_to_python;

/// Python wrapper for Client
#[pyclass(name = "Client")]
pub struct PyClient {
    inner: Arc<Client<CliAuthenticator, MeroboxFileStorage>>,
    runtime: Arc<Runtime>,
}

fn parse_group_member_role(role: &str) -> PyResult<GroupMemberRole> {
    match role.to_ascii_lowercase().as_str() {
        "admin" => Ok(GroupMemberRole::Admin),
        "member" => Ok(GroupMemberRole::Member),
        "readonly" | "read_only" | "read-only" => Ok(GroupMemberRole::ReadOnly),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid role '{}'. Expected one of: admin, member, read-only",
            role
        ))),
    }
}

/// Response conversion shared by the account methods.
///
/// The older methods each inline this match; sharing it here keeps the five
/// account bindings from repeating twenty lines of boilerplate five times. It
/// lives in a plain `impl` because `#[pymethods]` may only contain methods
/// exposed to Python.
impl PyClient {
    fn to_python<T: serde::Serialize, E: std::fmt::Display>(
        py: Python<'_>,
        result: Result<T, E>,
    ) -> PyResult<PyObject> {
        match result {
            Ok(data) => {
                let json_data = serde_json::to_value(data).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Failed to serialize response: {}",
                        e
                    ))
                })?;
                Ok(json_to_python(py, &json_data))
            }
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Client error: {}",
                e
            ))),
        }
    }
}

#[pymethods]
impl PyClient {
    #[new]
    pub fn new(connection: &PyConnectionInfo) -> PyResult<Self> {
        let runtime = Arc::new(
            Runtime::new()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?,
        );

        // Extract the inner connection from the Arc
        let connection_inner = connection.inner.as_ref().clone();
        let client = Client::new(connection_inner).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create client: {}",
                e
            ))
        })?;

        Ok(Self {
            inner: Arc::new(client),
            runtime,
        })
    }

    /// Get API URL
    pub fn get_api_url(&self) -> String {
        self.inner.api_url().to_string()
    }

    /// Get application information
    pub fn get_application(&self, app_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let app_id = app_id.parse::<ApplicationId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid application ID '{}': {}",
                app_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_application(&app_id).await });

            match result {
                Ok(data) => {
                    // Convert to JSON first, then to Python
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// List applications
    pub fn list_applications(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_applications().await });

            match result {
                Ok(data) => {
                    // Convert to JSON first, then to Python
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// List every locally-retained bytecode version of an application.
    ///
    /// Returns `{data: [{version, blobId, size, package}]}` — the row's latest
    /// install plus any older blobs still referenced by groups or context
    /// activation markers. The `blobId` doubles as the `app_key` accepted by
    /// `create_namespace`. Wraps `GET admin-api/applications/{id}/versions`.
    pub fn list_application_versions(&self, application_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let application_id = application_id.parse::<ApplicationId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid application ID '{}': {}",
                application_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_application_versions(&application_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Get context
    pub fn get_context(&self, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_context(&context_id).await });

            match result {
                Ok(data) => {
                    // Convert to JSON first, then to Python
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// List contexts
    pub fn list_contexts(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_contexts().await });

            match result {
                Ok(data) => {
                    // Convert to JSON first, then to Python
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Install application from URL
    #[pyo3(signature = (url, hash=None, metadata=None))]
    pub fn install_application(
        &self,
        url: &str,
        hash: Option<&str>,
        metadata: Option<&[u8]>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let url = url.to_string();
        let hash = hash.map(|h| h.to_string());
        let metadata = metadata.unwrap_or(b"{}").to_vec();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let url = url::Url::parse(&url).map_err(|e| eyre::eyre!("Invalid URL: {}", e))?;

                let hash = if let Some(hash_str) = hash {
                    let hash_bytes =
                        hex::decode(hash_str).map_err(|e| eyre::eyre!("Invalid hash: {}", e))?;
                    let hash_array: [u8; 32] = hash_bytes
                        .try_into()
                        .map_err(|_| eyre::eyre!("Hash must be 32 bytes"))?;
                    Some(Hash::from(hash_array))
                } else {
                    None
                };

                let request =
                    admin::InstallApplicationRequest::new(url, hash, metadata, None, None);

                inner.install_application(request).await
            });

            match result {
                Ok(data) => {
                    // Convert to JSON first, then to Python
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Install development application from local path
    #[pyo3(signature = (path, metadata=None))]
    pub fn install_dev_application(
        &self,
        path: &str,
        metadata: Option<&[u8]>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let path = path.to_string();
        let metadata = metadata.unwrap_or(b"{}").to_vec();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let path = camino::Utf8PathBuf::from(path);
                let metadata = metadata;

                let request = admin::InstallDevApplicationRequest::new(path, metadata, None, None);

                inner.install_dev_application(request).await
            });

            match result {
                Ok(data) => {
                    // Convert to JSON first, then to Python
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Uninstall application
    pub fn uninstall_application(&self, app_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let app_id = app_id.parse::<ApplicationId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid application ID '{}': {}",
                app_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.uninstall_application(&app_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Upload blob
    #[pyo3(signature = (data, context_id=None))]
    pub fn upload_blob(&self, data: &[u8], context_id: Option<&str>) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let data_vec = data.to_vec();
        let context_id_opt = context_id.map(|s| s.to_string());

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let context_id_parsed = if let Some(ctx_id) = context_id_opt {
                    Some(
                        ctx_id
                            .parse::<ContextId>()
                            .map_err(|e| eyre::eyre!("Invalid context ID '{}': {}", ctx_id, e))?,
                    )
                } else {
                    None
                };

                inner
                    .upload_blob(data_vec, context_id_parsed.as_ref())
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Download blob
    #[pyo3(signature = (blob_id, context_id=None))]
    pub fn download_blob(&self, blob_id: &str, context_id: Option<&str>) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let blob_id = blob_id.parse::<blobs::BlobId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid blob ID '{}': {}",
                blob_id, e
            ))
        })?;
        let context_id_opt = context_id.map(|s| s.to_string());

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let context_id_parsed = if let Some(ctx_id) = context_id_opt {
                    Some(
                        ctx_id
                            .parse::<ContextId>()
                            .map_err(|e| eyre::eyre!("Invalid context ID '{}': {}", ctx_id, e))?,
                    )
                } else {
                    None
                };

                inner
                    .download_blob(&blob_id, context_id_parsed.as_ref())
                    .await
            });

            match result {
                Ok(data) => {
                    // Return bytes directly as Python bytes object
                    Ok(pyo3::types::PyBytes::new_bound(py, &data).into_py(py))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// List blobs
    pub fn list_blobs(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_blobs().await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Get blob info
    pub fn get_blob_info(&self, blob_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let blob_id = blob_id.parse::<blobs::BlobId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid blob ID '{}': {}",
                blob_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_blob_info(&blob_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Delete blob
    pub fn delete_blob(&self, blob_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let blob_id = blob_id.parse::<blobs::BlobId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid blob ID '{}': {}",
                blob_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.delete_blob(&blob_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Generate context identity
    pub fn generate_context_identity(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.generate_context_identity().await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Get peers count
    pub fn get_peers_count(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_peers_count().await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Create context
    #[pyo3(signature = (application_id, group_id, params=None, service_name=None))]
    pub fn create_context(
        &self,
        application_id: &str,
        group_id: &str,
        params: Option<&str>,
        service_name: Option<&str>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let application_id = application_id.parse::<ApplicationId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid application ID '{}': {}",
                application_id, e
            ))
        })?;

        let params = params.map(|p| p.as_bytes().to_vec()).unwrap_or_default();
        let group_id = group_id.to_string();
        let service_name = service_name.map(|s| s.to_string());

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let request = admin::CreateContextRequest {
                    application_id,
                    service_name,
                    context_seed: None,
                    initialization_params: params,
                    group_id,
                    identity_secret: None,
                    name: None,
                };
                inner.create_context(request).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Delete context
    pub fn delete_context(&self, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;
        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.delete_context(&context_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Get context storage
    pub fn get_context_storage(&self, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_context_storage(&context_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Get context identities
    pub fn get_context_identities(&self, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_context_identities(&context_id, false).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Sync context
    pub fn sync_context(&self, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.sync_context(&context_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Execute function call via JSON-RPC
    ///
    /// The executor_public_key parameter is accepted for backward compatibility
    /// but ignored — the node auto-resolves the owned identity for the context.
    #[pyo3(signature = (context_id, method, args, executor_public_key=""))]
    pub fn execute_function(
        &self,
        context_id: &str,
        method: &str,
        args: &str,
        executor_public_key: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;
        // Ignored — node auto-resolves executor identity.
        let _ = executor_public_key;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                // Parse args as JSON
                let args_value: serde_json::Value = serde_json::from_str(args)
                    .map_err(|e| eyre::eyre!("Invalid JSON args: {}", e))?;

                let execution_request =
                    jsonrpc::ExecutionRequest::new(context_id, method.to_string(), args_value);

                let request = jsonrpc::Request::new(
                    jsonrpc::Version::TwoPointZero,
                    jsonrpc::RequestId::String("1".to_string()),
                    jsonrpc::RequestPayload::Execute(execution_request),
                );
                inner.execute_jsonrpc(request).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Update context application
    pub fn update_context_application(
        &self,
        context_id: &str,
        application_id: &str,
        executor_public_key: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;
        let application_id = application_id.parse::<ApplicationId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid application ID '{}': {}",
                application_id, e
            ))
        })?;
        let executor_public_key =
            executor_public_key
                .parse::<identity::PublicKey>()
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid executor public key '{}': {}",
                        executor_public_key, e
                    ))
                })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let request = admin::UpdateContextApplicationRequest::new(
                    application_id,
                    executor_public_key,
                );
                inner.update_context_application(&context_id, request).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Sync all contexts
    pub fn sync_all_contexts(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.sync_all_contexts().await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Resync a stranded context by adopting a peer's full state.
    ///
    /// Recovers a context that can no longer replay its upgrade ladder (e.g. an
    /// intermediate bytecode blob is unobtainable from every reachable peer) by
    /// discarding local DAG heads and pulling a peer's full-state snapshot.
    /// Destructive: `force` must be `True` when the context still holds local
    /// heads. Returns `{contextId, resyncStarted}`. Wraps
    /// `POST admin-api/contexts/{context_id}/resync`.
    #[pyo3(signature = (context_id, force=false))]
    pub fn resync_context(&self, context_id: &str, force: bool) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.to_string();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .resync_context(&context_id, admin::ResyncContextApiRequest { force })
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Create context alias
    pub fn create_context_alias(&self, alias: &str, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<ContextId>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.create_alias(alias_obj, context_id, None).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Create application alias
    pub fn create_application_alias(
        &self,
        alias: &str,
        application_id: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let application_id = application_id.parse::<ApplicationId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid application ID '{}': {}",
                application_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<ApplicationId>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.create_alias(alias_obj, application_id, None).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Delete context alias
    pub fn delete_context_alias(&self, alias: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<ContextId>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.delete_alias(alias_obj, None).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Delete context identity alias
    pub fn delete_context_identity_alias(
        &self,
        alias: &str,
        context_id: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<identity::PublicKey>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.delete_alias(alias_obj, Some(context_id)).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Delete application alias
    pub fn delete_application_alias(&self, alias: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<ApplicationId>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.delete_alias(alias_obj, None).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// List context aliases
    pub fn list_context_aliases(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_aliases::<ContextId>(None).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// List context identity aliases
    pub fn list_context_identity_aliases(&self, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .list_aliases::<identity::PublicKey>(Some(context_id))
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// List application aliases
    pub fn list_application_aliases(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_aliases::<ApplicationId>(None).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Lookup context alias
    pub fn lookup_context_alias(&self, alias: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<ContextId>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.lookup_alias(alias_obj, None).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Lookup context identity alias
    pub fn lookup_context_identity_alias(
        &self,
        alias: &str,
        context_id: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<identity::PublicKey>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.lookup_alias(alias_obj, Some(context_id)).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Lookup application alias
    pub fn lookup_application_alias(&self, alias: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<ApplicationId>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.lookup_alias(alias_obj, None).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Resolve context alias
    pub fn resolve_context_alias(&self, alias: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<ContextId>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.resolve_alias(alias_obj, None).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Resolve context identity alias
    pub fn resolve_context_identity_alias(
        &self,
        alias: &str,
        context_id: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<identity::PublicKey>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.resolve_alias(alias_obj, Some(context_id)).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Resolve application alias
    pub fn resolve_application_alias(&self, alias: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let alias_obj = Alias::<ApplicationId>::from_str(alias)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                inner.resolve_alias(alias_obj, None).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Create alias generic (Python wrapper for backward compatibility)
    #[pyo3(signature = (alias, value, scope=None))]
    pub fn create_alias_generic(
        &self,
        alias: &str,
        value: &str,
        scope: Option<&str>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let alias_str = alias.to_string();
        let value_str = value.to_string();
        let _scope_str = scope.map(|s| s.to_string());

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                // This is a simplified wrapper - in practice, you'd need to know the type T
                // For now, we'll use ContextId as a default type
                let alias_obj = Alias::<ContextId>::from_str(&alias_str)
                    .map_err(|e| eyre::eyre!("Invalid alias: {}", e))?;

                // Parse the value as ContextId
                let value_obj = value_str
                    .parse::<ContextId>()
                    .map_err(|e| eyre::eyre!("Invalid value: {}", e))?;

                // Create the alias
                inner.create_alias(alias_obj, value_obj, None).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }
    // ---- Namespace and Group Management ----

    #[pyo3(signature = (application_id, name=None, app_key=None))]
    pub fn create_namespace(
        &self,
        application_id: &str,
        name: Option<&str>,
        app_key: Option<&str>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let application_id = application_id.parse::<ApplicationId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid application ID '{}': {}",
                application_id, e
            ))
        })?;
        // Hex-encoded 32-byte blob id that pins the namespace to a specific
        // installed bytecode version; `None` lets the node use the app row's
        // latest blob.
        let request = admin::CreateNamespaceApiRequest {
            application_id,
            name: name.map(str::to_owned),
            // The Rust field is `bytecode_id` since core renamed it; the wire
            // name stayed `appKey` (see its serde attrs), and so does this
            // binding's Python keyword — renaming that would break every caller
            // for a change no caller can observe.
            bytecode_id: app_key.map(str::to_owned),
        };

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.create_namespace(request).await });
            Self::to_python(py, result)
        })
    }

    pub fn get_namespace(&self, namespace_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_group_info(&namespace_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    pub fn delete_namespace(&self, namespace_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();
        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .delete_namespace(&namespace_id, admin::DeleteNamespaceApiRequest {})
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    pub fn list_namespaces(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_namespaces().await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Who this node is, without naming a namespace.
    ///
    /// A node has one account, one device and one signing key, so none of them
    /// varies by namespace and the question needs no scope. Replaces
    /// `get_namespace_account`, which took a namespace it could not use.
    ///
    /// Returns `accountId`, `deviceId`, `publicKey` — the DEVICE's signing key,
    /// which is what op signatures verify against, not the account root — and
    /// `accountRootPublicKey`, the public half a second device needs in order to
    /// pair into this account. The private root is reachable from no endpoint
    /// at all; it leaves a node only via `merod account export`, as a mnemonic.
    ///
    /// 404s when the node holds neither a device nor an account root. `merod
    /// init` provisions the root, so that means a node initialized with
    /// `--no-account-root` and never paired.
    pub fn get_node_identity(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_node_identity().await });
            Self::to_python(py, result)
        })
    }

    /// Mint a device on this node for an account that already exists elsewhere -
    /// the first half of pairing.
    ///
    /// `namespaces` must not be empty: this node is a member of nothing yet, so
    /// it can neither read the account's namespace set off a DAG nor derive it.
    /// One device covers the whole set.
    ///
    /// Publishes nothing and needs no scope key. Returns the device id, both
    /// public keys, a signature over them, and a confirmation code; hand all of
    /// them to [`Self::pair_device_complete`] on the node that holds the account.
    pub fn pair_device_init(
        &self,
        account_root_public_key: &str,
        namespaces: Vec<String>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let request = admin::AccountPairInitApiRequest {
            account_root_public_key: account_root_public_key.to_string(),
            namespaces,
        };

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.pair_device_init(request).await });
            Self::to_python(py, result)
        })
    }

    /// Certify a device another node minted, link it, and deliver the scope keys -
    /// the second half of pairing.
    ///
    /// Run on the node holding the account. `applications` scopes the device by
    /// application rather than by namespace, because that is the question a person
    /// can answer; naming none means every namespace this node takes part in.
    ///
    /// `statement` is not optional: without it the keys above are only claims by
    /// the sender, and certifying them would make attacker-supplied keys a trusted
    /// device of this account. `confirmation_code` is checked against the keys that
    /// actually arrived, so the comparison a human is meant to make cannot be
    /// skipped.
    #[pyo3(signature = (
        device_id,
        kem_public_key,
        sign_public_key,
        statement,
        confirmation_code,
        applications=None,
    ))]
    pub fn pair_device_complete(
        &self,
        device_id: &str,
        kem_public_key: &str,
        sign_public_key: &str,
        statement: &str,
        confirmation_code: &str,
        applications: Option<Vec<String>>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let request = admin::AccountPairCompleteApiRequest {
            device_id: device_id.to_string(),
            kem_public_key: kem_public_key.to_string(),
            sign_public_key: sign_public_key.to_string(),
            statement: statement.to_string(),
            confirmation_code: confirmation_code.to_string(),
            applications: applications.unwrap_or_default(),
        };

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.pair_device_complete(request).await });
            Self::to_python(py, result)
        })
    }

    /// Repair or widen the reach of a device this account already certified.
    ///
    /// Re-runs pairing's fan-out against the namespaces this node takes part in
    /// now, closing the drift a namespace gained after pairing leaves behind. No
    /// handshake and no confirmation code, and the device need not be online.
    ///
    /// `applications` here means the opposite of what it means on
    /// `pair_device_complete`: naming none repairs WITHOUT widening, rather than
    /// meaning every application, so the accidental request is not the widest one.
    /// Returns the device's scope after the request and what the repair did in
    /// each namespace, since publication is per-DAG.
    #[pyo3(signature = (device_id, applications=None))]
    pub fn relink_device(
        &self,
        device_id: &str,
        applications: Option<Vec<String>>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let device_id = device_id.to_string();
        let request = admin::RelinkDeviceApiRequest {
            applications: applications.unwrap_or_default(),
        };

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.relink_device(&device_id, request).await });
            Self::to_python(py, result)
        })
    }

    /// Every device of this account, with the scope and bindings this node sees.
    ///
    /// Joined from the node-local certificate cache and the live bindings of
    /// every namespace this node takes part in, so a device certified elsewhere
    /// is reported too, with no scope this node can know.
    pub fn list_account_devices(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_account_devices().await });
            Self::to_python(py, result)
        })
    }

    /// The applications this account speaks in, and the namespaces serving each.
    ///
    /// The only route by which a paired device can learn them: it is a member of
    /// nothing, and a namespace summary is withheld from non-members.
    pub fn list_account_applications(&self) -> PyResult<PyObject> {
        let inner = self.inner.clone();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_account_applications().await });
            Self::to_python(py, result)
        })
    }

    /// Withdraw a device from its account, terminally.
    ///
    /// Three ways this is authorized, and `proof` is the third:
    ///
    /// - the calling node is a group **admin**, which also rotates the scope key
    ///   in the same op;
    /// - it holds the **account root** that owns the device, so it mints a proof
    ///   itself — that path cannot rotate, so the device stops writing at once and
    ///   keeps reading until an admin rotates;
    /// - `proof` carries one minted **elsewhere** (`merod account revoke-proof`),
    ///   which is the lost-device case: the root never reaches a node and the node
    ///   publishing needs no authority of its own.
    ///
    /// `proof` is hex-encoded and not a secret — it authorises exactly one
    /// revocation of one device, and only alongside a stored binding that already
    /// names the same account.
    #[pyo3(signature = (namespace_id, device_id, proof=None))]
    pub fn revoke_device(
        &self,
        namespace_id: &str,
        device_id: &str,
        proof: Option<&str>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();
        let request = admin::RevokeDeviceApiRequest {
            device_id: device_id.to_string(),
            // Trimmed because the proof normally arrives from a file or captured
            // command output, and hex with a trailing newline is not hex.
            proof: proof.map(|p| p.trim().to_owned()),
        };

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.revoke_device(&namespace_id, request).await });
            Self::to_python(py, result)
        })
    }
    /// Ask this node to run one method on a member's behalf, under a warrant
    /// that member signed.
    ///
    /// The caller supplies only the author's half — the warrant and the proof
    /// that the signing key is a device of the account it names. The node
    /// attaches its own credential, so a client never learns which of the node's
    /// processes runs the intent, and the node re-keying does not void a warrant
    /// already issued to it.
    ///
    /// Mint the `warrant` with `sign_warrant`, which needs no connection.
    #[pyo3(signature = (context_id, method, args, warrant, author_proof))]
    pub fn perform_intent(
        &self,
        context_id: &str,
        method: &str,
        args: &str,
        warrant: &str,
        author_proof: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        // Validated as a ContextId even though the route takes a string, so a
        // bad id fails here naming itself rather than as a 400 from the node.
        let _ = context_id.trim().parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;
        let context_id = context_id.trim().to_owned();

        let args_value: serde_json::Value = serde_json::from_str(args).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid JSON args: {}", e))
        })?;

        // Trimmed for the same reason `revoke_device` trims its proof: both
        // normally arrive from captured command output, and hex with a trailing
        // newline is not hex.
        let request = admin::PerformIntentApiRequest {
            method: method.to_owned(),
            args_json: args_value,
            warrant: warrant.trim().to_owned(),
            author_proof: author_proof.trim().to_owned(),
        };

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.perform_intent(&context_id, request).await });
            Self::to_python(py, result)
        })
    }

    pub fn list_namespaces_for_application(&self, application_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let application_id = application_id.parse::<ApplicationId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid application ID '{}': {}",
                application_id, e
            ))
        })?;
        let application_id = application_id.to_string();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner.list_namespaces_for_application(&application_id).await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    #[pyo3(signature = (namespace_id, recursive=None, expiration_timestamp=None))]
    pub fn create_namespace_invitation(
        &self,
        namespace_id: &str,
        recursive: Option<bool>,
        expiration_timestamp: Option<u64>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .create_namespace_invitation(
                        &namespace_id,
                        admin::CreateGroupInvitationApiRequest {
                            expiration_timestamp,
                            recursive,
                        },
                    )
                    .await
            });
            match result {
                Ok(data) => Ok(json_to_python(py, &data)),
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }
    pub fn join_namespace(&self, namespace_id: &str, invitation_json: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();
        let invitation: calimero_context_config::types::SignedGroupOpenInvitation =
            serde_json::from_str(invitation_json).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid invitation JSON: {}",
                    e
                ))
            })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .join_namespace(
                        &namespace_id,
                        admin::JoinGroupApiRequest {
                            invitation,
                            group_name: None,
                        },
                    )
                    .await
            });
            Self::to_python(py, result)
        })
    }

    pub fn list_namespace_groups(&self, namespace_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_namespace_groups(&namespace_id).await });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    #[pyo3(signature = (namespace_id, group_name=None))]
    pub fn create_group_in_namespace(
        &self,
        namespace_id: &str,
        group_name: Option<&str>,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();
        let group_name = group_name.map(|s| s.to_string());

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .create_group_in_namespace(&namespace_id, group_name)
                    .await
            });
            match result {
                Ok(data) => Ok(json_to_python(py, &data)),
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Atomic edge swap: move `group_id` to a new parent. Replaces the
    /// previous nest/unnest pair — orphan state is no longer reachable.
    /// Returns `{ "reparented": bool }` (false on idempotent no-op when
    /// `group_id` already had `new_parent_id` as its parent).
    pub fn reparent_group(&self, group_id: &str, new_parent_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let new_parent_id = new_parent_id.to_string();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .reparent_group(&group_id, admin::ReparentGroupApiRequest { new_parent_id })
                    .await
            });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    pub fn list_subgroups(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_subgroups(&group_id).await });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Get group information
    pub fn get_group_info(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_group_info(&group_id).await });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Delete a group
    pub fn delete_group(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let request = admin::DeleteGroupApiRequest {};
                inner.delete_group(&group_id, request).await
            });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Join a context (via group membership, context_id in path)
    pub fn join_context(&self, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;
        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let cid_str = context_id.to_string();
                inner.join_context(&cid_str).await
            });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Materialise an inherited Open-subgroup membership without an
    /// admin-signed invitation and without first joining a child
    /// context.
    ///
    /// Wraps `POST /admin-api/groups/:group_id/join-via-inheritance`.
    /// On success returns `{group_id, member_public_key, was_inherited}`.
    /// `was_inherited` is `false` for callers who were already direct
    /// members (no-op); `true` for callers whose membership was
    /// materialised via `RootOp::MemberJoinedOpen`. Returns HTTP 403
    /// if the caller has no inheritance path, 404 if the subgroup is
    /// not visible to this node.
    pub fn join_subgroup_inheritance(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.join_subgroup_inheritance(&group_id).await });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Leave a context locally on this node (no DAG op published).
    /// Stops sync, disarms auto-follow. Reversible by calling
    /// `join_context` again.
    pub fn leave_context(&self, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let context_id = context_id.parse::<ContextId>().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid context ID '{}': {}",
                context_id, e
            ))
        })?;
        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let cid_str = context_id.to_string();
                inner.leave_context(&cid_str).await
            });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Self-leave from a single group. Publishes `MemberLeft` so peers
    /// observe the leave. Subject to apply-side checks: must be a
    /// direct member, not the Owner, not the only admin.
    pub fn leave_group(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.leave_group(&group_id).await });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Self-leave from a namespace (root group). Cascades through
    /// every descendant where this node has a direct row. Rejects
    /// with `MustTransferOwnership` if the leaver owns any group in
    /// the subtree.
    pub fn leave_namespace(&self, namespace_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();
        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.leave_namespace(&namespace_id).await });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }
    /// The node names members by account; this returns whatever it sends.
    pub fn list_group_members(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_group_members(&group_id).await });
            Self::to_python(py, result)
        })
    }

    /// List contexts in a group
    pub fn list_group_contexts(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.list_group_contexts(&group_id).await });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Add members to a group.
    ///
    /// Each member is named by ACCOUNT - 64 hex, the ids `list_group_members`
    /// returns - or by a bs58 signing key, for a subject this node holds no
    /// account for yet. `MemberIdentity` reads whichever it is given.
    pub fn add_group_members(&self, group_id: &str, members_json: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let members: Vec<serde_json::Value> = serde_json::from_str(members_json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid members JSON: {}", e))
        })?;
        let api_members: Vec<admin::GroupMemberApiInput> = members
            .iter()
            .map(|m| {
                let identity_str = m.get("identity").and_then(|v| v.as_str()).unwrap_or("");
                let role_str = m.get("role").and_then(|v| v.as_str()).unwrap_or("Member");
                let identity = identity_str
                    .parse::<identity::MemberIdentity>()
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Invalid member identity '{}': {}",
                            identity_str, e
                        ))
                    })?;
                let role = match role_str {
                    "Admin" => GroupMemberRole::Admin,
                    "ReadOnly" => GroupMemberRole::ReadOnly,
                    _ => GroupMemberRole::Member,
                };
                Ok(admin::GroupMemberApiInput { identity, role })
            })
            .collect::<PyResult<Vec<_>>>()?;
        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let request = admin::AddGroupMembersApiRequest {
                    members: api_members,
                };
                inner.add_group_members(&group_id, request).await
            });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }
    /// Members are named by ACCOUNT — 64 hex, the ids `list_group_members`
    /// returns and the ids the membership rows are keyed by.
    ///
    /// Parsed rather than passed through as text: `RemoveGroupMembersApiRequest`
    /// declares `Vec<AccountId>`, so a malformed id fails here with the caller's
    /// traceback intact instead of as a rejection from the node.
    pub fn remove_group_members(&self, group_id: &str, members_json: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let members: Vec<AccountId> = serde_json::from_str(members_json).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid members JSON — members are accounts, 64 hex characters: {}",
                e
            ))
        })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .remove_group_members(
                        &group_id,
                        admin::RemoveGroupMembersApiRequest { members },
                    )
                    .await
            });
            Self::to_python(py, result)
        })
    }

    /// Set member capabilities in a group
    pub fn set_member_capabilities(
        &self,
        group_id: &str,
        member_id: &str,
        capabilities: u32,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let member_id = member_id.to_string();
        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let request = admin::SetMemberCapabilitiesApiRequest { capabilities };
                inner
                    .set_member_capabilities(&group_id, &member_id, request)
                    .await
            });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Set per-member auto-follow flags on a group.
    ///
    /// Authorized by group admin (for any `member_id`) or by the target itself
    /// (self-setting); apply path enforces admin-or-self. The node resolves the
    /// acting identity from the authenticated session.
    pub fn set_member_auto_follow(
        &self,
        group_id: &str,
        member_id: &str,
        auto_follow_contexts: bool,
        auto_follow_subgroups: bool,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let member_id = member_id.to_string();
        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                let request = admin::SetMemberAutoFollowApiRequest {
                    auto_follow_contexts,
                    auto_follow_subgroups,
                };
                inner
                    .set_member_auto_follow(&group_id, &member_id, request)
                    .await
            });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Get member capabilities in a group
    pub fn get_member_capabilities(&self, group_id: &str, member_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let member_id = member_id.to_string();
        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner.get_member_capabilities(&group_id, &member_id).await
            });
            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    pub fn set_group_metadata(&self, group_id: &str, body_json: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let request: admin::SetMetadataApiRequest =
            serde_json::from_str(body_json).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid metadata JSON: {}",
                    e
                ))
            })?;

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.set_group_metadata(&group_id, request).await });
            Self::to_python(py, result)
        })
    }

    pub fn set_member_metadata(
        &self,
        group_id: &str,
        member_id: &str,
        body_json: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let member_id = member_id.to_string();
        let request: admin::SetMetadataApiRequest =
            serde_json::from_str(body_json).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid metadata JSON: {}",
                    e
                ))
            })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .set_member_metadata(&group_id, &member_id, request)
                    .await
            });
            Self::to_python(py, result)
        })
    }

    pub fn set_context_metadata(
        &self,
        group_id: &str,
        context_id: &str,
        body_json: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let context_id = context_id.to_string();
        let request: admin::SetMetadataApiRequest =
            serde_json::from_str(body_json).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid metadata JSON: {}",
                    e
                ))
            })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .set_context_metadata(&group_id, &context_id, request)
                    .await
            });
            Self::to_python(py, result)
        })
    }

    pub fn get_group_metadata(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_group_metadata(&group_id).await });
            Self::to_python(py, result)
        })
    }

    pub fn get_member_metadata(&self, group_id: &str, member_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let member_id = member_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_member_metadata(&group_id, &member_id).await });
            Self::to_python(py, result)
        })
    }

    pub fn get_context_metadata(&self, group_id: &str, context_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let context_id = context_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_context_metadata(&group_id, &context_id).await });
            Self::to_python(py, result)
        })
    }

    pub fn update_member_role(
        &self,
        group_id: &str,
        member_id: &str,
        role: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let member_id = member_id.to_string();
        let role = parse_group_member_role(role)?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .update_member_role(
                        &group_id,
                        &member_id,
                        admin::UpdateMemberRoleApiRequest { role },
                    )
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    pub fn set_default_capabilities(
        &self,
        group_id: &str,
        capabilities: u32,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .set_default_capabilities(
                        &group_id,
                        admin::SetDefaultCapabilitiesApiRequest {
                            default_capabilities: capabilities,
                        },
                    )
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    pub fn set_subgroup_visibility(&self, group_id: &str, visibility: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let visibility = visibility.to_ascii_lowercase();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .set_subgroup_visibility(
                        &group_id,
                        admin::SetSubgroupVisibilityApiRequest {
                            subgroup_visibility: visibility,
                        },
                    )
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Deprecated alias for [`Self::set_subgroup_visibility`]. Kept so older
    /// merobox releases that still emit `set_default_visibility` keep working
    /// while the wider ecosystem rolls forward to the renamed surface (issue
    /// calimero-network/core#2256). New callers should use
    /// `set_subgroup_visibility` directly.
    pub fn set_default_visibility(&self, group_id: &str, visibility: &str) -> PyResult<PyObject> {
        self.set_subgroup_visibility(group_id, visibility)
    }

    pub fn sync_group(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .sync_group(&group_id, admin::SyncGroupApiRequest {})
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Initiate a target-application upgrade for a group.
    ///
    /// When `cascade=False` (default), upgrades only `group_id` — the
    /// existing per-group flow.
    ///
    /// When `cascade=True`, publishes `CascadeTargetApplicationSet`
    /// against `group_id` (treated as a namespace root): the core
    /// cascade engine fans the upgrade out to **every matching
    /// descendant subgroup and context** in a single sync round
    /// (calimero-network/core#2493). Use deliberately — the blast
    /// radius is the entire subtree.
    #[pyo3(signature = (group_id, target_application_id, cascade=false))]
    pub fn upgrade_group(
        &self,
        group_id: &str,
        target_application_id: &str,
        cascade: bool,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let target_application_id =
            target_application_id
                .parse::<ApplicationId>()
                .map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid application ID '{}': {}",
                        target_application_id, e
                    ))
                })?;

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .upgrade_group(
                        &group_id,
                        admin::UpgradeGroupApiRequest {
                            // Default (false): a target build with no embedded ABI
                            // refuses the upgrade rather than proceeding code-only.
                            // Exposing an override belongs with a caller that can
                            // assert layout-compatibility; the binding should not
                            // decide that silently.
                            force_code_only: false,
                            target_application_id,
                            cascade,
                        },
                    )
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    pub fn get_group_upgrade_status(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_group_upgrade_status(&group_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Per-descendant cascade migration status across a namespace subtree.
    ///
    /// Returns `{data: [{groupId, upgrade, cascadeHlc}]}` — one entry per
    /// descendant group (including the namespace root) that carries a cascade
    /// upgrade record. Wraps `GET admin-api/groups/{namespace_id}/cascade-status`.
    pub fn get_cascade_status(&self, namespace_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_cascade_status(&namespace_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Pinned-cohort migration rollup for a namespace.
    ///
    /// Returns `{targetVersion, expectedMembers, rollup, members}`. The `rollup`
    /// carries per-`state` counts (`migrated`/`inProgress`/`unknown`/`failed`),
    /// `total`, `membersPendingSignature`, and the `allMigrated` flag. A stranded
    /// context surfaces as a member with `state: "failed"` and
    /// `report.migrationFailed: "no_migration_path"` (the `state`/reason strings
    /// stay snake_case; the dict keys are camelCase). Observability only — never
    /// gates a write or apply.
    /// Wraps `GET admin-api/groups/{namespace_id}/migration-status`.
    pub fn get_migration_status(&self, namespace_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.get_migration_status(&namespace_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    /// Logically abort an in-flight namespace migration.
    ///
    /// Flips the group's pending migration target back to the pre-migration app
    /// id and drops the pending marker, cascading to every descendant subgroup
    /// carrying the same pending migration. Idempotent (a subtree with nothing
    /// pending is a no-op). Returns `{namespaceId, aborted}`. Wraps
    /// `POST admin-api/groups/{namespace_id}/migration/abort`.
    pub fn abort_migration(&self, namespace_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let namespace_id = namespace_id.to_string();

        Python::with_gil(|py| {
            let result = self
                .runtime
                .block_on(async move { inner.abort_migration(&namespace_id).await });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    pub fn retry_group_upgrade(&self, group_id: &str) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .retry_group_upgrade(&group_id, admin::RetryGroupUpgradeApiRequest {})
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }

    pub fn detach_context_from_group(
        &self,
        group_id: &str,
        context_id: &str,
    ) -> PyResult<PyObject> {
        let inner = self.inner.clone();
        let group_id = group_id.to_string();
        let context_id = context_id.to_string();

        Python::with_gil(|py| {
            let result = self.runtime.block_on(async move {
                inner
                    .detach_context_from_group(
                        &group_id,
                        &context_id,
                        admin::DetachContextFromGroupApiRequest {},
                    )
                    .await
            });

            match result {
                Ok(data) => {
                    let json_data = serde_json::to_value(data).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                            "Failed to serialize response: {}",
                            e
                        ))
                    })?;
                    Ok(json_to_python(py, &json_data))
                }
                Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Client error: {}",
                    e
                ))),
            }
        })
    }
}

/// Create a new client
#[pyfunction]
pub fn create_client(connection: &PyConnectionInfo) -> PyResult<PyClient> {
    PyClient::new(connection)
}
