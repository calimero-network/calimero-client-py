//! Accounts: sign-up and sign-in, passkeys, sessions, API keys, usage and aliases.

use super::prelude::*;

#[pymethods]
impl PyCloudClient {
    /// Create an account with a new passkey and sign in. Returns the session.
    /// The node then asks the new passkey to sign the account it creates
    /// (`412 CONFIRM_REQUIRED`), which `passkey` answers; a node from before
    /// that answers the registration at once. `origin`: the web origin the
    /// passkey is made for (one a package registered, `add_package_origin`);
    /// without it, the node's own.
    #[pyo3(signature = (passkey, name, origin = None))]
    fn sign_up(
        &self,
        py: Python<'_>,
        passkey: &PySoftPasskey,
        name: &str,
        origin: Option<String>,
    ) -> PyResult<PyObject> {
        let pk = Arc::clone(&passkey.inner);
        let name = name.to_owned();
        let s = self.run_once(py, async |c| {
            let mut auth = pk.lock().await;
            sign_up_from(&c, &mut *auth, &name, origin.as_deref()).await
        })?;
        self.forget_api_key();
        to_py(py, &s)
    }

    /// Sign in with an existing passkey. Returns the session. `origin` as
    /// `sign_up`: the session is then the app's that registered it.
    #[pyo3(signature = (passkey, origin = None))]
    fn sign_in(
        &self,
        py: Python<'_>,
        passkey: &PySoftPasskey,
        origin: Option<String>,
    ) -> PyResult<PyObject> {
        let pk = Arc::clone(&passkey.inner);
        let s = self.run_once(py, async |c| {
            let mut auth = pk.lock().await;
            match &origin {
                None => c.sign_in(&mut *auth).await,
                Some(o) => sign_in_from(&c, &mut *auth, o).await,
            }
        })?;
        self.forget_api_key();
        to_py(py, &s)
    }

    /// Add `passkey` (another device) to the signed-in account, approved
    /// with `approver`, a passkey the account already has (the node's `412
    /// CONFIRM_REQUIRED`). Returns the new credential's id.
    fn add_passkey(
        &self,
        py: Python<'_>,
        passkey: &PySoftPasskey,
        approver: &PySoftPasskey,
    ) -> PyResult<String> {
        if Arc::ptr_eq(&passkey.inner, &approver.inner) {
            return Err(PyValueError::new_err(
                "approve a new passkey with one the account already has, not itself",
            ));
        }
        let (new, approver) = (Arc::clone(&passkey.inner), Arc::clone(&approver.inner));
        self.run(py, async |c| {
            let mut new = new.lock().await;
            let mut approver = approver.lock().await;
            c.add_passkey(&mut *new, &mut *approver).await
        })
    }

    /// Exchange the refresh token for a new session, on the node in use (any
    /// node of the cluster takes it). One at a time with the refreshes the
    /// client makes on its own after a `401`: a refresh token is used once.
    fn refresh(&self, py: Python<'_>) -> PyResult<PyObject> {
        let nodes = Arc::clone(&self.nodes);
        let s = self.block(py, async move { nodes.refresh().await })?;
        to_py(py, &s)
    }

    /// Authenticate every request with an API key (`cal_...`).
    fn use_api_key(&self, py: Python<'_>, key: &str) -> PyResult<()> {
        self.nodes.set_api_key(Some(key.to_owned()));
        let (c, key) = (self.client(), key.to_owned());
        self.block(py, async move {
            c.use_api_key(&key).await;
            Ok(())
        })
    }

    /// Authenticate with a session returned by `sign_up`/`sign_in`.
    fn use_session(&self, py: Python<'_>, session: &Bound<'_, PyAny>) -> PyResult<()> {
        let s: Session = serde_json::from_value(to_json(py, Some(session))?)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let c = self.client();
        self.block(py, async move {
            c.use_session(s).await;
            Ok(())
        })?;
        self.forget_api_key();
        Ok(())
    }

    /// The session in use (`sign_up`, `sign_in`, `refresh`, `use_session`),
    /// or `None` (none yet, or an API key instead).
    fn session(&self, py: Python<'_>) -> PyResult<PyObject> {
        let c = self.client();
        let s = self.block(py, async move { Ok(c.session().await) })?;
        to_py(py, &s)
    }

    fn me(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.run(py, async |c| c.me().await)?;
        to_py(py, &v)
    }

    /// Create an API key, approved with the account's `passkey` (the node's
    /// `412 CONFIRM_REQUIRED`; a session's, never an API key's). Returns
    /// `(key_id, key)`; the key is shown once. `options` (or `None`) may set
    /// `label`, `apps` and `methods` (scopes), `readOnly`, `allowConfirm` and
    /// `ttlSecs` (at most a year).
    #[pyo3(signature = (options, passkey))]
    fn create_api_key(
        &self,
        py: Python<'_>,
        options: Option<&Bound<'_, PyAny>>,
        passkey: &PySoftPasskey,
    ) -> PyResult<(String, String)> {
        let o = to_json(py, options)?;
        let o = if o.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            o
        };
        let pk = Arc::clone(&passkey.inner);
        self.run(py, async |c| {
            let mut auth = pk.lock().await;
            c.create_api_key(o.clone(), &mut *auth).await
        })
    }

    /// End this session on the node (its refresh token stops working).
    fn sign_out(&self, py: Python<'_>) -> PyResult<()> {
        self.send(py, Verb::Post, "auth/logout".into(), Value::Null)
            .map(drop)
    }

    /// End every session of this account, on every device.
    fn sign_out_everywhere(&self, py: Python<'_>) -> PyResult<()> {
        self.run(py, async |c| c.sign_out_everywhere().await)
            .map(drop)
    }

    /// Remove a passkey (a device) from the signed-in account, approved with
    /// `passkey`, one the account has.
    fn remove_passkey(
        &self,
        py: Python<'_>,
        credential_id: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<()> {
        let (pk, cred) = (Arc::clone(&passkey.inner), credential_id.to_owned());
        self.run(py, async |c| {
            let mut auth = pk.lock().await;
            c.remove_passkey(&cred, &mut *auth).await
        })
        .map(drop)
    }

    /// The account's API keys (their records; the keys themselves are never shown again).
    fn list_api_keys(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, "auth/apikeys".into(), Value::Null)?;
        to_py(py, &v["keys"])
    }

    /// Revoke an API key by its id, approved with the account's `passkey`
    /// (the node's `412 CONFIRM_REQUIRED`).
    fn revoke_api_key(
        &self,
        py: Python<'_>,
        key_id: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<()> {
        let (pk, path) = (
            Arc::clone(&passkey.inner),
            format!("auth/apikeys/{}", seg(key_id)),
        );
        self.run(py, async |c| {
            let mut auth = pk.lock().await;
            c.delete_confirmed(&path, &mut *auth).await
        })
        .map(drop)
    }

    /// What this account uses, with the cluster's quotas.
    fn usage(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.run(py, async |c| c.usage().await)?;
        to_py(py, &v)
    }

    /// Name one of this account's passkeys; `""` clears the name.
    fn label_passkey(&self, py: Python<'_>, credential_id: &str, label: &str) -> PyResult<()> {
        let (cred, label) = (credential_id.to_owned(), label.to_owned());
        self.run(py, async |c| c.label_passkey(&cred, &label).await)
            .map(drop)
    }

    /// Name `app` for this account (`alias` then reaches it wherever an app id does).
    fn set_alias(&self, py: Python<'_>, alias: &str, app: &str) -> PyResult<()> {
        let (alias, app) = (alias.to_owned(), app.to_owned());
        self.run(py, async |c| c.set_alias(&alias, &app).await)
            .map(drop)
    }

    /// The app an alias names: `{"alias", "appId"}`.
    fn alias(&self, py: Python<'_>, alias: &str) -> PyResult<PyObject> {
        let alias = alias.to_owned();
        let v = self.run(py, async |c| c.alias(&alias).await)?;
        to_py(py, &v)
    }

    /// Every alias of this account: `{alias: app_id}`.
    fn aliases(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.run(py, async |c| c.aliases().await)?;
        to_py(py, &v["aliases"])
    }

    fn delete_alias(&self, py: Python<'_>, alias: &str) -> PyResult<()> {
        let alias = alias.to_owned();
        self.run(py, async |c| c.delete_alias(&alias).await)
            .map(drop)
    }
}

// Core's `sign_up` / `sign_in` send no `origin` with `begin`, so these are
// made here; the session is used as theirs use it.

/// Sign up: `register/begin` (naming `origin`, if any), the new passkey made
/// for the rp-id the node answers, and `register/finish`, whose `412
/// CONFIRM_REQUIRED` the same passkey answers (core's proofs at apply; a node
/// from before answers at once). Core's `sign_up` sends no `origin` and
/// answers no `412`, so it is made here.
async fn sign_up_from(
    c: &Client,
    passkey: &mut dyn Passkey,
    name: &str,
    origin: Option<&str>,
) -> Result<Session, Error> {
    let mut begin_body = serde_json::json!({ "name": name });
    if let (Some(b), Some(o)) = (begin_body.as_object_mut(), origin) {
        let _prev = b.insert("origin".into(), Value::from(o));
    }
    let begin = c.post("auth/passkey/register/begin", begin_body).await?;
    let registration = passkey.create(&begin["options"]);
    let done = c
        .post_confirmed(
            "auth/passkey/register/finish",
            serde_json::json!({ "challengeId": begin["challengeId"], "registration": registration }),
            passkey,
        )
        .await?;
    use_answer(c, done).await
}

/// [`Client::sign_in`] from `origin`.
async fn sign_in_from(
    c: &Client,
    passkey: &mut dyn Passkey,
    origin: &str,
) -> Result<Session, Error> {
    let begin = c
        .post(
            "auth/passkey/login/begin",
            serde_json::json!({ "origin": origin }),
        )
        .await?;
    let challenge =
        calimero_identity::from_b64url(begin["options"]["challenge"].as_str().unwrap_or_default())
            .map_err(|e| Error::Transport(eyre::eyre!("the node's challenge: {e}")))?;
    let assertion = passkey.get(&challenge);
    let done = c
        .post(
            "auth/passkey/login/finish",
            serde_json::json!({ "challengeId": begin["challengeId"], "assertion": assertion }),
        )
        .await?;
    use_answer(c, done).await
}

/// The session a `finish` answered, made the client's.
async fn use_answer(c: &Client, done: Value) -> Result<Session, Error> {
    let s: Session =
        serde_json::from_value(done).map_err(|e| Error::Transport(eyre::Report::from(e)))?;
    c.use_session(s.clone()).await;
    Ok(s)
}
