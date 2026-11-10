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

    /// Who the caller is (`GET /auth/me`): `accountId`, `name`, `passkeys`
    /// (each `{id, label, signCount, addedAt, lastSignInAt}`), `orgs` (each
    /// `{orgId, name, role}`), `via` (`"session"` or `"apiKey"`), `passkeyId`
    /// (the passkey this session was opened with; `None` for an API key), and
    /// `app` and `origin` for a session signed in from a package's origin.
    ///
    /// A passkey's `addedAt` and `lastSignInAt` (Unix seconds; core #4894)
    /// say when it was added and last signed in (a sign-up, a sign-in or an
    /// account recovery's finish; a refresh does not move it). `None` for
    /// what happened on a release that kept no times; absent on such a node
    /// (1.0.0-rc.14 and earlier).
    fn me(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.run(py, async |c| c.me().await)?;
        to_py(py, &v)
    }

    /// Create an API key, approved with the account's `passkey` (the node's
    /// `412 CONFIRM_REQUIRED`; a session's, never an API key's). Returns
    /// `(key_id, key)`; the key is shown once. `options` (or `None`) may set
    /// `label`, `apps` and `methods` (scopes), `readOnly`, `allowConfirm` and
    /// `ttlSecs` (at most a year).
    ///
    /// `upgrades: {"app", "package"?}` makes an upgrade key, for CI (core
    /// 1.0.0-rc.10): it upgrades that one app to signed releases of its
    /// package (default: the one the app runs, by that package's signer
    /// pinned now) with `upgrade_app(app, version=...)` and no passkey, and
    /// every other request with it is refused `403 UPGRADE_KEY_ONLY`. It
    /// takes no other scope field (`400 BAD_UPGRADE_KEY`); the caller must
    /// be one who may upgrade the app. Its record in `list_api_keys` names
    /// the key's public half, `upgrades.proofKey` (`api_key_public(key)`).
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

    /// The Ed25519 public key (hex) an API key stands for, byte-for-byte
    /// core's `proof::api_key_public`: the seed is `SHA-256("calimero.api-key.v1\0"
    /// || key)`. An upgrade key's record (`list_api_keys`) keeps it as
    /// `upgrades.proofKey`, and the node signs each upgrade the key makes
    /// with its private half, so the directory's log proves the key holder
    /// asked for it. Compare the two to check which key a record is.
    #[staticmethod]
    fn api_key_public(key: &str) -> String {
        hex::encode(api_key_signing_key(key).verifying_key().to_bytes())
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

    /// The account's live sessions (`GET /auth/sessions`, core #4894), latest
    /// active first, at most 256: a list of `{session, passkey, app, origin,
    /// openedAt, lastActiveAt, current}`. `passkey` is the passkey it was
    /// opened with, `app` and `origin` the package and registered origin it
    /// is bound to (`None`: the cluster's own origin), `openedAt` when it
    /// opened (`None`: before the node kept it), `current` marks this one.
    /// `lastActiveAt` is when its refresh token was last exchanged; the
    /// session may have been used up to about 1 h 15 min later. The account
    /// holder's own session on the cluster's origin only: an API key or a
    /// package-bound session is refused `403`. A node without the route
    /// answers a bare `404`.
    fn sessions(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, "auth/sessions".into(), Value::Null)?;
        to_py(py, &v["sessions"])
    }

    /// End one of the account's sessions (`DELETE /auth/sessions/{session}`),
    /// a lost device's say, from another; no passkey is asked. A session the
    /// account does not hold live (ended already, or another account's) is
    /// `404 NOT_FOUND`; gated as `sessions`. Ending this client's own
    /// session leaves it holding dead tokens: use `sign_out` for that.
    fn revoke_session(&self, py: Python<'_>, session: &str) -> PyResult<()> {
        let path = format!("auth/sessions/{}", seg(session));
        self.send(py, Verb::Delete, path, Value::Null).map(drop)
    }

    /// What the account's passkeys did (`GET /auth/activity`, core #4901),
    /// newest first, the last 100: a list of `{at, passkey, kind, app,
    /// origin, method, subject}`. `kind` is `"signIn"` (a sign-up, a sign-in
    /// or a recovery's finish; a refresh is none), with the package `app` and
    /// registered `origin` its session is bound to (`None`: the cluster's own
    /// origin), or `"approval"` (a change the passkey approved with a
    /// step-up), with the `method` and `subject` its consent states. `after`
    /// (Unix seconds) keeps only what came later. Gated as `sessions`: the
    /// account holder's own session on the cluster's origin only. A node
    /// without the route answers a bare `404`.
    #[pyo3(signature = (after = None))]
    fn activity(&self, py: Python<'_>, after: Option<u64>) -> PyResult<PyObject> {
        let path = after.map_or_else(
            || "auth/activity".to_owned(),
            |t| format!("auth/activity?after={t}"),
        );
        let v = self.send(py, Verb::Get, path, Value::Null)?;
        to_py(py, &v["activity"])
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

/// What an API key's Ed25519 key is derived from, before the key (core's
/// `proof::API_KEY_DOMAIN`).
const API_KEY_DOMAIN: &[u8] = b"calimero.api-key.v1\0";

/// The Ed25519 key an API key stands for (core's `proof::api_key_signing_key`).
fn api_key_signing_key(key: &str) -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&calimero_identity::sha256(
        &[API_KEY_DOMAIN, key.as_bytes()].concat(),
    ))
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
pub(super) async fn use_answer(c: &Client, done: Value) -> Result<Session, Error> {
    let s: Session =
        serde_json::from_value(done).map_err(|e| Error::Transport(eyre::Report::from(e)))?;
    c.use_session(s.clone()).await;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Computed apart from this code (Python's `cryptography`) from core's
    /// definition: `SHA-256("calimero.api-key.v1\0" || key)` as the Ed25519
    /// seed, the public key in hex.
    #[test]
    fn an_api_keys_public_key_is_cores() {
        for (key, seed, public) in [
            (
                "cal_0123456789abcdef",
                "d1a9611e1b88cf3dffd051bec6ae3826b8d506c60f9e1492139a2c475d1f23f3",
                "fb2a698c59f05bb66aaedd4badecbce8efb5a9abe976906c0eed14e48da14adf",
            ),
            (
                "cal_ci",
                "27953e2a85fa994e5b77b053c474015c5af18ee97ec58aae4d6fdbd7e5411f3a",
                "288164f1279d77ba71dd86e20149ecf9398f0e884bad8644a096ecefd5555a27",
            ),
        ] {
            assert_eq!(hex::encode(api_key_signing_key(key).to_bytes()), seed);
            assert_eq!(PyCloudClient::api_key_public(key), public);
        }
    }
}
