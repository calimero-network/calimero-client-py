"""
Per-package web origins (core's `/package-origins/{package}`) through
CloudClient: listing, the passkey step-up on adding and removing one,
`served=` for a custom domain (and the operator's `ops_served_origins`), and
`origin=` on sign-up and sign-in.

These check the signatures only; the routes themselves are exercised against
a real cluster in test_cloud_v2_api.py (skipped on a node without them).
"""

import inspect

from calimero_client_py import Client


def params(method) -> list:
    return list(inspect.signature(getattr(Client, method)).parameters)


def test_the_origin_methods_take_what_the_routes_take():
    assert params("list_package_origins")[1:] == ["package"]
    assert params("add_package_origin")[1:] == [
        "package",
        "origin",
        "passkey",
        "rp_id",
        "served",
    ]
    assert params("remove_package_origin")[1:] == ["package", "origin", "passkey"]
    rp_id = inspect.signature(Client.add_package_origin).parameters["rp_id"]
    assert rp_id.default is None
    served = inspect.signature(Client.add_package_origin).parameters["served"]
    assert served.default is False


def test_the_operator_lists_served_origins():
    assert params("ops_served_origins")[1:] == []


def test_sign_up_and_sign_in_take_an_optional_origin():
    for method in ("sign_up", "sign_in"):
        origin = inspect.signature(getattr(Client, method)).parameters["origin"]
        assert origin.default is None, method


def test_delete_confirmed_takes_an_optional_body():
    assert params("delete_confirmed")[1:] == ["path", "passkey", "body"]
