//! Accounts: sign-up and sign-in, passkeys, sessions, API keys, usage and aliases.

use super::prelude::*;

#[pymethods]
impl PyCloudClient {
    /// Create an account with a new passkey and sign in. Returns the session.
    fn sign_up(&self, py: Python<'_>, passkey: &PySoftPasskey, name: &str) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let name = name.to_owned();
        let s = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.sign_up(&mut *auth, &name).await
        })?;
        self.forget_api_key();
        to_py(py, &s)
    }

    /// Sign in with an existing passkey. Returns the session.
    fn sign_in(&self, py: Python<'_>, passkey: &PySoftPasskey) -> PyResult<PyObject> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        let s = self.run(py, async move {
            let mut auth = pk.lock().await;
            c.sign_in(&mut *auth).await
        })?;
        self.forget_api_key();
        to_py(py, &s)
    }

    /// Add `passkey` (another device) to the signed-in account. Returns its
    /// credential id.
    fn add_passkey(&self, py: Python<'_>, passkey: &PySoftPasskey) -> PyResult<String> {
        let (c, pk) = (Arc::clone(&self.inner), Arc::clone(&passkey.inner));
        self.run(py, async move {
            let mut auth = pk.lock().await;
            c.add_passkey(&mut *auth).await
        })
    }

    /// Exchange the refresh token for a new session.
    fn refresh(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let s = self.run(py, async move { c.refresh().await })?;
        to_py(py, &s)
    }

    /// Authenticate every request with an API key (`cal_...`).
    fn use_api_key(&self, py: Python<'_>, key: &str) -> PyResult<()> {
        *self
            .api_key
            .lock()
            .map_err(|_| PyValueError::new_err("the API key lock is poisoned"))? =
            Some(key.to_owned());
        let (c, key) = (Arc::clone(&self.inner), key.to_owned());
        self.run(py, async move {
            c.use_api_key(&key).await;
            Ok(())
        })
    }

    /// Authenticate with a session returned by `sign_up`/`sign_in`.
    fn use_session(&self, py: Python<'_>, session: &Bound<'_, PyAny>) -> PyResult<()> {
        let s: Session = serde_json::from_value(to_json(py, Some(session))?)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let c = Arc::clone(&self.inner);
        self.run(py, async move {
            c.use_session(s).await;
            Ok(())
        })?;
        self.forget_api_key();
        Ok(())
    }

    /// The session in use (`sign_up`, `sign_in`, `refresh`, `use_session`),
    /// or `None` (none yet, or an API key instead).
    fn session(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let s = self.run(py, async move { Ok(c.session().await) })?;
        to_py(py, &s)
    }

    fn me(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.me().await })?;
        to_py(py, &v)
    }

    /// Create an API key. Returns `(key_id, key)`; the key is shown once.
    /// `options` may set `label`, `apps` and `methods` (scopes), `readOnly`,
    /// `allowConfirm` and `ttlSecs` (at most a year).
    #[pyo3(signature = (options = None))]
    fn create_api_key(
        &self,
        py: Python<'_>,
        options: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<(String, String)> {
        let (c, o) = (Arc::clone(&self.inner), to_json(py, options)?);
        self.run(py, async move { c.create_api_key(o).await })
    }

    /// End this session on the node (its refresh token stops working).
    fn sign_out(&self, py: Python<'_>) -> PyResult<()> {
        self.send(py, Verb::Post, "auth/logout".into(), Value::Null)
            .map(drop)
    }

    /// End every session of this account, on every device.
    fn sign_out_everywhere(&self, py: Python<'_>) -> PyResult<()> {
        let c = Arc::clone(&self.inner);
        self.run(py, async move { c.sign_out_everywhere().await })
            .map(drop)
    }

    /// Remove a passkey (a device) from the signed-in account.
    fn remove_passkey(&self, py: Python<'_>, credential_id: &str) -> PyResult<()> {
        self.send(
            py,
            Verb::Delete,
            format!("auth/passkeys/{}", seg(credential_id)),
            Value::Null,
        )
        .map(drop)
    }

    /// The account's API keys (their records; the keys themselves are never shown again).
    fn list_api_keys(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, "auth/apikeys".into(), Value::Null)?;
        to_py(py, &v["keys"])
    }

    /// Revoke an API key by its id.
    fn revoke_api_key(&self, py: Python<'_>, key_id: &str) -> PyResult<()> {
        self.send(
            py,
            Verb::Delete,
            format!("auth/apikeys/{}", seg(key_id)),
            Value::Null,
        )
        .map(drop)
    }

    /// What this account uses, with the cluster's quotas.
    fn usage(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.usage().await })?;
        to_py(py, &v)
    }

    /// Name one of this account's passkeys; `""` clears the name.
    fn label_passkey(&self, py: Python<'_>, credential_id: &str, label: &str) -> PyResult<()> {
        let (c, cred, label) = (
            Arc::clone(&self.inner),
            credential_id.to_owned(),
            label.to_owned(),
        );
        self.run(py, async move { c.label_passkey(&cred, &label).await })
            .map(drop)
    }

    /// Name `app` for this account (`alias` then reaches it wherever an app id does).
    fn set_alias(&self, py: Python<'_>, alias: &str, app: &str) -> PyResult<()> {
        let (c, alias, app) = (Arc::clone(&self.inner), alias.to_owned(), app.to_owned());
        self.run(py, async move { c.set_alias(&alias, &app).await })
            .map(drop)
    }

    /// The app an alias names: `{"alias", "appId"}`.
    fn alias(&self, py: Python<'_>, alias: &str) -> PyResult<PyObject> {
        let (c, alias) = (Arc::clone(&self.inner), alias.to_owned());
        let v = self.run(py, async move { c.alias(&alias).await })?;
        to_py(py, &v)
    }

    /// Every alias of this account: `{alias: app_id}`.
    fn aliases(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = Arc::clone(&self.inner);
        let v = self.run(py, async move { c.aliases().await })?;
        to_py(py, &v["aliases"])
    }

    fn delete_alias(&self, py: Python<'_>, alias: &str) -> PyResult<()> {
        let (c, alias) = (Arc::clone(&self.inner), alias.to_owned());
        self.run(py, async move { c.delete_alias(&alias).await })
            .map(drop)
    }
}
