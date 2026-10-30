# Calimero Client Python Library

The Python client for [Calimero 1.0](https://calimero-network.github.io/core/),
backed by core's Rust `calimero-client` via [PyO3](https://pyo3.rs/). Verify a
node's attestation, sign in with a passkey or API key, deploy apps, call and
view their methods, read their events, administer organizations and teams, and
run the operator's Ops API — all from synchronous Python.

Up to 0.9.0 this package also shipped a client for v1's peer-to-peer node
(`create_connection`, `create_client`, v1's `Client`, `sign_warrant`, ...).
Calimero 1.0 deleted that node and this package deleted its client (1.0.0-rc.1
is a breaking release); the [changelog](CHANGELOG.md) says what replaces each
name.

## Documentation

Full documentation is published at
**<https://calimero-network.github.io/calimero-client-py/>** — installation,
quickstart, concepts, and the complete Python API reference. The source lives
under [`docs/`](docs/).

## Installation

```bash
pip install --pre calimero-client-py      # 1.0.0rc1, until 1.0.0 is out
```

PyPI carries a prebuilt `abi3` wheel (any CPython 3.9+) for Linux x86_64 and
aarch64 (manylinux2014), macOS arm64 and x86_64, and Windows x86_64, so nothing
compiles. Elsewhere pip builds the source distribution, which needs a Rust
toolchain on `PATH` ([rustup](https://rustup.rs/)). See the
[installation guide](https://calimero-network.github.io/calimero-client-py/get-started/installation/)
for details.

## Quick start

`Client` talks to one node of a Calimero 1.0 cluster (`merod`). Every method
blocks until the node answers and returns a plain Python value. Attestation, the sealed
channel and passkey handling run in core's Rust `calimero-client`, so Python
applies the same trust check as every other client. `Client` is the preferred
name; `CloudClient` is the same class, kept for existing imports.

```python
from calimero_client_py import Client, CloudError

client = Client("https://eu1.cloud.calimero.network")  # allow_mock=True for dev clusters
client.use_api_key("cal_...")
r = client.call(app_id, "set", {"key": "a", "value": "1"}, idempotency_key="k1")
print(client.view(app_id, "get", {"key": "a"})["result"])
for e in client.events(app_id, start=0, limit=10, timeout=5):
    print(e["kind"], e["data"])
```

Organizations, teams and invites:

```python
org = client.create_org("Acme")
# What grants access is approved with the acting account's passkey (412 first).
token = client.invite(org, "member", passkey, teams=[], ttl_secs=86400, uses=1)
other.redeem_invite(token)
team = client.create_team(org, "eng", join="invite")      # or join="everyone", parent=...
client.set_team_member(org, team, account_id, passkey, role="readOnly")
app = client.create_app("eng-kv", client.upload_bundle(wasm),
                        {"org": org, "access": {"kind": "team", "team": team}})
client.update_team(org, team, name="engineering", metadata={"colour": "teal"})
client.delete_team(org, team)                               # takes its sub-teams and apps
```

| Area | Methods |
| --- | --- |
| Session | `sign_up`, `sign_in`, `refresh`, `sign_out`, `sign_out_everywhere`, `use_session`, `use_api_key`, `session`, `me`, `usage` |
| Devices (passkeys) | `add_passkey(new, approver)`, `remove_passkey(credential_id, passkey)`, `label_passkey` |
| API keys | `create_api_key({label, apps, methods, readOnly, allowConfirm, ttlSecs}, passkey)`, `list_api_keys`, `revoke_api_key` |
| Aliases | `set_alias`, `alias`, `aliases`, `delete_alias` |
| Packages | `upload_package`, `install_package`, `packages`, `package`, `uninstall_package` |
| Apps | `upload_bundle`, `create_app`, `create_app_from_package`, `list_apps`, `get_app`, `delete_app`, `set_access`, `set_app_metadata`, `app_storage`, `app_abi`, `upgrade_app`, `transfer_app`, `verify_app_cluster` |
| Approvals | `approve_release`, `approve_outbound` (the owner's passkey) |
| Upgrade runs | `upgrade_org`, `upgrade_run`, `retry_upgrade_run` |
| Presence | `publish_presence`, `presence`, `presence_stream` |
| Calls | `call`, `call_confirmed`, `view`, `events` |
| Blobs | `upload_blob`, `download_blob`, `list_blobs`, `blob_info`, `delete_blob` |
| Organizations | `create_org`, `get_org`, `delete_org`, `set_org_metadata`, `set_member`, `set_member_metadata`, `remove_member`, `set_capabilities`, `invite`, `redeem_invite` |
| Teams | `create_team`, `get_team`, `update_team`, `delete_team`, `set_team_member`, `remove_team_member` |
| Ops API (ops token via `use_api_key`, decisions signed after `use_operator_key`) | `ops_status`, `ops_metrics`, `ops_register_node`, `ops_remove_node`, `ops_set_replicas`, `ops_backup`, `ops_quotas`, `ops_set_quotas`, `ops_approve_release`, `ops_approve_outbound`, `ops_upgrade_app`, `ops_upgrade_org`, `ops_packages`, `ops_upload_package`, `ops_install_package`, `ops_uninstall_package` |
| Anything else | `get`, `get_text`, `post`, `put`, `delete` on any API path; `post_confirmed`, `delete_confirmed` where the node asks for a passkey (`412`); `post_operator_signed`, `delete_operator_signed` where it asks for the operator's signature (`428`); `attestation` |

`SoftPasskey` is an in-memory passkey for scripts and tests against development
clusters (`client.sign_up(SoftPasskey(), "alice")`, `client.call_confirmed(...)`).
Errors raise `CloudError(message, status, code, details)`; `LoginRequired`
(`401` the client's own refresh did not answer) and `WrongNode` (`421
WRONG_NODE` with no other node to move to, given in `nodes`) are its subclasses.
Operators authenticate with the node's ops token, and sign their decisions
(nodes, quotas, replica moves, and the releases, outbound approvals and
upgrades governance gives them) with the operator's Ed25519 key, whose verify
half the nodes were started with (`merod --operator-key
$(python -c 'from calimero_client_py import Client; print(Client.operator_public_key("<seed hex>"))')`):

```python
ops = Client(node_url, tee_profiles=["..."])
ops.use_api_key(ops_token)
ops.use_operator_key(operator_seed_hex)   # answers 428 OPERATOR_SIGNATURE_REQUIRED
print(ops.ops_status()["nodes"])
ops.ops_register_node(4, "https://node4.example/", zone="eu1-b")
ops.ops_set_replicas(app_id, [1, 2, 4])   # sent on to the app group's leader
print(ops.ops_backup(), "snapshots written")
```

`tests/test_cloud_v2.py`, `tests/test_cloud_v2_api.py` and
`tests/test_cloud_v2_ops.py` run against a local 3-node cluster of core 1.0's
`merod` (set `MEROD_BINARY` and `KV_STORE_WASM`, or keep a `core` checkout
beside this one).

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

The Rust core depends on `calimero-client` and `calimero-identity` from core's
`claude/proofs-at-apply` branch (until core #4791 merges; then master) on
[`github.com/calimero-network/core`](https://github.com/calimero-network/core),
so the first build fetches and compiles those too (a network connection is
required for the initial build).

## Support

- Documentation: <https://calimero-network.github.io/calimero-client-py/>
- Issues: <https://github.com/calimero-network/calimero-client-py/issues>
- Contact: team@calimero.network

## License

See [LICENSE](LICENSE).
