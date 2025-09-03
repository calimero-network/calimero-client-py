#!/bin/bash

# Build script for calimero-client-py-bindings using Python 3.13 virtual environment
# This script activates the venv and builds the Python bindings

set -e

echo "🚀 Building calimero-client-py-bindings with Python 3.13 venv..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Cargo.toml not found. Please run this script from the bindings directory."
    exit 1
fi

# Check if virtual environment exists
if [ ! -d "venv" ]; then
    echo "❌ Virtual environment not found!"
    echo ""
    echo "📦 To create the virtual environment, run:"
    echo "  python3.13 -m venv venv"
    echo "  source venv/bin/activate.fish  # or source venv/bin/activate for bash"
    echo "  pip install maturin"
    echo ""
    exit 1
fi

# Activate virtual environment
echo "🐍 Activating Python 3.13 virtual environment..."
if [[ "$SHELL" == *"fish"* ]]; then
    source venv/bin/activate.fish
else
    source venv/bin/activate
fioul

# Verify Python version
echo "📋 Python version: $(python --version)"
echo "📋 Maturin version: $(maturin --version)"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
rm -rf target/ dist/ build/ *.egg-info/

# Build the bindings
echo "🔨 Building with maturin..."
maturin build --release

echo "✅ Build completed successfully!"
echo "📦 Wheel files created in dist/ directory"

# Optional: install in development mode
if [ "$1" = "--install" ]; then
    echo "📥 Installing in development mode..."
    maturin develop --release
    echo "✅ Installed successfully!"
fi
