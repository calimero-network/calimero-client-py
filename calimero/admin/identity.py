"""
Identity Management Module for Calimero Admin Client.

This module handles all identity-related operations including generation,
listing, and management of identities within contexts.
"""

from ..types import GenerateIdentityResponse, ListIdentitiesResponse, IdentityInfo


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
        result = await self.client._make_request("POST", "/admin-api/identity/context")
        if isinstance(result, dict):
            # Handle both formats: {'success': true, 'data': {...}} and {'data': {...}}
            if result.get("success") or "data" in result:
                # Add success field if it doesn't exist, so the workflow engine can access it
                if "success" not in result:
                    result["success"] = True
                return result
        raise ValueError(f"Failed to generate identity: {result}")

    async def list_in_context(self, context_id: str) -> ListIdentitiesResponse:
        """
        List all identities in a context.

        Args:
            context_id: The ID of the context.

        Returns:
            The list identities response containing the list of identities.
        """
        result = await self.client._make_request(
            "GET", f"/admin-api/contexts/{context_id}/identities"
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to list identities: {result}")


# Export the class
__all__ = ["IdentityManager"]
