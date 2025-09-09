#!/usr/bin/env python3
"""
Example usage of calimero-client-py package

To use this example:
1. Install the package: pip install calimero-client-py
2. Run this script: python example_usage.py

Note: This example assumes you have a Calimero server running on localhost:2528
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
        print("Creating connection...")
        connection = calimero.create_connection(
            api_url="http://localhost:2528", node_name="example-node"
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


if __name__ == "__main__":
    main()
