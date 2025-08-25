"""
Proposal Management Module for Calimero Admin Client.

This module handles all proposal-related operations including listing,
retrieval, and management of proposals within contexts.
"""

from ..types import (
    GetProposalsResponse, GetProposalResponse,
    GetNumberOfActiveProposalsResponse, GetProposalApprovalsCountResponse,
    GetProposalApproversResponse, ProposalInfo
)


class ProposalManager:
    """Manages proposal operations for the Calimero Admin Client."""
    
    def __init__(self, client):
        """Initialize the proposal manager with a reference to the main client."""
        self.client = client
    
    async def list_all(self, context_id: str, offset: int = 0, limit: int = 100) -> GetProposalsResponse:
        """
        Get proposals for a context.
        
        Args:
            context_id: The ID of the context.
            offset: Number of proposals to skip.
            limit: Maximum number of proposals to return.
            
        Returns:
            The get proposals response containing the list of proposals.
        """
        params = {"offset": offset, "limit": limit}
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/proposals', params)
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to get proposals: {result}")
    
    async def get(self, context_id: str, proposal_id: str) -> GetProposalResponse:
        """
        Get a specific proposal.
        
        Args:
            context_id: The ID of the context.
            proposal_id: The ID of the proposal.
            
        Returns:
            The get proposal response containing the proposal information.
        """
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/proposals/{proposal_id}')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to get proposal: {result}")
    
    async def get_active_count(self, context_id: str) -> GetNumberOfActiveProposalsResponse:
        """
        Get the number of active proposals in a context.
        
        Args:
            context_id: The ID of the context.
            
        Returns:
            The get number of active proposals response.
        """
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/proposals/active/count')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to get active proposals count: {result}")
    
    async def get_approvals_count(self, context_id: str, proposal_id: str) -> GetProposalApprovalsCountResponse:
        """
        Get the approval count for a proposal.
        
        Args:
            context_id: The ID of the context.
            proposal_id: The ID of the proposal.
            
        Returns:
            The get proposal approvals count response.
        """
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/proposals/{proposal_id}/approvals/count')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to get proposal approvals count: {result}")
    
    async def get_approvers(self, context_id: str, proposal_id: str) -> GetProposalApproversResponse:
        """
        Get the approvers for a proposal.
        
        Args:
            context_id: The ID of the context.
            proposal_id: The ID of the proposal.
            
        Returns:
            The get proposal approvers response.
        """
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/proposals/{proposal_id}/approvers')
        if isinstance(result, dict) and (result.get('success') or 'data' in result):
            # Add success field if it doesn't exist, so the workflow engine can access it
            if 'success' not in result:
                result['success'] = True
            return result
        else:
            raise ValueError(f"Failed to get proposal approvers: {result}")


# Export the class
__all__ = ['ProposalManager']
