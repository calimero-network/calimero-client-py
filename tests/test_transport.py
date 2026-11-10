"""
The runtime / route split: `Client.request` is the one call a route layer
(hand-written or generated from the node's OpenAPI document) sends through,
and `calimero.transport` describes it. The signatures and `route_path` here;
the call itself against a real cluster in test_cloud_v2_api.py.
"""

import inspect

import pytest

from calimero.transport import Transport, route_path
from calimero_client_py import Client


def test_request_takes_method_path_and_keyword_options():
    sig = inspect.signature(Client.request)
    names = list(sig.parameters)[1:]
    assert names == ["method", "path", "query", "body", "passkey", "repeatable"]
    for name in ("query", "body", "passkey", "repeatable"):
        assert sig.parameters[name].kind is inspect.Parameter.KEYWORD_ONLY, name
        assert sig.parameters[name].default is None, name


def test_the_protocol_names_the_same_call():
    proto = list(inspect.signature(Transport.request).parameters)
    assert proto == list(inspect.signature(Client.request).parameters)


def test_route_path_encodes_each_segment():
    assert (
        route_path("apps/{app}/blobs/{id}", app="a/../b", id=7)
        == "apps/a%2F..%2Fb/blobs/7"
    )
    assert route_path("orgs") == "orgs"
    with pytest.raises(KeyError):
        route_path("orgs/{org}")
