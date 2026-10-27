# Calimero Client Python Library

A Python client SDK for [Calimero Cloud](https://calimero-network.github.io/core/)
(release-v2), backed by core's Rust `calimero-cloud-client` via
[PyO3](https://pyo3.rs/). Verify a node's attestation, sign in with a passkey or
API key, deploy apps, call and view their methods, read their events, and
administer organizations and teams — all from synchronous Python.

Up to 0.9.0 this package also shipped a client for v1's peer-to-peer node
(`create_connection`, `create_client`, `Client`, `sign_warrant`, ...). Release-v2
deleted that node and this package deleted its client; the
[changelog](CHANGELOG.md) says what replaces each name.

## Documentation

Full documentation is published at
**<https://calimero-network.github.io/calimero-client-py/>** — installation,
quickstart, concepts, and the complete Python API reference. The source lives
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

`CloudClient` talks to one node of a release-v2 cluster (`merod`). Every method
blocks until the node answers and returns a plain Python value. Attestation, the sealed
channel and passkey handling run in core's Rust `calimero-cloud-client`, so Python
applies the same trust check as every other client.

```python
from calimero_client_py import CloudClient, CloudError

client = CloudClient("https://eu1.cloud.calimero.network")  # allow_mock=True for dev clusters
client.use_api_key("cal_...")
r = client.call(app_id, "set", {"key": "a", "value": "1"}, idempotency_key="k1")
print(client.view(app_id, "get", {"key": "a"})["result"])
for e in client.events(app_id, start=0, limit=10, timeout=5):
    print(e["kind"], e["data"])
```

Organizations, teams and invites:

```python
org = client.create_org("Acme")
token = client.invite(org, "member", teams=[], ttl_secs=86400, uses=1)  # hand it over
other.redeem_invite(token)
team = client.create_team(org, "eng", join="invite")      # or join="everyone", parent=...
client.set_team_member(org, team, account_id, role="readOnly")
app = client.create_app("eng-kv", client.upload_bundle(wasm),
                        {"org": org, "access": {"kind": "team", "team": team}})
client.update_team(org, team, name="engineering", metadata={"colour": "teal"})
client.delete_team(org, team)                               # takes its sub-teams and apps
```

| Area | Methods |
| --- | --- |
| Session | `sign_up`, `sign_in`, `refresh`, `sign_out`, `use_session`, `use_api_key`, `me` |
| Devices (passkeys) | `add_passkey`, `remove_passkey` |
| API keys | `create_api_key({label, apps, methods, readOnly, allowConfirm, ttlSecs})`, `list_api_keys`, `revoke_api_key` |
| Apps | `upload_bundle`, `create_app`, `list_apps`, `get_app`, `set_access`, `upgrade_app` |
| Calls | `call`, `call_confirmed`, `view`, `events` |
| Blobs | `upload_blob`, `download_blob` |
| Organizations | `create_org`, `get_org`, `set_member`, `remove_member`, `set_capabilities`, `invite`, `redeem_invite` |
| Teams | `create_team`, `get_team`, `update_team`, `delete_team`, `set_team_member`, `remove_team_member` |
| Anything else | `get`, `post`, `delete` on any API path; `attestation` |

`SoftPasskey` is an in-memory passkey for scripts and tests against development
clusters (`client.sign_up(SoftPasskey(), "alice")`, `client.call_confirmed(...)`).
Errors raise `CloudError(message, status, code, details)`.
`tests/test_cloud_v2.py` and `tests/test_cloud_v2_api.py` run against a local 3-node
cluster built from core's `release-v2-parity` branch (set `MEROD_BINARY` and
`KV_STORE_WASM`, or keep a `core` checkout beside this one).

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

The Rust core depends on `calimero-cloud-client` and `calimero-identity` from
core's `release-v2-parity` branch on
[`github.com/calimero-network/core`](https://github.com/calimero-network/core),
so the first build fetches and compiles those too (a network connection is
required for the initial build).

## Support

- Documentation: <https://calimero-network.github.io/calimero-client-py/>
- Issues: <https://github.com/calimero-network/calimero-client-py/issues>
- Contact: team@calimero.network

## License

See [LICENSE](LICENSE).
