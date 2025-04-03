#!/bin/bash

# Set node name
export CALIMERO_NODE_NAME="node1"

# Create config directory if it doesn't exist
mkdir -p ~/.calimero/${CALIMERO_NODE_NAME}

# Create config file
cat > ~/.calimero/${CALIMERO_NODE_NAME}/config.toml << EOL
[identity]
peer_id = "12D3KooWJMt7v6HNDUfu3z3iebPchZ36HZzVPXJSFqPBvGqt4Uqj"
keypair = "23jhTdSZPXYnuaYBfUXZHAgxJj9LE8vXpK7JQmgFtqEX6QtZks3hK8945LALAH3dVtqog5dYB6gmdKKn71v4RfXKRnsZo"

[swarm]
listen = [
    "/ip4/0.0.0.0/tcp/2528",
    "/ip4/0.0.0.0/udp/2528/quic-v1",
    "/ip6/::/tcp/2528",
    "/ip6/::/udp/2528/quic-v1",
]

[server]
listen = [
    "/ip4/127.0.0.1/tcp/2428",
    "/ip6/::1/tcp/2428",
]

[server.admin]
enabled = true

[server.jsonrpc]
enabled = true

[server.websocket]
enabled = true
EOL

# Set environment variables
export CONTEXT_ID="test_context"
export EXECUTOR_PUBLIC_KEY="ed25519:DcPDQFw8VljMCizVs9vV+CLjjanQdLHgX1HCqMTK2KeZ"
export RPC_URL="http://localhost:2428"
export VERBOSE=1

# Activate virtual environment and run the example
source example_venv/bin/activate.fish && python3 kv_store_example.py 