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
        if isinstance(result, dict) and result.get('success'):
            proposals_data = result.get('data', [])
            proposals = [
                ProposalInfo(
                    id=proposal.get('id', ''),
                    context_id=proposal.get('contextId', ''),
                    title=proposal.get('title', ''),
                    description=proposal.get('description', ''),
                    status=proposal.get('status', ''),
                    created_at=proposal.get('createdAt'),
                    updated_at=proposal.get('updatedAt')
                )
                for proposal in proposals_data
            ]
            return GetProposalsResponse(
                success=True,
                proposals=proposals,
                total_count=len(proposals)
            )
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
        if isinstance(result, dict) and result.get('success'):
            proposal_data = result.get('data', {})
            proposal = ProposalInfo(
                id=proposal_data.get('id', ''),
                context_id=proposal_data.get('contextId', ''),
                title=proposal_data.get('title', ''),
                description=proposal_data.get('description', ''),
                status=proposal_data.get('status', ''),
                created_at=proposal_data.get('createdAt'),
                updated_at=proposal_data.get('updatedAt')
            )
            return GetProposalResponse(
                success=True,
                proposal=proposal
            )
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
        if isinstance(result, dict) and result.get('success'):
            return GetNumberOfActiveProposalsResponse(
                success=True,
                context_id=context_id,
                count=result.get('data', 0)
            )
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
        if isinstance(result, dict) and result.get('success'):
            return GetProposalApprovalsCountResponse(
                success=True,
                context_id=context_id,
                proposal_id=proposal_id,
                count=result.get('data', 0)
            )
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
        if isinstance(result, dict) and result.get('success'):
            approvers_data = result.get('data', [])
            return GetProposalApproversResponse(
                success=True,
                context_id=context_id,
                proposal_id=proposal_id,
                approvers=approvers_data,
                total_count=len(approvers_data)
            )
        else:
            raise ValueError(f"Failed to get proposal approvers: {result}")


# Export the class
__all__ = ['ProposalManager']
