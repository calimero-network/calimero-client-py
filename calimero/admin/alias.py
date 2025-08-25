"""
Alias Management Module for Calimero Admin Client.

This module handles all alias-related operations including creation,
lookup, listing, and deletion of aliases for contexts, applications, and identities.
"""

from ..types import (
    CreateAliasResponse, LookupAliasResponse, ListAliasesResponse,
    DeleteAliasResponse, AliasInfo
)


class AliasManager:
    """Manages alias operations for the Calimero Admin Client."""
    
    def __init__(self, client):
        """Initialize the alias manager with a reference to the main client."""
        self.client = client
    
    async def create_context_alias(self, name: str, context_id: str) -> CreateAliasResponse:
        """
        Create an alias for a context ID.
        
        Args:
            name: The alias name.
            context_id: The context ID to alias.
            
        Returns:
            The create alias response.
        """
        payload = {
            "name": name,
            "contextId": context_id,
            "type": "context"
        }
        result = await self.client._make_request('POST', '/admin-api/aliases', payload)
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to create context alias: {result}")
    
    async def create_application_alias(self, name: str, application_id: str) -> CreateAliasResponse:
        """
        Create an alias for an application ID.
        
        Args:
            name: The alias name.
            application_id: The application ID to alias.
            
        Returns:
            The create alias response.
        """
        payload = {
            "name": name,
            "applicationId": application_id,
            "type": "application"
        }
        result = await self.client._make_request('POST', '/admin-api/aliases', payload)
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to create application alias: {result}")
    
    async def create_identity_alias(self, context_id: str, name: str, identity_id: str) -> CreateAliasResponse:
        """
        Create an alias for an identity in a context.
        
        Args:
            context_id: The ID of the context.
            name: The alias name.
            identity_id: The identity ID to alias.
            
        Returns:
            The create alias response.
        """
        payload = {
            "name": name,
            "identityId": identity_id,
            "contextId": context_id,
            "type": "identity"
        }
        result = await self.client._make_request('POST', '/admin-api/aliases', payload)
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to create identity alias: {result}")
    
    async def lookup_context_alias(self, name: str) -> LookupAliasResponse:
        """
        Look up a context ID by alias.
        
        Args:
            name: The alias name.
            
        Returns:
            The lookup alias response.
        """
        result = await self.client._make_request('GET', f'/admin-api/aliases/{name}')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to lookup context alias: {result}")
    
    async def lookup_application_alias(self, name: str) -> LookupAliasResponse:
        """
        Look up an application ID by alias.
        
        Args:
            name: The alias name.
            
        Returns:
            The lookup alias response.
        """
        result = await self.client._make_request('GET', f'/admin-api/aliases/{name}')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            data = result.get('data', {})
            return LookupAliasResponse(
                success=True,
                alias=AliasInfo(
                    name=name,
                    target_id=data.get('applicationId', ''),
                    type="application"
                )
            )
        else:
            raise ValueError(f"Failed to lookup application alias: {result}")
    
    async def lookup_identity_alias(self, context_id: str, name: str) -> LookupAliasResponse:
        """
        Look up an identity by alias in a context.
        
        Args:
            context_id: The ID of the context.
            name: The alias name.
            
        Returns:
            The lookup alias response.
        """
        result = await self.client._make_request('GET', f'/admin-api/aliases/{name}')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            data = result.get('data', {})
            return LookupAliasResponse(
                success=True,
                alias=AliasInfo(
                    name=name,
                    target_id=data.get('identityId', ''),
                    type="identity",
                    context_id=context_id
                )
            )
        else:
            raise ValueError(f"Failed to lookup identity alias: {result}")
    
    async def list_context_aliases(self) -> ListAliasesResponse:
        """
        List all context ID aliases.
        
        Returns:
            The list aliases response.
        """
        result = await self.client._make_request('GET', '/admin-api/aliases/contexts')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            aliases_data = result.get('data', [])
            aliases = [
                AliasInfo(
                    name=alias.get('name', ''),
                    target_id=alias.get('contextId', ''),
                    type="context"
                )
                for alias in aliases_data
            ]
            return ListAliasesResponse(
                success=True,
                aliases=aliases,
                total_count=len(aliases)
            )
        else:
            raise ValueError(f"Failed to list context aliases: {result}")
    
    async def list_application_aliases(self) -> ListAliasesResponse:
        """
        List all application ID aliases.
        
        Returns:
            The list aliases response.
        """
        result = await self.client._make_request('GET', '/admin-api/aliases/applications')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            aliases_data = result.get('data', [])
            aliases = [
                AliasInfo(
                    name=alias.get('name', ''),
                    target_id=alias.get('applicationId', ''),
                    type="application"
                )
                for alias in aliases_data
            ]
            return ListAliasesResponse(
                success=True,
                aliases=aliases,
                total_count=len(aliases)
            )
        else:
            raise ValueError(f"Failed to list application aliases: {result}")
    
    async def list_identity_aliases(self, context_id: str) -> ListAliasesResponse:
        """
        List all identity aliases in a context.
        
        Args:
            context_id: The ID of the context.
            
        Returns:
            The list aliases response.
        """
        result = await self.client._make_request('GET', f'/admin-api/aliases/contexts/{context_id}/identities')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            aliases_data = result.get('data', [])
            aliases = [
                AliasInfo(
                    name=alias.get('name', ''),
                    target_id=alias.get('identityId', ''),
                    type="identity",
                    context_id=context_id
                )
                for alias in aliases_data
            ]
            return ListAliasesResponse(
                success=True,
                aliases=aliases,
                total_count=len(aliases)
            )
        else:
            raise ValueError(f"Failed to list identity aliases: {result}")
    
    async def delete_context_alias(self, name: str) -> DeleteAliasResponse:
        """
        Delete a context ID alias.
        
        Args:
            name: The alias name to delete.
            
        Returns:
            The delete alias response.
        """
        result = await self.client._make_request('DELETE', f'/admin-api/aliases/{name}')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            return DeleteAliasResponse(
                success=True,
                alias_name=name
            )
        else:
            raise ValueError(f"Failed to delete context alias: {result}")
    
    async def delete_application_alias(self, name: str) -> DeleteAliasResponse:
        """
        Delete an application ID alias.
        
        Args:
            name: The alias name to delete.
            
        Returns:
            The delete alias response.
        """
        result = await self.client._make_request('DELETE', f'/admin-api/aliases/{name}')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            return DeleteAliasResponse(
                success=True,
                alias_name=name
            )
        else:
            raise ValueError(f"Failed to delete application alias: {result}")
    
    async def delete_identity_alias(self, context_id: str, name: str) -> DeleteAliasResponse:
        """
        Delete an identity alias in a context.
        
        Args:
            context_id: The ID of the context.
            name: The alias name to delete.
            
        Returns:
            The delete alias response.
        """
        result = await self.client._make_request('DELETE', f'/admin-api/aliases/{name}')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            return DeleteAliasResponse(
                success=True,
                alias_name=name
            )
        else:
            raise ValueError(f"Failed to delete identity alias: {result}")


# Export the class
__all__ = ['AliasManager']
