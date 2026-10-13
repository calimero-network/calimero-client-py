//! Python wrapper for JwtToken

use calimero_client::JwtToken;
use pyo3::prelude::*;

fn access_token_preview(access_token: &str) -> &str {
    &access_token[..access_token.len().min(10)]
}

/// Python wrapper for JwtToken
#[pyclass(name = "JwtToken")]
#[derive(Debug, Clone)]
pub struct PyJwtToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<i64>,
}

#[pymethods]
impl PyJwtToken {
    #[new]
    #[pyo3(signature = (access_token, refresh_token=None, expires_at=None))]
    pub fn new(access_token: &str, refresh_token: Option<&str>, expires_at: Option<i64>) -> Self {
        Self {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.map(|s| s.to_string()),
            expires_at,
        }
    }

    #[getter]
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    #[getter]
    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    #[getter]
    pub fn expires_at(&self) -> Option<i64> {
        self.expires_at
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = chrono::Utc::now().timestamp();
            now >= expires_at
        } else {
            false
        }
    }

    fn __str__(&self) -> String {
        format!(
            "JwtToken(access_token='{}...', expires_at={:?})",
            access_token_preview(&self.access_token),
            self.expires_at
        )
    }
}

impl From<JwtToken> for PyJwtToken {
    fn from(token: JwtToken) -> Self {
        // Cloned, not moved: `JwtToken` implements `Drop` upstream (it wipes the
        // secret when it goes out of scope), and moving a field out of a `Drop`
        // type is not allowed — the destructor still has to see a whole value.
        Self {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.clone(),
            expires_at: token.expires_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::access_token_preview;

    #[test]
    fn preview_keeps_the_first_ten_characters() {
        assert_eq!(access_token_preview("abcdefghijklmnop"), "abcdefghij");
        assert_eq!(access_token_preview("short"), "short");
    }

    #[test]
    fn preview_does_not_split_a_multibyte_character() {
        let token = "\u{65e5}".repeat(12);
        assert_eq!(access_token_preview(&token), "\u{65e5}".repeat(10));
        assert_eq!(
            access_token_preview("a\u{65e5}\u{65e5}\u{65e5}\u{65e5}"),
            "a\u{65e5}\u{65e5}\u{65e5}\u{65e5}"
        );
    }
}
