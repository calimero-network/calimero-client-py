#!/bin/bash

# Set environment variables
export CONTEXT_ID="test_context"
export EXECUTOR_PUBLIC_KEY="test_public_key"
export RPC_URL="http://localhost:2428"

# Create config directory and file if it doesn't exist
mkdir -p ~/.calimero/node1
if [ ! -f ~/.calimero/node1/config.toml ]; then
    cat > ~/.calimero/node1/config.toml << EOL
[network]
node_id = "test_node"
listen_addr = "0.0.0.0:2428"

[keypair]
public_key = "test_public_key"
private_key = "test_private_key"
EOL
fi

# Run the example
python3 kv_store_example.py 