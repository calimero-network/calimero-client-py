import pytest
import json
from unittest.mock import Mock, patch, AsyncMock
from calimero import WsSubscriptionsClient

@pytest.mark.asyncio
async def test_ws_client_creation():
    """Test WsSubscriptionsClient initialization."""
    client = WsSubscriptionsClient("http://localhost:2428")
    assert client.base_url == "http://localhost:2428"
    assert client.endpoint == "/ws"
    assert not client._running
    assert len(client.callbacks) == 0
    assert len(client.subscribed_apps) == 0

@pytest.mark.asyncio
async def test_connect_and_disconnect(mock_ws_url):
    """Test WebSocket connection and disconnection."""
    client = WsSubscriptionsClient("http://localhost:2428")
    
    with patch('websockets.connect', new_callable=AsyncMock) as mock_connect:
        mock_ws = AsyncMock()
        mock_connect.return_value = mock_ws
        
        await client.connect()
        assert client._running
        assert client.ws == mock_ws
        
        await client.disconnect()
        assert not client._running
        assert client.ws is None
        mock_ws.close.assert_called_once()

@pytest.mark.asyncio
async def test_subscribe_and_unsubscribe():
    """Test subscription management."""
    client = WsSubscriptionsClient("http://localhost:2428")
    client.ws = AsyncMock()
    
    # Test subscribe
    app_ids = ["app1", "app2"]
    client.subscribe(app_ids)
    assert client.subscribed_apps == app_ids
    
    # Verify subscribe message was sent
    expected_message = {
        "type": "subscribe",
        "data": {
            "applicationIds": app_ids
        }
    }
    client.ws.send.assert_called_once_with(json.dumps(expected_message))
    
    # Test unsubscribe
    client.unsubscribe(["app1"])
    assert client.subscribed_apps == ["app2"]
    
    # Verify unsubscribe message was sent
    expected_message = {
        "type": "unsubscribe",
        "data": {
            "applicationIds": ["app1"]
        }
    }
    assert client.ws.send.call_count == 2
    client.ws.send.assert_called_with(json.dumps(expected_message))

@pytest.mark.asyncio
async def test_callback_management():
    """Test callback management."""
    client = WsSubscriptionsClient("http://localhost:2428")
    
    # Test adding callback
    callback1 = Mock()
    callback2 = Mock()
    client.add_callback(callback1)
    client.add_callback(callback2)
    assert len(client.callbacks) == 2
    
    # Test removing callback
    client.remove_callback(callback1)
    assert len(client.callbacks) == 1
    assert client.callbacks[0] == callback2

@pytest.mark.asyncio
async def test_message_handling():
    """Test WebSocket message handling."""
    client = WsSubscriptionsClient("http://localhost:2428")
    client.ws = AsyncMock()
    client._running = True
    
    # Setup callback
    callback = Mock()
    client.add_callback(callback)
    
    # Test message handling
    test_message = {"type": "test", "data": "test_data"}
    client.ws.recv.return_value = json.dumps(test_message)
    
    await client._listen()
    
    # Verify callback was called with correct data
    callback.assert_called_once_with(test_message)

@pytest.mark.asyncio
async def test_connection_recovery():
    """Test WebSocket connection recovery."""
    client = WsSubscriptionsClient("http://localhost:2428")
    client.ws = AsyncMock()
    client._running = True
    
    # Simulate connection closed
    client.ws.recv.side_effect = Exception("Connection closed")
    
    with patch('websockets.connect', new_callable=AsyncMock) as mock_connect:
        mock_ws = AsyncMock()
        mock_connect.return_value = mock_ws
        
        await client._listen()
        
        # Verify reconnection was attempted
        mock_connect.assert_called_once()
        assert client.ws == mock_ws 