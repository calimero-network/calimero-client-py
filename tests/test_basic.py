#!/usr/bin/env python3
"""
Basic tests for Calimero Client Python Library.
"""

import pytest
from calimero_client_py import create_connection, create_client, ClientError, AuthMode


def test_imports():
    """Test that all required imports work."""
    from calimero_client_py import (
        create_connection,
        create_client,
        ConnectionInfo,
        Client,
        JwtToken,
        ClientError,
        AuthMode,
    )

    assert create_connection is not None
    assert create_client is not None
    assert ClientError is not None
    assert AuthMode is not None


def test_connection_creation():
    """Test basic connection creation."""
    connection = create_connection(
        api_url="https://test.merod.dev.p2p.aws.calimero.network",
        node_name="test-dev-node",
    )
    assert connection is not None
    assert hasattr(connection, "api_url")


def test_client_creation():
    """Test basic client creation."""
    connection = create_connection(
        api_url="https://test.merod.dev.p2p.aws.calimero.network",
        node_name="test-dev-node",
    )
    client = create_client(connection)
    assert client is not None
    assert hasattr(client, "get_api_url")


def test_auth_mode():
    """Test AuthMode enum."""
    auth_none = AuthMode("none")
    auth_required = AuthMode("required")

    assert auth_none is not None
    assert auth_required is not None


def test_client_methods_exist():
    """Test that client has expected methods."""
    connection = create_connection(
        api_url="https://test.merod.dev.p2p.aws.calimero.network",
        node_name="test-dev-node",
    )
    client = create_client(connection)

    # Check that client has some expected methods
    expected_methods = [
        "list_contexts",
        "list_applications",
        "get_peers_count",
        "list_blobs",
    ]

    for method_name in expected_methods:
        assert hasattr(client, method_name), f"Client missing method: {method_name}"
        assert callable(
            getattr(client, method_name)
        ), f"Method {method_name} is not callable"


@pytest.mark.asyncio
async def test_async_fixtures():
    """Test that async fixtures work correctly."""
    # This test will use the async fixtures from conftest.py
    # The fixtures will be injected by pytest
    pass


# -----------------------------------------------------------------------
# Strict-tree refactor coordination (calimero-network/core PR #2200):
# the orphan-creating nest_group/unnest_group methods are removed and
# replaced by the atomic reparent_group primitive. These introspection
# tests pin the API surface so a regression (re-adding the old methods,
# losing the new one) fails CI fast without needing a live node.
# -----------------------------------------------------------------------


def _client():
    connection = create_connection(
        api_url="https://test.merod.dev.p2p.aws.calimero.network",
        node_name="test-dev-node",
    )
    return create_client(connection)


def test_client_has_reparent_group_method():
    """The pyo3 wrapper must expose reparent_group()."""
    client = _client()
    assert hasattr(
        client, "reparent_group"
    ), "Client.reparent_group missing — pyo3 binding not registered"
    assert callable(getattr(client, "reparent_group"))


def test_client_does_not_have_nest_group_method():
    """nest_group has been removed in the strict-tree refactor."""
    client = _client()
    assert not hasattr(
        client, "nest_group"
    ), "Client.nest_group should be removed — orphan-creating primitive"


def test_client_does_not_have_unnest_group_method():
    """unnest_group has been removed in the strict-tree refactor."""
    client = _client()
    assert not hasattr(
        client, "unnest_group"
    ), "Client.unnest_group should be removed — orphan-creating primitive"


# -----------------------------------------------------------------------
# `requester` is gone. core#3492 deleted the field from every admin request
# DTO — the node resolves the acting identity from the authenticated session
# instead — so a binding that still accepted the kwarg would take a value it
# could not send anywhere. These tests pin the removal from the Python side:
# the kwarg must be rejected outright rather than accepted and dropped, and
# the positional call every existing caller already makes must keep working.
# -----------------------------------------------------------------------


def _method_rejects_kwarg(method, kwarg_name: str) -> bool:
    """Pyo3 methods don't expose a Python inspect.signature — probe by call.

    A binding that does not declare the kwarg raises
    TypeError("unexpected keyword argument ...") before running any of its
    own code. Anything else means the kwarg was accepted.
    """
    try:
        method("a" * 44, **{kwarg_name: "not-a-real-public-key"})
    except TypeError as e:
        return kwarg_name in str(e) and "unexpected keyword" in str(e)
    except Exception:
        # Reached the body, so the signature took it.
        return False
    return False


def test_delete_context_rejects_requester_kwarg():
    """delete_context must not accept a `requester` the node cannot use."""
    client = _client()
    assert _method_rejects_kwarg(
        client.delete_context, "requester"
    ), "delete_context still accepts 'requester' — pyo3 signature not updated"


def test_delete_group_rejects_requester_kwarg():
    """delete_group must not accept a `requester` the node cannot use."""
    client = _client()
    assert _method_rejects_kwarg(
        client.delete_group, "requester"
    ), "delete_group still accepts 'requester' — pyo3 signature not updated"


def test_delete_namespace_rejects_requester_kwarg():
    """delete_namespace must not accept a `requester` the node cannot use."""
    client = _client()
    assert _method_rejects_kwarg(
        client.delete_namespace, "requester"
    ), "delete_namespace still accepts 'requester' — pyo3 signature not updated"


def test_set_member_auto_follow_rejects_requester_kwarg():
    """set_member_auto_follow carried the last `requester` on the group path."""
    client = _client()
    try:
        client.set_member_auto_follow(
            "g", "m", True, True, requester="not-a-real-public-key"
        )
    except TypeError as e:
        assert "unexpected keyword" in str(e) and "requester" in str(e)
    except Exception as e:  # reached the body — the signature took it
        raise AssertionError(
            f"set_member_auto_follow still accepts 'requester': {e}"
        ) from e


def test_client_does_not_have_create_account_method():
    """core#3470 deleted `create_account` — enrolment is implicit."""
    client = _client()
    assert not hasattr(
        client, "create_account"
    ), "Client.create_account should be removed — enrolment happens on join"


def test_delete_group_still_takes_a_bare_group_id():
    """The positional call every existing caller makes must keep working.

    We can't actually DELETE from a test environment — we only pin that the
    call dispatches. Any failure past the signature (network, server-side)
    is fine; a TypeError is not.
    """
    client = _client()
    with pytest.raises(Exception) as exc_info:
        client.delete_group("some-group-id")
    assert not isinstance(
        exc_info.value, TypeError
    ), f"delete_group('id') raised TypeError (signature regression): {exc_info.value}"
