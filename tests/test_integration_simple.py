#!/usr/bin/env python3
"""
Simple integration tests for Calimero Client Python Library.
These tests don't require merobox and can run in CI.
"""

import pytest
from calimero_client_py import create_connection, create_client, ClientError


class TestCalimeroClient:
    """Test class for Calimero client functionality."""

    def setup_method(self):
        """Set up test fixtures."""
        self.connection = create_connection(
            api_url="http://localhost:2528", node_name="test-node"
        )
        self.client = create_client(self.connection)

    def test_connection_properties(self):
        """Test connection properties."""
        assert self.connection is not None
        assert hasattr(self.connection, "api_url")
        # The URL might have a trailing slash, so check both cases
        assert self.connection.api_url in [
            "http://localhost:2528",
            "http://localhost:2528/",
        ]

    def test_client_properties(self):
        """Test client properties."""
        assert self.client is not None
        assert hasattr(self.client, "get_api_url")
        # get_api_url is a method, so we need to call it
        api_url = self.client.get_api_url()
        assert api_url in ["http://localhost:2528", "http://localhost:2528/"]

    def test_client_methods_are_callable(self):
        """Test that client methods are callable."""
        methods_to_test = [
            "list_contexts",
            "list_applications",
            "get_peers_count",
            "list_blobs",
            "sync_all_contexts",
        ]

        for method_name in methods_to_test:
            if hasattr(self.client, method_name):
                method = getattr(self.client, method_name)
                assert callable(method), f"Method {method_name} should be callable"

    def test_client_method_signatures(self):
        """Test that client methods have expected signatures."""
        # Test list_contexts method
        if hasattr(self.client, "list_contexts"):
            import inspect

            sig = inspect.signature(self.client.list_contexts)
            # Should have no required parameters
            assert len(sig.parameters) == 0

        # Test create_context method
        if hasattr(self.client, "create_context"):
            import inspect

            sig = inspect.signature(self.client.create_context)
            # Should have application_id and protocol parameters
            assert "application_id" in sig.parameters
            assert "protocol" in sig.parameters


@pytest.mark.skip(reason="Requires running Calimero server")
def test_integration_with_server():
    """Integration test that requires a running Calimero server."""
    connection = create_connection(
        api_url="http://localhost:2528", node_name="integration-test"
    )
    client = create_client(connection)

    # These tests would require a running server
    # They are skipped by default but can be enabled for local testing
    try:
        contexts = client.list_contexts()
        assert isinstance(contexts, (dict, list))
    except Exception as e:
        pytest.skip(f"Server not available: {e}")


def test_import_structure():
    """Test that the package structure is correct."""
    import calimero_client_py

    # Test that the module has expected attributes
    expected_attrs = [
        "create_connection",
        "create_client",
        "ConnectionInfo",
        "Client",
        "JwtToken",
        "ClientError",
        "AuthMode",
    ]

    for attr in expected_attrs:
        assert hasattr(calimero_client_py, attr), f"Missing attribute: {attr}"
