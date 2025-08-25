.PHONY: help test test-verbose test-quick clean format format-check setup-pre-commit ci-check build build-clean publish

help:  ## Show this help message
	@echo "Calimero Client Commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

test:  ## Run all tests with basic output
	venv/bin/python -m pytest tests/

test-verbose:  ## Run all tests with verbose output
	venv/bin/python -m pytest tests/ -v --tb=short --capture=no

test-quick:  ## Run tests quickly (minimal output)
	venv/bin/python -m pytest tests/ -q

format:  ## Format code with Black
	venv/bin/black calimero/ tests/ setup.py

format-check:  ## Check if code is formatted with Black
	venv/bin/black --check calimero/ tests/ setup.py

setup-pre-commit:  ## Install and setup pre-commit hooks
	venv/bin/pip install pre-commit
	venv/bin/pre-commit install

ci-check:  ## Run CI checks locally (formatting + tests)
	@echo "Running CI checks locally..."
	@echo "1. Checking code formatting..."
	@make format-check
	@echo "2. Running tests..."
	@make test
	@echo "✅ All CI checks passed!"

build:  ## Build the package distribution
	@echo "Building package distribution..."
	venv/bin/python -m build
	@echo "✅ Build completed! Check dist/ directory"

build-clean:  ## Clean build artifacts and rebuild
	@echo "Cleaning build artifacts..."
	rm -rf build/ dist/ *.egg-info/
	@echo "Building package distribution..."
	venv/bin/python -m build
	@echo "✅ Clean build completed! Check dist/ directory"

publish:  ## Publish to PyPI (requires TWINE_USERNAME and TWINE_PASSWORD)
	@echo "Publishing to PyPI..."
	@if [ -z "$$TWINE_USERNAME" ] || [ -z "$$TWINE_PASSWORD" ]; then \
		echo "❌ Error: TWINE_USERNAME and TWINE_PASSWORD environment variables must be set"; \
		exit 1; \
	fi
	venv/bin/twine upload dist/*
	@echo "✅ Package published to PyPI!"

clean:  ## Clean up test artifacts and cache
	rm -rf .pytest_cache/
	rm -rf data/
	find . -name "*.pyc" -delete
	find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
