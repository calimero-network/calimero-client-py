# Calimero Client - Rust-based Implementation

## Overview

This document describes the Calimero client that provides better performance and more features than the previous Python-only implementation. The old client has been completely replaced with this Rust-based version.

## Installation

The new client requires the Rust bindings package:

```bash
pip install calimero-client-py-bindings
```

## Key Benefits

- **🚀 Better Performance**: Rust implementation provides faster execution
- **📚 More Methods**: 64 methods vs. fewer in the old client
- **🛡️ Better Error Handling**: Improved error handling and validation
- **🔄 Backward Compatibility**: 100% compatibility with existing code
- **🏗️ Organized Access**: Manager-based organization for better code structure

## Available Methods

The new client provides 64 methods organized into several categories:

### Core Operations
- `create_context()` - Create new contexts
- `list_contexts()` - List all contexts
- `get_context()` - Get context information
- `delete_context()` - Delete contexts
- `execute()` - Execute JSON-RPC methods

### Identity Management
- `generate_identity()` - Generate new identities
- `list_identities()` - List identities in a context
- `invite_to_context()` - Invite identities to contexts
- `join_context()` - Join contexts using invitations

### Application Management
- `install_application()` - Install applications from URLs
- `install_dev_application()` - Install development applications
- `list_applications()` - List all applications
- `get_application()` - Get application information
- `uninstall_application()` - Uninstall applications

### New Features (Rust-specific)
- `create_alias()` - Create aliases for resources
- `delete_alias()` - Delete aliases
- `lookup_alias()` - Lookup alias information
- `resolve_alias()` - Resolve aliases to targets
- `list_aliases()` - List all aliases
- `get_supported_alias_types()` - Get supported alias types
- `grant_permissions()` - Grant permissions to identities
- `revoke_permissions()` - Revoke permissions from identities
- `get_context_storage()` - Get context storage information
- `get_context_client_keys()` - Get context client keys
- `get_proposal()` - Get proposal information
- `list_proposals()` - List all proposals
- `get_proposal_approvers()` - Get proposal approvers
- `update_context_application()` - Update context applications

### Strongly Typed Methods (Request-based)
- `create_context_request()` - Create context using request object
- `invite_to_context_request()` - Invite identity using request object
- `join_context_request()` - Join context using request object
- `install_dev_application_request()` - Install dev app using request object
- `install_application_request()` - Install app using request object
- `grant_capabilities_request()` - Grant capabilities using request object
- `revoke_capabilities_request()` - Revoke capabilities using request object
- `create_alias_request()` - Create alias using request object
- `execute_jsonrpc_request()` - Execute JSON-RPC using request object

### System Management
- `get_peers_count()` - Get connected peer count
- `sync_context()` - Synchronize contexts
- `get_api_url()` - Get the API URL

## Usage Examples

### Basic Client Creation

```python
from calimero import CalimeroClient

# Create a client instance
client = CalimeroClient("http://localhost:2528")

# Set context and executor for JSON-RPC operations
client.set_context("your-context-id")
client.set_executor("your-executor-public-key")
```

### Context Management

```python
# Create a new context
context_response = await client.create_context(
    application_id="your-app-id",
    protocol="near"
)

# List all contexts
contexts = await client.list_contexts()

# Get specific context
context = await client.get_context("context-id")
```

### Application Management

```python
# Install an application
app_response = await client.install_application(
    url="https://example.com/app",
    hash="app-hash",
    metadata=b"app-metadata"
)

# List applications
apps = await client.list_applications()

# Get application info
app = await client.get_application("app-id")
```

### Identity and Invitation Workflow

```python
# Generate a new identity
identity = await client.generate_identity()

# Invite identity to context
invitation = await client.invite_to_context(
    context_id="context-id",
    granter_id="granter-id",
    grantee_id="grantee-id",
    capability="member"
)

# Join context using invitation
join_result = await client.join_context(
    context_id="context-id",
    invitee_id="invitee-id",
    invitation="invitation-data"
)
```

### New Alias Features

```python
# Create an alias
alias_result = await client.create_alias(
    alias_type="context",
    alias_value="my-context",
    target_id="actual-context-id"
)

# Lookup an alias
lookup_result = await client.lookup_alias(
    alias_type="context",
    alias_value="my-context"
)

# List all aliases
aliases = await client.list_aliases()

# Get supported alias types
alias_types = await client.get_supported_alias_types()
```

### Permission Management

```python
# Grant permissions
grant_result = await client.grant_permissions(
    context_id="context-id",
    grantee_id="grantee-id",
    permissions=["ManageApplication", "ManageMembers"]
)

# Revoke permissions
revoke_result = await client.revoke_permissions(
    context_id="context-id",
    grantee_id="grantee-id",
    permissions=["ManageApplication"]
)
```

### JSON-RPC Execution

```python
# Set context and executor
client.set_context("context-id")
client.set_executor("executor-public-key")

# Execute a method
result = await client.execute(
    method="set",
    args={"key": "value"}
)
```

## Manager-Based Access

The client provides organized access through manager objects:

```python
# Context management
contexts = client.contexts
await contexts.create_context("app-id")
await contexts.list_contexts()

# Application management
apps = client.applications
await apps.install_application("url")
await apps.list_applications()

# Capability management
caps = client.capabilities
await caps.grant_capability("context-id", "grantee-id", "capability")

# Identity management
idents = client.identities
await idents.generate_identity()
await idents.list_identities("context-id")

# Blob management
blobs = client.blobs
await blobs.get_blob_info("blob-id")
await blobs.list_blobs()

# Proposal management
props = client.proposals
await props.get_proposal("proposal-id")
await props.list_proposals()

# Alias management
aliases = client.aliases
await aliases.create_alias("type", "value", "target")
await aliases.list_aliases()

# System management
system = client.system
await system.get_peers_count()
await system.sync_context()
```

## Migration from Old Client

The old client has been completely replaced with this Rust-based implementation. All existing code will continue to work without changes:

```python
# This still works exactly the same
from calimero import CalimeroClient
client = CalimeroClient("http://localhost:2528")

# But now you have access to many more features and better performance
```

The new client maintains 100% backward compatibility while providing additional functionality.

## Error Handling

The new client provides better error handling with proper response types:

```python
try:
    result = await client.create_context("app-id")
    if result.success:
        print(f"Context created: {result.context_id}")
    else:
        print(f"Failed to create context")
except Exception as e:
    print(f"Error: {e}")
```

## Testing

You can test the client using the existing test suite:

```bash
# Run all tests
python -m pytest tests/ -v

# Run specific test categories
python -m pytest tests/test_application_workflows.py -v
python -m pytest tests/test_context_workflows.py -v
python -m pytest tests/test_identity_workflows.py -v
python -m pytest tests/test_execute_workflows.py -v
```

## Performance Comparison

The Rust-based implementation provides:
- **Faster execution** for API calls
- **Lower memory usage**
- **Better concurrency handling**
- **More efficient data serialization**

## Support

For issues or questions about the new client:
1. Check the error messages for specific validation issues
2. Ensure all required fields are provided in responses
3. Verify the Rust bindings package is properly installed
4. Check that the Calimero node is accessible

## Future Enhancements

The new client architecture allows for:
- Additional Rust-specific features
- Better integration with Calimero backend
- Improved performance optimizations
- Extended API coverage
