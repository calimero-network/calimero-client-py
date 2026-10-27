"""
CloudClient against a real 3-node merod cluster (release-v2).

Needs `merod` and the kv-store fixture from core's `release-v2-parity` branch:

    cargo build -p merod
    cargo build -p kv-store --target wasm32-unknown-unknown --profile app-release

Paths default to a sibling `core` checkout; override with MEROD_BINARY (or
MEROD_V2_BINARY) and KV_STORE_WASM. Skipped when the binary is missing.
"""

import json
import os
import socket
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

import pytest

from calimero_client_py import CloudClient, CloudError, SoftPasskey

CORE = Path(__file__).resolve().parents[2] / "core"
BINARY = (
    os.environ.get("MEROD_BINARY")
    or os.environ.get("MEROD_V2_BINARY")
    or str(CORE / "target/debug/merod")
)
KV_STORE_WASM = os.environ.get(
    "KV_STORE_WASM",
    str(CORE / "target/wasm32-unknown-unknown/app-release/kv_store.wasm"),
)
ORIGIN = "http://localhost:3000"

pytestmark = pytest.mark.skipif(
    not (Path(BINARY).exists() and Path(KV_STORE_WASM).exists()),
    reason="merod or the kv-store fixture is not built",
)


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Cluster:
    def __init__(self, n: int = 3):
        ports = [free_port() for _ in range(n)]
        self.urls = [f"http://127.0.0.1:{p}" for p in ports]
        peers = ",".join(f"{i + 1}={u}/" for i, u in enumerate(self.urls))
        logs = None if os.environ.get("MEROD_V2_LOGS") else subprocess.DEVNULL
        self.procs = []
        for i, (port, url) in enumerate(zip(ports, self.urls)):
            # `merod run FLAGS`: a node from flags alone, no `init`.
            args = [
                BINARY,
                "run",
                "--node-id", str(i + 1),
                "--listen", f"127.0.0.1:{port}",
                "--public-url", f"{url}/",
                "--data-dir", tempfile.mkdtemp(prefix="merod-"),
                "--peers", peers,
                "--cluster", "py-test",
                "--dev",
                "--origins", ORIGIN,
            ]
            if i == 0:
                args.append("--bootstrap")
            self.procs.append(subprocess.Popen(args, stdout=logs, stderr=logs))
        deadline = time.time() + 30
        while True:
            try:
                with urllib.request.urlopen(f"{self.urls[0]}/health", timeout=1) as r:
                    if json.load(r).get("systemLeader"):
                        break
            except OSError:
                pass
            if time.time() > deadline:
                self.stop()
                raise RuntimeError("cluster did not start")
            time.sleep(0.2)
        # Let the leader register nodes and mint the first session key.
        time.sleep(1.5)

    def client(self, i: int, allow_mock: bool = True) -> CloudClient:
        return CloudClient(self.urls[i], allow_mock=allow_mock)

    def stop(self, i=None):
        for j in range(len(self.procs)) if i is None else [i]:
            p = self.procs[j]
            if p is not None:
                p.kill()
                p.wait()
                self.procs[j] = None


@pytest.fixture(scope="module")
def cluster():
    c = Cluster(3)
    yield c
    c.stop()


@pytest.fixture(scope="module")
def wasm() -> bytes:
    return Path(KV_STORE_WASM).read_bytes()


def person(cluster, i, name):
    c = cluster.client(i)
    pk = SoftPasskey("localhost", ORIGIN)
    c.sign_up(pk, name)
    return c, pk


def code(e: CloudError) -> str:
    return e.args[2]


def test_refuses_a_mock_quote_unless_allowed(cluster):
    with pytest.raises(CloudError, match="mock quote refused"):
        cluster.client(0, allow_mock=False).post("auth/passkey/login/begin")


def test_sign_up_deploy_call_view_confirm(cluster, wasm):
    alice, pk = person(cluster, 0, "alice")
    me = alice.me()
    assert me["via"] == "session"

    # The same passkey signs in on another node.
    again = cluster.client(1)
    s = again.sign_in(pk)
    assert s["accountId"] == me["accountId"]

    bundle = alice.upload_bundle(wasm)
    app = alice.create_app("kv", bundle, {"confirmMethods": ["clear"]})

    r = alice.call(app, "set", {"key": "a", "value": "1"}, idempotency_key="k1")
    assert r["index"] > 0
    assert len(r["events"]) == 1
    replay = alice.call(app, "set", {"key": "a", "value": "other"}, idempotency_key="k1")
    assert replay["index"] == r["index"]

    assert again.view(app, "get", {"key": "a"})["result"] == "1"
    assert again.view(app, "get", {"key": "a"}, stale=True, min_index=r["index"])["result"] == "1"

    events = alice.events(app, limit=1, timeout=5)
    assert len(events) == 1 and events[0]["kind"]

    # A confirm method needs a fresh passkey assertion over the exact call.
    with pytest.raises(CloudError) as e:
        alice.call(app, "clear", {})
    assert code(e.value) == "CONFIRM_REQUIRED"
    alice.call_confirmed(app, "clear", {}, pk)
    assert alice.view(app, "len")["result"] == 0

    # Others cannot reach an owner-only app.
    bob, _ = person(cluster, 2, "bob")
    with pytest.raises(CloudError) as e:
        bob.call(app, "set", {"key": "x", "value": "y"})
    assert code(e.value) == "NOT_FOUND"

    # A read-only API key reads, never writes.
    _key_id, key = alice.create_api_key({"readOnly": True, "apps": [app]})
    agent = cluster.client(2)
    agent.use_api_key(key)
    agent.view(app, "len")
    with pytest.raises(CloudError) as e:
        agent.call(app, "set", {"key": "x", "value": "y"})
    assert code(e.value) == "FORBIDDEN"

    # Refresh rotates the session, and a saved session can be reused.
    fresh = alice.refresh()
    assert fresh["sessionToken"] != s["sessionToken"]
    later = cluster.client(1)
    later.use_session(fresh)
    assert later.me()["accountId"] == me["accountId"]


def test_organizations_gate_apps(cluster, wasm):
    alice, _ = person(cluster, 0, "alice-org")
    bob, _ = person(cluster, 1, "bob-org")
    bob_id = bob.me()["accountId"]
    org = alice.post("orgs", {"name": "Acme"})["orgId"]
    team = alice.post(f"orgs/{org}/teams", {"name": "eng", "join": "invite"})["teamId"]
    token = alice.post(f"orgs/{org}/invites", {"role": "member", "teams": [team]})["token"]
    bob.post(f"invites/{token}/redeem")

    app = alice.create_app("team-kv", alice.upload_bundle(wasm), {"org": org})
    bob.call(app, "set", {"key": "from", "value": "bob"})

    alice.delete(f"orgs/{org}/members/{bob_id}")
    time.sleep(0.3)
    with pytest.raises(CloudError) as e:
        bob.call(app, "set", {"key": "x", "value": "y"})
    assert code(e.value) == "NOT_FOUND"


def test_blobs_belong_to_their_app(cluster, wasm):
    alice, _pk = person(cluster, 0, "alice-blobs")
    bundle = alice.upload_bundle(wasm)
    app = alice.create_app("files", bundle)
    other = alice.create_app("other", bundle)
    data = b"%PDF-1.7 " * 500
    up = alice.upload_blob(app, data)
    assert up["size"] == len(data)
    assert alice.download_blob(app, up["blobId"]) == data
    assert alice.download_blob(app, up["id"]) == data
    with pytest.raises(CloudError) as e:
        alice.download_blob(other, up["blobId"])
    assert code(e.value) == "NOT_FOUND"


def test_a_second_passkey_is_another_device(cluster):
    laptop, _pk = person(cluster, 0, "alice-devices")
    phone_key = SoftPasskey()
    cred = laptop.add_passkey(phone_key)
    phone = cluster.client(1)
    phone.sign_in(phone_key)
    a, b = laptop.me(), phone.me()
    assert a["accountId"] == b["accountId"]
    assert b["passkeyId"] == cred != a["passkeyId"]


def test_keeps_serving_after_a_node_dies(cluster, wasm):
    alice, pk = person(cluster, 1, "alice-failover")
    app = alice.create_app("kv-ha", alice.upload_bundle(wasm))
    alice.call(app, "set", {"key": "k", "value": "v"})
    cluster.stop(0)
    other = cluster.client(2)
    other.sign_in(pk)
    for _ in range(40):
        try:
            other.call(app, "set", {"key": "k2", "value": "v2"}, idempotency_key="after-failover")
            break
        except CloudError:
            time.sleep(0.25)
    else:
        pytest.fail("no write after failover")
    assert other.view(app, "get", {"key": "k"})["result"] == "v"

