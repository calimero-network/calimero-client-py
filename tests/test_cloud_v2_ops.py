"""
The Ops API through CloudClient: what mdma (and any operator) runs against a
merod 1.0 cluster with the node's ops token.

Same requirements as test_cloud_v2.py; its own cluster, started with an ops
token, the operator's key, a backup directory and every decision given to the
operator, so the operator's release, outbound and upgrade calls are allowed.
Every operator decision is signed with that key (`use_operator_key`).
"""

import tempfile
import time
from pathlib import Path

import pytest

from calimero_client_py import Client, CloudError
from test_cloud_v2 import KV_STORE_WASM, Cluster, code, person, pytestmark  # noqa: F401

OPS_TOKEN = "py-test-ops-token"
# The operator's Ed25519 seed; the nodes get its verify key (--operator-key).
OPERATOR_SEED = "11" * 32


@pytest.fixture(scope="module")
def cluster():
    backups = tempfile.mkdtemp(prefix="merod-backup-")
    c = Cluster(
        3,
        flags=[
            "--ops-token",
            OPS_TOKEN,
            "--operator-key",
            Client.operator_public_key(OPERATOR_SEED),
            "--backup-dir",
            backups,
            "--release-approvals",
            "operator",
            "--app-upgrades",
            "operator",
            "--outbound-approvals",
            "operator",
        ],
    )
    yield c
    c.stop()


@pytest.fixture(scope="module")
def wasm() -> bytes:
    return Path(KV_STORE_WASM).read_bytes()


def operator(cluster, i: int = 0, signed: bool = True) -> Client:
    c = Client(cluster.urls[i], allow_mock=True)
    c.use_api_key(OPS_TOKEN)
    if signed:
        c.use_operator_key(OPERATOR_SEED)
    return c


def test_decisions_need_the_operators_signature(cluster):
    unsigned = operator(cluster, signed=False)
    name, value = next(
        (k, v) for k, v in unsigned.ops_quotas().items() if isinstance(v, int)
    )
    with pytest.raises(CloudError) as e:
        unsigned.ops_set_quotas({name: value})
    assert (e.value.args[1], code(e.value)) == (428, "OPERATOR_SIGNATURE_REQUIRED")
    # A signature by another key is refused.
    with pytest.raises(CloudError) as e:
        unsigned.post_operator_signed(
            "ops/quotas", {name: value}, operator_key="22" * 32
        )
    assert e.value.args[1] == 403, e.value.args
    unsigned.post_operator_signed("ops/quotas", {name: value}, OPERATOR_SEED)


def test_status_metrics_and_quotas(cluster):
    ops = operator(cluster)
    status = ops.ops_status()
    assert sorted(n["id"] for n in status["nodes"]) == [1, 2, 3], status
    assert status["governance"]["appUpgrades"] == "operator"
    assert ops.ops_metrics().strip()

    quotas = ops.ops_quotas()
    name, value = next((k, v) for k, v in quotas.items() if isinstance(v, int))
    ops.ops_set_quotas({name: value + 1})
    after = ops.ops_quotas()
    assert after[name] == value + 1
    assert {k: v for k, v in after.items() if k != name} == {
        k: v for k, v in quotas.items() if k != name
    }
    with pytest.raises(CloudError) as e:
        ops.ops_set_quotas({"no_such_quota": 1})
    assert code(e.value) == "UNKNOWN_QUOTA"


def test_the_ops_token_is_required(cluster):
    alice, _pk = person(cluster, 0, "alice-not-ops")
    with pytest.raises(CloudError) as e:
        alice.ops_status()
    assert e.value.args[1] in (401, 403), e.value.args


def test_replica_moves_follow_the_leader(cluster, wasm):
    alice, _pk = person(cluster, 0, "alice-ops")
    app = alice.create_app("moved", alice.upload_bundle(wasm))
    alice.call(app, "set", {"key": "k", "value": "v"})
    # Every node is asked; those that do not lead the app's group answer
    # NOT_LEADER, which the client follows.
    for i in range(3):
        out = operator(cluster, i).ops_set_replicas(app, [1, 2, 3])
        assert "status" in out, out
    assert alice.view(app, "get", {"key": "k"})["result"] == "v"
    # Without the redirect, the nodes that do not lead refuse it (once
    # signed: the signature is asked for first).
    refused = 0
    for i in range(3):
        try:
            operator(cluster, i).post_operator_signed(
                f"ops/apps/{app}/replicas", {"replicas": [1, 2, 3]}
            )
        except CloudError as e:
            assert code(e) == "NOT_LEADER", e.args
            refused += 1
    assert refused == 2


def test_a_registered_node_is_removed(cluster):
    ops = operator(cluster, 1)
    ops.ops_register_node(4, "http://127.0.0.1:9/", zone="z1")
    assert 4 in {n["id"] for n in ops.ops_status()["nodes"]}
    # Removal runs on the system leader; asked of every node, it is followed there.
    operator(cluster, 2).ops_remove_node(4)
    time.sleep(0.5)
    assert 4 not in {n["id"] for n in ops.ops_status()["nodes"]}


def test_backup_releases_outbound_and_upgrades(cluster, wasm):
    alice, alice_pk = person(cluster, 0, "alice-governed")
    bundle = alice.upload_bundle(wasm)
    app = alice.create_app("governed", bundle)
    alice.call(app, "set", {"key": "k", "value": "v"})

    written = sum(operator(cluster, i).ops_backup() for i in range(3))
    assert written >= 1

    ops = operator(cluster)
    ops.ops_approve_release("9.9.9")
    ops.ops_approve_release("9.9.9", withdraw=True)
    # Past the ops token and the operator's hold on outbound approvals, the
    # node looks the package up; none is installed here (no signed .mpk).
    with pytest.raises(CloudError) as e:
        ops.ops_approve_outbound("com.example.kv", prefixes=["https://example.com/"])
    assert (e.value.args[1], code(e.value)) == (400, "REJECTED"), e.value.args
    assert isinstance(ops.ops_packages(), dict)

    # The operator holds app upgrades here, so the owner's passkey does not.
    with pytest.raises(CloudError):
        alice.upgrade_app(app, bundle, passkey=alice_pk)
    out = ops.ops_upgrade_app(app, bundle=bundle)
    assert out["status"] == "ok", out
    assert alice.view(app, "get", {"key": "k"})["result"] == "v"

    # An org-wide run over an org with no app on that package: a run, empty.
    org = alice.create_org("Governed")
    run = ops.ops_upgrade_org(org, "com.example.kv", "1.0.0")
    assert run["runId"], run
    assert alice.upgrade_run(org, run["runId"])["runId"] == run["runId"]
