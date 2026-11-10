//! Organizations, their members and teams, and invites. What grants access
//! (members, roles, capabilities, team rows, invites) is approved with the
//! acting account's `passkey`: the node answers `412 CONFIRM_REQUIRED` first.

use super::prelude::*;

#[pymethods]
impl PyCloudClient {
    /// Create an organization; the caller owns it. Returns its id.
    /// `recovery_key` (`recovery_public_key(seed)`) lets whoever holds the
    /// seed recover it if its owner loses every passkey.
    #[pyo3(signature = (name, recovery_key = None))]
    fn create_org(
        &self,
        py: Python<'_>,
        name: &str,
        recovery_key: Option<String>,
    ) -> PyResult<String> {
        let mut body = serde_json::json!({ "name": name });
        if let Some(key) = recovery_key {
            body["recoveryKey"] = Value::String(key);
        }
        let v = self.send(py, Verb::Post, "orgs".into(), body)?;
        field(&v, "orgId")
    }

    /// An organization: members, roles and the teams the caller can see.
    fn get_org(&self, py: Python<'_>, org: &str) -> PyResult<PyObject> {
        let v = self.send(py, Verb::Get, format!("orgs/{}", seg(org)), Value::Null)?;
        to_py(py, &v)
    }

    /// Each member's lockout risk, for the organization's owner and admins
    /// (`GET /orgs/{org}/members?detail=security`, core #4902): a list of
    /// `{account, name, role, teams, passkeys, recoveryCode, lastActiveDay,
    /// sessions}`. `passkeys` is `1`, `2` or `"3+"`; `lastActiveDay` the UTC
    /// day (`YYYY-MM-DD`) of the member's latest sign-in or session exchange,
    /// `None` when unknown. Never a credential id, a passkey label, a session
    /// or an activity entry. A member below admin gets `403`, someone outside
    /// the organization `404 NOT_FOUND`; the admin's own session on the
    /// cluster's origin only. A node without the route answers a bare `404`
    /// or `405`.
    fn org_members_security(&self, py: Python<'_>, org: &str) -> PyResult<PyObject> {
        let path = format!("orgs/{}/members?detail=security", seg(org));
        let v = self.send(py, Verb::Get, path, Value::Null)?;
        to_py(py, &v["members"])
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

    /// The organization's mode (`legacy`, `standalone`, `shared`) and its app
    /// teams (one per package): `{"mode", "appTeams"}`. A guest sees only
    /// the app teams they are in.
    fn list_app_teams(&self, py: Python<'_>, org: &str) -> PyResult<PyObject> {
        let v = self.send(
            py,
            Verb::Get,
            format!("orgs/{}/app-teams", seg(org)),
            Value::Null,
        )?;
        to_py(py, &v)
    }

    /// Give apps their own access lists in one step (owner or admin),
    /// approved with `passkey`. `apps`: a list of `{"package", "name"?,
    /// "instances"?, "join"?, "keep"?}`; absent, every package with an
    /// organization-access instance. A package's `join` (`"invite"` closes
    /// its app team, `"everyone"` opens it) changes that team alone; a close
    /// keeps everyone who reaches the app, or, with `keep` (account ids),
    /// only the team's leads and those accounts. `shared=True` closes every
    /// app team. Returns `{"mode", "teams"}`.
    #[pyo3(signature = (org, passkey, apps = None, shared = None))]
    fn scope_apps(
        &self,
        py: Python<'_>,
        org: &str,
        passkey: &PySoftPasskey,
        apps: Option<&Bound<'_, PyAny>>,
        shared: Option<bool>,
    ) -> PyResult<PyObject> {
        // The node refuses unknown fields and reads an absent one as its
        // default, so only the fields given go.
        let mut body = serde_json::Map::new();
        if let Some(apps) = apps {
            let _prev = body.insert("apps".into(), to_json(py, Some(apps))?);
        }
        if let Some(shared) = shared {
            let _prev = body.insert("shared".into(), shared.into());
        }
        let v = self.confirmed(
            py,
            format!("orgs/{}/app-teams", seg(org)),
            Value::Object(body),
            passkey,
        )?;
        to_py(py, &v)
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

    /// Delete a team, the teams below it and their apps. When any of those
    /// teams holds apps the node asks for `passkey` (`412 CONFIRM_REQUIRED`,
    /// its details listing the `apps` that go, which the approval names);
    /// without one that `412` is raised. A team with no apps needs none.
    #[pyo3(signature = (org, team, passkey = None))]
    fn delete_team(
        &self,
        py: Python<'_>,
        org: &str,
        team: &str,
        passkey: Option<&PySoftPasskey>,
    ) -> PyResult<()> {
        let path = format!("orgs/{}/teams/{}", seg(org), seg(team));
        match passkey {
            Some(pk) => self.confirmed_delete(py, path, pk).map(drop),
            None => self.send(py, Verb::Delete, path, Value::Null).map(drop),
        }
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
