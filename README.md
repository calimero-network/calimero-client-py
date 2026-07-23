# Calimero Client Python Library

A Python client SDK for [Calimero Network](https://calimero-network.github.io/core/),
backed by a native Rust core via [PyO3](https://pyo3.rs/). Connect to a node,
manage applications and contexts, move blobs, administer namespaces and groups,
and call your app's methods over JSON-RPC — all from synchronous Python.

## Documentation

Full documentation is published at
**<https://calimero-network.github.io/calimero-client-py/>** — installation,
quickstart, task guides, and the complete Python API reference. The source lives
under [`docs/`](docs/).

## Installation

```bash
pip install calimero-client-py
```

The package is published as a **source distribution** (no prebuilt wheels), so
installing compiles the native Rust extension on your machine. You need a Rust
toolchain on `PATH` — install one with [rustup](https://rustup.rs/) if you don't
already have `cargo`. Python 3.9+ is required. See the
[installation guide](https://calimero-network.github.io/calimero-client-py/get-started/installation/)
for details.

## Quick start

Every `Client` method is **synchronous**: it blocks until the node responds and
returns a plain Python value (a `dict`, `list`, `bytes`, or scalar). There are no
coroutines and nothing to `await`.

```python
from calimero_client_py import create_connection, create_client

connection = create_connection(
    api_url="http://localhost:2428",
    node_name="local-dev",  # stable name; used for the token cache
)
client = create_client(connection)

print("connected to", client.get_api_url())

contexts = client.list_contexts()
print(f"found {len(contexts)} contexts")

# call a method on the app running in a context (args is a JSON string)
result = client.execute_function(
    context_id="<context-id>",
    method="set",
    args='{"key": "greeting", "value": "hello"}',
)
print(result)
```

See the [quickstart](https://calimero-network.github.io/calimero-client-py/get-started/quickstart/)
and [guides](https://calimero-network.github.io/calimero-client-py/guides/connecting/)
for connecting, authentication, contexts, blobs, and namespaces & groups.

## CLI

Installing the package puts a small `calimero-client-py` command on your `PATH`
for quick checks:

```bash
calimero-client-py --version
calimero-client-py --base-url http://localhost:2428 list-contexts
```

See the [CLI reference](https://calimero-network.github.io/calimero-client-py/reference/cli/).

## Development

Build from source with [maturin](https://www.maturin.rs/):

```bash
pip install maturin
maturin develop --release        # compile the extension into the active venv
python -c "import calimero_client_py; print('ok', calimero_client_py.VERSION)"
```

Run the tests:

```bash
cargo test          # Rust unit tests
pytest              # Python tests
```

The Rust core depends on Calimero crates pulled from
[`github.com/calimero-network/core`](https://github.com/calimero-network/core),
so the first build fetches and compiles those too (a network connection is
required for the initial build).

## Support

- Documentation: <https://calimero-network.github.io/calimero-client-py/>
- Issues: <https://github.com/calimero-network/calimero-client-py/issues>
- Contact: team@calimero.network

## License

See [LICENSE](LICENSE).
