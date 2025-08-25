"""
Capability Management Module for Calimero Admin Client.

This module handles all capability-related operations including granting
and revoking capabilities for identities within contexts.
"""

from ..types import (
    GrantCapabilitiesResponse, RevokeCapabilitiesResponse
)


class CapabilityManager:
    """Manages capability operations for the Calimero Admin Client."""
    
    def __init__(self, client):
        """Initialize the capability manager with a reference to the main client."""
        self.client = client
    
    async def grant(self, context_id: str, granter_id: str, grantee_id: str, capability: str) -> GrantCapabilitiesResponse:
        """
        Grant capabilities to a user in a context.
        
        Args:
            context_id: The ID of the context.
            granter_id: The public key of the identity granting the capability.
            grantee_id: The public key of the identity receiving the capability.
            capability: The capability to grant.
            
        Returns:
            The grant capabilities response.
        """
        payload = {
            "contextId": context_id,
            "granterId": granter_id,
            "granteeId": grantee_id,
            "capability": capability
        }
        result = await self.client._make_request('POST', f'/admin-api/contexts/{context_id}/capabilities/grant', payload)
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to grant capability: {result}")
    
    async def revoke(self, context_id: str, revoker_id: str, revokee_id: str, capability: str) -> RevokeCapabilitiesResponse:
        """
        Revoke capabilities from a user in a context.
        
        Args:
            context_id: The ID of the context.
            revoker_id: The public key of the identity revoking the capability.
            revokee_id: The public key of the identity losing the capability.
            capability: The capability to revoke.
            
        Returns:
            The revoke capabilities response.
        """
        payload = {
            "contextId": context_id,
            "revokerId": revoker_id,
            "revokeeId": revokee_id,
            "capability": capability
        }
        result = await self.client._make_request('POST', f'/admin-api/contexts/{context_id}/capabilities/revoke', payload)
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to revoke capability: {result}")


# Export the class
__all__ = ['CapabilityManager']
