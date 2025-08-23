"""
System Management Module for Calimero Admin Client.

This module handles all system-related operations including health checks,
authentication status, peer information, and system certificates.
"""

from ..types import (
    HealthCheckResponse, IsAuthenticatedResponse, GetPeersResponse,
    GetPeersCountResponse, GetCertificateResponse, PeerInfo
)


class SystemManager:
    """Manages system operations for the Calimero Admin Client."""
    
    def __init__(self, client):
        """Initialize the system manager with a reference to the main client."""
        self.client = client
    
    async def health_check(self) -> HealthCheckResponse:
        """
        Check the health status of the server.
        
        Returns:
            The health check response.
        """
        result = await self.client._make_request('GET', '/admin-api/health')
        if isinstance(result, dict):
            # Handle both formats: {'success': true, 'data': {...}} and {'data': {...}}
            if result.get('success') or 'data' in result:
                data = result.get('data', result)
                return HealthCheckResponse(
                    success=True,
                    status=data.get('status', 'unknown'),
                    timestamp=data.get('timestamp')
                )
        raise ValueError(f"Failed to check health: {result}")
    
    async def is_authenticated(self) -> IsAuthenticatedResponse:
        """
        Check if the current session is authenticated.
        
        Returns:
            The is authenticated response.
        """
        result = await self.client._make_request('GET', '/admin-api/is-authed')
        if isinstance(result, dict):
            # Handle both formats: {'success': true, 'data': {...}} and {'data': {...}}
            if result.get('success') or 'data' in result:
                data = result.get('data', result)
                return IsAuthenticatedResponse(
                    success=True,
                    authenticated=data.get('authenticated', False)
                )
        raise ValueError(f"Failed to check authentication: {result}")
    
    async def get_peers(self) -> GetPeersResponse:
        """
        Get information about connected peers.
        
        Returns:
            The get peers response containing the list of peers.
        """
        result = await self.client._make_request('GET', '/admin-api/peers')
        if isinstance(result, dict):
            # Handle different response formats
            if result.get('success') or 'data' in result or 'count' in result:
                # If we get just a count, return empty peers list with the count
                if 'count' in result and 'data' not in result:
                    return GetPeersResponse(
                        success=True,
                        peers=[],
                        total_count=result.get('count', 0)
                    )
                
                # Handle normal data response
                data = result.get('data', [])
                peers = [
                    PeerInfo(
                        id=peer.get('id', ''),
                        address=peer.get('address', ''),
                        port=peer.get('port', 0),
                        version=peer.get('version', ''),
                        last_seen=peer.get('lastSeen')
                    )
                    for peer in data if isinstance(peer, dict)
                ]
                return GetPeersResponse(
                    success=True,
                    peers=peers,
                    total_count=result.get('count', len(peers))
                )
        raise ValueError(f"Failed to get peers: {result}")
    
    async def get_peers_count(self) -> GetPeersCountResponse:
        """
        Get the count of connected peers.
        
        Returns:
            The get peers count response.
        """
        result = await self.client._make_request('GET', '/admin-api/peers/count')
        if isinstance(result, dict):
            # Handle both formats and fallback to peers endpoint if count endpoint doesn't exist
            if result.get('success') or 'data' in result or 'count' in result:
                count = result.get('data', result.get('count', 0))
                return GetPeersCountResponse(
                    success=True,
                    peers_count=count
                )
            elif result.get('success') == False or result.get('success') == "False":  # Handle both boolean and string
                # Check if it's a 404 error
                if '404' in str(result.get('error', '')):
                    # Fallback: try to get count from peers endpoint
                    try:
                        peers_result = await self.get_peers()
                        return GetPeersCountResponse(
                            success=True,
                            peers_count=peers_result.total_count
                        )
                    except:
                        # If that fails too, return 0
                        return GetPeersCountResponse(
                            success=True,
                            peers_count=0
                        )
                else:
                    # Other error, raise it
                    raise ValueError(f"Failed to get peers count: {result}")
        
        # Check if it's a 404 specifically in the string representation
        if '404' in str(result):
            # 404 error - fallback or return 0 count
            try:
                peers_result = await self.get_peers()
                return GetPeersCountResponse(
                    success=True,
                    peers_count=peers_result.total_count
                )
            except:
                # If that fails too, return 0
                return GetPeersCountResponse(
                    success=True,
                    peers_count=0
                )
        
        raise ValueError(f"Failed to get peers count: {result}")
    
    async def get_certificate(self) -> GetCertificateResponse:
        """
        Get the server certificate.
        
        Returns:
            The get certificate response.
        """
        result = await self.client._make_request('GET', '/admin-api/certificate')
        if isinstance(result, dict):
            # Handle both formats
            if result.get('success') or 'data' in result:
                data = result.get('data', result)
                return GetCertificateResponse(
                    success=True,
                    certificate=data.get('certificate', ''),
                    expires_at=data.get('expiresAt')
                )
            elif result.get('success') == False:  # Use == instead of is for boolean comparison
                # Check if it's a 404 error
                if '404' in str(result.get('error', '')):
                    # Certificate not found is acceptable - return empty certificate
                    return GetCertificateResponse(
                        success=True,
                        certificate="",
                        expires_at=None
                    )
                else:
                    # Other error, raise it
                    raise ValueError(f"Failed to get certificate: {result}")
        
        # Check if it's a 404 specifically in the string representation
        if '404' in str(result):
            # Certificate not found is acceptable - return empty certificate
            return GetCertificateResponse(
                success=True,
                certificate="",
                expires_at=None
            )
        
        raise ValueError(f"Failed to get certificate: {result}")


# Export the class
__all__ = ['SystemManager']
