import pytest
import os
from pathlib import Path
import tempfile
import json
from calimero import Ed25519Keypair, Config

@pytest.fixture
def temp_config_file():
    """Create a temporary config file for testing."""
    config_data = {
        "node": {
            "url": "http://localhost:2428"
        },
        "application": {
            "id": "test_app_id"
        },
        "executor": {
            "public_key": "test_public_key"
        }
    }
    
    with tempfile.NamedTemporaryFile(mode='w', suffix='.toml', delete=False) as f:
        json.dump(config_data, f)
        return f.name

@pytest.fixture
def ed25519_keypair():
    """Create a test Ed25519 keypair."""
    # Generate a new keypair for testing
    signing_key = nacl.signing.SigningKey.generate()
    return Ed25519Keypair(signing_key._signing_key)

@pytest.fixture
def config():
    """Create a test Config instance."""
    return Config(
        node_url="http://localhost:2428",
        application_id="test_app_id",
        executor_public_key="test_public_key"
    )

@pytest.fixture
def mock_ws_url():
    """Return a mock WebSocket URL for testing."""
    return "ws://localhost:2428/ws"

@pytest.fixture
def mock_rpc_url():
    """Return a mock RPC URL for testing."""
    return "http://localhost:2428/jsonrpc" 