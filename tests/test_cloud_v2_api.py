"""
The rest of merod's API through CloudClient's typed methods: API keys,
organizations and teams, the app directory and access, upgrades, passkeys.

Same requirements as test_cloud_v2.py (a merod 1.0 binary and the kv-store
fixture); its own cluster, since that module kills a node.
"""

import base64
import hashlib
import io
import json
import tarfile
import time
from pathlib import Path

import pytest
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from calimero_client_py import CloudError, LoginRequired, SoftPasskey
from test_cloud_v2 import KV_STORE_WASM, Cluster, code, person, pytestmark  # noqa: F401


@pytest.fixture(scope="module")
def cluster():
    c = Cluster(3)
    yield c
    c.stop()


@pytest.fixture(scope="module")
def wasm() -> bytes:
    return Path(KV_STORE_WASM).read_bytes()


def test_api_keys_are_scoped_listed_and_revoked(cluster, wasm):
    alice, pk = person(cluster, 0, "alice-keys")
    app = alice.create_app("keyed", alice.upload_bundle(wasm))
    alice.call(app, "set", {"key": "k", "value": "v"})
    key_id, key = alice.create_api_key(
        {"label": "reader", "apps": [app], "readOnly": True}, pk
    )
    listed = {k["keyId"]: k for k in alice.list_api_keys()}
    assert listed[key_id]["label"] == "reader" and listed[key_id]["readOnly"]

    agent = cluster.client(1)
    agent.use_api_key(key)
    assert agent.view(app, "get", {"key": "k"})["result"] == "v"
    with pytest.raises(CloudError):
        agent.call(app, "set", {"key": "k", "value": "w"})

    # A node from before revokes at once; this one asks for the passkey (412).
    alice.revoke_api_key(key_id, pk)
    time.sleep(0.3)
    assert key_id not in {k["keyId"] for k in alice.list_api_keys()}
    with pytest.raises(CloudError) as e:
        agent.view(app, "get", {"key": "k"})
    assert code(e.value) == "UNAUTHENTICATED"


def test_orgs_and_teams_end_to_end(cluster, wasm):
    alice, pk = person(cluster, 0, "alice-teams")
    bob, _ = person(cluster, 1, "bob-teams")
    bob_id = bob.me()["accountId"]
    org = alice.create_org("Initech")
    token = alice.invite(org, "member", pk, ttl_secs=600, uses=1)
    bob.redeem_invite(token)
    assert alice.get_org(org)["members"][bob_id] == "member"
    with pytest.raises(CloudError):
        cluster.client(2).redeem_invite(token)  # one use only

    team = alice.create_team(org, "eng")  # restricted by default
    sub = alice.create_team(org, "backend", join="everyone", parent=team)
    alice.set_team_member(org, team, bob_id, pk, role="readOnly")
    app = alice.create_app(
        "eng-kv",
        alice.upload_bundle(wasm),
        {"org": org, "access": {"kind": "team", "team": team}},
    )
    view = bob.get_team(org, team)
    assert view["myRole"] == "readOnly" and sub in view["children"]
    assert [a["appId"] for a in view["apps"]] == [app]
    with pytest.raises(CloudError) as e:
        bob.call(app, "set", {"key": "k", "value": "bob"})
    assert e.value.args[1] == 403

    alice.set_team_member(org, team, bob_id, pk, role="member")
    bob.call(app, "set", {"key": "k", "value": "bob"})

    alice.update_team(org, team, name="engineering", metadata={"colour": "teal"})
    view = alice.get_team(org, team)
    assert view["name"] == "engineering" and view["metadata"] == {"colour": "teal"}
    other = alice.create_team(org, "ops")
    alice.update_team(org, sub, parent=other)
    assert alice.get_team(org, sub)["parent"] == other

    alice.set_capabilities(org, 0, pk, account=bob_id)
    assert alice.get_org(org)["capabilities"][bob_id] == 0

    alice.remove_team_member(org, team, bob_id, pk)
    time.sleep(0.3)
    with pytest.raises(CloudError) as e:
        bob.get_team(org, team)
    assert code(e.value) == "NOT_FOUND"

    # A team with no apps (`other` keeps only `sub`) goes without a passkey.
    alice.delete_team(org, other)
    time.sleep(0.3)
    for gone in (other, sub):
        with pytest.raises(CloudError) as e:
            alice.get_team(org, gone)
        assert code(e.value) == "NOT_FOUND"
    # The team holds an app: deleting it takes alice's passkey, and the node's
    # ask names the apps that go (core#4891). A node from before deletes it at
    # once, so the 412 is checked only where it is asked.
    try:
        alice.delete_team(org, team)
    except CloudError as asked:
        assert code(asked) == "CONFIRM_REQUIRED", asked.args
        assert json.loads(asked.args[3])["apps"] == [app]
        assert alice.get_app(app)["name"] == "eng-kv"
        alice.delete_team(org, team, pk)
    time.sleep(0.3)
    with pytest.raises(CloudError) as e:
        alice.get_app(app)
    assert code(e.value) == "NOT_FOUND"

    alice.remove_member(org, bob_id, pk)
    time.sleep(0.3)
    assert bob_id not in alice.get_org(org)["members"]


def test_app_directory_access_and_upgrade(cluster, wasm):
    alice, alice_pk = person(cluster, 0, "alice-apps")
    carol, _ = person(cluster, 2, "carol-apps")
    bundle = alice.upload_bundle(wasm)
    app = alice.create_app("dir", bundle)
    assert app in {a["appId"] for a in alice.list_apps()}
    assert alice.get_app(app)["name"] == "dir"
    alice.call(app, "set", {"key": "k", "value": "v"})

    with pytest.raises(CloudError):
        carol.view(app, "get", {"key": "k"})
    alice.set_access(app, {"kind": "accounts", "accounts": [carol.me()["accountId"]]})
    time.sleep(0.3)
    assert carol.view(app, "get", {"key": "k"})["result"] == "v"

    # The owner approves an upgrade with their passkey; without it the node asks.
    with pytest.raises(CloudError) as asked:
        alice.upgrade_app(app, bundle)
    assert asked.value.args[2] == "CONFIRM_REQUIRED"
    out = alice.upgrade_app(app, bundle, passkey=alice_pk)
    assert out["status"] == "ok", out
    assert alice.view(app, "get", {"key": "k"})["result"] == "v"


def test_passkey_removal_and_sign_out(cluster):
    laptop, laptop_key = person(cluster, 0, "alice-signout")
    phone_key = SoftPasskey()
    cred = laptop.add_passkey(phone_key, laptop_key)
    laptop.remove_passkey(cred, laptop_key)
    time.sleep(0.3)
    with pytest.raises(CloudError):
        cluster.client(1).sign_in(phone_key)

    laptop.sign_out()
    with pytest.raises(CloudError):
        laptop.refresh()


def test_a_team_moves_to_the_top(cluster):
    alice, _ = person(cluster, 0, "alice-move")
    org = alice.create_org("Movers")
    top = alice.create_team(org, "top")
    sub = alice.create_team(org, "sub", parent=top)
    alice.update_team(org, sub, to_top=True)
    assert alice.get_team(org, sub)["parent"] is None


def test_aliases_org_deletion_and_signing_out_everywhere(cluster, wasm):
    alice, pk = person(cluster, 0, "alice-everywhere")
    bundle = alice.upload_bundle(wasm)
    app = alice.create_app("kv-alias", bundle)
    alice.put("auth/aliases/notes", {"app": app})
    assert alice.get("auth/aliases")["aliases"] == {"notes": app}

    org = alice.create_org("Doomed")
    with pytest.raises(CloudError) as e:
        alice.delete(f"orgs/{org}")
    assert code(e.value) == "CONFIRM_REQUIRED"
    alice.delete_confirmed(f"orgs/{org}", pk)
    with pytest.raises(CloudError):
        alice.get(f"orgs/{org}")

    laptop = cluster.client(1)
    laptop.sign_in(pk)
    alice.sign_out_everywhere()
    with pytest.raises(CloudError):
        laptop.get("auth/me")


def test_passkey_times_and_sessions_listed_and_ended_one_at_a_time(cluster):
    laptop, pk = person(cluster, 0, "alice-sessions")
    me = laptop.me()
    mine = next(p for p in me["passkeys"] if p["id"] == me["passkeyId"])
    phone = cluster.client(1)
    phone.sign_in(pk)
    try:
        listed = laptop.sessions()
    except CloudError as e:
        # 1.0.0-rc.14 and earlier: no `/auth/sessions` (a bare `404`), and
        # no passkey times.
        if e.args[1] != 404 or code(e) == "NOT_FOUND":
            raise
        assert mine.get("addedAt") is None, me
        pytest.skip("the node has no session list or passkey times (core #4894)")
    assert isinstance(mine["addedAt"], int), me
    assert mine["lastSignInAt"] >= mine["addedAt"], me

    assert len(listed) == 2, listed
    own = next(s for s in listed if s["current"])
    other = next(s for s in listed if not s["current"])
    for s in listed:
        assert s["passkey"] == me["passkeyId"], s
        assert s["app"] is None, s
        assert isinstance(s["lastActiveAt"], int), s
    assert [s["session"] for s in phone.sessions() if s["current"]] == [
        other["session"]
    ]

    # The laptop ends the phone's session: the phone must sign in again.
    laptop.revoke_session(other["session"])
    with pytest.raises(LoginRequired):
        phone.me()
    assert [s["session"] for s in laptop.sessions()] == [own["session"]]
    with pytest.raises(CloudError) as e:
        laptop.revoke_session(other["session"])
    assert (e.value.args[1], code(e.value)) == (404, "NOT_FOUND"), e.value
    assert laptop.me()["accountId"] == me["accountId"]


def test_activity_lists_sign_ins_and_approvals(cluster):
    laptop, pk = person(cluster, 0, "alice-activity")
    me = laptop.me()
    try:
        listed = laptop.activity()
    except CloudError as e:
        # A node before core #4901: no `/auth/activity` (a bare `404`).
        if e.args[1] != 404 or code(e) == "NOT_FOUND":
            raise
        pytest.skip("the node has no activity list (core #4901)")
    assert [(a["kind"], a["passkey"], a["app"]) for a in listed] == [
        ("signIn", me["passkeyId"], None)
    ], listed
    signed_up = listed[0]["at"]

    laptop.create_api_key({"label": "ci"}, pk)
    listed = laptop.activity()
    assert [(a["kind"], a["method"]) for a in listed] == [
        ("approval", "__create_api_key"),
        ("signIn", None),
    ], listed
    assert all(a["passkey"] == me["passkeyId"] for a in listed), listed
    assert all(a["at"] > signed_up for a in laptop.activity(after=signed_up))

    # Each account reads only its own.
    bob, _ = person(cluster, 1, "bob-activity")
    bobs = bob.activity()
    assert len(bobs) == 1 and bobs[0]["passkey"] != me["passkeyId"], bobs


def test_blobs_presence_transfer_and_upgrade_runs_have_typed_calls(cluster, wasm):
    alice, pk = person(cluster, 0, "alice-typed")
    bob, _ = person(cluster, 1, "bob-typed")
    bundle = alice.upload_bundle(wasm)
    app = alice.create_app("typed", bundle)

    up = alice.upload_blob(app, b"hello " * 100)
    listed = alice.list_blobs(app)
    assert any(b["id"] == up["id"] for b in listed["blobs"]), listed
    assert alice.blob_info(app, up["id"])["size"] == 600
    alice.delete_blob(app, up["id"])
    with pytest.raises(CloudError) as e:
        alice.blob_info(app, up["id"])
    assert code(e.value) == "NOT_FOUND"

    alice.publish_presence(app, 1, b"typing")
    seen = alice.presence(app)
    assert seen, seen
    alice.publish_presence(app, 2)

    with pytest.raises(CloudError) as e:
        alice.upgrade_run(alice.create_org("Runs"), "no-such-run")
    assert code(e.value) == "NOT_FOUND"

    bob_id = bob.me()["accountId"]
    alice.transfer_app(app, bob_id, pk)
    assert bob.get(f"apps/{app}")["owner"] == bob_id


def test_receipts_verify_against_the_attested_cluster(cluster, wasm):
    alice, _pk = person(cluster, 0, "alice-receipts")
    app = alice.create_app("receipted", alice.upload_bundle(wasm))
    r = alice.call(app, "set", {"key": "k", "value": "v"})
    attested = alice.verify_app_cluster(app)
    assert sorted(attested["keys"]) == [1, 2, 3], attested
    assert attested["governance"] == {
        "releases": "owner",
        "appUpgrades": "owner",
        "outbound": "owner",
    }
    receipt = r["receipt"]
    assert receipt["key"] == attested["keys"][receipt["node"]]


def test_session_aliases_metadata_and_usage(cluster, wasm):
    alice, pk = person(cluster, 0, "alice-more")
    session = alice.session()
    assert session["accountId"] == alice.me()["accountId"]
    agent = cluster.client(1)
    assert agent.session() is None

    bundle = alice.upload_bundle(wasm)
    app = alice.create_app("more", bundle)
    alice.set_alias("notes", app)
    assert alice.alias("notes")["appId"] == app
    assert alice.aliases() == {"notes": app}
    alice.delete_alias("notes")
    assert alice.aliases() == {}

    alice.set_app_metadata(app, {"colour": "teal"})
    assert alice.get_app(app)["metadata"] == {"colour": "teal"}
    alice.set_app_metadata(app, None)
    assert alice.get_app(app).get("metadata") is None

    alice.call(app, "set", {"key": "k", "value": "v"})
    assert alice.app_storage(app)["keys"] >= 1
    assert "abi" in alice.app_abi(app)
    assert "quotas" in alice.usage() or alice.usage()
    assert isinstance(alice.packages(), list)

    alice.label_passkey(pk.credential_id, "laptop")
    alice.label_passkey(pk.credential_id, "")

    org = alice.create_org("Meta")
    alice.set_org_metadata(org, {"plan": "team"})
    assert alice.get_org(org)["metadata"] == {"plan": "team"}
    alice.set_member_metadata(org, alice.me()["accountId"], {"title": "owner"})
    alice.delete_org(org, pk)
    with pytest.raises(CloudError) as e:
        alice.get_org(org)
    assert code(e.value) == "NOT_FOUND"

    alice.delete_app(app)
    time.sleep(0.3)
    with pytest.raises(CloudError) as e:
        alice.get_app(app)
    assert code(e.value) == "NOT_FOUND"


def test_presence_streams_a_snapshot_then_changes(cluster, wasm):
    alice, _pk = person(cluster, 0, "alice-presence")
    app = alice.create_app("here", alice.upload_bundle(wasm))
    alice.publish_presence(app, 1, b"typing")
    frames = alice.presence_stream(app, limit=1, timeout=5)
    assert frames and frames[0]["kind"] == "snapshot", frames


# --------------------------------------------------- per-package web origins

NOTES = "com.test.py-notes"
OTHER = "com.test.py-other"
# The developer's own site for NOTES, and the rp-id the node resolves for it.
SITE = "https://notes.example.com"
SITE_RP = "notes.example.com"
B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def b64url(b: bytes) -> str:
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode()


def base58(b: bytes) -> str:
    n, out = int.from_bytes(b, "big"), ""
    while n:
        n, r = divmod(n, 58)
        out = B58[r] + out
    return "1" * (len(b) - len(b.lstrip(b"\0"))) + out


def mpk(package: str, version: str, wasm: bytes) -> bytes:
    """A signed single-service `.mpk` of `package@version`, as core's
    `mero-sign` makes one: the manifest canonicalized (RFC 8785; plain
    strings and integers, so sorted compact JSON is the same bytes), signed
    with Ed25519 over `sha256("calimero.bundle.manifest.v1\\0" || bytes)`.
    The WASM gets a custom section naming the package, so each package
    ships its own."""
    key = Ed25519PrivateKey.from_private_bytes(bytes([5] * 32))
    public = key.public_key().public_bytes_raw()
    tag = package.encode()
    wasm = wasm + bytes([0, 1 + len(tag), len(tag)]) + tag
    manifest = {
        "version": "1.0",
        "package": package,
        "appVersion": version,
        "signerId": "did:key:z" + base58(bytes([0xED, 0x01]) + public),
        "minRuntimeVersion": "0.1.0",
        "wasm": {
            "path": "app.wasm",
            "hash": hashlib.sha256(wasm).hexdigest(),
            "size": len(wasm),
        },
    }
    canonical = json.dumps(
        manifest, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode()
    payload = hashlib.sha256(b"calimero.bundle.manifest.v1\0" + canonical).digest()
    manifest["signature"] = {
        "algorithm": "ed25519",
        "publicKey": b64url(public),
        "signature": b64url(key.sign(payload)),
    }
    out = io.BytesIO()
    with tarfile.open(fileobj=out, mode="w:gz") as tar:
        for name, data in (
            ("manifest.json", json.dumps(manifest).encode()),
            ("app.wasm", wasm),
        ):
            info = tarfile.TarInfo(name)
            info.size = len(data)
            tar.addfile(info, io.BytesIO(data))
    return out.getvalue()


@pytest.fixture(scope="module")
def origins(cluster):
    """Skip on a node without per-package origins (before core #4846): an
    unknown route there is a bare `404`, an unknown package here
    `NOT_FOUND`."""
    probe, _ = person(cluster, 0, "origins-probe")
    try:
        probe.list_package_origins("com.test.none")
    except CloudError as e:
        if code(e) != "NOT_FOUND":
            pytest.skip("the node has no per-package origins (core #4846)")


def test_a_packages_manager_registers_and_removes_its_web_origin(
    cluster, wasm, origins
):
    # The first installer manages a package: alice NOTES, bob OTHER.
    alice, alice_pk = person(cluster, 0, "alice-origins")
    bob, bob_pk = person(cluster, 1, "bob-origins")
    alice.upload_package(mpk(NOTES, "1.0.0", wasm))
    bob.upload_package(mpk(OTHER, "1.0.0", wasm))

    # Bob does not manage NOTES: he does not even see its origins.
    for refused in (
        lambda: bob.list_package_origins(NOTES),
        lambda: bob.add_package_origin(NOTES, SITE, bob_pk),
    ):
        with pytest.raises(CloudError) as e:
            refused()
        assert e.value.args[1] == 404, e.value
    # Nor is the site anybody's yet.
    with pytest.raises(CloudError) as e:
        cluster.client(2).sign_up(SoftPasskey(SITE_RP, SITE), "early", origin=SITE)
    assert code(e.value) == "ORIGIN_NOT_REGISTERED"

    # Without her passkey the node asks alice first. The step-up is the same
    # shape as every other one: what she approves (`package:{p}` /
    # `__add_origin` / `{origin, rpId}`) is bound inside the challenge, so the
    # retry below succeeding is what proves the node signed that statement.
    with pytest.raises(CloudError) as e:
        alice.post(f"package-origins/{NOTES}", {"origin": SITE})
    assert code(e.value) == "CONFIRM_REQUIRED"
    asked = json.loads(e.value.args[3])
    assert {"challengeId", "challenge", "rpId", "allowCredentials"} <= set(asked)
    with pytest.raises(CloudError) as e:
        alice.add_package_origin(NOTES, SITE, alice_pk, rp_id="example.org")
    assert code(e.value) == "BAD_RP_ID"

    added = alice.add_package_origin(NOTES, SITE, alice_pk)
    assert (added["origin"], added["rpId"]) == (SITE, SITE_RP), added
    listed = alice.list_package_origins(NOTES)
    assert [o["origin"] for o in listed["origins"]] == [SITE], listed
    # Another package cannot take it.
    with pytest.raises(CloudError) as e:
        bob.add_package_origin(OTHER, SITE, bob_pk)
    assert code(e.value) == "ORIGIN_TAKEN"

    # Carol signs up on the site: a session bound to NOTES, naming the site.
    carol_pk = SoftPasskey(SITE_RP, SITE)
    carol = cluster.client(2)
    carol.sign_up(carol_pk, "carol-origins", origin=SITE)
    me = carol.me()
    assert (me["app"], me["origin"]) == (NOTES, SITE), me
    elsewhere = cluster.client(1)
    elsewhere.sign_in(carol_pk, origin=SITE)
    assert elsewhere.me()["origin"] == SITE

    # Removing the origin ends those sessions and their refresh, and the site
    # may not begin a ceremony any more.
    removed = alice.remove_package_origin(NOTES, SITE, alice_pk)
    assert removed["removed"] is True, removed
    for session in (carol, elsewhere):
        with pytest.raises(LoginRequired):
            session.me()
    with pytest.raises(CloudError) as e:
        cluster.client(0).sign_in(carol_pk, origin=SITE)
    assert code(e.value) == "ORIGIN_NOT_REGISTERED"
    assert alice.list_package_origins(NOTES)["origins"] == []
    with pytest.raises(CloudError) as e:
        alice.remove_package_origin(NOTES, SITE, alice_pk)
    assert e.value.args[1] == 404, e.value


def test_an_admin_closes_and_opens_one_app_team(cluster, wasm):
    # core #4903: a package's own `join` (and `keep`) in POST /orgs/{org}/app-teams.
    alice, pk = person(cluster, 0, "alice-appteams")
    package = "com.test.appteams"
    alice.upload_package(mpk(package, "1.0.0", wasm))
    org = alice.create_org("Appteams")
    assert alice.list_app_teams(org) == {"mode": "legacy", "appTeams": []}

    made = alice.scope_apps(org, pk, apps=[{"package": package, "instances": []}])
    assert made["mode"] == "standalone" and made["teams"][0]["created"]
    closed = alice.scope_apps(
        org, pk, apps=[{"package": package, "join": "invite", "keep": []}]
    )
    assert closed["mode"] == "shared" and closed["teams"][0]["join"] == "invite"
    assert alice.list_app_teams(org)["appTeams"][0]["join"] == "invite"
    opened = alice.scope_apps(org, pk, apps=[{"package": package, "join": "everyone"}])
    assert opened["mode"] == "standalone"
    with pytest.raises(CloudError):
        alice.scope_apps(
            org, pk, apps=[{"package": package, "join": "everyone"}], shared=True
        )
