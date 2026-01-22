#!/usr/bin/env python3
"""
Example usage of calimero-client-py package

To use this example:
1. Install the package: pip install calimero-client-py
2. Run this script: python example_usage.py

Note: This example uses the test development node for demonstration

## Important: node_name for Authenticated Connections

The `node_name` parameter is critical for remote authenticated connections:

1. **Must be stable**: Use the same node_name across all sessions to ensure
   cached JWT tokens are found and reused.

2. **Must be unique per node**: Different remote nodes should have different
   node_names to avoid token collisions in the cache.

3. **Token caching**: Tokens are stored at ~/.merobox/auth_cache/{node_name}-{hash}.json

Example:
    # Good: Stable, descriptive names
    node_name="prod-api-node"
    node_name="dev-local-node"

    # Bad: Dynamic or non-descriptive names
    node_name=f"node-{uuid.uuid4()}"  # Different each time!
    node_name="node"  # Not unique across different servers

For local development without authentication, node_name can be any string or None.
"""

# This import will work after pip installation
# import calimero

# For development/testing, we'll use the local module
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "calimero"))

import calimero


def main():
    """Main example function demonstrating calimero-client-py usage."""
    print("Calimero Client Python Library Example")
    print("=" * 40)

    try:
        # Create a connection
        # NOTE: For authenticated remote connections, node_name should be:
        # - Stable: Same value across sessions (to find cached tokens)
        # - Unique: Different for each remote node (to avoid token collisions)
        print("Creating connection...")
        connection = calimero.create_connection(
            api_url="https://test.merod.dev.p2p.aws.calimero.network",
            node_name="test-dev-node",  # Use a stable name for production!
        )
        print(f"✓ Connection created: {connection}")

        # Create a client
        print("Creating client...")
        client = calimero.create_client(connection)
        print("✓ Client created")

        # List contexts
        print("Listing contexts...")
        contexts = client.list_contexts()
        print(f"✓ Found contexts: {contexts}")

        # Example of other operations (commented out as they require specific setup)
        # print("Listing applications...")
        # apps = client.list_applications()
        # print(f"✓ Found applications: {apps}")

    except calimero.ClientError as e:
        print(f"✗ Client error: {e}")
    except Exception as e:
        print(f"✗ Unexpected error: {e}")


def demo_token_cache_utilities():
    """Demonstrate token cache path utilities."""
    print("\n" + "=" * 40)
    print("Token Cache Utilities Demo")
    print("=" * 40)

    try:
        # Get the cache directory
        cache_dir = calimero.get_token_cache_dir()
        print(f"Token cache directory: {cache_dir}")

        # Get the cache path for a specific node
        node_name = "my-production-node"
        cache_path = calimero.get_token_cache_path(node_name)
        print(f"Token cache path for '{node_name}': {cache_path}")

        # Show how different node names produce different paths
        print("\nFilename derivation examples:")
        examples = [
            "test-dev-node",
            "https://api.example.com:2428",
            "my-node",
            "my-node",  # Same input = same output (stable)
        ]
        for name in examples:
            path = calimero.get_token_cache_path(name)
            filename = os.path.basename(path)
            print(f"  '{name}' -> {filename}")

    except Exception as e:
        print(f"✗ Error: {e}")


if __name__ == "__main__":
    main()
    demo_token_cache_utilities()
