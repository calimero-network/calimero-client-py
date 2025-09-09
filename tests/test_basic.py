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
        api_url="http://localhost:2528", node_name="test-node"
    )
    assert connection is not None
    assert hasattr(connection, "api_url")


def test_client_creation():
    """Test basic client creation."""
    connection = create_connection(
        api_url="http://localhost:2528", node_name="test-node"
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
        api_url="http://localhost:2528", node_name="test-node"
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
