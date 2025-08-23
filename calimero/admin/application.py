"""
Application Management Module for Calimero Admin Client.

This module handles all application-related operations including installation,
listing, retrieval, and uninstallation of applications.
"""

from ..types import (
    InstallDevApplicationRequest, InstallDevApplicationResponse,
    InstallApplicationRequest, InstallApplicationResponse,
    ListApplicationsResponse, GetApplicationResponse,
    UninstallApplicationResponse, ApplicationInfo
)


class ApplicationManager:
    """Manages application operations for the Calimero Admin Client."""
    
    def __init__(self, client):
        """Initialize the application manager with a reference to the main client."""
        self.client = client
    
    async def install_dev(self, path: str, metadata: bytes = b"") -> InstallDevApplicationResponse:
        """
        Install a development application.
        
        Args:
            path: The local path to install the application from.
            metadata: Application metadata as bytes.
            
        Returns:
            The install dev application response containing the application ID.
        """
        request = InstallDevApplicationRequest(
            path=path,
            metadata=metadata
        )
        return await self._install_dev_typed(request)
    
    async def _install_dev_typed(self, request: InstallDevApplicationRequest) -> InstallDevApplicationResponse:
        """
        Install a development application with strongly typed request.
        
        Args:
            request: The install dev application request containing path and metadata.
            
        Returns:
            The install dev application response containing the application ID.
        """
        payload = {
            "path": request.path,
            "metadata": list(request.metadata)  # Convert bytes to list for JSON serialization
        }
        result = await self.client._make_request('POST', '/admin-api/install-dev-application', payload)
        if isinstance(result, dict) and result.get('success'):
            data = result.get('data', {})
            return InstallDevApplicationResponse(
                success=True,
                application_id=data.get('applicationId', ''),
                path=request.path
            )
        else:
            raise ValueError(f"Failed to install dev application: {result}")
    
    async def install_from_url(self, url: str, hash: str = None, metadata: bytes = b"") -> InstallApplicationResponse:
        """
        Install an application from URL.
        
        Args:
            url: The URL to install the application from.
            hash: Optional hash for verification.
            metadata: Application metadata as bytes.
            
        Returns:
            The install application response containing the application ID.
        """
        request = InstallApplicationRequest(
            url=url,
            hash=hash,
            metadata=metadata
        )
        return await self._install_typed(request)
    
    async def _install_typed(self, request: InstallApplicationRequest) -> InstallApplicationResponse:
        """
        Install an application from URL with strongly typed request.
        
        Args:
            request: The install application request containing URL, hash, and metadata.
            
        Returns:
            The install application response containing the application ID.
        """
        payload = {
            "url": request.url,
            "metadata": list(request.metadata)  # Convert bytes to list for JSON serialization
        }
        if request.hash:
            payload["hash"] = request.hash
        result = await self.client._make_request('POST', '/admin-api/install-application', payload)
        if isinstance(result, dict) and result.get('success'):
            data = result.get('data', {})
            return InstallApplicationResponse(
                success=True,
                application_id=data.get('applicationId', ''),
                url=request.url,
                hash=request.hash
            )
        else:
            raise ValueError(f"Failed to install application: {result}")
    
    async def list_all(self) -> ListApplicationsResponse:
        """
        List all installed applications.
        
        Returns:
            The list applications response containing the list of applications.
        """
        result = await self.client._make_request('GET', '/admin-api/applications')
        if isinstance(result, dict):
            # Handle both formats: {'success': true, 'data': [...]} and {'data': {'apps': [...]}}
            if result.get('success') or 'data' in result:
                data = result.get('data', result)
                # Handle nested apps structure
                applications_data = data.get('apps', data) if isinstance(data, dict) else data
                if not isinstance(applications_data, list):
                    applications_data = []
                
                applications = [
                    ApplicationInfo(
                        id=app.get('id', ''),
                        name=app.get('name'),
                        version=app.get('version'),
                        status=app.get('status', 'installed'),  # Default to 'installed' if not provided
                        installed_at=app.get('installedAt'),
                        metadata=bytes(app.get('metadata', [])) if app.get('metadata') else None
                    )
                    for app in applications_data if isinstance(app, dict)
                ]
                return ListApplicationsResponse(
                    success=True,
                    applications=applications,
                    total_count=len(applications)
                )
        raise ValueError(f"Failed to list applications: {result}")
    
    async def get(self, application_id: str) -> GetApplicationResponse:
        """
        Get information about a specific application.
        
        Args:
            application_id: The ID of the application to retrieve.
            
        Returns:
            The get application response containing the application information.
        """
        result = await self.client._make_request('GET', f'/admin-api/applications/{application_id}')
        if isinstance(result, dict) and result.get('success'):
            app_data = result.get('data', {})
            application = ApplicationInfo(
                id=app_data.get('id', ''),
                name=app_data.get('name'),
                version=app_data.get('version'),
                status=app_data.get('status', 'installed'),  # Default to 'installed' if not provided
                installed_at=app_data.get('installedAt'),
                metadata=bytes(app_data.get('metadata', [])) if app_data.get('metadata') else None
            )
            return GetApplicationResponse(
                success=True,
                application=application
            )
        else:
            raise ValueError(f"Failed to get application: {result}")
    
    async def uninstall(self, application_id: str) -> UninstallApplicationResponse:
        """
        Uninstall an application.
        
        Args:
            application_id: The ID of the application to uninstall.
            
        Returns:
            The uninstall application response confirming the uninstallation.
        """
        result = await self.client._make_request('DELETE', f'/admin-api/applications/{application_id}')
        if isinstance(result, dict) and result.get('success'):
            return UninstallApplicationResponse(
                success=True,
                application_id=application_id
            )
        else:
            raise ValueError(f"Failed to uninstall application: {result}")


# Export the class
__all__ = ['ApplicationManager']
