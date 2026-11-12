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

> **Before committing, titling a pull request, or releasing**, read [RELEASING.md](RELEASING.md): the PR title is the release commit, and the rules are shared by every Calimero repository that ships releases.

## Documentation

Full documentation is published at
**<https://calimero-network.github.io/calimero-client-py/>** — installation,
quickstart, concepts, and the complete Python API reference. The source lives
under [`docs/`](docs/).

## Installation

```bash
pip install --pre calimero-client-py      # 1.0.0rc2, until 1.0.0 is out
```

PyPI carries a prebuilt `abi3` wheel (any CPython 3.9+) for Linux x86_64 and
aarch64 (manylinux2014), macOS arm64 and x86_64, and Windows x86_64, so nothing
compiles. Elsewhere pip builds the source distribution, which needs a Rust
toolchain on `PATH` ([rustup](https://rustup.rs/)). See the
[installation guide](https://calimero-network.github.io/calimero-client-py/get-started/installation/)
for details.

## The generated client

The routes are generated from core's OpenAPI document (`docs/public/openapi.json`
in core, vendored as [`openapi/openapi.json`](openapi/openapi.json) with the core
commit in [`openapi/source.json`](openapi/source.json)):

- `calimero_client_py/openapi/` is [openapi-python-client](https://github.com/openapi-generators/openapi-python-client)
  0.26.2's output: one module per operation, typed models.
- `calimero_client_py/facade.py` puts one method per operation on `Client`, grouped
  by tag: `client.auth`, `client.apps`, `client.orgs`, `client.packages`,
  `client.ops`, `client.deployments`, `client.consent`, `client.public`.

```python
client.use_passkey(passkey)                    # answers each 412 CONFIRM_REQUIRED
org = client.orgs.create(name="Acme").org_id
token = client.orgs.invite_create(org=org, role="member").token
r = client.apps.call(app=app_id, method="set", args={"key": "a", "value": "1"})
client.orgs.team_delete(org=org, team=team, confirm=other_passkey)  # per call
```

Arguments are keyword-only (the path's, the query's and the JSON body's
fields; a raw-byte body is `body=`), each method returns the generated model of
the answer and raises `CloudError` on a refusal. A `412 CONFIRM_REQUIRED` is
answered with the client's passkey (`use_passkey`) and a `428
OPERATOR_SIGNATURE_REQUIRED` with its operator key (`use_operator_key`);
`confirm=` and `operator_proof=` override them per call. `client.api()` runs the
generated modules directly on the same sealed transport.

Hand-written, because a generated client cannot be made of it: the attested
sealed channel (the native `SealedClient`, which the generated code's httpx
transport sends through), the step-ups (`calimero_client_py/sealed.py`), the
passkey ceremonies (`sign_up`, `sign_in`, `add_passkey`, account recovery) and
the event streams (`events`, `presence_stream`). Earlier releases had a method
per route; 1.0.0rc4 removed them (CHANGELOG.md maps each to its facade method).

Regenerate after a change to core's API:

```bash
pip install openapi-python-client==0.26.2 black==26.10.1
python scripts/refresh_openapi.py              # core's master (or --ref <ref>, --core ../core)
python scripts/generate_openapi_client.py      # CI runs it with --check
```

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
r = client.apps.call(app=app_id, method="set", args={"key": "a", "value": "1"},
                     idempotency_key="k1")
print(client.apps.view(app=app_id, method="get", args={"key": "a"}).to_dict()["result"])
for e in client.events(app_id, start=0, limit=10, timeout=5):
    print(e["kind"], e["data"])
```

A cluster by name: `Client.connect` asks the Calimero Cloud node directory for
its nodes, the ones in `zone` first, and fails over between them. The directory
is not trusted: every node is attested under your policy before anything is
sent to it.

```python
client = Client.connect("eu-shared-1", tee_profiles=["..."], zone="europe-west4-a")
print(client.node_url, client.node_urls)  # the node in use, and the rest in order
```

Organizations, teams and invites:

```python
client.use_passkey(passkey)  # what grants access is approved with it (412 first)
org = client.orgs.create(name="Acme").org_id
token = client.orgs.invite_create(org=org, role="member", ttl_secs=86400, uses=1).token
other.orgs.invite_redeem(token=token)
team = client.orgs.team_create(org=org, name="eng", join="invite").team_id
client.orgs.team_set_member(org=org, team=team, account=account_id, role="readOnly")
bundle = client.packages.bundle_put(body=wasm).hash_
app = client.apps.create(name="eng-kv", bundle=bundle, org=org,
                         access={"kind": "team", "team": team}).app_id
client.orgs.team_update(org=org, team=team, name="engineering", metadata={"colour": "teal"})
client.orgs.team_delete(org=org, team=team)  # takes its sub-teams and apps
```

Every route is a method of its group: `client.auth`, `client.apps`,
`client.orgs`, `client.packages`, `client.ops`, `client.deployments`,
`client.consent`, `client.public` (`help(client.orgs)` lists them; see "The
generated client" above). `client.json.<group>` answers the node's JSON as it
came, for a node older than the document. Not routes, and so methods of
`Client` itself: `sign_up`, `sign_in`, `add_passkey`, `refresh`,
`use_session`, `use_api_key`, `session`, `use_passkey`, `use_operator_key`,
`use_operator_signer`; account recovery's `start_account_recovery(code,
passkey)` and `finish_account_recovery(passkey, account_id=None)`; an
organization's `start_recovery(org, seed)`; the streams `events` and
`presence_stream`; `verify_app_cluster`, `attestation`; the key helpers
(`api_key_public`, `normalize_recovery_code`, `recovery_code_key`,
`new_recovery_seed`, `recovery_public_key`, `sign_recovery`,
`operator_public_key`); and, for any path, `get`, `get_text`, `post`, `put`,
`delete`, `post_confirmed`, `delete_confirmed`, `post_operator_signed`,
`delete_operator_signed`. The method per route of 1.0.0rc3 and earlier is gone in 1.0.0rc4;
[CHANGELOG.md](CHANGELOG.md) maps each to its facade method.

`SoftPasskey` is an in-memory passkey for scripts and tests against development
clusters (`client.sign_up(SoftPasskey(), "alice")`, `client.apps.call(..., confirm=pk)`).
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
print(ops.json.ops.status()["nodes"])
ops.ops.register_node(id=4, addr="https://node4.example/", zone="eu1-b")
ops.ops.set_replicas(app=app_id, replicas=[1, 2, 4])  # sent on to the app group's leader
print(ops.ops.backup().written, "snapshots written")
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
