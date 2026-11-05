"""
Upgrade keys (core #4852, core 1.0.0-rc.10): an API key made with a passkey
that upgrades one app to signed releases of its package, so CI deploys
without a passkey, and is refused everything else (`403 UPGRADE_KEY_ONLY`).

The key's public half is checked against core's derivation everywhere; the
cluster test has the same requirements as test_cloud_v2.py (a merod binary,
from core 1.0.0-rc.10 or later: the first release with upgrade keys).
"""

import hashlib

import pytest
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from calimero_client_py import CloudClient, CloudError
from test_cloud_v2 import KV_STORE_WASM, Cluster, code, person
from test_cloud_v2 import pytestmark as needs_merod
from test_cloud_v2_api import mpk

# core: crates/node/src/proof.rs, `API_KEY_DOMAIN`.
API_KEY_DOMAIN = b"calimero.api-key.v1\0"


def core_api_key_public(key: str) -> str:
    """Core's `proof::api_key_public`, written out: `SHA-256(domain || key)`
    is the Ed25519 seed, the public key hex."""
    seed = hashlib.sha256(API_KEY_DOMAIN + key.encode()).digest()
    return (
        Ed25519PrivateKey.from_private_bytes(seed).public_key().public_bytes_raw().hex()
    )


def test_an_api_keys_public_half_is_the_one_core_derives():
    # Pinned, so a change on either side shows here and not only on a node.
    assert (
        CloudClient.api_key_public("cal_ci")
        == "288164f1279d77ba71dd86e20149ecf9398f0e884bad8644a096ecefd5555a27"
    )
    for key in ("cal_0123456789abcdef", "cal_" + "x" * 64, ""):
        assert CloudClient.api_key_public(key) == core_api_key_public(key)


def test_create_api_key_and_upgrade_app_take_what_an_upgrade_key_needs():
    import inspect

    from calimero_client_py import Client

    params = inspect.signature(Client.create_api_key).parameters
    assert list(params)[1:] == ["options", "passkey"]
    upgrade = inspect.signature(Client.upgrade_app).parameters
    assert upgrade["passkey"].default is None
    assert upgrade["version"].default is None


@pytest.fixture(scope="module")
def cluster():
    c = Cluster(3)
    yield c
    c.stop()


def variant(wasm: bytes, tag: str) -> bytes:
    """`wasm` with a custom section named `tag`: another module that runs the same."""
    t = tag.encode()
    return wasm + bytes([0, 1 + len(t), len(t)]) + t


@needs_merod
def test_an_upgrade_key_upgrades_its_app_and_nothing_else(cluster):
    from pathlib import Path

    kv = Path(KV_STORE_WASM).read_bytes()
    alice, apk = person(cluster, 0, "alice-upgrade-key")
    package = "com.test.pyci"
    for version in ("1.0.0", "1.1.0"):
        alice.upload_package(mpk(package, version, variant(kv, version)))
    app = alice.create_app_from_package("ci-target", package, "1.0.0")
    other = alice.create_app_from_package("not-the-target", package, "1.0.0")

    # It takes no other scope field.
    with pytest.raises(CloudError) as e:
        alice.create_api_key({"upgrades": {"app": app}, "methods": ["set"]}, apk)
    assert code(e.value) == "BAD_UPGRADE_KEY"

    key_id, key = alice.create_api_key({"label": "ci", "upgrades": {"app": app}}, apk)
    listed = {k["keyId"]: k for k in alice.list_api_keys()}[key_id]
    assert listed["upgrades"]["app"] == app
    assert listed["upgrades"]["package"] == package
    assert listed["upgrades"]["proofKey"] == CloudClient.api_key_public(key)

    # CI: the key alone, on another node, no passkey.
    ci = cluster.client(1)
    ci.use_api_key(key)
    before = alice.get_app(app)["bundle"]
    ci.upgrade_app(app, version="1.1.0")
    assert alice.get_app(app)["bundle"] != before

    # Nothing else, reads included.
    for refused in (ci.me, ci.list_apps, lambda: ci.get_app(app)):
        with pytest.raises(CloudError) as e:
            refused()
        assert code(e.value) == "UPGRADE_KEY_ONLY"
    with pytest.raises(CloudError) as e:
        ci.upgrade_app(other, version="1.1.0")
    assert e.value.args[1] == 403

    # Revoked, it upgrades nothing.
    alice.revoke_api_key(key_id, apk)
    with pytest.raises(CloudError):
        ci.upgrade_app(app, version="1.0.0")
