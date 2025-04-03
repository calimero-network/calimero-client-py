import json
from typing import Any, Dict, Optional
import aiohttp
from pydantic import BaseModel

class RpcQueryParams(BaseModel):
    application_id: str
    method: str
    args_json: Dict[str, Any]
    executor_public_key: str

class RequestConfig(BaseModel):
    timeout: int = 1000
    headers: Dict[str, str] = {}

class JsonRpcClient:
    def __init__(self, base_url: str, endpoint: str = "/jsonrpc"):
        self.base_url = base_url.rstrip("/")
        self.endpoint = endpoint
        self.session: Optional[aiohttp.ClientSession] = None

    async def __aenter__(self):
        self.session = aiohttp.ClientSession()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self.session:
            await self.session.close()

    async def _ensure_session(self):
        if not self.session:
            self.session = aiohttp.ClientSession()

    async def execute(
        self,
        params: RpcQueryParams,
        config: Optional[RequestConfig] = None
    ) -> Dict[str, Any]:
        await self._ensure_session()
        
        config = config or RequestConfig()
        url = f"{self.base_url}{self.endpoint}"
        
        payload = {
            "jsonrpc": "2.0",
            "method": params.method,
            "params": {
                "applicationId": params.application_id,
                "argsJson": params.args_json,
                "executorPublicKey": params.executor_public_key
            },
            "id": 1
        }

        async with self.session.post(
            url,
            json=payload,
            headers=config.headers,
            timeout=config.timeout/1000  # Convert ms to seconds
        ) as response:
            response.raise_for_status()
            return await response.json()

    async def mutate(self, params: RpcQueryParams, config: Optional[RequestConfig] = None) -> Dict[str, Any]:
        return await self.execute(params, config)

    async def query(self, params: RpcQueryParams, config: Optional[RequestConfig] = None) -> Dict[str, Any]:
        return await self.execute(params, config) 