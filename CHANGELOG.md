# Changelog

## Unreleased

- fix(build): declare the version once, in `Cargo.toml`. It was written in three files — `Cargo.toml`, `pyproject.toml` and `calimero/__init__.py` — kept in step by a test that could only report drift after it happened, and twice it did not stop the drift landing: 0.6.20 missed `pyproject.toml`, so the publish gate saw no change and shipped nothing while every check passed, and 0.6.19 shipped a `__version__` reading `0.3.0`. 0.6.31 missed it again, in the other direction. `pyproject.toml` now declares `dynamic = ["version"]` so maturin reads `Cargo.toml`, `calimero/__init__.py` reads the installed distribution's metadata, and the publish gate compares `Cargo.toml` against the previous commit's. The test now guards the shape — that no second copy exists — instead of comparing values

## 0.6.31

- build(deps)!: bump the core dependency from `808dbcdbc` to `c2ea737af`, 16 commits of drift. Picks up calimero-network/core#3598, which names the id `POST /admin-api/namespaces/:namespace_id/join` returns `namespaceId` rather than `groupId` — a namespace is a root group internally, and the endpoint had been sharing its response DTO with `POST /admin-api/groups/join`, so it leaked that noun. No source here changes: `join_namespace` forwards the response DTO generically. What changes is the dict a caller gets back, and what a build made *before* this can do with the response — it deserializes into the old struct, so it rejects a current node outright with `missing field \`groupId\``, the same version-skew shape as the `governanceOp` failure 0.6.29 fixed. `POST /admin-api/groups/join` is untouched and still returns `groupId`

## 0.6.30

- fix(groups)!: `add_group_members` accepts an ACCOUNT in `identity`, and no longer takes the interpreter down on one. It parsed with `.parse::<PublicKey>().expect("invalid identity")`, so an account - the only id a caller gets from `list_group_members`, and what every other verb on that resource takes - panicked the extension module before any request was sent. A nested-membership app therefore could not add somebody it had just read back: no path through this binding could express the call. `identity` now parses as `MemberIdentity` (calimero-network/core#3573), which reads either encoding, and a malformed one raises `ValueError` naming the offending string instead of aborting
- build(deps): bump the core dependency from `ee344bb7a` to `808dbcdbc`, which is where `MemberIdentity` lands
- docs(groups): `remove_group_members` takes accounts, not public keys. It has parsed `Vec<AccountId>` since 0.6.26; the reference and the guide still said public-key strings

## 0.6.29

- build(deps)!: bump the core dependency from `da787d2b9` to `ee344bb7a`, 41 commits of drift. The lock pinned an exact revision despite `branch = "master"` in `Cargo.toml`, so the sdist compiled against rc.21 no matter how far master moved — the reason a released merobox refuses a join response from a current node with `missing field \`governanceOp\``. Picks up `#[serde(default)]` on that field (calimero-network/core#3530), so this build tolerates its removal in core#3485
- refactor(api)!: drop the `requester` keyword from `delete_context`, `delete_namespace`, `delete_group` and `set_member_auto_follow`. core#3492 deleted the field from every admin request DTO — the node resolves the acting identity from the authenticated session — so the binding was parsing a public key it had nowhere to send. Callers passing it now get a `TypeError` naming the kwarg rather than having it silently dropped. Positional calls are unaffected
- refactor(account)!: delete the `create_account` binding. core#3470 removed the endpoint behind it ("enrolment is implicit, so it created nothing"); the binding had no route to call

## 0.6.26

- build(deps)!: bump the core dependency from `fc4babde2` to current master. The lock was the reason four bindings could not use the typed path: at the old revision `RemoveGroupMembersApiRequest::members` was still `Vec<PublicKey>` and `AccountId` did not exist in `calimero-primitives` at all, so the structs genuinely could not express the ids the node uses. **This also picks up the removal of `selfIdentity` from `list_group_members`** — a caller looking for itself in a member list now asks `get_namespace_identity` and matches its `account` against `members[].identity`, one id space on both sides
- refactor(groups): retire the 0.6.23 pass-throughs now that the types can express what the node sends. `list_group_members`, `remove_group_members`, `get_namespace_identity` and `join_namespace` go back through `calimero-client`. `remove_group_members` parses into `Vec<AccountId>`, so a malformed id fails with the caller's traceback rather than as a rejection from the node — the thing the original bs58 parsing got wrong was the id *space*, not the parsing
- `create_namespace` still builds its body by hand. That shim is about released **nodes** still requiring `upgradePolicy`, which no dependency bump changes

## 0.6.25

- refactor(metadata): route the six metadata bindings through `calimero-client` instead of hand-rolling their HTTP. `get_group_metadata`, `set_group_metadata`, `get_member_metadata`, `set_member_metadata`, `get_context_metadata` and `set_context_metadata` each re-declared the endpoint path and verb that the client crate already owns, so a route change there would have left these six silently pointing at the old URL — and each repeated twenty lines of response-conversion the shared helper exists for. They were already using the typed request/response structs; what was duplicated was the routing. Net 113 lines removed, no behaviour change

## 0.6.24

- fix(namespaces): keep sending `upgradePolicy` on `create_namespace`. The concept was deleted server-side, but only on master — every RELEASED node still requires the field and rejects a request without it, so 0.6.22 and 0.6.23 can create a namespace on no released node at all (merobox's whole suite went red on exactly this). A node that has dropped the field ignores the extra key, since the request type does not deny unknown fields, so sending it is the one shape that works against both. The body is built by hand because the typed request no longer has the field. Remove once no supported release predates the removal

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
