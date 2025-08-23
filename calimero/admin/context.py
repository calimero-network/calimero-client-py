"""
Context Management Module for Calimero Admin Client.

This module handles all context-related operations including creation, listing,
retrieval, deletion, and context-specific operations.
"""

from typing import List
from ..types import (
    CreateContextRequest, CreateContextResponse, ListContextsResponse,
    GetContextResponse, DeleteContextResponse, ContextInfo,
    UpdateContextApplicationResponse, GetContextStorageResponse,
    GetContextValueResponse, GetContextStorageEntriesResponse,
    GetProxyContractResponse, SyncContextResponse
)


class ContextManager:
    """Manages context operations for the Calimero Admin Client."""
    
    def __init__(self, client):
        """Initialize the context manager with a reference to the main client."""
        self.client = client
    
    async def create(self, application_id: str, protocol: str = "near", initialization_params: List = None) -> CreateContextResponse:
        """
        Create a new context.
        
        Args:
            application_id: The ID of the application to run in the context.
            protocol: The protocol to use for the context (default: "near").
            initialization_params: Optional initialization parameters.
            
        Returns:
            The create context response containing the new context ID and member public key.
        """
        request = CreateContextRequest(
            application_id=application_id,
            protocol=protocol,
            initialization_params=initialization_params or []
        )
        return await self._create_typed(request)
    
    async def _create_typed(self, request: CreateContextRequest) -> CreateContextResponse:
        """
        Create a new context with strongly typed request.
        
        Args:
            request: The create context request containing application ID, protocol, and initialization parameters.
            
        Returns:
            The create context response containing the new context ID and member public key.
        """
        payload = {
            "applicationId": request.application_id,
            "protocol": request.protocol,
            "initializationParams": request.initialization_params
        }
        result = await self.client._make_request('POST', '/admin-api/contexts', payload)
        if isinstance(result, dict) and result.get('success'):
            return CreateContextResponse(
                success=True,
                data=result.get('data', {}),
                context_id=result.get('data', {}).get('contextId'),
                member_public_key=result.get('data', {}).get('memberPublicKey')
            )
        else:
            raise ValueError(f"Failed to create context: {result}")
    
    async def list_all(self) -> ListContextsResponse:
        """
        List all contexts.
        
        Returns:
            The list contexts response containing the list of contexts.
        """
        result = await self.client._make_request('GET', '/admin-api/contexts')
        if isinstance(result, dict):
            # Handle both formats: {'success': true, 'data': [...]} and {'data': {'contexts': [...]}}
            if result.get('success') or 'data' in result:
                data = result.get('data', result)
                # Handle nested contexts structure
                contexts_data = data.get('contexts', data) if isinstance(data, dict) else data
                if not isinstance(contexts_data, list):
                    contexts_data = []
                
                contexts = [
                    ContextInfo(
                        id=ctx.get('id', ''),
                        application_id=ctx.get('applicationId', ''),
                        protocol=ctx.get('protocol', ''),
                        status=ctx.get('status', ''),
                        created_at=ctx.get('createdAt'),
                        member_count=ctx.get('memberCount')
                    )
                    for ctx in contexts_data if isinstance(ctx, dict)
                ]
                return ListContextsResponse(
                    success=True,
                    contexts=contexts,
                    total_count=len(contexts)
                )
        raise ValueError(f"Failed to list contexts: {result}")
    
    async def get(self, context_id: str) -> GetContextResponse:
        """
        Get information about a specific context.
        
        Args:
            context_id: The ID of the context to retrieve.
            
        Returns:
            The get context response containing the context information.
        """
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}')
        if isinstance(result, dict) and result.get('success'):
            ctx_data = result.get('data', {})
            context = ContextInfo(
                id=ctx_data.get('id', ''),
                application_id=ctx_data.get('applicationId', ''),
                protocol=ctx_data.get('protocol', ''),
                status=ctx_data.get('status', ''),
                created_at=ctx_data.get('createdAt'),
                member_count=ctx_data.get('memberCount')
            )
            return GetContextResponse(
                success=True,
                context=context
            )
        else:
            raise ValueError(f"Failed to get context: {result}")
    
    async def delete(self, context_id: str) -> DeleteContextResponse:
        """
        Delete a context.
        
        Args:
            context_id: The ID of the context to delete.
            
        Returns:
            The delete context response.
        """
        result = await self.client._make_request('DELETE', f'/admin-api/contexts/{context_id}')
        if isinstance(result, dict) and result.get('success'):
            return DeleteContextResponse(
                success=True,
                context_id=context_id
            )
        else:
            raise ValueError(f"Failed to delete context: {result}")
    
    async def update_application(self, context_id: str, application_id: str) -> UpdateContextApplicationResponse:
        """
        Update the application running in a context.
        
        Args:
            context_id: The ID of the context.
            application_id: The ID of the new application to run.
            
        Returns:
            The update context application response.
        """
        payload = {"applicationId": application_id}
        result = await self.client._make_request('PUT', f'/admin-api/contexts/{context_id}/application', payload)
        if isinstance(result, dict) and result.get('success'):
            return UpdateContextApplicationResponse(
                success=True,
                context_id=context_id,
                application_id=application_id
            )
        else:
            raise ValueError(f"Failed to update context application: {result}")
    
    async def get_storage(self, context_id: str) -> GetContextStorageResponse:
        """
        Get the storage for a context.
        
        Args:
            context_id: The ID of the context.
            
        Returns:
            The get context storage response.
        """
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/storage')
        if isinstance(result, dict) and result.get('success'):
            return GetContextStorageResponse(
                success=True,
                context_id=context_id,
                data=result.get('data', {})
            )
        else:
            raise ValueError(f"Failed to get context storage: {result}")
    
    async def get_value(self, context_id: str, key: str) -> GetContextValueResponse:
        """
        Get a value from context storage.
        
        Args:
            context_id: The ID of the context.
            key: The storage key.
            
        Returns:
            The get context value response.
        """
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/storage/{key}')
        if isinstance(result, dict) and result.get('success'):
            return GetContextValueResponse(
                success=True,
                context_id=context_id,
                key=key,
                value=result.get('data')
            )
        else:
            raise ValueError(f"Failed to get context value: {result}")
    
    async def get_storage_entries(self, context_id: str, prefix: str = "", limit: int = 100) -> GetContextStorageEntriesResponse:
        """
        Get storage entries from a context.
        
        Args:
            context_id: The ID of the context.
            prefix: Optional prefix to filter entries.
            limit: Maximum number of entries to return.
            
        Returns:
            The get context storage entries response.
        """
        params = {"prefix": prefix, "limit": limit}
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/storage/entries', params)
        if isinstance(result, dict) and result.get('success'):
            entries_data = result.get('data', [])
            entries = [
                {
                    "key": entry.get('key', ''),
                    "value": entry.get('value')
                }
                for entry in entries_data
            ]
            return GetContextStorageEntriesResponse(
                success=True,
                context_id=context_id,
                entries=entries,
                total_count=len(entries)
            )
        else:
            raise ValueError(f"Failed to get context storage entries: {result}")
    
    async def get_proxy_contract(self, context_id: str) -> GetProxyContractResponse:
        """
        Get the proxy contract for a context.
        
        Args:
            context_id: The ID of the context.
            
        Returns:
            The get proxy contract response.
        """
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/proxy-contract')
        if isinstance(result, dict) and result.get('success'):
            return GetProxyContractResponse(
                success=True,
                context_id=context_id,
                contract_data=result.get('data', {})
            )
        else:
            raise ValueError(f"Failed to get proxy contract: {result}")
    
    async def sync(self, context_id: str = None) -> SyncContextResponse:
        """
        Sync a context or all contexts.
        
        Args:
            context_id: Optional context ID to sync. If None, syncs all contexts.
            
        Returns:
            The sync context response.
        """
        if context_id:
            result = await self.client._make_request('POST', f'/admin-api/contexts/{context_id}/sync')
        else:
            result = await self.client._make_request('POST', '/admin-api/contexts/sync')
        
        if isinstance(result, dict) and result.get('success'):
            return SyncContextResponse(
                success=True,
                context_id=context_id,
                sync_result=result.get('data', {})
            )
        else:
            raise ValueError(f"Failed to sync context: {result}")


# Export the class
__all__ = ['ContextManager']
