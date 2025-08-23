# Testing Guide for Calimero Client

This document describes how to run tests for the Calimero client with merobox integration.

## Prerequisites

1. **Python Environment**: Ensure you have Python 3.8+ installed
2. **Virtual Environment**: Activate the virtual environment:
   ```bash
   # For Fish shell
   source .venv/bin/activate.fish
   
   # For Bash/Zsh
   source .venv/bin/activate
   ```
3. **Dependencies**: Install the package in development mode:
   ```bash
   pip install -e .
   ```

## Running Tests

### Option 1: Python Script (Recommended)

The most convenient way to run tests with full output:

```bash
python scripts/run_tests.py
```

This script:
- ✅ Shows all merobox workflow output
- ✅ Displays test execution details
- ✅ Uses proper pytest flags for verbose output
- ✅ Shows test durations and performance metrics

### Option 2: Makefile Commands

Use the provided Makefile for different testing scenarios:

```bash
# Show all available commands
make help

# Run all tests with verbose output and stdout enabled
make test-verbose

# Run all tests with basic output
make test

# Run tests quickly (minimal output)
make test-quick

# Run only admin tests
make test-admin

# Run only JSON-RPC tests
make test-jsonrpc

# Run only WebSocket tests
make test-websocket

# Run the test script
make run-script

# Clean up test artifacts
make clean
```

### Option 3: Direct Pytest Commands

For custom testing scenarios:

```bash
# Run all tests with full output
python -m pytest tests/ -v --tb=short --capture=no --color=yes --durations=10 -m merobox

# Run specific test file
python -m pytest tests/admin/test_admin_integration.py -v --capture=no -m merobox

# Run specific test method
python -m pytest tests/admin/test_admin_integration.py::TestAdminClientIntegration::test_health_check -v --capture=no -m merobox

# Run tests with coverage
python -m pytest tests/ --cov=calimero --cov-report=html -m merobox
```

## Test Output

When running tests with `--capture=no`, you'll see:

1. **Merobox Workflow Output**: 
   - Node initialization and startup
   - Port assignments and configuration
   - Workflow execution steps
   - Node readiness status

2. **Test Execution Details**:
   - Individual test results (PASS/FAIL/SKIP)
   - Test durations
   - Performance metrics
   - Any stdout/stderr output from tests

3. **Test Summary**:
   - Total tests run
   - Passed/Failed/Skipped counts
   - Slowest test durations
   - Overall execution time

## Test Categories

### Admin Tests (`tests/admin/`)
- Client creation and configuration
- Health checks and system status
- Peer information and counts
- Certificate retrieval
- Context management
- Application management
- Blob operations
- Identity management

### JSON-RPC Tests (`tests/core/test_json_rpc_integration.py`)
- Client creation and configuration
- Method execution
- Error handling
- Performance testing
- Real-world usage scenarios

### WebSocket Tests (`tests/core/test_ws_subscriptions_client.py`)
- Client creation and configuration
- Connection management
- Subscription handling
- Message processing
- Error handling
- Lifecycle management

## Understanding Test Results

### Passed Tests ✅
Tests that successfully validate functionality with the merobox node.

### Skipped Tests ⏭️
Tests that are gracefully skipped when functionality isn't available:
- Context creation when applications lack valid WASM files
- Blob operations when endpoints aren't supported
- JSON-RPC methods when specific functionality isn't available
- Identity and storage operations when contexts can't be created

### Failed Tests ❌
Tests that encounter unexpected errors (should be rare after fixes).

## Troubleshooting

### Common Issues

1. **Virtual Environment Not Activated**:
   ```bash
   source .venv/bin/activate.fish  # or activate for bash
   ```

2. **Dependencies Not Installed**:
   ```bash
   pip install -e .
   ```

3. **Merobox Node Issues**:
   - Check Docker is running
   - Ensure ports 2428 and 2528 are available
   - Check merobox installation

4. **Test Timeouts**:
   - Tests may take longer on first run due to Docker image downloads
   - Subsequent runs reuse existing containers

### Cleaning Up

To clean up test artifacts and start fresh:

```bash
make clean
```

This removes:
- Pytest cache
- Test data directories
- Python cache files

## Performance Notes

- **First Run**: ~2-3 minutes (includes Docker image download and node setup)
- **Subsequent Runs**: ~1-2 minutes (reuses existing containers)
- **Individual Tests**: Most tests complete in under 10 seconds
- **Setup/Teardown**: Node startup takes ~20-30 seconds per test session

## Continuous Integration

The test suite is designed to work in CI environments:
- Uses session-scoped fixtures for efficiency
- Gracefully handles missing functionality
- Provides clear pass/fail/skip results
- Includes performance metrics for monitoring
