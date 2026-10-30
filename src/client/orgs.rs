//! Organizations, their members and teams, and invites. What grants access
//! (members, roles, capabilities, team rows, invites) is approved with the
//! acting account's `passkey`: the node answers `412 CONFIRM_REQUIRED` first.

use super::prelude::*;

#[pymethods]
impl PyCloudClient {
    /// Create an organization; the caller owns it. Returns its id.
    fn create_org(&self, py: Python<'_>, name: &str) -> PyResult<String> {
        let v = self.send(
            py,
            Verb::Post,
            "orgs".into(),
            serde_json::json!({ "name": name }),
        )?;
        field(&v, "orgId")
    }

    /// An organization: members, roles and the teams the caller can see.
    fn get_org(&self, py: Python<'_>, org: &str) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, format!("orgs/{}", seg(org)), Value::Null)?;
        to_py(py, &v)
    }

    /// Add `account` to the organization, or change its role
    /// (`guest`, `member`, `admin`, `owner`), approved with `passkey`.
    fn set_member(
        &self,
        py: Python<'_>,
        org: &str,
        account: &str,
        role: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<()> {
        let body = serde_json::json!({ "account": account, "role": role });
        self.confirmed(py, format!("orgs/{}/members", seg(org)), body, passkey)
            .map(drop)
    }

    /// Remove `account` from the organization (yourself, to leave it),
    /// approved with `passkey`.
    fn remove_member(
        &self,
        py: Python<'_>,
        org: &str,
        account: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<()> {
        self.confirmed_delete(
            py,
            format!("orgs/{}/members/{}", seg(org), seg(account)),
            passkey,
        )
        .map(drop)
    }

    /// Set capability bits (v1's `MemberCapabilities`): for `account` (in
    /// `team`, if given), or, with no account, the organization's default;
    /// approved with `passkey`.
    #[pyo3(signature = (org, capabilities, passkey, account = None, team = None))]
    fn set_capabilities(
        &self,
        py: Python<'_>,
        org: &str,
        capabilities: u32,
        passkey: &PySoftPasskey,
        account: Option<&str>,
        team: Option<&str>,
    ) -> PyResult<()> {
        let body =
            serde_json::json!({ "capabilities": capabilities, "account": account, "team": team });
        self.confirmed(py, format!("orgs/{}/capabilities", seg(org)), body, passkey)
            .map(drop)
    }

    /// Create a team: `join="everyone"` (open: whoever reaches the parent is a
    /// member) or `"invite"` (restricted), under `parent` or at the top.
    /// Returns its id.
    #[pyo3(signature = (org, name, join = "invite", parent = None))]
    fn create_team(
        &self,
        py: Python<'_>,
        org: &str,
        name: &str,
        join: &str,
        parent: Option<&str>,
    ) -> PyResult<String> {
        let body = serde_json::json!({ "name": name, "join": join, "parent": parent });
        let v = self.send(py, Verb::Post, format!("orgs/{}/teams", seg(org)), body)?;
        field(&v, "teamId")
    }

    /// A team: its members, sub-teams and apps.
    fn get_team(&self, py: Python<'_>, org: &str, team: &str) -> PyResult<PyObject> {
        let v = self.send(
            py,
            Verb::Get,
            format!("orgs/{}/teams/{}", seg(org), seg(team)),
            Value::Null,
        )?;
        to_py(py, &v)
    }

    /// Rename, change how it is joined, set its metadata, or move it: under
    /// `parent`, or to the top with `to_top=True`. Unset arguments stay as they are.
    #[pyo3(signature = (org, team, name = None, join = None, metadata = None, parent = None, to_top = false))]
    #[allow(clippy::too_many_arguments)]
    fn update_team(
        &self,
        py: Python<'_>,
        org: &str,
        team: &str,
        name: Option<&str>,
        join: Option<&str>,
        metadata: Option<&Bound<'_, PyAny>>,
        parent: Option<&str>,
        to_top: bool,
    ) -> PyResult<()> {
        if to_top && parent.is_some() {
            return Err(PyValueError::new_err("pass parent or to_top, not both"));
        }
        let mut body = serde_json::Map::new();
        if let Some(name) = name {
            let _prev = body.insert("name".into(), name.into());
        }
        if let Some(join) = join {
            let _prev = body.insert("join".into(), join.into());
        }
        if let Some(metadata) = metadata {
            let _prev = body.insert("metadata".into(), to_json(py, Some(metadata))?);
        }
        if let Some(parent) = parent {
            let _prev = body.insert("parent".into(), parent.into());
        } else if to_top {
            let _prev = body.insert("parent".into(), Value::Null);
        }
        self.send(
            py,
            Verb::Post,
            format!("orgs/{}/teams/{}", seg(org), seg(team)),
            Value::Object(body),
        )
        .map(drop)
    }

    /// Delete a team, the teams below it and their apps.
    fn delete_team(&self, py: Python<'_>, org: &str, team: &str) -> PyResult<()> {
        self.send(
            py,
            Verb::Delete,
            format!("orgs/{}/teams/{}", seg(org), seg(team)),
            Value::Null,
        )
        .map(drop)
    }

    /// Add `account` to a team, or change its row, approved with `passkey`:
    /// `role` is `lead`, `member` or `readOnly`; `capabilities` overrides the
    /// organization's default.
    #[pyo3(signature = (org, team, account, passkey, role = None, capabilities = None))]
    #[allow(clippy::too_many_arguments)]
    fn set_team_member(
        &self,
        py: Python<'_>,
        org: &str,
        team: &str,
        account: &str,
        passkey: &PySoftPasskey,
        role: Option<&str>,
        capabilities: Option<u32>,
    ) -> PyResult<()> {
        let body = serde_json::json!({
            "account": account,
            "role": role,
            "capabilities": capabilities,
        });
        self.confirmed(
            py,
            format!("orgs/{}/teams/{}/members", seg(org), seg(team)),
            body,
            passkey,
        )
        .map(drop)
    }

    /// Remove `account` from a team, approved with `passkey`. It does not
    /// come back through an open parent until it is added again.
    fn remove_team_member(
        &self,
        py: Python<'_>,
        org: &str,
        team: &str,
        account: &str,
        passkey: &PySoftPasskey,
    ) -> PyResult<()> {
        self.confirmed_delete(
            py,
            format!(
                "orgs/{}/teams/{}/members/{}",
                seg(org),
                seg(team),
                seg(account)
            ),
            passkey,
        )
        .map(drop)
    }

    /// Invite someone into the organization with `role`, and into `teams`,
    /// approved with `passkey`. Returns the token to hand them (valid
    /// `ttl_secs`, default a week, for `uses` redemptions, default one).
    #[pyo3(signature = (org, role, passkey, teams = Vec::new(), ttl_secs = None, uses = None))]
    #[allow(clippy::too_many_arguments)]
    fn invite(
        &self,
        py: Python<'_>,
        org: &str,
        role: &str,
        passkey: &PySoftPasskey,
        teams: Vec<String>,
        ttl_secs: Option<u64>,
        uses: Option<u32>,
    ) -> PyResult<String> {
        let body =
            serde_json::json!({ "role": role, "teams": teams, "ttlSecs": ttl_secs, "uses": uses });
        let v = self.confirmed(py, format!("orgs/{}/invites", seg(org)), body, passkey)?;
        field(&v, "token")
    }

    /// Join an organization (and its invited teams) with an invite token.
    fn redeem_invite(&self, py: Python<'_>, token: &str) -> PyResult<PyObject> {
        let v = self.send(
            py,
            Verb::Post,
            format!("invites/{}/redeem", seg(token)),
            Value::Null,
        )?;
        to_py(py, &v)
    }

    /// Delete an organization with its teams, invites and apps: the owner's,
    /// approved with their `passkey`.
    fn delete_org(&self, py: Python<'_>, org: &str, passkey: &PySoftPasskey) -> PyResult<()> {
        let (pk, org) = (Arc::clone(&passkey.inner), org.to_owned());
        self.run(py, async |c| {
            let mut auth = pk.lock().await;
            c.delete_org(&org, &mut *auth).await
        })
        .map(drop)
    }

    /// Set an organization's metadata (an admin's, or a holder of
    /// `MANAGE_METADATA`); `None` clears it.
    #[pyo3(signature = (org, metadata))]
    fn set_org_metadata(
        &self,
        py: Python<'_>,
        org: &str,
        metadata: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let (org, m) = (org.to_owned(), metadata_json(py, metadata)?);
        self.run(py, async |c| c.set_org_metadata(&org, m.clone()).await)
            .map(drop)
    }

    /// Set a member's metadata (their own, or an admin's); `None` clears it.
    #[pyo3(signature = (org, account, metadata))]
    fn set_member_metadata(
        &self,
        py: Python<'_>,
        org: &str,
        account: &str,
        metadata: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let (org, account, m) = (
            org.to_owned(),
            account.to_owned(),
            metadata_json(py, metadata)?,
        );
        self.run(py, async |c| {
            c.set_member_metadata(&org, &account, m.clone()).await
        })
        .map(drop)
    }
}
