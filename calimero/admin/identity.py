"""
Identity Management Module for Calimero Admin Client.

This module handles all identity-related operations including generation,
listing, and management of identities within contexts.
"""

from ..types import (
    GenerateIdentityResponse, ListIdentitiesResponse, IdentityInfo
)


class IdentityManager:
    """Manages identity operations for the Calimero Admin Client."""
    
    def __init__(self, client):
        """Initialize the identity manager with a reference to the main client."""
        self.client = client
    
    async def generate(self) -> GenerateIdentityResponse:
        """
        Generate a new identity.
        
        Returns:
            The generate identity response containing the new identity public key.
        """
        result = await self.client._make_request('POST', '/admin-api/identity/context')
        if isinstance(result, dict):
            # Handle both formats: {'success': true, 'data': {...}} and {'data': {...}}
            if result.get('success') or 'data' in result:
                data = result.get('data', result)
                return GenerateIdentityResponse(
                    success=True,
                    public_key=data.get('publicKey', ''),
                    context_id=data.get('contextId')
                )
        raise ValueError(f"Failed to generate identity: {result}")
    
    async def list_in_context(self, context_id: str) -> ListIdentitiesResponse:
        """
        List all identities in a context.
        
        Args:
            context_id: The ID of the context.
            
        Returns:
            The list identities response containing the list of identities.
        """
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/identities')
        if isinstance(result, dict) and result.get('success'):
            identities_data = result.get('data', [])
            identities = [
                IdentityInfo(
                    public_key=identity.get('publicKey', ''),
                    context_id=identity.get('contextId'),
                    capabilities=identity.get('capabilities', []),
                    created_at=identity.get('createdAt')
                )
                for identity in identities_data
            ]
            return ListIdentitiesResponse(
                success=True,
                identities=identities,
                total_count=len(identities)
            )
        else:
            raise ValueError(f"Failed to list identities: {result}")


# Export the class
__all__ = ['IdentityManager']
