# Testing with Merobox

This document explains how to use [Merobox](https://github.com/calimero-network/merobox) for testing the Calimero Python client with real KV store applications.

## What is Merobox?

Merobox is a Python CLI tool for managing Calimero nodes in Docker containers and executing complex blockchain workflows. It provides:

- **Node Management**: Start, stop, and monitor Calimero nodes in Docker
- **Workflow Orchestration**: Execute multi-step workflows with YAML configuration
- **Application Management**: Install and manage WASM applications
- **Context Management**: Create and manage blockchain contexts
- **Identity Management**: Generate and manage cryptographic identities

## Prerequisites

1. **Python 3.8+** with virtual environment
2. **Docker** running and accessible
3. **Merobox** installed: `pip install merobox`

## Quick Start

### 1. Install Merobox

```bash
# Activate your virtual environment
source venv/bin/activate

# Install merobox
pip install merobox
```

### 2. Run Tests with Pytest

The merobox integration is built directly into the pytest testing framework:

```bash
# Run all tests (this will automatically start merobox and run workflows)
pytest tests/ -v

# Run specific test categories
pytest tests/core/ -v      # JSON-RPC and WebSocket tests
pytest tests/admin/ -v     # Admin API tests

# Run with coverage
pytest --cov=calimero tests/ -v
```

The test framework automatically:
- Starts merobox nodes using the workflow configurations
- Sets up the KV store application and context
- Provides fixtures for testing
- Cleans up the environment after tests complete

### 3. Manual Workflow Execution

Alternatively, you can run workflows manually:

```bash
# Set up the test environment
merobox bootstrap run workflows/kv-store-setup.yml

# Run the complete test suite
merobox bootstrap run workflows/kv-store-testing.yml

# Check node status
merobox list
merobox health

# Stop all nodes
merobox stop --all
```

## Pytest Integration

The merobox testing is integrated directly into the pytest framework through `tests/conftest.py`:

### Fixtures

```python
# Import merobox testing utilities
from merobox.testing import run_workflow, nodes

# Merobox workflow fixtures using decorators
@run_workflow("workflows/kv-store-setup.yml", scope="session", prefix="calimero-node")
def merobox_kv_store_setup():
    """Merobox fixture that sets up the KV store testing environment."""
    pass

@run_workflow("workflows/kv-store-testing.yml", scope="session", prefix="calimero-node")
def merobox_kv_store_testing():
    """Merobox fixture that runs the complete KV store testing workflow."""
    pass

# Test fixtures that use merobox
@pytest.fixture(scope="session")
def merobox_node(merobox_kv_store_setup):
    """Provides merobox node configuration from the workflow."""
    # Extract endpoints and configuration from the workflow
    pass

@pytest.fixture(scope="session")
def kv_store_environment(merobox_kv_store_setup):
    """Provides KV store environment configuration."""
    pass
```

### How It Works

1. **Session Scope**: The merobox workflows run once per test session
2. **Automatic Setup**: Before any tests run, merobox starts nodes and runs the setup workflow
3. **Fixture Dependencies**: Test fixtures depend on the merobox fixtures
4. **Automatic Cleanup**: When the session ends, merobox automatically stops and cleans up nodes

## Workflow Files

### KV Store Setup (`workflows/kv-store-setup.yml`)

This workflow sets up the testing environment:
- Starts a Calimero node
- Installs the KV store WASM application from the [official release](https://github.com/calimero-network/core/releases/download/v0.8.0/kv_store.wasm)
- Creates a context for the application
- Generates test identities
- Sets up permissions

### KV Store Testing (`workflows/kv-store-testing.yml`)

This workflow runs comprehensive tests:
- All setup steps from the setup workflow
- KV store operations (set, get, remove)
- Verification of operations
- Results display

## Testing the JSON-RPC Client

The test script demonstrates how to test the JSON-RPC client with the running merobox node:

```python
from calimero.json_rpc_client import JsonRpcClient

# Create client connected to merobox node
client = JsonRpcClient("http://localhost:2528/jsonrpc/dev")

# Test KV store operations
set_result = await client.mutate({
    "method": "set",
    "args": {"key": "test_key", "value": "test_value"}
})

get_result = await client.query({
    "method": "get",
    "args": {"key": "test_key"}
})

remove_result = await client.mutate({
    "method": "remove",
    "args": {"key": "test_key"}
})
```

## Node Endpoints

When merobox is running, the following endpoints are available:

- **Admin API**: `http://localhost:2428`
- **JSON-RPC**: `http://localhost:2528`
- **WebSocket**: `ws://localhost:2628`

## Workflow Structure

Merobox workflows use YAML configuration with the following structure:

```yaml
name: Workflow Name
description: Workflow description
nodes:
  chain_id: testnet-1
  count: 1
  image: ghcr.io/calimero-network/merod:6a47604
  prefix: calimero-node

steps:
  - name: Step Name
    type: step_type
    node: calimero-node-1
    # ... step-specific parameters
    outputs:
      variable_name: output_field
```

## Available Step Types

- **`install_application`**: Install WASM applications
- **`create_context`**: Create blockchain contexts
- **`create_identity`**: Generate cryptographic identities
- **`invite_identity`**: Invite identities to contexts
- **`join_context`**: Join contexts using invitations
- **`call`**: Execute smart contract functions
- **`script`**: Run shell scripts on nodes

## Troubleshooting

### Common Issues

1. **Docker not running**
   ```bash
   # Check Docker status
   docker ps
   ```

2. **Port conflicts**
   ```bash
   # Check what's using the ports
   lsof -i :2428
   lsof -i :2528
   lsof -i :2628
   ```

3. **Workflow validation errors**
   ```bash
   # Validate workflow before running
   merobox bootstrap validate workflows/kv-store-setup.yml
   ```

4. **Node startup issues**
   ```bash
   # Check node logs
   merobox logs calimero-node-1 --follow
   ```

### Debug Mode

Enable debug logging for more verbose output:

```bash
export LOG_LEVEL=DEBUG
merobox bootstrap run workflows/kv-store-setup.yml
```

### Clean Slate

If you encounter issues, you can start fresh:

```bash
# Stop all nodes
merobox stop --all

# Remove all data (WARNING: This deletes everything)
merobox nuke

# Start over
merobox bootstrap run workflows/kv-store-setup.yml
```

## Integration with Pytest

You can also integrate merobox workflows with your pytest tests:

```python
import pytest
import subprocess

@pytest.fixture(scope="session")
def merobox_environment():
    """Set up merobox environment for testing."""
    # Run setup workflow
    subprocess.run(["merobox", "bootstrap", "run", "workflows/kv-store-setup.yml"], check=True)
    
    yield
    
    # Cleanup
    subprocess.run(["merobox", "stop", "--all"], check=False)

@pytest.mark.asyncio
async def test_kv_store_operations(merobox_environment):
    """Test KV store operations with merobox."""
    client = JsonRpcClient("http://localhost:2528/jsonrpc/dev")
    
    # Your test code here
    result = await client.query({"method": "get", "args": {"key": "test"}})
    assert result is not None
```

## Next Steps

1. **Customize Workflows**: Modify the workflow files to test specific scenarios
2. **Add More Applications**: Install and test other WASM applications
3. **Performance Testing**: Use merobox for load testing and performance validation
4. **CI/CD Integration**: Integrate merobox workflows into your CI/CD pipeline

## Resources

- [Merobox GitHub Repository](https://github.com/calimero-network/merobox)
- [Merobox PyPI Package](https://pypi.org/project/merobox/)
- [Calimero Network Documentation](https://docs.calimero.network/)
- [KV Store Application Source](https://github.com/calimero-network/core)
