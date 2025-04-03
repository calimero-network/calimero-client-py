#!/usr/bin/env python3

import os
import sys
import asyncio
from datetime import datetime
from calimero import JsonRpcClient, RpcQueryParams, RequestConfig, Config
import base58

async def get_all_posts():
    """Get all posts from OnlyPeers application."""
    # Get required environment variables
    context_id = os.getenv('CONTEXT_ID')
    if not context_id:
        print("Error: CONTEXT_ID environment variable is not set")
        sys.exit(1)
    print(f"Context id: {context_id}")

    executor_public_key = os.getenv('EXECUTOR_PUBLIC_KEY')
    if not executor_public_key:
        print("Error: EXECUTOR_PUBLIC_KEY environment variable is not set")
        sys.exit(1)
    print(f"Executor public key: {executor_public_key}")

    # Load config
    config_path = os.path.join(Config.get_default_config_path().parent, "node1", "config.toml")
    config = Config.load_from_file(config_path)

    # Generate timestamp and signature
    timestamp = str(int(datetime.utcnow().timestamp()))
    signature = config.keypair.sign(timestamp)
    signature_b58 = base58.b58encode(signature).decode()

    # Prepare request headers
    headers = {
        'Content-Type': 'application/json',
        'X-Signature': signature_b58,
        'X-Timestamp': timestamp
    }

    # Initialize client
    node_url = os.getenv('RPC_URL', 'http://localhost:2428')
    jsonrpc_path = '/jsonrpc/dev'
    client = JsonRpcClient(node_url, jsonrpc_path)
    print(f"Initialized Calimero JSON RPC Client with URL: {node_url}{jsonrpc_path}")

    # Prepare query parameters
    method_name = 'posts'
    args_json = {'feedRequest': {}}
    query_params = RpcQueryParams(
        application_id=context_id,
        method=method_name,
        args_json=args_json,
        executor_public_key=executor_public_key
    )

    # Execute query
    request_config = RequestConfig(timeout=1000, headers=headers)
    result = await client.execute(query_params, request_config)

    # Print results
    if os.getenv('VERBOSE') == '1':
        print(result)

    if result.get('result'):
        print(result['result'])
    else:
        print(result.get('error', 'Unknown error occurred'))

if __name__ == '__main__':
    asyncio.run(get_all_posts()) 