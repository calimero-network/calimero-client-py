import asyncio
import json
from typing import Optional, Dict, Any, Union
import aiohttp

from .types import (
    JsonRpcRequest, JsonRpcExecuteRequest, JsonRpcResponse, JsonRpcErrorInfo,
    JsonRpcApiResponse, ErrorResponse
)

class JsonRpcClient:
    """JSON-RPC client for Calimero.
    
    This client handles communication with the Calimero JSON-RPC server,
    including request formatting, signing, and response handling.
    """
    
    # Constants
    JSONRPC_VERSION = '2.0'
    DEFAULT_TIMEOUT = 1000
    JSONRPC_PATH = '/jsonrpc/dev'
    
    def __init__(
        self,
        rpc_url: str,
        context_id: str = None,
        executor_public_key: str = None
    ):
        """Initialize the JSON-RPC client with all required parameters.
        
        Args:
            rpc_url: The URL of the Calimero JSON-RPC server.
            context_id: Optional context ID for the requests.
            executor_public_key: Optional public key of the executor.
        """
        self.rpc_url = rpc_url.rstrip('/')
        self.context_id = context_id
        self.executor_public_key = executor_public_key
    
    def _prepare_headers(self) -> Dict[str, str]:
        """Prepare request headers.
        
        Returns:
            Dictionary containing request headers.
        """
        return {
            'Content-Type': 'application/json'
        }
    
    def _prepare_request(self, method: str, args: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Prepare the JSON-RPC request payload.
        
        Args:
            method: The method to call.
            args: Optional arguments for the method.
            
        Returns:
            Dictionary containing the JSON-RPC request payload.
        """
        return {
            'jsonrpc': self.JSONRPC_VERSION,
            'id': 1,
            'method': 'execute',
            'params': {
                'contextId': self.context_id,
                'method': method,
                'argsJson': args or {},
                'executorPublicKey': self.executor_public_key,
                'timeout': self.DEFAULT_TIMEOUT
            }
        }
    
    async def _handle_response(self, response: aiohttp.ClientResponse) -> JsonRpcResponse:
        """Handle the JSON-RPC response.
        
        Args:
            response: The aiohttp response object.
            
        Returns:
            The parsed JSON-RPC response.
            
        Raises:
            ValueError: If the response indicates an error.
        """
        try:
            data = await response.json()
            if 'error' in data and data['error']:
                raise ValueError(f"JSON-RPC error: {data['error']}")
            return data
        except json.JSONDecodeError as e:
            raise ValueError(f"Failed to decode JSON response: {str(e)}")
    
    async def execute(self, method: str, args: Optional[Dict[str, Any]] = None) -> JsonRpcResponse:
        """Execute a JSON-RPC method.
        
        Args:
            method: The method to call.
            args: Optional arguments for the method.
            
        Returns:
            The JSON-RPC response.
            
        Raises:
            ValueError: If the request fails or returns an error.
        """
        # Only add JSONRPC_PATH if it's not already in the URL
        if self.JSONRPC_PATH in self.rpc_url:
            url = self.rpc_url
        else:
            url = f"{self.rpc_url}{self.JSONRPC_PATH}"
        headers = self._prepare_headers()
        payload = self._prepare_request(method, args)
        
        async with aiohttp.ClientSession() as session:
            async with session.post(url, json=payload, headers=headers) as response:
                return await self._handle_response(response)
