# Standalone Build for calimero-client-py-bindings

This directory now contains a standalone version of the `calimero-client-py-bindings` that can be built independently without requiring the full Calimero workspace.

## Prerequisites

- Rust toolchain (1.70+)
- Python 3.8+
- maturin (will be installed automatically if missing)

## Quick Build

```bash
# Navigate to this directory
cd core/crates/client/python

# Build the bindings
./build.sh

# Or build and install in development mode
./build.sh --install
```

## Manual Build

```bash
# Install maturin if you haven't already
pip install maturin

# Build the wheel
maturin build --release

# Install in development mode
maturin develop --release
```

## What Changed

1. **Standalone Cargo.toml**: Created a new `Cargo.toml` that doesn't depend on workspace paths
2. **Published Dependencies**: The bindings now depend on `calimero-client` from crates.io instead of workspace paths
3. **Independent Build**: Can be built anywhere without cloning the full repository

## Publishing to crates.io

Before using this standalone build, you need to publish the `calimero-client` crate:

```bash
# From the core/crates/client directory
cargo publish
```

## Using in Python Projects

```python
# Install from the built wheel
pip install dist/calimero_client_py_bindings-*.whl

# Or install from source
pip install -e .
```

## Troubleshooting

- **Build fails**: Make sure `calimero-client` is published to crates.io
- **Import errors**: Verify the wheel was built for your Python version
- **Runtime errors**: Check that all dependencies are properly linked

## Benefits

✅ **Independent builds** - No need for full workspace  
✅ **Easier distribution** - Can be built anywhere  
✅ **CI/CD friendly** - Simpler build pipelines  
✅ **Version control** - Clear dependency management  
✅ **Faster builds** - Only builds what's needed
