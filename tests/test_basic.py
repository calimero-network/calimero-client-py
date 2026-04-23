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
# Optional `requester` parameter on delete_context / delete_group /
# delete_namespace. The server requires an admin requester for
# group-registered operations (e.g.
# core/crates/context/src/handlers/delete_context.rs:54-68). These
# tests pin the parameter into the Python binding so a regression
# (dropping the param, renaming it, or breaking backward compat) fails
# CI without needing a live node.
# -----------------------------------------------------------------------


def _method_accepts_kwarg(method, kwarg_name: str) -> bool:
    """Pyo3 methods don't expose a Python inspect.signature — probe by call.

    We pass the kwarg along with a bogus value that triggers our own
    ValueError at parse time. If the binding accepts the kwarg name, we
    reach the parse step and see ValueError. If the binding doesn't know
    the kwarg, pyo3 raises TypeError("unexpected keyword argument ...")
    before any parse happens.
    """
    try:
        # Bogus but well-formed-looking public-key string. Real call will
        # fail server-side; we only care whether the binding accepts the kwarg.
        method("a" * 44, **{kwarg_name: "not-a-real-public-key"})
    except TypeError as e:
        # "got an unexpected keyword argument 'requester'" means the
        # binding doesn't declare the kwarg.
        if kwarg_name in str(e) and "unexpected keyword" in str(e):
            return False
        # Any other TypeError: signature accepted the kwarg but failed
        # later (e.g. required positional was a dummy) — which still
        # means the kwarg is known.
        return True
    except ValueError:
        # Our own "Invalid requester public key '...'" — the kwarg was
        # accepted and reached our PublicKey parser. That's what we want.
        return True
    except Exception:
        # Any other exception (RuntimeError from network, etc.) means
        # the kwarg was accepted and the call progressed past signature
        # parsing. Treat as accepted.
        return True
    return True


def test_delete_context_accepts_requester_kwarg():
    """delete_context must accept an optional `requester` keyword."""
    client = _client()
    assert _method_accepts_kwarg(
        client.delete_context, "requester"
    ), "delete_context is missing the 'requester' kwarg — pyo3 signature not updated"


def test_delete_group_accepts_requester_kwarg():
    """delete_group must accept an optional `requester` keyword."""
    client = _client()
    assert _method_accepts_kwarg(
        client.delete_group, "requester"
    ), "delete_group is missing the 'requester' kwarg — pyo3 signature not updated"


def test_delete_namespace_accepts_requester_kwarg():
    """delete_namespace must accept an optional `requester` keyword."""
    client = _client()
    assert _method_accepts_kwarg(
        client.delete_namespace, "requester"
    ), "delete_namespace is missing the 'requester' kwarg — pyo3 signature not updated"


def test_delete_group_rejects_invalid_requester_public_key():
    """Invalid requester string should fail fast with ValueError.

    delete_group takes group_id as a plain string (no ContextId parse),
    so an invalid requester hits the PublicKey parser deterministically.
    This pins the validation path: the pyo3 binding parses the requester
    string as PublicKey before any network round-trip. A workflow author
    who typos the key gets a meaningful Python ValueError, not a generic
    server 500.
    """
    client = _client()
    with pytest.raises(ValueError, match="Invalid requester public key"):
        client.delete_group("some-group-id", requester="not-a-real-public-key")


def test_delete_namespace_rejects_invalid_requester_public_key():
    """Same contract as delete_group — validates the requester before dispatch."""
    client = _client()
    with pytest.raises(ValueError, match="Invalid requester public key"):
        client.delete_namespace("some-namespace-id", requester="not-a-real-public-key")


def test_delete_group_backward_compatible_without_requester():
    """Existing callers that omit requester must keep working (no signature break).

    We can't actually DELETE from a test environment — we just verify
    the call doesn't raise TypeError about the signature. Any other
    failure (network, server-side) is fine; what we're pinning is that
    the old single-argument call still dispatches.
    """
    client = _client()
    with pytest.raises(Exception) as exc_info:
        client.delete_group("some-group-id")
    assert not isinstance(
        exc_info.value, TypeError
    ), f"delete_group without requester raised TypeError (signature regression): {exc_info.value}"
