# Changelog

## 0.6.23

- fix(groups)!: decode member ids as the node writes them, in `list_group_members`, `remove_group_members`, `get_namespace_identity`, and `join_namespace`. These deserialized through structs compiled from one version of the server, which decided for this package what a member id looks like. `GroupMemberApiEntry` declares it a `PublicKey`, so ids were parsed as bs58 — and a server that names members by account sends 64 hex, so the *entire listing* failed to deserialize and surfaced as a transport-shaped error rather than version skew. `remove_group_members` had the mirror problem outbound and parsed with `.expect("invalid public key")`, taking the interpreter down on a typo with no traceback into the caller's own code. The two identity calls dropped newly added ids silently, serde discarding unknown fields, so a caller could not see one until this package was rebuilt and released. All four now pass the JSON through: the node validates, and its error names the id space it wants
- **Breaking-ish:** those four return plain `dict`s shaped exactly as the node's JSON. Field names are unchanged (the structs were `rename_all = "camelCase"` mirrors), and fields previously dropped as unknown are now present, so callers see a superset. What is gone is the local `ValueError` on a malformed member id — the node's 400 surfaces instead

## 0.6.20

- feat(client): add the five account-identity bindings — `create_account(namespace_id)`, `get_namespace_account(namespace_id)`, `pair_device_init(namespace_id, account_root_key, account_nonce)`, `pair_device_complete(namespace_id, device_id, kem_public_key, sign_public_key, statement, confirmation_code)`, and `revoke_device(namespace_id, device_id)`. The Rust client already wrapped all five endpoints (meroctl's `account` subcommands drive them); only the Python bindings were missing, which pushed callers like merobox into hand-rolled `requests` calls against `admin-api/` — bypassing the token cache, the error mapping, and everything else this layer exists to provide
- fix: compile against current core master, which had drifted from the pinned commit — `ConnectionInfo::api_url`/`node_name` became accessors, `JwtToken` gained a `Drop` impl (so its fields are cloned rather than moved out), `ClientError` gained an `Http { status, message }` variant (surfaced as `error_type: "Http<status>"` so a caller can classify without parsing prose), and `UpgradeGroupApiRequest` gained `force_code_only` (passed `false`, the refuse-without-ABI default)

## 0.6.19

- feat(client): add `resync_context(context_id, force=False)` binding — recover a stranded context by discarding local DAG heads and adopting a peer's full-state snapshot. Wraps `POST admin-api/contexts/{context_id}/resync` (depends on calimero-network/core#2768)
- feat(client): add `get_migration_status(namespace_id)` binding — pinned-cohort migration rollup with per-member `state` and the `all_migrated` flag; observability only (depends on calimero-network/core#2768)
- feat(client): add `list_application_versions(application_id)` binding — every locally-retained bytecode version `{version, blob_id, size, package}`; `blob_id` doubles as the `app_key` accepted by `create_namespace`
- feat(client): add optional `app_key` to `create_namespace` — hex-encoded blob id pinning the namespace to a specific installed bytecode version
- feat(client)!: drop `migrate_method` from `upgrade_group` — core now resolves whether/what to migrate from the apps' embedded ABIs (the field was removed from `UpgradeGroupApiRequest` upstream)
- fix: sync the stale `__version__`/CLI `--version` (was `0.3.0`) to the real package version

## 0.6.18

- feat(client): add `abort_migration(namespace_id)` binding — logically abort an in-flight namespace migration (flips the pending target back to the pre-migration app id and drops the marker, cascading to descendants; idempotent). Wraps `POST admin-api/groups/{namespace_id}/migration/abort` (depends on calimero-network/core#2681)
- fix(client): drop the removed `Coordinated` upgrade-policy variant — rejected upstream (deadline was inert; migrate converges only under LazyOnAccess) — to track core master

## 0.6.17

- feat(client): add `get_cascade_status(namespace_id)` binding — per-descendant cascade migration status across a namespace subtree (depends on calimero-network/core#2524)

## 0.6.16

- feat(client): pick up `app_key` field on `SignedGroupOpenInvitation` (depends on calimero-network/core#2507)
