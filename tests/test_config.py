import pytest
import os
from calimero import Config

def test_config_creation(config):
    """Test Config instance creation."""
    assert config.node_url == "http://localhost:2428"
    assert config.application_id == "test_app_id"
    assert config.executor_public_key == "test_public_key"
    assert config.keypair is None

def test_config_from_file(temp_config_file):
    """Test loading config from file."""
    config = Config.load_from_file(temp_config_file)
    assert config.node_url == "http://localhost:2428"
    assert config.application_id == "test_app_id"
    assert config.executor_public_key == "test_public_key"

def test_config_from_env(monkeypatch):
    """Test loading config from environment variables."""
    # Set environment variables
    monkeypatch.setenv('CALIMERO_NODE_URL', 'http://test:2428')
    monkeypatch.setenv('CALIMERO_APPLICATION_ID', 'test_app')
    monkeypatch.setenv('CALIMERO_EXECUTOR_PUBLIC_KEY', 'test_key')
    
    config = Config.load_from_env()
    assert config.node_url == 'http://test:2428'
    assert config.application_id == 'test_app'
    assert config.executor_public_key == 'test_key'

def test_config_default_values(monkeypatch):
    """Test config default values when environment variables are not set."""
    # Clear environment variables
    monkeypatch.delenv('CALIMERO_NODE_URL', raising=False)
    monkeypatch.delenv('CALIMERO_APPLICATION_ID', raising=False)
    monkeypatch.delenv('CALIMERO_EXECUTOR_PUBLIC_KEY', raising=False)
    
    config = Config.load_from_env()
    assert config.node_url == 'http://localhost:2428'
    assert config.application_id == ''
    assert config.executor_public_key == ''

def test_config_with_keypair(config, ed25519_keypair):
    """Test Config with Ed25519Keypair."""
    config.keypair = ed25519_keypair
    assert config.keypair == ed25519_keypair
    assert config.keypair.public_key is not None 