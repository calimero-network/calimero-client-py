import pytest
import aiohttp
from unittest.mock import Mock, patch
from calimero import JsonRpcClient, RpcQueryParams, RequestConfig

@pytest.mark.asyncio
async def test_json_rpc_client_creation():
    """Test JsonRpcClient initialization."""
    client = JsonRpcClient("http://localhost:2428")
    assert client.base_url == "http://localhost:2428"
    assert client.endpoint == "/jsonrpc"

@pytest.mark.asyncio
async def test_json_rpc_client_context_manager():
    """Test JsonRpcClient context manager."""
    async with JsonRpcClient("http://localhost:2428") as client:
        assert isinstance(client, JsonRpcClient)
        assert client.session is not None

@pytest.mark.asyncio
async def test_execute_request(mock_rpc_url):
    """Test executing RPC request."""
    client = JsonRpcClient("http://localhost:2428")
    params = RpcQueryParams(
        application_id="test_app",
        method="test_method",
        args_json={"arg1": "value1"},
        executor_public_key="test_key"
    )
    
    mock_response = {
        "jsonrpc": "2.0",
        "result": {"success": True},
        "id": 1
    }
    
    with patch('aiohttp.ClientSession.post') as mock_post:
        mock_post.return_value.__aenter__.return_value.json.return_value = mock_response
        mock_post.return_value.__aenter__.return_value.raise_for_status = Mock()
        
        response = await client.execute(params)
        assert response == mock_response
        
        # Verify request was made with correct parameters
        mock_post.assert_called_once()
        call_args = mock_post.call_args[1]
        assert "json" in call_args
        assert call_args["json"]["method"] == "test_method"
        assert call_args["json"]["params"]["applicationId"] == "test_app"

@pytest.mark.asyncio
async def test_execute_with_custom_config():
    """Test executing RPC request with custom config."""
    client = JsonRpcClient("http://localhost:2428")
    params = RpcQueryParams(
        application_id="test_app",
        method="test_method",
        args_json={"arg1": "value1"},
        executor_public_key="test_key"
    )
    
    config = RequestConfig(
        timeout=2000,
        headers={"X-Custom-Header": "value"}
    )
    
    with patch('aiohttp.ClientSession.post') as mock_post:
        mock_post.return_value.__aenter__.return_value.json.return_value = {"result": "success"}
        mock_post.return_value.__aenter__.return_value.raise_for_status = Mock()
        
        await client.execute(params, config)
        
        # Verify custom config was used
        call_args = mock_post.call_args[1]
        assert call_args["timeout"] == 2.0  # Converted from ms to seconds
        assert call_args["headers"]["X-Custom-Header"] == "value"

@pytest.mark.asyncio
async def test_mutate_and_query():
    """Test mutate and query methods."""
    client = JsonRpcClient("http://localhost:2428")
    params = RpcQueryParams(
        application_id="test_app",
        method="test_method",
        args_json={"arg1": "value1"},
        executor_public_key="test_key"
    )
    
    with patch('aiohttp.ClientSession.post') as mock_post:
        mock_post.return_value.__aenter__.return_value.json.return_value = {"result": "success"}
        mock_post.return_value.__aenter__.return_value.raise_for_status = Mock()
        
        # Test mutate
        mutate_response = await client.mutate(params)
        assert mutate_response == {"result": "success"}
        
        # Test query
        query_response = await client.query(params)
        assert query_response == {"result": "success"} 