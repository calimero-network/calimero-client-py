#!/usr/bin/env python3

import asyncio
import os
import json
import toml
from calimero import JsonRpcClient, Ed25519Keypair

def get_required_env(key: str) -> str:
    """Get a required environment variable or raise an error."""
    value = os.getenv(key)
    if not value:
        raise ValueError(f"Environment variable {key} is not set")
    return value

def load_keypair_from_config(config_path: str) -> Ed25519Keypair:
    """Load only the keypair from the config file."""
    try:
        with open(config_path, 'r') as f:
            config_data = toml.load(f)
            keypair_value = config_data.get('identity', {}).get('keypair')
            if not keypair_value:
                raise ValueError("'keypair' not found in [identity] section")
            return Ed25519Keypair.from_base58(keypair_value)
    except Exception as e:
        raise ValueError(f"Failed to load keypair from config: {str(e)}")

async def kv_store_example():
    """Example of using the KV store."""
    # Get required environment variables
    context_id = get_required_env('CONTEXT_ID')
    print(f"Context id: {context_id}")
    
    executor_public_key = get_required_env('EXECUTOR_PUBLIC_KEY')
    print(f"Executor public key: {executor_public_key}")
    
    rpc_url = os.getenv('RPC_URL', 'http://localhost:2428')
    print(f"RPC URL: {rpc_url}")
    
    # Load keypair from config file
    config_path = os.path.expanduser('~/.calimero/node1/config.toml')
    keypair = load_keypair_from_config(config_path)
    
    # Initialize client with all parameters
    client = JsonRpcClient(
        rpc_url=rpc_url,
        keypair=keypair,
        context_id=context_id,
        executor_public_key=executor_public_key
    )
    print(f"Initialized Calimero JSON RPC Client")
    
    # Example operations
    try:
        # 1. Set a value
        print("\n1. Setting a value...")
        set_result = await client.execute("set", {"key": "test_key", "value": "test_value"})
        print(f"Set result: {set_result}")
        
        # 2. Get the value
        print("\n2. Getting the value...")
        get_result = await client.execute("get", {"key": "test_key"})
        print(f"Get result: {get_result}")
        
        # 3. Update the value
        print("\n3. Updating the value...")
        update_result = await client.execute("set", {"key": "test_key", "value": "updated_value"})
        print(f"Update result: {update_result}")
        
        # 4. Get the updated value
        print("\n4. Getting the updated value...")
        get_updated_result = await client.execute("get", {"key": "test_key"})
        print(f"Get updated result: {get_updated_result}")
        
        # 5. Remove the key
        print("\n5. Removing the key...")
        remove_result = await client.execute("remove", {"key": "test_key"})
        print(f"Remove result: {remove_result}")
        
        # 6. Try to get the removed key
        print("\n6. Getting the removed key...")
        get_removed_result = await client.execute("get", {"key": "test_key"})
        print(f"Get removed result: {get_removed_result}")
        
    except Exception as e:
        print(f"Error: {str(e)}")

if __name__ == "__main__":
    asyncio.run(kv_store_example()) 