# Calimero Admin Client - Modular Structure

## Overview

The Calimero Admin Client has been refactored from a single monolithic file into a modular, organized structure that provides better maintainability, separation of concerns, and extensibility.

## New Structure

```
calimero/
├── admin/
│   ├── __init__.py          # Main AdminClient class and backward compatibility
│   ├── context.py           # Context management operations
│   ├── identity.py          # Identity management operations
│   ├── application.py       # Application management operations
│   ├── blob.py             # Blob management operations
│   ├── capability.py        # Capability management operations
│   ├── proposal.py          # Proposal management operations
│   ├── alias.py            # Alias management operations
│   └── system.py           # System operations and health checks
├── types.py                 # All Pydantic request/response models
├── json_rpc_client.py       # JSON-RPC client
├── ws_subscriptions_client.py # WebSocket client
└── __init__.py             # Package exports
```

## Benefits of the New Structure

### 1. **Separation of Concerns**
- Each module handles a specific domain of functionality
- Clear boundaries between different types of operations
- Easier to understand and maintain

### 2. **Improved Maintainability**
- Smaller, focused files are easier to work with
- Changes to one domain don't affect others
- Better code organization and readability

### 3. **Enhanced Extensibility**
- New functionality can be added to specific modules
- Easy to add new managers for additional domains
- Clear patterns for extending the system

### 4. **Better Testing**
- Individual modules can be tested in isolation
- Easier to mock specific functionality
- More focused unit tests

### 5. **Team Collaboration**
- Multiple developers can work on different modules
- Reduced merge conflicts
- Clear ownership of different domains

## Module Details

### `admin/__init__.py` - Main AdminClient
- **Purpose**: Main client class that orchestrates all operations
- **Responsibilities**: 
  - HTTP session management
  - Backward compatibility methods
  - Manager initialization and coordination
- **Key Features**:
  - Async context manager support
  - Automatic session management
  - Delegation to specialized managers

### `admin/context.py` - ContextManager
- **Purpose**: Manages all context-related operations
- **Operations**:
  - Create, list, get, delete contexts
  - Update context applications
  - Manage context storage
  - Sync contexts
  - Get proxy contracts

### `admin/identity.py` - IdentityManager
- **Purpose**: Handles identity operations
- **Operations**:
  - Generate new identities
  - List identities in contexts

### `admin/application.py` - ApplicationManager
- **Purpose**: Manages application lifecycle
- **Operations**:
  - Install applications (dev and URL-based)
  - List, get, uninstall applications

### `admin/blob.py` - BlobManager
- **Purpose**: Handles blob storage operations
- **Operations**:
  - Upload, download, list blobs
  - Get blob information
  - Delete blobs

### `admin/capability.py` - CapabilityManager
- **Purpose**: Manages user capabilities
- **Operations**:
  - Grant capabilities to users
  - Revoke capabilities from users

### `admin/proposal.py` - ProposalManager
- **Purpose**: Handles governance proposals
- **Operations**:
  - List and get proposals
  - Get proposal counts and approvers
  - Manage proposal lifecycle

### `admin/alias.py` - AliasManager
- **Purpose**: Manages human-readable aliases
- **Operations**:
  - Create aliases for contexts, applications, identities
  - Lookup aliases
  - List and delete aliases

### `admin/system.py` - SystemManager
- **Purpose**: Handles system-level operations
- **Operations**:
  - Health checks
  - Authentication status
  - Peer information
  - Certificate management

## Usage Examples

### Traditional Usage (Backward Compatible)
```python
from calimero.admin import AdminClient

client = AdminClient("http://localhost:2428")

# Old-style method calls still work
context = await client.create_context("app123", "near")
applications = await client.list_applications()
health = await client.health_check()
```

### New Modular Usage
```python
from calimero.admin import AdminClient

client = AdminClient("http://localhost:2428")

# Use specialized managers for better organization
context = await client.contexts.create("app123", "near")
applications = await client.applications.list_all()
health = await client.system.health_check()

# More specific operations
blob = await client.blobs.upload(b"data", b"metadata")
capability = await client.capabilities.grant("ctx1", "user1", "user2", "read")
proposals = await client.proposals.list_all("ctx1", offset=0, limit=10)
```

## Migration Guide

### For Existing Code
**No changes required!** All existing code continues to work exactly as before. The backward compatibility layer ensures that all old method calls are automatically routed to the appropriate managers.

### For New Code
Consider using the new modular approach for better organization:

```python
# Instead of:
await client.create_context("app123", "near")

# Consider:
await client.contexts.create("app123", "near")
```

## Adding New Functionality

To add new functionality to a specific domain:

1. **Add to existing manager**: Extend the appropriate manager class
2. **Create new manager**: Add a new manager file and register it in `AdminClient.__init__`
3. **Add types**: Define new request/response models in `types.py`

### Example: Adding a new context operation
```python
# In admin/context.py
class ContextManager:
    async def get_context_metrics(self, context_id: str) -> GetContextMetricsResponse:
        """Get performance metrics for a context."""
        result = await self.client._make_request('GET', f'/admin-api/contexts/{context_id}/metrics')
        # ... implementation
```

## Testing

Each module can be tested independently:

```python
# Test specific manager
from calimero.admin.context import ContextManager
from unittest.mock import Mock

client_mock = Mock()
context_manager = ContextManager(client_mock)
# Test context manager methods
```

## Future Enhancements

- **Plugin System**: Allow external modules to register new managers
- **Configuration**: Per-manager configuration options
- **Metrics**: Performance monitoring per manager
- **Caching**: Manager-level caching strategies
- **Validation**: Manager-specific validation rules

## Conclusion

The new modular structure provides significant benefits in terms of maintainability, extensibility, and code organization while maintaining 100% backward compatibility. This makes the codebase easier to work with for both current and future development efforts.
