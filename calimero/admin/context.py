"""
Context Management Module for Calimero Admin Client.

This module handles all context-related operations including creation, listing,
retrieval, deletion, context-specific operations, and capability management.
"""

from enum import Enum
from typing import List
from ..types import (
    CreateContextRequest,
    CreateContextResponse,
    ListContextsResponse,
    GetContextResponse,
    DeleteContextResponse,
    ContextInfo,
    UpdateContextApplicationResponse,
    GetContextStorageResponse,
    GetContextValueResponse,
    GetContextStorageEntriesResponse,
    GetProxyContractResponse,
    SyncContextResponse,
    GrantCapabilitiesResponse,
    RevokeCapabilitiesResponse,
)


class Capability(str, Enum):
    """Enumeration of available capabilities for contexts."""

    MANAGE_APPLICATION = "ManageApplication"
    MANAGE_MEMBERS = "ManageMembers"
    PROXY = "Proxy"


class ContextManager:
    """Manages context operations for the Calimero Admin Client."""

    def __init__(self, client):
        """Initialize the context manager with a reference to the main client."""
        self.client = client

    async def create(
        self,
        application_id: str,
        protocol: str = "near",
        initialization_params: List = None,
    ) -> CreateContextResponse:
        """Create a new context."""
        request = CreateContextRequest(
            application_id=application_id,
            protocol=protocol,
            initialization_params=initialization_params or [],
        )
        return await self._create_typed(request)

    async def _create_typed(
        self, request: CreateContextRequest
    ) -> CreateContextResponse:
        """Create a new context with strongly typed request."""
        payload = {
            "applicationId": request.application_id,
            "protocol": request.protocol,
            "initializationParams": request.initialization_params,
        }
        result = await self.client._make_request("POST", "/admin-api/contexts", payload)
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to create context: {result}")

    async def list_all(self) -> ListContextsResponse:
        """List all contexts."""
        result = await self.client._make_request("GET", "/admin-api/contexts")
        if isinstance(result, dict):
            if result.get("success") or "data" in result:
                data = result.get("data", result)
                contexts_data = (
                    data.get("contexts", data) if isinstance(data, dict) else data
                )
                if not isinstance(contexts_data, list):
                    contexts_data = []

                contexts = [
                    ContextInfo(
                        id=ctx.get("id", ""),
                        application_id=ctx.get("applicationId", ""),
                        protocol=ctx.get("protocol", ""),
                        status=ctx.get("status", ""),
                        created_at=ctx.get("createdAt"),
                        member_count=ctx.get("memberCount"),
                    )
                    for ctx in contexts_data
                    if isinstance(ctx, dict)
                ]
                return ListContextsResponse(
                    success=True, contexts=contexts, total_count=len(contexts)
                )
        raise ValueError(f"Failed to list contexts: {result}")

    async def get(self, context_id: str) -> GetContextResponse:
        """Get information about a specific context."""
        result = await self.client._make_request(
            "GET", f"/admin-api/contexts/{context_id}"
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to get context: {result}")

    async def delete(self, context_id: str) -> DeleteContextResponse:
        """Delete a context."""
        result = await self.client._make_request(
            "DELETE", f"/admin-api/contexts/{context_id}"
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to delete context: {result}")

    async def update_application(
        self, context_id: str, application_id: str
    ) -> UpdateContextApplicationResponse:
        """Update the application running in a context."""
        payload = {"applicationId": application_id}
        result = await self.client._make_request(
            "PUT", f"/admin-api/contexts/{context_id}/application", payload
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to update context application: {result}")

    async def get_storage(self, context_id: str) -> GetContextStorageResponse:
        """Get the storage for a context."""
        result = await self.client._make_request(
            "GET", f"/admin-api/contexts/{context_id}/storage"
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to get context storage: {result}")

    async def get_value(self, context_id: str, key: str) -> GetContextValueResponse:
        """Get a value from context storage."""
        result = await self.client._make_request(
            "GET", f"/admin-api/contexts/{context_id}/storage/{key}"
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to get context value: {result}")

    async def get_storage_entries(
        self, context_id: str, prefix: str = "", limit: int = 100
    ) -> GetContextStorageEntriesResponse:
        """Get storage entries from a context."""
        params = {"prefix": prefix, "limit": limit}
        result = await self.client._make_request(
            "GET", f"/admin-api/contexts/{context_id}/storage/entries", params
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to get context storage entries: {result}")

    async def get_proxy_contract(self, context_id: str) -> GetProxyContractResponse:
        """Get the proxy contract for a context."""
        result = await self.client._make_request(
            "GET", f"/admin-api/contexts/{context_id}/proxy-contract"
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to get proxy contract: {result}")

    async def sync(self, context_id: str = None) -> SyncContextResponse:
        """Sync a context or all contexts."""
        if context_id:
            result = await self.client._make_request(
                "POST", f"/admin-api/contexts/{context_id}/sync"
            )
        else:
            result = await self.client._make_request("POST", "/admin-api/contexts/sync")

        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to sync context: {result}")

    async def grant_capability(
        self, context_id: str, granter_id: str, grantee_id: str, capability: Capability
    ) -> GrantCapabilitiesResponse:
        """Grant capabilities to a user in a context."""
        payload = {
            "signer_id": granter_id,
            "capabilities": [(grantee_id, capability.value)],
        }
        result = await self.client._make_request(
            "POST", f"/admin-api/contexts/{context_id}/capabilities/grant", payload
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to grant capability: {result}")

    async def revoke_capability(
        self, context_id: str, revoker_id: str, revokee_id: str, capability: Capability
    ) -> RevokeCapabilitiesResponse:
        """Revoke capabilities from a user in a context."""
        payload = {
            "signer_id": revoker_id,
            "capabilities": [(revokee_id, capability.value)],
        }
        result = await self.client._make_request(
            "POST", f"/admin-api/contexts/{context_id}/capabilities/revoke", payload
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to revoke capability: {result}")

    async def invite_to_context(
        self, context_id: str, inviter_id: str, invitee_id: str
    ):
        """Invite an identity to join a context."""
        payload = {
            "contextId": context_id,
            "inviterId": inviter_id,
            "inviteeId": invitee_id,
        }
        result = await self.client._make_request(
            "POST", "/admin-api/contexts/invite", payload
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to create invitation: {result}")

    async def join_context(
        self, context_id: str, invitee_id: str, invitation_payload: str
    ):
        """Join a context using an invitation."""
        payload = {
            "contextId": context_id,
            "inviteeId": invitee_id,
            "invitationPayload": invitation_payload,
        }
        result = await self.client._make_request(
            "POST", "/admin-api/contexts/join", payload
        )
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to join context: {result}")


__all__ = ["ContextManager", "Capability"]
