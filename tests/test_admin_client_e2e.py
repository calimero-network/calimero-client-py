import pytest
from calimero import AdminClient
from merobox.testing import nodes


@nodes(count=1, prefix="admin-test", scope="function")
def admin_node():
    """Single Calimero node for testing AdminClient functionality."""
    pass


@pytest.mark.asyncio
async def test_admin_client_health_check_e2e(admin_node):
    """Test AdminClient health check against a real Calimero node."""
    # Get the endpoint for the first (and only) node
    node_endpoint = admin_node.endpoint(0)
    
    # Create AdminClient pointing to the running node
    client = AdminClient(node_endpoint)
    
    # Test health check
    resp = await client.health_check()
    
    assert resp.get("success") is True
    assert isinstance(resp.get("data"), dict)
    # API returns {"data": {"status": "alive"}}
    assert resp["data"].get("data", {}).get("status") == "alive"


@pytest.mark.asyncio
async def test_admin_client_context_operations_e2e(admin_node):
    """Test AdminClient context operations against a real Calimero node."""
    node_endpoint = admin_node.endpoint(0)
    client = AdminClient(node_endpoint)
    
    # Test listing contexts (should work even if empty)
    resp = await client.list_contexts()
    assert resp.get("success") is True
    assert isinstance(resp.get("data"), dict)
    
    # Test getting peers info (more likely to be available)
    resp = await client.get_peers()
    assert resp.get("success") is True
    assert isinstance(resp.get("data"), dict)


