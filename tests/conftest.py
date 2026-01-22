"""
Pytest configuration and fixtures for Calimero Client Python tests.
"""

import pytest
import asyncio
from calimero_client_py import create_connection, create_client, AuthMode


@pytest.fixture
def event_loop():
    """Create an instance of the default event loop for the test session."""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()


@pytest.fixture
async def test_connection():
    """Create a test connection for integration tests."""
    connection = create_connection(
        api_url="https://test.merod.dev.p2p.aws.calimero.network",
        node_name="test-dev-node",
    )
    return connection


@pytest.fixture
async def test_client(test_connection):
    """Create a test client for integration tests."""
    client = create_client(test_connection)
    return client
