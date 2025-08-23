"""
Blob Management Module for Calimero Admin Client.

This module handles all blob-related operations including upload, download,
listing, retrieval, and deletion of blobs.
"""

from ..types import (
    UploadBlobRequest, UploadBlobResponse, DownloadBlobResponse,
    ListBlobsResponse, GetBlobInfoResponse, DeleteBlobResponse, BlobInfo
)


class BlobManager:
    """Manages blob operations for the Calimero Admin Client."""
    
    def __init__(self, client):
        """Initialize the blob manager with a reference to the main client."""
        self.client = client
    
    async def upload(self, data: bytes, metadata: bytes = b"") -> UploadBlobResponse:
        """
        Upload a blob to the server.
        
        Args:
            data: The blob data to upload.
            metadata: Optional metadata for the blob.
            
        Returns:
            The upload blob response containing the blob ID.
        """
        request = UploadBlobRequest(
            data=data,
            metadata=metadata
        )
        return await self._upload_typed(request)
    
    async def _upload_typed(self, request: UploadBlobRequest) -> UploadBlobResponse:
        """
        Upload a blob with strongly typed request.
        
        Args:
            request: The upload blob request containing data and metadata.
            
        Returns:
            The upload blob response containing the blob ID.
        """
        payload = {
            "data": list(request.data),  # Convert bytes to list for JSON serialization
            "metadata": list(request.metadata)  # Convert bytes to list for JSON serialization
        }
        result = await self.client._make_request('POST', '/admin-api/blobs', payload)
        if isinstance(result, dict):
            if result.get('success') or 'data' in result:
                data = result.get('data', result)
                return UploadBlobResponse(
                    success=True,
                    blob_id=data.get('blobId', data.get('blob_id', '')),
                    size=len(request.data)
                )
        
        # Check if it's a 405 error (either in dict or in string representation)
        if '405' in str(result):
            # Method not allowed - blob upload might not be supported
            raise ValueError(f"Blob upload not supported: {result}")
        
        raise ValueError(f"Failed to upload blob: {result}")
    
    async def download(self, blob_id: str) -> DownloadBlobResponse:
        """
        Download a blob from the server.
        
        Args:
            blob_id: The ID of the blob to download.
            
        Returns:
            The download blob response containing the blob data.
        """
        result = await self.client._make_request('GET', f'/admin-api/blobs/{blob_id}')
        if isinstance(result, dict) and result.get('success'):
            return DownloadBlobResponse(
                success=True,
                blob_id=blob_id,
                data=result.get('data', b''),
                metadata=result.get('metadata', b'')
            )
        else:
            raise ValueError(f"Failed to download blob: {result}")
    
    async def list_all(self) -> ListBlobsResponse:
        """
        List all blobs on the server.
        
        Returns:
            The list blobs response containing the list of blobs.
        """
        result = await self.client._make_request('GET', '/admin-api/blobs')
        if isinstance(result, dict):
            # Handle both formats: {'success': true, 'data': [...]} and {'data': {'blobs': [...]}}
            if result.get('success') or 'data' in result:
                data = result.get('data', result)
                # Handle nested blobs structure
                blobs_data = data.get('blobs', data) if isinstance(data, dict) else data
                if not isinstance(blobs_data, list):
                    blobs_data = []
                
                blobs = [
                    BlobInfo(
                        id=blob.get('id', blob.get('blob_id', '')),
                        size=blob.get('size', 0),
                        metadata=blob.get('metadata', b''),
                        created_at=blob.get('createdAt'),
                        updated_at=blob.get('updatedAt')
                    )
                    for blob in blobs_data if isinstance(blob, dict)
                ]
                return ListBlobsResponse(
                    success=True,
                    blobs=blobs,
                    total_count=len(blobs)
                )
        raise ValueError(f"Failed to list blobs: {result}")
    
    async def get_info(self, blob_id: str) -> GetBlobInfoResponse:
        """
        Get information about a specific blob.
        
        Args:
            blob_id: The ID of the blob.
            
        Returns:
            The get blob info response containing the blob information.
        """
        result = await self.client._make_request('GET', f'/admin-api/blobs/{blob_id}/info')
        if isinstance(result, dict) and result.get('success'):
            blob_data = result.get('data', {})
            blob_info = BlobInfo(
                id=blob_data.get('id', ''),
                size=blob_data.get('size', 0),
                metadata=blob_data.get('metadata', b''),
                created_at=blob_data.get('createdAt'),
                updated_at=blob_data.get('updatedAt')
            )
            return GetBlobInfoResponse(
                success=True,
                blob=blob_info
            )
        else:
            raise ValueError(f"Failed to get blob info: {result}")
    
    async def delete(self, blob_id: str) -> DeleteBlobResponse:
        """
        Delete a blob from the server.
        
        Args:
            blob_id: The ID of the blob to delete.
            
        Returns:
            The delete blob response confirming the deletion.
        """
        result = await self.client._make_request('DELETE', f'/admin-api/blobs/{blob_id}')
        if isinstance(result, dict) and result.get('success'):
            return DeleteBlobResponse(
                success=True,
                blob_id=blob_id
            )
        else:
            raise ValueError(f"Failed to delete blob: {result}")


# Export the class
__all__ = ['BlobManager']
