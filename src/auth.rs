//! Python wrapper for AuthMode, and the authenticator connections use

use async_trait::async_trait;
use calimero_client::auth::HeadlessAuthenticator;
use calimero_client::connection::AuthMode;
use calimero_client::{CliAuthenticator, ClientAuthenticator, JwtToken, Result, Url};
use eyre::WrapErr;
use pyo3::prelude::*;

/// Python wrapper for AuthMode
#[pyclass(name = "AuthMode")]
#[derive(Debug, Clone, Copy)]
pub struct PyAuthMode {
    pub(crate) mode: AuthMode,
}

#[pymethods]
impl PyAuthMode {
    #[new]
    pub fn new(mode: &str) -> PyResult<Self> {
        let mode = match mode.to_lowercase().as_str() {
            "required" => AuthMode::Required,
            "none" => AuthMode::None,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "AuthMode must be 'required' or 'none'",
                ))
            }
        };
        Ok(Self { mode })
    }

    #[getter]
    pub fn value(&self) -> &str {
        match self.mode {
            AuthMode::Required => "required",
            AuthMode::None => "none",
        }
    }

    fn __str__(&self) -> &str {
        self.value()
    }

    fn __repr__(&self) -> String {
        format!("AuthMode('{}')", self.value())
    }
}

/// Core's headless and stdin-prompting authenticators as one type, since a
/// pyclass cannot be generic. Prompting is opt-in: a library must not block on stdin.
#[derive(Clone, Debug)]
pub enum Authenticator {
    Headless(HeadlessAuthenticator),
    Interactive(CliAuthenticator),
}

impl Authenticator {
    pub fn new(interactive: bool) -> Self {
        if interactive {
            Self::Interactive(CliAuthenticator::new())
        } else {
            Self::Headless(HeadlessAuthenticator::new())
        }
    }

    fn inner(&self) -> &dyn ClientAuthenticator {
        match self {
            Self::Headless(authenticator) => authenticator,
            Self::Interactive(authenticator) => authenticator,
        }
    }
}

#[async_trait]
impl ClientAuthenticator for Authenticator {
    async fn authenticate(&self, api_url: &Url) -> Result<JwtToken> {
        let result = self.inner().authenticate(api_url).await;
        match self {
            Self::Headless(_) => result.wrap_err(
                "no usable token is stored for this node: save one to its token cache, \
                 or pass interactive=True to be prompted for one",
            ),
            Self::Interactive(_) => result,
        }
    }

    async fn refresh_tokens(&self, refresh_token: &str) -> Result<JwtToken> {
        self.inner().refresh_tokens(refresh_token).await
    }

    async fn handle_auth_failure(&self, api_url: &Url) -> Result<JwtToken> {
        self.inner().handle_auth_failure(api_url).await
    }

    async fn check_auth_required(&self, api_url: &Url) -> Result<bool> {
        self.inner().check_auth_required(api_url).await
    }

    fn get_auth_method(&self) -> &'static str {
        self.inner().get_auth_method()
    }

    fn supports_refresh(&self) -> bool {
        self.inner().supports_refresh()
    }
}

#[cfg(test)]
mod tests {
    use super::{Authenticator, ClientAuthenticator, Url};

    #[test]
    fn only_an_opt_in_prompts() {
        assert!(matches!(
            Authenticator::new(false),
            Authenticator::Headless(_)
        ));
        assert!(matches!(
            Authenticator::new(true),
            Authenticator::Interactive(_)
        ));
    }

    #[test]
    fn the_default_fails_with_a_way_out_instead_of_prompting() {
        let url = Url::parse("http://127.0.0.1:2428").unwrap();
        let error = tokio_test::block_on(Authenticator::new(false).authenticate(&url)).unwrap_err();
        assert!(error.to_string().contains("interactive=True"));
    }
}
