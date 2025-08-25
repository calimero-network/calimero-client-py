"""
Calimero Network Python Client SDK - Unified Client

This module provides a unified Calimero client that combines Admin and JSON-RPC functionality.
"""

import json
from typing import Optional, Dict, Any, Union
import aiohttp

from .admin import AdminClient
from .types import (
    # Admin types
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
    # JSON-RPC types
    JsonRpcRequest,
    JsonRpcExecuteRequest,
    JsonRpcResponse,
    JsonRpcErrorInfo,
    JsonRpcApiResponse,
    ErrorResponse,
)


class CalimeroClient:
    """
    Unified Calimero client that combines Admin and JSON-RPC functionality.

    This client provides a comprehensive interface to both the Calimero Admin API
    and JSON-RPC server, with all methods returning strongly typed response objects.
    """

    # JSON-RPC Constants
    JSONRPC_VERSION = "2.0"
    DEFAULT_TIMEOUT = 1000
    JSONRPC_PATH = "/jsonrpc"

    def __init__(
        self, base_url: str, context_id: str = None, executor_public_key: str = None
    ):
        """
        Initialize the unified Calimero client.

        Args:
            base_url: The base URL of the Calimero node (e.g., "http://localhost:2528").
            context_id: Optional context ID for JSON-RPC operations.
            executor_public_key: Optional public key of the executor for JSON-RPC operations.
        """
        self.base_url = base_url.rstrip("/")
        self.context_id = context_id
        self.executor_public_key = executor_public_key

        # Initialize the admin client
        self.admin = AdminClient(base_url)

        # Session management
        self.session = None

    async def __aenter__(self):
        """Async context manager entry."""
        await self._ensure_session()
        await self.admin.__aenter__()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.admin.__aexit__(exc_type, exc_val, exc_tb)
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

    # ============================================================================
    # JSON-RPC Methods
    # ============================================================================

    def _prepare_headers(self) -> Dict[str, str]:
        """Prepare request headers for JSON-RPC requests."""
        return {"Content-Type": "application/json"}

    def _prepare_request(
        self, method: str, args: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """Prepare the JSON-RPC request payload."""
        return {
            "jsonrpc": self.JSONRPC_VERSION,
            "id": 1,
            "method": "execute",
            "params": {
                "contextId": self.context_id,
                "method": method,
                "argsJson": args or {},
                "executorPublicKey": self.executor_public_key,
                "timeout": self.DEFAULT_TIMEOUT,
            },
        }

    async def _handle_response(
        self, response: aiohttp.ClientResponse
    ) -> JsonRpcResponse:
        """Handle the JSON-RPC response."""
        try:
            data = await response.json()
            if "error" in data and data["error"]:
                raise ValueError(f"JSON-RPC error: {data['error']}")
            return data
        except json.JSONDecodeError as e:
            raise ValueError(f"Failed to decode JSON response: {str(e)}")

    async def execute(
        self, method: str, args: Optional[Dict[str, Any]] = None
    ) -> JsonRpcResponse:
        """
        Execute a JSON-RPC method.

        Args:
            method: The method to call.
            args: Optional arguments for the method.

        Returns:
            The JSON-RPC response.

        Raises:
            ValueError: If the request fails or returns an error.
        """
        await self._ensure_session()

        # Only add JSONRPC_PATH if it's not already in the URL
        if self.JSONRPC_PATH in self.base_url:
            url = self.base_url
        else:
            url = f"{self.base_url}{self.JSONRPC_PATH}"

        headers = self._prepare_headers()
        payload = self._prepare_request(method, args)

        async with self.session.post(url, json=payload, headers=headers) as response:
            return await self._handle_response(response)

    # ============================================================================
    # Admin Method Aliases (for workflow compatibility)
    # ============================================================================

    # Context Management
    async def create_context(
        self,
        application_id: str,
        protocol: str = "near",
        initialization_params: list = None,
    ) -> CreateContextResponse:
        """Create a new context (workflow compatibility)."""
        return await self.admin.create_context(
            application_id, protocol, initialization_params or []
        )

    async def list_contexts(self) -> ListContextsResponse:
        """List all contexts (workflow compatibility)."""
        return await self.admin.list_contexts()

    async def get_context(self, context_id: str) -> GetContextResponse:
        """Get context information (workflow compatibility)."""
        return await self.admin.get_context(context_id)

    async def delete_context(self, context_id: str) -> DeleteContextResponse:
        """Delete a context (workflow compatibility)."""
        return await self.admin.delete_context(context_id)

    # Identity Management
    async def generate_identity(self) -> GenerateIdentityResponse:
        """Generate a new identity (workflow compatibility)."""
        return await self.admin.generate_identity()

    async def list_identities(self, context_id: str) -> ListIdentitiesResponse:
        """List identities in a context (workflow compatibility)."""
        return await self.admin.list_identities(context_id)

    # Application Management
    async def install_dev_application(
        self, path: str, metadata: bytes = b""
    ) -> InstallDevApplicationResponse:
        """Install a development application (workflow compatibility)."""
        return await self.admin.install_dev_application(path, metadata)

    async def install_application(
        self, url: str, hash: str = None, metadata: bytes = b""
    ) -> InstallApplicationResponse:
        """Install an application from URL (workflow compatibility)."""
        return await self.admin.install_application(url, hash, metadata)

    async def list_applications(self) -> ListApplicationsResponse:
        """List all applications (workflow compatibility)."""
        return await self.admin.list_applications()

    async def get_application(self, application_id: str) -> GetApplicationResponse:
        """Get application information (workflow compatibility)."""
        return await self.admin.get_application(application_id)

    async def uninstall_application(
        self, application_id: str
    ) -> UninstallApplicationResponse:
        """Uninstall an application (workflow compatibility)."""
        return await self.admin.uninstall_application(application_id)

    # Invitation and Join
    async def invite(
        self,
        context_id: str,
        granter_id: str,
        grantee_id: str,
        capability: str = "member",
    ):
        """Invite an identity to a context (workflow compatibility)."""
        return await self.admin.invite(context_id, granter_id, grantee_id, capability)

    async def invite_to_context(
        self,
        context_id: str,
        granter_id: str,
        grantee_id: str,
        capability: str = "member",
    ):
        """Invite an identity to a context (workflow compatibility)."""
        return await self.admin.invite_to_context(
            context_id, granter_id, grantee_id, capability
        )

    async def join_context(self, context_id: str, invitee_id: str, invitation: str):
        """Join a context using an invitation (workflow compatibility)."""
        return await self.admin.join_context(context_id, invitee_id, invitation)

    # System Management
    async def health_check(self) -> HealthCheckResponse:
        """Check system health (workflow compatibility)."""
        return await self.admin.health_check()

    async def is_authenticated(self) -> IsAuthenticatedResponse:
        """Check authentication status (workflow compatibility)."""
        return await self.admin.is_authenticated()

    async def get_peers(self) -> GetPeersResponse:
        """Get peer information (workflow compatibility)."""
        return await self.admin.get_peers()

    async def get_peers_count(self) -> GetPeersCountResponse:
        """Get peer count (workflow compatibility)."""
        return await self.admin.get_peers_count()

    async def get_certificate(self) -> GetCertificateResponse:
        """Get server certificate (workflow compatibility)."""
        return await self.admin.get_certificate()

    async def sync_context(self) -> SyncContextResponse:
        """Synchronize contexts (workflow compatibility)."""
        return await self.admin.sync_context()

    # ============================================================================
    # Convenience Methods
    # ============================================================================

    def set_context(self, context_id: str):
        """Set the context ID for JSON-RPC operations."""
        self.context_id = context_id
        return self

    def set_executor(self, executor_public_key: str):
        """Set the executor public key for JSON-RPC operations."""
        self.executor_public_key = executor_public_key
        return self

    def get_context_id(self) -> Optional[str]:
        """Get the current context ID."""
        return self.context_id

    def get_executor_public_key(self) -> Optional[str]:
        """Get the current executor public key."""
        return self.executor_public_key

    def has_context(self) -> bool:
        """Check if the client has a context ID."""
        return self.context_id is not None

    def has_executor(self) -> bool:
        """Check if the client has an executor public key."""
        return self.executor_public_key is not None

    def can_execute(self) -> bool:
        """Check if the client can execute JSON-RPC methods."""
        return self.has_context() and self.has_executor()

    # ============================================================================
    # Manager Access (for advanced usage)
    # ============================================================================

    @property
    def contexts(self):
        """Access to context management operations."""
        return self.admin.contexts

    @property
    def identities(self):
        """Access to identity management operations."""
        return self.admin.identities

    @property
    def applications(self):
        """Access to application management operations."""
        return self.admin.applications

    @property
    def blobs(self):
        """Access to blob management operations."""
        return self.admin.blobs

    @property
    def capabilities(self):
        """Access to capability management operations."""
        return self.admin.capabilities

    @property
    def proposals(self):
        """Access to proposal management operations."""
        return self.admin.proposals

    @property
    def aliases(self):
        """Access to alias management operations."""
        return self.admin.aliases

    @property
    def system(self):
        """Access to system management operations."""
        return self.admin.system


# Export the main class
__all__ = ["CalimeroClient"]
