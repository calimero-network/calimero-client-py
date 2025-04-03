# Calimero Python SDK Examples

This directory contains example scripts demonstrating how to use the Calimero Python SDK.

## OnlyPeers Example

The `onlypeers_get_all_posts.py` script demonstrates how to fetch all posts from the OnlyPeers application.

### Prerequisites

1. Install the Calimero Python SDK:
```bash
pip install calimero-client-py
```

2. Set up your Calimero configuration:
   - Create a config file at `~/.calimero/node1/config.toml`
   - The config file should contain your node configuration and keypair

### Step-by-Step Guide to Run the Example

1. **Clone the Repository**
```bash
git clone https://github.com/calimero-network/calimero-client-py.git
cd calimero-client-py
```

2. **Install Dependencies**
```bash
pip install -e ".[test]"  # Install the package in development mode with test dependencies
```

3. **Set Up Environment Variables**
```bash
# Required variables
export CONTEXT_ID="your_application_context_id"  # Get this from your Calimero node
export EXECUTOR_PUBLIC_KEY="your_executor_public_key"  # Your public key

# Optional variables
export RPC_URL="http://your_calimero_node:2428"  # Defaults to http://localhost:2428
export VERBOSE=1  # Set to 1 for detailed output
```

4. **Verify Your Configuration**
```bash
# Check if your config file exists
ls ~/.calimero/node1/config.toml

# Check if environment variables are set
echo "CONTEXT_ID: $CONTEXT_ID"
echo "EXECUTOR_PUBLIC_KEY: $EXECUTOR_PUBLIC_KEY"
```

5. **Run the Example**
```bash
# Make the script executable
chmod +x examples/onlypeers_get_all_posts.py

# Run the script
python examples/onlypeers_get_all_posts.py
```

### Expected Output

If everything is set up correctly, you should see output similar to:
```
Context id: your_application_context_id
Executor public key: your_executor_public_key
Initialized Calimero JSON RPC Client with URL: http://localhost:2428/jsonrpc/dev
[Your posts data will be displayed here]
```

If VERBOSE=1 is set, you'll also see the full JSON-RPC response.

### Troubleshooting

1. **Missing Environment Variables**
   - Error: "CONTEXT_ID environment variable is not set"
   - Solution: Make sure you've set all required environment variables

2. **Configuration File Issues**
   - Error: "Could not load config file"
   - Solution: Verify your config file exists at `~/.calimero/node1/config.toml`

3. **Connection Issues**
   - Error: "Connection refused" or timeout errors
   - Solution: 
     - Check if your Calimero node is running
     - Verify the RPC_URL is correct
     - Check your network connectivity

4. **Authentication Issues**
   - Error: "Invalid signature" or "Unauthorized"
   - Solution:
     - Verify your keypair in the config file
     - Check if your executor public key is correct
     - Ensure your node accepts the provided credentials

### Common Issues and Solutions

1. **Python Version**
   - Ensure you're using Python 3.8 or higher
   - Check version: `python --version`

2. **Dependencies**
   - If you get import errors, try reinstalling dependencies:
   ```bash
   pip uninstall calimero-client-py
   pip install -e .
   ```

3. **Permission Issues**
   - If you get permission errors on the config file:
   ```bash
   chmod 600 ~/.calimero/node1/config.toml
   ```

4. **Network Configuration**
   - If using a remote node, ensure:
     - The node is accessible from your machine
     - Firewall rules allow the connection
     - The correct port (2428) is open

### Additional Resources

- [Calimero Documentation](https://docs.calimero.network)
- [Python SDK API Reference](https://docs.calimero.network/python-sdk)
- [Node Configuration Guide](https://docs.calimero.network/node-configuration)

### Support

If you encounter issues not covered in this guide:
1. Check the [GitHub Issues](https://github.com/calimero-network/calimero-client-py/issues)
2. Join the [Calimero Discord](https://discord.gg/calimero)
3. Contact support at support@calimero.network 