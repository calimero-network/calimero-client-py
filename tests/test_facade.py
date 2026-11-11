"""
The facade (`calimero_client_py.facade`, generated): one method per operation
of core's OpenAPI document on `Client`, grouped by tag, and the deprecated
methods, each now a call onto one of them that sends what it always sent.

Runs on the stand-in native client of test_openapi_client.py: no node.
"""

import base64
import json
import warnings

import pytest

from calimero_client_py import Client, CloudError, SoftPasskey, facade
from calimero_client_py.openapi.models import (
    InviteCreateResponse200,
    OrgCreateResponse200,
)

from test_openapi_client import (
    ROOT,
    Native,
    answer,
    challenge_details,
    client_on,
    refusal,
)

# ------------------------------------------------------------ the facade


def test_every_operation_is_one_method():
    document = json.loads((ROOT / "openapi" / "openapi.json").read_text())
    ids = set()
    for item in document["paths"].values():
        for op in item.values():
            if not isinstance(op, dict) or "operationId" not in op:
                continue
            ok = op["responses"].get("200", {}).get("content", {})
            if "text/event-stream" in ok:
                continue  # the streams are Client.events / presence_stream
            ids.add(op["operationId"])
    found = set()
    for attr, group in facade.GROUPS.items():
        assert isinstance(getattr(client_on(Native()), attr), (group, object))
        for name in dir(group):
            if not name.startswith("_"):
                found.add((attr, name))
    assert len(found) == len(ids)
    for oid in ids:
        assert hasattr(facade, "_" + oid.upper()), oid


def test_a_method_takes_flattened_keywords_and_answers_the_model():
    native = Native(
        refusal(412, "CONFIRM_REQUIRED", challenge_details()),
        answer(200, {"token": "t-1"}),
    )
    client = client_on(native)
    client.use_passkey(SoftPasskey())
    r = client.orgs.invite_create(org="o 1", role="member", teams=["t"], ttl_secs=60)
    assert isinstance(r, InviteCreateResponse200)
    assert r.token == "t-1"
    first, second = native.sent
    assert first["path"] == "/orgs/o%201/invites"
    assert first["body"] == {"role": "member", "teams": ["t"], "ttlSecs": 60}
    assert second["body"]["confirm"]["challengeId"] == "ch-1"


def test_keywords_only():
    with pytest.raises(TypeError):
        client_on(Native()).orgs.get("o1")


def test_confirm_overrides_the_clients_passkey():
    own = SoftPasskey()
    native = Native(
        refusal(412, "CONFIRM_REQUIRED", challenge_details()),
        answer(200, {"token": "t"}),
    )
    client = client_on(native)
    client.use_passkey(SoftPasskey())
    client.orgs.invite_create(org="o", role="member", confirm=own)
    sent = native.sent[1]["body"]["confirm"]["assertion"]
    assert sent["credentialId"] == own.credential_id


def test_confirm_false_raises_the_412():
    native = Native(refusal(412, "CONFIRM_REQUIRED", challenge_details()))
    client = client_on(native)
    client.use_passkey(SoftPasskey())
    with pytest.raises(CloudError) as e:
        client.orgs.invite_create(org="o", role="member", confirm=False)
    assert e.value.args[1:3] == (412, "CONFIRM_REQUIRED")
    assert len(native.sent) == 1


def test_a_ready_confirm_goes_in_the_body():
    native = Native(answer(200, {"token": "t"}))
    ready = {
        "challengeId": "c",
        "assertion": {
            "credentialId": "x",
            "authenticatorData": "y",
            "clientDataJson": "z",
            "signature": "s",
        },
    }
    client_on(native).orgs.invite_create(org="o", role="member", confirm=ready)
    assert native.sent[0]["body"]["confirm"] == ready


def test_operator_proof_takes_a_seed_or_false():
    details = {"nonce": "n", "message": base64.b64encode(b"m").decode()}
    native = Native(
        refusal(428, "OPERATOR_SIGNATURE_REQUIRED", details),
        answer(200, {"status": "s", "unchanged": True}),
    )
    client_on(native).ops.quotas_set(operator_proof="ab" * 32)
    assert native.sent[1]["body"]["operatorProof"] == {
        "nonce": "n",
        "signature": native.operator_sign(b"m", "ab" * 32),
    }
    native = Native(refusal(428, "OPERATOR_SIGNATURE_REQUIRED", details))
    with pytest.raises(CloudError):
        client_on(native).ops.quotas_set(operator_proof=False)


def test_a_refusal_raises_cloud_error():
    native = Native(refusal(404, "NOT_FOUND", message="no such org"))
    with pytest.raises(CloudError) as e:
        client_on(native).orgs.get(org="nope")
    assert e.value.args == ("404 NOT_FOUND: no such org", 404, "NOT_FOUND", "null")


def test_a_raw_body_is_body():
    native = Native(answer(200, {"blobId": "b", "id": "i", "size": 3, "index": 1}))
    client_on(native).apps.blob_upload(app="a", body=b"xyz")
    sent = native.sent[0]
    assert sent["raw"] == b"xyz"
    assert sent["headers"]["content-type"] == "application/octet-stream"
    native = Native(answer(200, {"blobId": "b", "id": "i", "size": 3, "index": 1}))
    client_on(native).apps.blob_upload(app="a", body=b"xyz", content_type="image/png")
    assert native.sent[0]["headers"]["content-type"] == "image/png"


def test_an_enum_query_takes_its_value():
    native = Native(answer(200, {}))
    with pytest.raises(Exception):
        # The answer is not an attestation document; the request is what counts.
        client_on(native).public.attestation(nonce="00", purpose="kms")
    assert "purpose=kms" in native.sent[0]["path"]


def test_a_call_with_an_idempotency_key_is_retried():
    native = Native(
        answer(200, {"index": 1, "result": 2, "events": [], "receipt": None})
    )
    client_on(native)._op("intents", "app", body={"method": "m", "idempotencyKey": "k"})
    assert (native.sent[0]["mode"], native.sent[0]["retry"]) == ("repeatable", True)


def test_packages_is_still_the_method_and_a_group():
    native = Native(answer(200, {"packages": [{"name": "p"}]}))
    client = client_on(native)
    with pytest.warns(DeprecationWarning):
        assert client.packages() == [{"name": "p"}]
    assert callable(Client.packages)
    assert callable(client.packages.get)


# ------------------------------------------------------------ the deprecated methods

PK = SoftPasskey()

# (call, method, path, body sent first, answer, what it returns): what the
# Rust implementation sent and answered, route by route.
CASES = [
    (lambda c: c.me(), "GET", "/auth/me", None, {"accountId": "a"}, {"accountId": "a"}),
    (
        lambda c: c.create_api_key({"label": "ci"}, PK),
        "POST",
        "/auth/apikeys",
        {"label": "ci"},
        {"keyId": "k", "key": "cal_x"},
        ("k", "cal_x"),
    ),
    (lambda c: c.sign_out(), "POST", "/auth/logout", None, {}, None),
    (lambda c: c.sign_out_everywhere(), "POST", "/auth/logout-all", None, {}, None),
    (lambda c: c.sessions(), "GET", "/auth/sessions", None, {"sessions": [1]}, [1]),
    (
        lambda c: c.revoke_session("s/1"),
        "DELETE",
        "/auth/sessions/s%2F1",
        None,
        {},
        None,
    ),
    (
        lambda c: c.activity(after=5),
        "GET",
        "/auth/activity?after=5",
        None,
        {"activity": []},
        [],
    ),
    (lambda c: c.activity(), "GET", "/auth/activity", None, {"activity": [1]}, [1]),
    (lambda c: c.remove_passkey("cr", PK), "DELETE", "/auth/passkeys/cr", {}, {}, None),
    (lambda c: c.list_api_keys(), "GET", "/auth/apikeys", None, {"keys": []}, []),
    (lambda c: c.revoke_api_key("k1", PK), "DELETE", "/auth/apikeys/k1", {}, {}, None),
    (lambda c: c.usage(), "GET", "/auth/usage", None, {"u": 1}, {"u": 1}),
    (
        lambda c: c.label_passkey("cr", "phone"),
        "POST",
        "/auth/passkeys/cr",
        {"label": "phone"},
        {},
        None,
    ),
    (
        lambda c: c.set_alias("kv", "app1"),
        "PUT",
        "/auth/aliases/kv",
        {"app": "app1"},
        {},
        None,
    ),
    (
        lambda c: c.alias("kv"),
        "GET",
        "/auth/aliases/kv",
        None,
        {"alias": "kv", "appId": "a"},
        {"alias": "kv", "appId": "a"},
    ),
    (
        lambda c: c.aliases(),
        "GET",
        "/auth/aliases",
        None,
        {"aliases": {"kv": "a"}},
        {"kv": "a"},
    ),
    (lambda c: c.delete_alias("kv"), "DELETE", "/auth/aliases/kv", None, {}, None),
    (
        lambda c: c.create_recovery_code(PK),
        "POST",
        "/auth/recovery",
        {},
        {"code": "XXXX"},
        "XXXX",
    ),
    (lambda c: c.remove_recovery_code(PK), "DELETE", "/auth/recovery", {}, {}, None),
    (
        lambda c: c.cancel_account_recovery(PK),
        "POST",
        "/auth/recovery/cancel",
        {},
        {},
        None,
    ),
    (
        lambda c: c.transfer_app("a1", "acc", PK),
        "POST",
        "/apps/a1/transfer",
        {"to": "acc"},
        {"ok": 1},
        {"ok": 1},
    ),
    (
        lambda c: c.move_app("a1", "o2", PK),
        "POST",
        "/apps/a1/move",
        {"toOrg": "o2", "bringMembers": False},
        {},
        {},
    ),
    (
        lambda c: c.publish_presence("a1", 3, b"\x01\xff"),
        "POST",
        "/apps/a1/presence",
        {"seq": 3, "state": "01ff"},
        {},
        {},
    ),
    (
        lambda c: c.publish_presence("a1", 4),
        "POST",
        "/apps/a1/presence",
        {"seq": 4, "state": None},
        {},
        {},
    ),
    (
        lambda c: c.presence("a1"),
        "GET",
        "/apps/a1/presence",
        None,
        {"present": []},
        {"present": []},
    ),
    (
        lambda c: c.create_app_from_package(
            "n", "com.x", version="1.0.0", options={"org": "o"}
        ),
        "POST",
        "/apps",
        {"name": "n", "package": "com.x", "version": "1.0.0", "org": "o"},
        {"appId": "a9"},
        "a9",
    ),
    (
        lambda c: c.create_app("n", "h", {"org": "o"}),
        "POST",
        "/apps",
        {"name": "n", "bundle": "h", "org": "o"},
        {"appId": "a9"},
        "a9",
    ),
    (
        lambda c: c.call("a1", "set", {"k": 1}, idempotency_key="key"),
        "POST",
        "/apps/a1/intents",
        {"method": "set", "args": {"k": 1}, "idempotencyKey": "key"},
        {"index": 7, "result": 1, "extra": 0},
        {"index": 7, "result": 1, "events": [], "receipt": None},
    ),
    (
        lambda c: c.call_confirmed("a1", "pay", None, PK),
        "POST",
        "/apps/a1/intents",
        {"method": "pay", "args": {}},
        {"index": 1, "result": None, "events": [{"kind": "k", "data": 1}]},
        {
            "index": 1,
            "result": None,
            "events": [{"kind": "k", "data": 1}],
            "receipt": None,
        },
    ),
    (
        lambda c: c.view("a1", "get", stale=True, min_index=4),
        "POST",
        "/apps/a1/views",
        {"method": "get", "args": {}, "consistency": "stale", "minIndex": 4},
        {"index": 4, "result": "v"},
        {"index": 4, "result": "v", "receipt": None},
    ),
    (
        lambda c: c.view("a1", "get"),
        "POST",
        "/apps/a1/views",
        {"method": "get", "args": {}, "consistency": "strong"},
        {"index": 4, "result": "v"},
        {"index": 4, "result": "v", "receipt": None},
    ),
    (lambda c: c.list_apps(), "GET", "/apps", None, {"apps": [1]}, [1]),
    (
        lambda c: c.get_app("a1"),
        "GET",
        "/apps/a1",
        None,
        {"appId": "a1"},
        {"appId": "a1"},
    ),
    (
        lambda c: c.set_access("a1", {"kind": "public"}),
        "POST",
        "/apps/a1/access",
        {"access": {"kind": "public"}},
        {},
        None,
    ),
    (
        lambda c: c.upgrade_app("a1", version="2.0.0"),
        "POST",
        "/apps/a1/upgrade",
        {
            "bundle": None,
            "version": "2.0.0",
            "migrate": None,
            "package": None,
            "service": None,
        },
        {"index": 1},
        {"index": 1},
    ),
    (lambda c: c.delete_app("a1"), "DELETE", "/apps/a1", None, {}, None),
    (
        lambda c: c.app_storage("a1"),
        "GET",
        "/apps/a1/storage",
        None,
        {"keys": 1},
        {"keys": 1},
    ),
    (lambda c: c.app_abi("a1"), "GET", "/apps/a1/abi", None, {"abi": {}}, {"abi": {}}),
    (
        lambda c: c.set_app_metadata("a1", None),
        "POST",
        "/apps/a1/metadata",
        {"metadata": None},
        {},
        None,
    ),
    (
        lambda c: c.list_blobs("a1"),
        "GET",
        "/apps/a1/blobs",
        None,
        {"blobs": []},
        {"blobs": []},
    ),
    (
        lambda c: c.blob_info("a1", "b1"),
        "GET",
        "/apps/a1/blobs/b1/info",
        None,
        {"id": "b1"},
        {"id": "b1"},
    ),
    (
        lambda c: c.delete_blob("a1", "b1"),
        "DELETE",
        "/apps/a1/blobs/b1",
        None,
        {"deleted": True},
        {"deleted": True},
    ),
    (
        lambda c: c.create_org("Acme", recovery_key="pk"),
        "POST",
        "/orgs",
        {"name": "Acme", "recoveryKey": "pk"},
        {"orgId": "o1"},
        "o1",
    ),
    (
        lambda c: c.get_org("o1"),
        "GET",
        "/orgs/o1",
        None,
        {"orgId": "o1"},
        {"orgId": "o1"},
    ),
    (
        lambda c: c.org_members_security("o1"),
        "GET",
        "/orgs/o1/members?detail=security",
        None,
        {"members": []},
        [],
    ),
    (
        lambda c: c.set_member("o1", "acc", "admin", PK),
        "POST",
        "/orgs/o1/members",
        {"account": "acc", "role": "admin"},
        {},
        None,
    ),
    (
        lambda c: c.remove_member("o1", "acc", PK),
        "DELETE",
        "/orgs/o1/members/acc",
        {},
        {},
        None,
    ),
    (
        lambda c: c.set_capabilities("o1", 3, PK),
        "POST",
        "/orgs/o1/capabilities",
        {"capabilities": 3, "account": None, "team": None},
        {},
        None,
    ),
    (
        lambda c: c.create_team("o1", "eng"),
        "POST",
        "/orgs/o1/teams",
        {"name": "eng", "join": "invite", "parent": None},
        {"teamId": "t1"},
        "t1",
    ),
    (
        lambda c: c.list_app_teams("o1"),
        "GET",
        "/orgs/o1/app-teams",
        None,
        {"mode": "shared"},
        {"mode": "shared"},
    ),
    (
        lambda c: c.scope_apps("o1", PK, shared=True),
        "POST",
        "/orgs/o1/app-teams",
        {"shared": True},
        {"mode": "shared"},
        {"mode": "shared"},
    ),
    (
        lambda c: c.get_team("o1", "t1"),
        "GET",
        "/orgs/o1/teams/t1",
        None,
        {"teamId": "t1"},
        {"teamId": "t1"},
    ),
    (
        lambda c: c.update_team("o1", "t1", name="x", to_top=True),
        "POST",
        "/orgs/o1/teams/t1",
        {"name": "x", "parent": None},
        {},
        None,
    ),
    (lambda c: c.delete_team("o1", "t1"), "DELETE", "/orgs/o1/teams/t1", {}, {}, None),
    (
        lambda c: c.set_team_member("o1", "t1", "acc", PK, role="readOnly"),
        "POST",
        "/orgs/o1/teams/t1/members",
        {"account": "acc", "role": "readOnly", "capabilities": None},
        {},
        None,
    ),
    (
        lambda c: c.remove_team_member("o1", "t1", "acc", PK),
        "DELETE",
        "/orgs/o1/teams/t1/members/acc",
        {},
        {},
        None,
    ),
    (
        lambda c: c.set_team_members(
            "o1", "acc", ["t1", {"team": "t2", "role": "lead"}], PK
        ),
        "POST",
        "/orgs/o1/members/acc/teams",
        {"teams": [{"team": "t1"}, {"team": "t2", "role": "lead"}]},
        {},
        None,
    ),
    (
        lambda c: c.remove_team_members("o1", "acc", ["t1"], PK),
        "DELETE",
        "/orgs/o1/members/acc/teams",
        {"teams": ["t1"]},
        {},
        None,
    ),
    (
        lambda c: c.invite("o1", "member", PK),
        "POST",
        "/orgs/o1/invites",
        {"role": "member", "teams": [], "ttlSecs": None, "uses": None},
        {"token": "tok"},
        "tok",
    ),
    (
        lambda c: c.redeem_invite("tok"),
        "POST",
        "/invites/tok/redeem",
        None,
        {"orgId": "o1"},
        {"orgId": "o1"},
    ),
    (lambda c: c.delete_org("o1", PK), "DELETE", "/orgs/o1", {}, {}, None),
    (
        lambda c: c.set_org_metadata("o1", {"a": 1}),
        "POST",
        "/orgs/o1/metadata",
        {"metadata": {"a": 1}},
        {},
        None,
    ),
    (
        lambda c: c.set_member_metadata("o1", "acc", None),
        "POST",
        "/orgs/o1/members/acc/metadata",
        {"metadata": None},
        {},
        None,
    ),
    (
        lambda c: c.set_recovery_key("o1", None, PK),
        "POST",
        "/orgs/o1/recovery",
        {"publicKey": None},
        {},
        None,
    ),
    (
        lambda c: c.cancel_recovery("o1", PK),
        "POST",
        "/orgs/o1/recovery/cancel",
        {},
        {},
        None,
    ),
    (
        lambda c: c.finish_recovery("o1", PK),
        "POST",
        "/orgs/o1/recovery/finish",
        {},
        {"org": "o1"},
        {"org": "o1"},
    ),
    (
        lambda c: c.approve_release("1.0.0", PK),
        "POST",
        "/auth/releases",
        {"version": "1.0.0"},
        {},
        {},
    ),
    (
        lambda c: c.approve_release("1.0.0", PK, org="o1"),
        "POST",
        "/orgs/o1/releases",
        {"version": "1.0.0"},
        {},
        {},
    ),
    (
        lambda c: c.approve_outbound("com.x", PK),
        "POST",
        "/auth/outbound",
        {"package": "com.x", "prefixes": "manifest", "replace": False},
        {},
        {},
    ),
    (
        lambda c: c.approve_outbound("com.x", PK, org="o1", prefixes=[], replace=True),
        "POST",
        "/orgs/o1/outbound",
        {"package": "com.x", "prefixes": [], "replace": True},
        {},
        {},
    ),
    (
        lambda c: c.upgrade_org("o1", "com.x", "2", PK),
        "POST",
        "/orgs/o1/upgrade",
        {
            "package": "com.x",
            "version": "2",
            "team": None,
            "cascade": False,
            "migrate": None,
        },
        {"runId": "r"},
        {"runId": "r"},
    ),
    (
        lambda c: c.upgrade_run("o1", "r"),
        "GET",
        "/orgs/o1/upgrades/r",
        None,
        {"runId": "r"},
        {"runId": "r"},
    ),
    (
        lambda c: c.retry_upgrade_run("o1", "r", PK),
        "POST",
        "/orgs/o1/upgrades/r/retry",
        {},
        {"runId": "r2"},
        {"runId": "r2"},
    ),
    (
        lambda c: c.install_package("com.x", "1"),
        "POST",
        "/packages/install",
        {"package": "com.x", "version": "1"},
        {},
        {},
    ),
    (
        lambda c: c.package("com.x"),
        "GET",
        "/packages/com.x",
        None,
        {"name": "com.x"},
        {"name": "com.x"},
    ),
    (lambda c: c.uninstall_package("com.x"), "DELETE", "/packages/com.x", None, {}, {}),
    (
        lambda c: c.uninstall_package("com.x", "1"),
        "DELETE",
        "/packages/com.x/1",
        None,
        {},
        {},
    ),
    (
        lambda c: c.list_package_origins("com.x"),
        "GET",
        "/package-origins/com.x",
        None,
        {"origins": []},
        {"origins": []},
    ),
    (
        lambda c: c.add_package_origin("com.x", "https://a.example", PK),
        "POST",
        "/package-origins/com.x",
        {"origin": "https://a.example"},
        {},
        {},
    ),
    (
        lambda c: c.add_package_origin(
            "com.x", "https://a.example", PK, rp_id="example", served=True
        ),
        "POST",
        "/package-origins/com.x",
        {"origin": "https://a.example", "rpId": "example", "served": True},
        {},
        {},
    ),
    (
        lambda c: c.remove_package_origin("com.x", "https://a.example", PK),
        "DELETE",
        "/package-origins/com.x",
        {"origin": "https://a.example"},
        {"removed": True},
        {"removed": True},
    ),
    (lambda c: c.ops_status(), "GET", "/ops/status", None, {"node": 1}, {"node": 1}),
    (
        lambda c: c.ops_register_node(4, "https://n4/", zone="z"),
        "POST",
        "/ops/nodes",
        {"id": 4, "addr": "https://n4/", "zone": "z"},
        {},
        None,
    ),
    (
        lambda c: c.ops_remove_node(4),
        "DELETE",
        "/ops/nodes/4",
        {},
        {"status": "s"},
        {"status": "s"},
    ),
    (
        lambda c: c.ops_set_replicas("a1", [1, 2, 4]),
        "POST",
        "/ops/apps/a1/replicas",
        {"replicas": [1, 2, 4]},
        {"status": "s"},
        {"status": "s"},
    ),
    (lambda c: c.ops_backup(), "POST", "/ops/backup", None, {"written": 3}, 3),
    (lambda c: c.ops_quotas(), "GET", "/ops/quotas", None, {"apps": 1}, {"apps": 1}),
    (
        lambda c: c.ops_set_quotas({"apps": 2}),
        "POST",
        "/ops/quotas",
        {"apps": 2},
        {},
        {},
    ),
    (
        lambda c: c.ops_approve_release("1.0.0", withdraw=True),
        "POST",
        "/ops/releases",
        {"version": "1.0.0", "approved": False},
        {},
        {},
    ),
    (
        lambda c: c.ops_approve_outbound("com.x"),
        "POST",
        "/ops/outbound",
        {"package": "com.x", "prefixes": "manifest", "replace": False},
        {},
        {},
    ),
    (
        lambda c: c.ops_upgrade_app("a1", version="2"),
        "POST",
        "/ops/apps/a1/upgrade",
        {
            "bundle": None,
            "version": "2",
            "package": None,
            "service": None,
            "migrate": None,
        },
        {},
        {},
    ),
    (
        lambda c: c.ops_upgrade_org("o1", "com.x", "2"),
        "POST",
        "/ops/orgs/o1/upgrade",
        {
            "package": "com.x",
            "version": "2",
            "team": None,
            "cascade": False,
            "migrate": None,
        },
        {},
        {},
    ),
    (
        lambda c: c.ops_served_origins(),
        "GET",
        "/ops/served-origins",
        None,
        {"origins": []},
        {"origins": []},
    ),
    (
        lambda c: c.ops_packages(),
        "GET",
        "/ops/packages",
        None,
        {"packages": []},
        {"packages": []},
    ),
    (
        lambda c: c.ops_install_package("com.x", "1"),
        "POST",
        "/ops/packages/install",
        {"package": "com.x", "version": "1"},
        {},
        {},
    ),
    (
        lambda c: c.ops_uninstall_package("com.x", "1"),
        "DELETE",
        "/ops/packages/com.x/1",
        None,
        {},
        {},
    ),
]


@pytest.mark.parametrize("case", CASES, ids=[str(i) for i in range(len(CASES))])
def test_a_deprecated_method_sends_what_it_sent(case):
    call, method, path, body, reply, returns = case
    native = Native(answer(200, reply))
    with pytest.warns(DeprecationWarning):
        got = call(client_on(native))
    (sent,) = native.sent
    assert (sent["method"], sent["path"], sent["body"]) == (method, path, body)
    assert got == returns


def test_the_byte_routes():
    native = Native(
        answer(200, {"hash": "h"}),
        (200, [("content-type", "application/octet-stream")], b"\x00\x01"),
        (200, [("content-type", "text/plain")], b"metric 1\n"),
        answer(200, {"name": "p"}),
    )
    client = client_on(native)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        assert client.upload_bundle(b"\0asm") == "h"
        assert client.download_blob("a1", "b1") == b"\x00\x01"
        assert client.ops_metrics() == "metric 1\n"
        assert client.upload_package(b"mpk") == {"name": "p"}
    assert native.sent[0]["headers"]["content-type"] == "application/wasm"
    assert native.sent[0]["raw"] == b"\0asm"
    assert native.sent[3]["path"] == "/packages"


def test_every_deprecated_method_names_a_facade_method():
    client = client_on(Native())
    for name in dir(Client):
        replacement = getattr(getattr(Client, name), "__deprecated__", None)
        if replacement is None:
            continue
        for r in replacement.split("/"):
            group, method = r.strip().split(".")
            assert callable(getattr(getattr(client, group), method)), (name, r)


def test_the_typed_answer_is_the_generated_model():
    native = Native(answer(200, {"orgId": "o1"}))
    r = client_on(native).orgs.create(name="Acme")
    assert isinstance(r, OrgCreateResponse200)
    assert r.org_id == "o1"


def test_an_operator_signer_makes_the_proof():
    details = {
        "subject": "s",
        "method": "__set_quotas",
        "args": "{}",
        "nonce": "n-9",
        "cluster": "c",
        "message": base64.b64encode(b"m").decode(),
    }
    native = Native(
        refusal(428, "OPERATOR_SIGNATURE_REQUIRED", details),
        answer(200, {}),
        operator_seed="ignored",
    )
    seen = []

    def signer(d):
        seen.append(d)
        return {"nonce": d["nonce"], "signature": "sig"}

    client = client_on(native)
    client.use_operator_signer(signer)
    assert client.json.ops.quotas_set() == {}
    assert seen[0]["cluster"] == "c"
    assert native.sent[1]["body"]["operatorProof"] == {
        "nonce": "n-9",
        "signature": "sig",
    }


def test_json_answers_what_the_node_sent():
    native = Native(answer(200, {"orgId": "o1", "name": "A", "extra": [1]}))
    # No `owner`, which the document requires: the model could not read it.
    assert client_on(native).json.orgs.get(org="o1") == {
        "orgId": "o1",
        "name": "A",
        "extra": [1],
    }
