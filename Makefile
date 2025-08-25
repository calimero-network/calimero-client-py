.PHONY: help test test-verbose test-quick test-admin test-jsonrpc test-websocket clean format format-check

help:  ## Show this help message
	@echo "Calimero Client Test Commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

test:  ## Run all tests with basic output
	python3 -m pytest tests/ -m merobox

test-verbose:  ## Run all tests with verbose output and stdout enabled
	python3 -m pytest tests/ -v --tb=short --capture=no --color=yes --durations=10 -m merobox

test-quick:  ## Run tests quickly (minimal output)
	python3 -m pytest tests/ -q -m merobox

test-admin:  ## Run only admin tests
	python3 -m pytest tests/admin/ -v --tb=short --capture=no --color=yes -m merobox

test-jsonrpc:  ## Run only JSON-RPC tests
	python3 -m pytest tests/core/test_json_rpc_integration.py -v --tb=short --capture=no --color=yes -m merobox

test-websocket:  ## Run only WebSocket tests
	python3 -m pytest tests/core/test_ws_subscriptions_client.py -v --tb=short --capture=no --color=yes -m merobox

run-script:  ## Run the test script with full output
	python3 scripts/run_tests.py

format:  ## Format code with Black
	venv/bin/black calimero/ tests/ setup.py

format-check:  ## Check if code is formatted with Black
	venv/bin/black --check calimero/ tests/ setup.py

clean:  ## Clean up test artifacts
	rm -rf .pytest_cache/
	rm -rf data/
	find . -name "*.pyc" -delete
	find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
