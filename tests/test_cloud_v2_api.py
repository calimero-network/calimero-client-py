"""
The rest of merod-v2's API through CloudClient's typed methods: API keys,
organizations and teams, the app directory and access, upgrades, passkeys.

Same requirements as test_cloud_v2.py (a merod-v2 binary and the kv-store
fixture); its own cluster, since that module kills a node.
"""

import time
from pathlib import Path

import pytest

from calimero_client_py import CloudError, SoftPasskey
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
    alice, _pk = person(cluster, 0, "alice-keys")
    app = alice.create_app("keyed", alice.upload_bundle(wasm))
    alice.call(app, "set", {"key": "k", "value": "v"})
    key_id, key = alice.create_api_key(
        {"label": "reader", "apps": [app], "readOnly": True}
    )
    listed = {k["keyId"]: k for k in alice.list_api_keys()}
    assert listed[key_id]["label"] == "reader" and listed[key_id]["readOnly"]

    agent = cluster.client(1)
    agent.use_api_key(key)
    assert agent.view(app, "get", {"key": "k"})["result"] == "v"
    with pytest.raises(CloudError):
        agent.call(app, "set", {"key": "k", "value": "w"})

    alice.revoke_api_key(key_id)
    time.sleep(0.3)
    assert key_id not in {k["keyId"] for k in alice.list_api_keys()}
    with pytest.raises(CloudError) as e:
        agent.view(app, "get", {"key": "k"})
    assert code(e.value) == "UNAUTHENTICATED"


def test_orgs_and_teams_end_to_end(cluster, wasm):
    alice, _ = person(cluster, 0, "alice-teams")
    bob, _ = person(cluster, 1, "bob-teams")
    bob_id = bob.me()["accountId"]
    org = alice.create_org("Initech")
    token = alice.invite(org, "member", ttl_secs=600, uses=1)
    bob.redeem_invite(token)
    assert alice.get_org(org)["members"][bob_id] == "member"
    with pytest.raises(CloudError):
        cluster.client(2).redeem_invite(token)  # one use only

    team = alice.create_team(org, "eng")  # restricted by default
    sub = alice.create_team(org, "backend", join="everyone", parent=team)
    alice.set_team_member(org, team, bob_id, role="readOnly")
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

    alice.set_team_member(org, team, bob_id, role="member")
    bob.call(app, "set", {"key": "k", "value": "bob"})

    alice.update_team(org, team, name="engineering", metadata={"colour": "teal"})
    view = alice.get_team(org, team)
    assert view["name"] == "engineering" and view["metadata"] == {"colour": "teal"}
    other = alice.create_team(org, "ops")
    alice.update_team(org, sub, parent=other)
    assert alice.get_team(org, sub)["parent"] == other

    alice.set_capabilities(org, 0, account=bob_id)
    assert alice.get_org(org)["capabilities"][bob_id] == 0

    alice.remove_team_member(org, team, bob_id)
    time.sleep(0.3)
    with pytest.raises(CloudError) as e:
        bob.get_team(org, team)
    assert code(e.value) == "NOT_FOUND"

    alice.delete_team(org, team)
    time.sleep(0.3)
    with pytest.raises(CloudError) as e:
        alice.get_app(app)
    assert code(e.value) == "NOT_FOUND"

    alice.remove_member(org, bob_id)
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
    laptop, _pk = person(cluster, 0, "alice-signout")
    phone_key = SoftPasskey()
    cred = laptop.add_passkey(phone_key)
    laptop.remove_passkey(cred)
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
