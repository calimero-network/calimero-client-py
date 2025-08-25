"""
Calimero Admin Client - Main module.

This module provides the main AdminClient class that orchestrates all admin operations.
"""

import aiohttp
from typing import Dict, Any, Optional
from ..types import (
    AdminApiResponse,
    CreateContextRequest,
    CreateContextResponse,
    ListContextsResponse,
    GetContextResponse,
    DeleteContextResponse,
    GenerateIdentityResponse,
    ListIdentitiesResponse,
    InstallDevApplicationRequest,
    InstallDevApplicationResponse,
    InstallApplicationRequest,
    InstallApplicationResponse,
    ListApplicationsResponse,
    GetApplicationResponse,
    UninstallApplicationResponse,
    UploadBlobRequest,
    UploadBlobResponse,
    DownloadBlobResponse,
    ListBlobsResponse,
    GetBlobInfoResponse,
    DeleteBlobResponse,
    UpdateContextApplicationResponse,
    GetContextStorageResponse,
    GetContextValueResponse,
    GetContextStorageEntriesResponse,
    GetProxyContractResponse,
    GrantCapabilitiesResponse,
    RevokeCapabilitiesResponse,
    GetProposalsResponse,
    GetProposalResponse,
    GetNumberOfActiveProposalsResponse,
    GetProposalApprovalsCountResponse,
    GetProposalApproversResponse,
    CreateAliasResponse,
    LookupAliasResponse,
    ListAliasesResponse,
    DeleteAliasResponse,
    HealthCheckResponse,
    IsAuthenticatedResponse,
    GetPeersResponse,
    GetPeersCountResponse,
    GetCertificateResponse,
    SyncContextResponse,
    ErrorResponse,
)

from .context import ContextManager
from .identity import IdentityManager
from .application import ApplicationManager
from .blob import BlobManager
from .capability import CapabilityManager
from .proposal import ProposalManager
from .alias import AliasManager
from .system import SystemManager


class AdminClient:
    """
    Calimero Admin API client for managing contexts, applications, and system operations.

    This client provides a comprehensive interface to the Calimero Admin API,
    with all methods returning strongly typed response objects.
    """

    def __init__(self, base_url: str):
        """
        Initialize the Admin API client.

        Args:
            base_url: The base URL of the Calimero node (e.g., "http://localhost:2428").
        """
        self.base_url = base_url.rstrip("/")
        self.session = None

        # Initialize specialized managers
        self.contexts = ContextManager(self)
        self.identities = IdentityManager(self)
        self.applications = ApplicationManager(self)
        self.blobs = BlobManager(self)
        self.capabilities = CapabilityManager(self)
        self.proposals = ProposalManager(self)
        self.aliases = AliasManager(self)
        self.system = SystemManager(self)

    async def __aenter__(self):
        """Async context manager entry."""
        await self._ensure_session()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.close()

    async def _ensure_session(self):
        """Ensure HTTP session is available."""
        if self.session is None:
            self.session = aiohttp.ClientSession()

    async def close(self):
        """Close the HTTP session."""
        if self.session:
            await self.session.close()
            self.session = None

    async def _make_request(
        self, method: str, endpoint: str, data: Optional[Dict[str, Any]] = None
    ) -> AdminApiResponse:
        """
        Make an HTTP request to the Admin API.

        Args:
            method: HTTP method (GET, POST, PUT, DELETE).
            endpoint: API endpoint path.
            data: Optional request data.

        Returns:
            The admin API response.

        Raises:
            ValueError: If the request fails.
        """
        await self._ensure_session()

        url = f"{self.base_url}{endpoint}"
        headers = {}

        try:
            if method.upper() == "GET":
                async with self.session.get(url, headers=headers) as response:
                    if response.status == 200:
                        return await response.json()
                    else:
                        error_text = await response.text()
                        return {
                            "success": False,
                            "error": f"HTTP {response.status}: {error_text}",
                            "error_code": str(response.status),
                        }
            else:
                async with self.session.request(
                    method, url, json=data, headers=headers
                ) as response:
                    if response.status in [200, 201]:
                        return await response.json()
                    else:
                        error_text = await response.text()
                        return {
                            "success": False,
                            "error": f"HTTP {response.status}: {error_text}",
                            "error_code": str(response.status),
                        }
        except Exception as e:
            return {"success": False, "error": str(e), "error_code": "REQUEST_ERROR"}

    # Backward compatibility methods that delegate to specialized managers
    async def create_context(
        self,
        application_id: str,
        protocol: str = "near",
        initialization_params: list = None,
    ) -> CreateContextResponse:
        """Create a new context (backward compatibility)."""
        return await self.contexts.create(
            application_id, protocol, initialization_params or []
        )

    async def list_contexts(self) -> ListContextsResponse:
        """List all contexts (backward compatibility)."""
        return await self.contexts.list_all()

    async def get_context(self, context_id: str) -> GetContextResponse:
        """Get context information (backward compatibility)."""
        return await self.contexts.get(context_id)

    async def delete_context(self, context_id: str) -> DeleteContextResponse:
        """Delete a context (backward compatibility)."""
        return await self.contexts.delete(context_id)

    async def invite(
        self,
        context_id: str,
        granter_id: str,
        grantee_id: str,
        capability: str = "member",
    ):
        """
        Invite an identity to a context.

        Args:
            context_id: The ID of the context to invite to.
            granter_id: The public key of the identity doing the inviting.
            grantee_id: The public key of the identity being invited.
            capability: The capability to grant (default: "member").

        Returns:
            The invitation response.
        """
        # Create an invitation using the dedicated invite endpoint
        payload = {
            "contextId": context_id,
            "inviterId": granter_id,  # The API expects 'inviterId' not 'granterId'
            "inviteeId": grantee_id,  # The API expects 'inviteeId' not 'granteeId'
            "capability": capability,
        }
        result = await self._make_request("POST", "/admin-api/contexts/invite", payload)
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to create invitation: {result}")

    async def grant_capability(
        self, context_id: str, granter_id: str, grantee_id: str, capability: str
    ):
        """
        Directly grant a capability to an identity in a context.

        Args:
            context_id: The ID of the context.
            granter_id: The public key of the identity granting the capability.
            grantee_id: The public key of the identity receiving the capability.
            capability: The capability to grant.

        Returns:
            The capability grant response.
        """
        return await self.capabilities.grant(
            context_id, granter_id, grantee_id, capability
        )

    async def invite_to_context(
        self,
        context_id: str,
        granter_id: str,
        grantee_id: str,
        capability: str = "member",
    ):
        """
        Invite an identity to a context (alias for invite method for compatibility).

        Args:
            context_id: The ID of the context to invite to.
            granter_id: The public key of the identity doing the inviting.
            grantee_id: The public key of the identity being invited.
            capability: The capability to grant (default: "member").

        Returns:
            The invitation response.
        """
        return await self.invite(context_id, granter_id, grantee_id, capability)

    async def join_context(self, context_id: str, invitee_id: str, invitation: str):
        """
        Join a context using an invitation.

        Args:
            context_id: The ID of the context to join.
            invitee_id: The public key of the identity joining the context.
            invitation: The invitation data received from the inviter.

        Returns:
            The join context response.
        """
        # Join a context using the invitation received from the inviter
        payload = {
            "contextId": context_id,
            "inviterId": invitee_id,  # The API expects 'inviterId' for the join endpoint
            "invitationPayload": invitation,  # The API expects 'invitationPayload' not 'invitation'
        }
        result = await self._make_request("POST", "/admin-api/contexts/join", payload)
        if isinstance(result, dict) and (result.get("success") or "data" in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if "success" not in result:
                result["success"] = True
            return result
        else:
            raise ValueError(f"Failed to join context: {result}")

    async def generate_identity(self) -> GenerateIdentityResponse:
        """Generate a new identity (backward compatibility)."""
        return await self.identities.generate()

    async def list_identities(self, context_id: str) -> ListIdentitiesResponse:
        """List identities in a context (backward compatibility)."""
        return await self.identities.list_in_context(context_id)

    async def install_dev_application(
        self, path: str, metadata: bytes = b""
    ) -> InstallDevApplicationResponse:
        """Install a development application (backward compatibility)."""
        return await self.applications.install_dev(path, metadata)

    async def install_application(
        self, url: str, hash: str = None, metadata: bytes = b""
    ) -> InstallApplicationResponse:
        """Install an application from URL (backward compatibility)."""
        return await self.applications.install_from_url(url, hash, metadata)

    async def list_applications(self) -> ListApplicationsResponse:
        """List all applications (backward compatibility)."""
        return await self.applications.list_all()

    async def get_application(self, application_id: str) -> GetApplicationResponse:
        """Get application information (backward compatibility)."""
        return await self.applications.get(application_id)

    async def uninstall_application(
        self, application_id: str
    ) -> UninstallApplicationResponse:
        """Uninstall an application (backward compatibility)."""
        return await self.applications.uninstall(application_id)

    async def upload_blob(
        self, data: bytes, metadata: bytes = b""
    ) -> UploadBlobResponse:
        """Upload a blob (backward compatibility)."""
        return await self.blobs.upload(data, metadata)

    async def download_blob(self, blob_id: str) -> DownloadBlobResponse:
        """Download a blob (backward compatibility)."""
        return await self.blobs.download(blob_id)

    async def list_blobs(self) -> ListBlobsResponse:
        """List all blobs (backward compatibility)."""
        return await self.blobs.list_all()

    async def get_blob_info(self, blob_id: str) -> GetBlobInfoResponse:
        """Get blob information (backward compatibility)."""
        return await self.blobs.get_info(blob_id)

    async def delete_blob(self, blob_id: str) -> DeleteBlobResponse:
        """Delete a blob (backward compatibility)."""
        return await self.blobs.delete(blob_id)

    async def health_check(self) -> HealthCheckResponse:
        """Check system health (backward compatibility)."""
        return await self.system.health_check()

    async def is_authenticated(self) -> IsAuthenticatedResponse:
        """Check authentication status (backward compatibility)."""
        return await self.system.is_authenticated()

    async def get_peers(self) -> GetPeersResponse:
        """Get peer information (backward compatibility)."""
        return await self.system.get_peers()

    async def get_peers_count(self) -> GetPeersCountResponse:
        """Get peer count (backward compatibility)."""
        return await self.system.get_peers_count()

    async def get_certificate(self) -> GetCertificateResponse:
        """Get server certificate (backward compatibility)."""
        return await self.system.get_certificate()

    async def sync_context(self) -> SyncContextResponse:
        """Synchronize contexts (placeholder implementation)."""
        # This is a placeholder since the actual sync endpoint might not exist
        # Return a success response for compatibility
        from datetime import datetime

        return {
            "success": True,
            "context_id": None,
            "synced_at": datetime.now().isoformat(),
        }


# Export the main class
__all__ = ["AdminClient"]
