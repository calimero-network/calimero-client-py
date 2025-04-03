# Calimero Network Python Client SDK

The **Calimero Python Client SDK** helps developers interact with decentralized apps by handling server communication. It simplifies the process, letting you focus on building your app while the SDK manages the technical details.

## Features

- JSON-RPC client for sending queries and updates to Calimero nodes
- WebSocket client for real-time subscriptions
- Authentication handling with Ed25519 keypairs
- Configuration management
- Type hints and comprehensive documentation

## Installation

```bash
pip install calimero-client-py
```

## Quick Start

### Using JsonRpcClient

```python
from calimero import JsonRpcClient

client = JsonRpcClient(
    base_url="http://localhost:2428",
    endpoint="/jsonrpc"
)

params = {
    "applicationId": "your_application_id",
    "method": "create_post",
    "argsJson": {"title": "My First Post", "text": "This is my first post"}
}

response = await client.mutate(params)
print(response)
```

### Using WsSubscriptionsClient

```python
from calimero import WsSubscriptionsClient

client = WsSubscriptionsClient(
    base_url="http://localhost:2428",
    endpoint="/ws"
)

await client.connect()
client.subscribe(["your_application_id"])

def callback(data):
    print(data)

client.add_callback(callback)
```

## Documentation

For detailed documentation, please visit [our documentation site](https://docs.calimero.network).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details. 

## Test Dependencies and Commands

To run tests, you need to install the test dependencies and then run the tests.

```bash
# Install test dependencies
pip install -e ".[test]"

# Run tests
pytest

# Run tests with coverage
pytest --cov=calimero

# Run specific test file
pytest tests/test_keypair.py
``` 