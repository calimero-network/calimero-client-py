# Calimero Network Python Client SDK

The **Calimero Python Client SDK** helps developers interact with decentralized apps by handling server communication. It simplifies the process, letting you focus on building your app while the SDK manages the technical details.

## Features

- **Strongly Typed API**: Full Pydantic-based request/response models for type safety and validation
- **JSON-RPC client** for sending queries and updates to Calimero nodes
- **WebSocket client** for real-time subscriptions
- **Admin API client** for administrative operations
- **Authentication handling** with merobox integration
- **Configuration management**
- **Type hints** and comprehensive documentation

## Installation

```bash
pip install calimero-client-py==0.1.2
```

## Quick Start

### Basic JSON-RPC Example

Here's a complete example of using the SDK to interact with a key-value store:

```python
import asyncio
import toml
import os
from pathlib import Path
from calimero import JsonRpcClient

async def main():
    # Initialize the client
    client = JsonRpcClient(
        rpc_url="http://localhost:2428/jsonrpc/dev"
    )

    # Example: Set a key-value pair
    set_params = {
        "applicationId": "your_application_id",
        "method": "set",
        "argsJson": {"key": "my_key", "value": "my_value"}
    }
    set_response = await client.mutate(set_params)
    print("Set response:", set_response)

    # Example: Get a value
    get_params = {
        "applicationId": "your_application_id",
        "method": "get",
        "argsJson": {"key": "my_key"}
    }
    get_response = await client.query(get_params)
    print("Get response:", get_response)

if __name__ == "__main__":
    asyncio.run(main())
```

### WebSocket Example

Here's how to use the WebSocket client for real-time updates:

```python
import asyncio
import toml
import os
from pathlib import Path
from calimero import WsSubscriptionsClient

async def main():
    # Initialize the client
    client = WsSubscriptionsClient(
        base_url="http://localhost:2428",
        endpoint="/ws"
    )

    # Connect and subscribe
    await client.connect()
    client.subscribe(["your_application_id"])

    # Add callback for received messages
    def callback(data):
        print("Received update:", data)

    client.add_callback(callback)

    # Keep the connection alive
    try:
        while True:
            await asyncio.sleep(1)
    except KeyboardInterrupt:
        await client.disconnect()

if __name__ == "__main__":
    asyncio.run(main())
```

## Strongly Typed API

The Calimero client now features a comprehensive strongly typed Request and Response system built with Pydantic, providing:

- **Type Safety**: Compile-time and runtime type checking
- **Validation**: Automatic validation of request/response data
- **Documentation**: Self-documenting API with field descriptions
- **IDE Support**: Better autocomplete and error detection
- **Consistency**: Standardized error handling and response formats

### Admin Client with Strong Types

```python
from calimero.types import CreateContextRequest, InstallApplicationRequest
from calimero.admin_client import AdminClient

# Initialize client
admin_client = AdminClient("http://localhost:2428")

# Create context with strongly typed request
create_request = CreateContextRequest(
    application_id="my-app-123",
    protocol="near",
    initialization_params=["param1", "param2"]
)

response = await admin_client.create_context(create_request)
print(f"Context ID: {response.context_id}")

# Install application with strongly typed request
install_request = InstallApplicationRequest(
    url="https://example.com/app.zip",
    hash="abc123def456",
    metadata=b"app metadata"
)

response = await admin_client.install_application(install_request)
print(f"Application ID: {response.application_id}")
```

### WebSocket Client with Strong Types

```python
from calimero.types import SubscribeRequest, UnsubscribeRequest
from calimero.ws_subscriptions_client import WsSubscriptionsClient

# Initialize client
ws_client = WsSubscriptionsClient("http://localhost:2428")

# Subscribe with strongly typed request
subscribe_request = SubscribeRequest(
    application_ids=["app1", "app2", "app3"]
)
ws_client.subscribe(subscribe_request)

# Unsubscribe with strongly typed request
unsubscribe_request = UnsubscribeRequest(
    application_ids=["app2"]
)
ws_client.unsubscribe(unsubscribe_request)
```

### Type Validation

Pydantic automatically validates all data:

```python
from calimero.types import CreateContextRequest

# This will work
valid_request = CreateContextRequest(
    application_id="valid-app",
    protocol="near"
)

# This will raise a validation error
try:
    invalid_request = CreateContextRequest(
        application_id="",  # Empty string not allowed
        protocol="invalid-protocol"  # Not in allowed values
    )
except Exception as e:
    print(f"Validation error: {e}")
```

### Error Handling

All responses are properly typed and include error information:

```python
try:
    response = await admin_client.create_context(create_request)
    if response.success:
        print(f"Success: {response.context_id}")
    else:
        print(f"Error: {response.error}")
except ValueError as e:
    print(f"Request failed: {e}")
```

## Available Types

### Admin API Types

#### Context Management
- `CreateContextRequest` / `CreateContextResponse`
- `ListContextsResponse`
- `GetContextResponse`
- `DeleteContextResponse`

#### Identity Management
- `GenerateIdentityResponse`
- `ListIdentitiesResponse`

#### Application Management
- `InstallDevApplicationRequest` / `InstallDevApplicationResponse`
- `InstallApplicationRequest` / `InstallApplicationResponse`
- `ListApplicationsResponse`
- `GetApplicationResponse`
- `UninstallApplicationResponse`

#### Blob Management
- `UploadBlobRequest` / `UploadBlobResponse`
- `DownloadBlobResponse`
- `ListBlobsResponse`
- `GetBlobInfoResponse`
- `DeleteBlobResponse`

#### Context Operations
- `UpdateContextApplicationRequest` / `UpdateContextApplicationResponse`
- `GetContextStorageResponse`
- `GetContextValueResponse`
- `GetContextStorageEntriesRequest` / `GetContextStorageEntriesResponse`
- `GetProxyContractResponse`

#### Capability Management
- `GrantCapabilitiesRequest` / `GrantCapabilitiesResponse`
- `RevokeCapabilitiesRequest` / `RevokeCapabilitiesResponse`

#### Proposal Management
- `GetProposalsRequest` / `GetProposalsResponse`
- `GetProposalResponse`
- `GetNumberOfActiveProposalsResponse`
- `GetProposalApprovalsCountResponse`
- `GetProposalApproversResponse`

#### Alias Management
- `CreateContextAliasRequest` / `CreateAliasResponse`
- `CreateApplicationAliasRequest`
- `CreateIdentityAliasRequest`
- `LookupAliasResponse`
- `ListAliasesResponse`
- `DeleteAliasResponse`

#### System Operations
- `HealthCheckResponse`
- `IsAuthenticatedResponse`
- `GetPeersResponse`
- `GetPeersCountResponse`
- `GetCertificateResponse`
- `SyncContextResponse`

### JSON-RPC Types
- `JsonRpcRequest`
- `JsonRpcExecuteRequest`
- `JsonRpcResponse`
- `JsonRpcErrorInfo`

### WebSocket Types
- `WebSocketMessage`
- `SubscribeRequest`
- `UnsubscribeRequest`
- `SubscriptionUpdate`

## Migration Guide

### From Old AdminClient Usage

**Before:**
```python
response = await admin_client.create_context(
    application_id="app123",
    protocol="near",
    initialization_params=["param1"]
)
```

**After:**
```python
from calimero.types import CreateContextRequest

request = CreateContextRequest(
    application_id="app123",
    protocol="near",
    initialization_params=["param1"]
)
response = await admin_client.create_context(request)
```

### From Old WebSocket Usage

**Before:**
```python
ws_client.subscribe(["app1", "app2"])
```

**After:**
```python
from calimero.types import SubscribeRequest

request = SubscribeRequest(application_ids=["app1", "app2"])
ws_client.subscribe(request)
```

## Documentation

For detailed documentation, please visit [our documentation site](https://docs.calimero.network).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 

## Development

### Setting Up Development Environment

```bash
# Create and activate virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install development dependencies
pip install -e ".[test]"
```

### Running Tests

```bash
# Run all tests
pytest

# Run tests with coverage
pytest --cov=calimero

# Run specific test file
pytest tests/core/ -v
```

### Testing with Merobox

For real integration testing with actual Calimero nodes and the KV store application, Merobox is integrated directly into the pytest framework:

```bash
# Install merobox
pip install merobox

# Run all tests (automatically uses merobox workflows)
pytest tests/ -v

# Run specific test categories
pytest tests/core/ -v      # JSON-RPC and WebSocket tests
pytest tests/admin/ -v     # Admin API tests
```

The test framework automatically:
- Starts merobox nodes using workflow configurations
- Sets up the KV store application and context
- Provides fixtures for testing
- Cleans up the environment after tests complete

See [MERODOX_TESTING.md](MERODOX_TESTING.md) for detailed instructions on the merobox integration.

### Building and Publishing

```bash
# Install build tools
pip install --upgrade build twine

# Build the package
python -m build

# Publish to PyPI
twine upload dist/*
```

## Requirements

- Python 3.8+
- Pydantic 2.5.0+
- All other existing dependencies

## Benefits of Strong Types

1. **Type Safety**: Catch errors at development time
2. **Validation**: Ensure data integrity
3. **Documentation**: Self-documenting API
4. **IDE Support**: Better autocomplete and error detection
5. **Consistency**: Standardized error handling
6. **Maintainability**: Easier to refactor and maintain
7. **Backward Compatibility**: All existing code continues to work unchanged 