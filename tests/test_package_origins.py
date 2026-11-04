"""
Per-package web origins (core's `/package-origins/{package}`) through
CloudClient: listing, the passkey step-up on adding and removing one, and
`origin=` on sign-up and sign-in.

The signatures are checked always. The routes need a merod built from a core
with per-package origins (branch `claude/package-origins`, not yet released):
those tests run only with `MEROD_PACKAGE_ORIGINS=1` set and the same binary and
fixture test_cloud_v2.py needs.
"""

import inspect
import json
import os

import pytest

from calimero_client_py import Client, CloudError, SoftPasskey


def params(method) -> list:
    return list(inspect.signature(getattr(Client, method)).parameters)


def test_the_origin_methods_take_what_the_routes_take():
    assert params("list_package_origins")[1:] == ["package"]
    assert params("add_package_origin")[1:] == ["package", "origin", "passkey", "rp_id"]
    assert params("remove_package_origin")[1:] == ["package", "origin", "passkey"]
    rp_id = inspect.signature(Client.add_package_origin).parameters["rp_id"]
    assert rp_id.default is None


def test_sign_up_and_sign_in_take_an_optional_origin():
    for method in ("sign_up", "sign_in"):
        origin = inspect.signature(getattr(Client, method)).parameters["origin"]
        assert origin.default is None, method


def test_delete_confirmed_takes_an_optional_body():
    assert params("delete_confirmed")[1:] == ["path", "passkey", "body"]


needs_routes = pytest.mark.skipif(
    not os.environ.get("MEROD_PACKAGE_ORIGINS"),
    reason="needs a merod with per-package origins (MEROD_PACKAGE_ORIGINS=1)",
)


@pytest.fixture(scope="module")
def cluster():
    from test_cloud_v2 import Cluster, pytestmark as needs_merod

    if needs_merod.args[0]:
        pytest.skip(needs_merod.kwargs["reason"])
    c = Cluster(1)
    yield c
    c.stop()


@needs_routes
def test_an_unknown_package_has_no_origins(cluster):
    from test_cloud_v2 import code, person

    alice, pk = person(cluster, 0, "alice-origins")
    with pytest.raises(CloudError) as e:
        alice.list_package_origins("com.example.none")
    assert code(e.value) == "NOT_FOUND"
    with pytest.raises(CloudError) as e:
        alice.remove_package_origin("com.example.none", "https://a.example", pk)
    assert code(e.value) in ("NOT_FOUND", "FORBIDDEN")


@needs_routes
def test_adding_an_origin_asks_for_the_passkey(cluster):
    from test_cloud_v2 import code, person

    alice, _ = person(cluster, 0, "alice-origin-ask")
    # Without the passkey the node asks first, naming what is approved, or
    # refuses outright an account that may not change the package.
    with pytest.raises(CloudError) as e:
        alice.post("package-origins/com.example.none", {"origin": "https://a.example"})
    assert code(e.value) in ("CONFIRM_REQUIRED", "NOT_FOUND", "FORBIDDEN")
    if code(e.value) == "CONFIRM_REQUIRED":
        details = json.loads(e.value.args[3])
        assert details["subject"] == "package:com.example.none"
        assert details["method"] == "__add_origin"
        assert details["args"]["origin"] == "https://a.example"


@needs_routes
def test_sign_in_from_the_nodes_own_origin(cluster):
    from test_cloud_v2 import ORIGIN

    pk = SoftPasskey("localhost", ORIGIN)
    cluster.client(0).sign_up(pk, "alice-from-origin", origin=ORIGIN)
    c = cluster.client(0)
    c.sign_in(pk, origin=ORIGIN)
    assert "origin" in c.me()
