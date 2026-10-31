# Changelog

## Unreleased

- **feat!: answer core's step-ups for credentials, memberships and the fleet**
  (core #4791, proofs at apply). What grants access now takes the acting
  account's `passkey` and answers the node's `412 CONFIRM_REQUIRED`:
  `add_passkey(new, approver)` (approved by a passkey the account already
  has, never the new one), `remove_passkey(credential_id, passkey)`,
  `create_api_key(options, passkey)`, `set_member(org, account, role,
  passkey)`, `remove_member(org, account, passkey)`, `set_capabilities(org,
  capabilities, passkey, ...)`, `invite(org, role, passkey, ...)`,
  `set_team_member(org, team, account, passkey, ...)` and
  `remove_team_member(org, team, account, passkey)`. The Ops API's decisions
  (`ops_register_node`, `ops_remove_node`, `ops_set_replicas`,
  `ops_set_quotas`, and `ops_approve_release`, `ops_approve_outbound`,
  `ops_upgrade_app`, `ops_upgrade_org`) answer `428
  OPERATOR_SIGNATURE_REQUIRED` with the key `use_operator_key(seed)` sets;
  `post_operator_signed` / `delete_operator_signed` do it for any path, and
  `Client.operator_public_key(seed)` gives the `--operator-key` the nodes
  take. Builds against core's master, which has #4791.
- **feat: `delete(path, body=None)`** sends `body` as JSON on a DELETE, for
  a caller that makes its own proof (`{"operatorProof": ...}` on
  `ops/nodes/{id}`, `{"confirm": ...}` on a passkey, member or team row);
  `delete(path)` is unchanged.
- **feat: connect to a cluster by name and fail over between its nodes.**
  `Client.connect(cluster, *, tee_profiles=None, min_release=None,
  allow_mock=False, app_hash=None, allow_debug_profiles=False, directory=None,
  zone=None)` reads the cluster's nodes from the Calimero Cloud node directory
  (`GET {directory}/{cluster}`, default
  `https://manager.cloud.calimero.network/api/v2/directory`), drops unusable
  entries, and orders them `zone` first, then weighted random; it raises
  `CloudError` (code `DIRECTORY`) for an unknown cluster (`404`), a directory it
  cannot reach, or a listing with no usable node. Every node is still attested
  under the caller's policy. The listing is fetched again once its
  `refreshAfterSeconds` pass and after a failover (the node in use stays first;
  nodes no longer listed are dropped). New `node_urls`: every node, in order.
- **feat: fail over to the next node when one is unavailable.** With more than
  one node (`nodes`, or `Client.connect`), a request whose node cannot be
  connected to goes to the next; a read (`view`, `get`, ...) or a `call` whose
  node keeps answering `503` or times out does too, a call with the same
  idempotency key (the client now makes the key itself when none is given, so
  every node gets the same one; the cluster answers it once). Each node is tried
  once per request, then the last error is raised. `sign_in`, `sign_up` and
  refreshes are never sent again for this. One node: unchanged.

- **feat: survive a lost issuing node and `421 WRONG_NODE`** (for core #4774's
  node-signed sessions). A `401 UNAUTHENTICATED` to a request made with a
  session is answered by one refresh, shared by the requests refused at the
  same time, and the request is sent again; a `421 WRONG_NODE` moves the client
  to the next of the new `nodes` argument (`Client(base_url, nodes=[...])`,
  `node_url` names the node in use) and sends it there once. A refresh
  answered `421` goes on to each next node with the same token, once per node;
  `sign_in` and `sign_up` are never sent again. New `CloudError` subclasses
  `LoginRequired` (`401`) and `WrongNode` (`421 WRONG_NODE`).

## 1.0.0-rc.1

The Python client for Calimero 1.0, and only for it. **Breaking**: every v1
name is gone (below). On PyPI this is `1.0.0rc1`, a pre-release: `pip install
--pre calimero-client-py`, or `==1.0.0rc1`.

- **feat!: the Calimero 1.0 client; the v1 client is removed.** Calimero 1.0
  replaced v1's peer-to-peer node with an attested cluster, and this package
  now binds only that: `Client` (also `CloudClient`), `SoftPasskey` and
  `CloudError`, over core 1.0's `calimero-client`. The v1 client, built on
  core v1's `calimero-client`,
  `calimero-server-primitives`, `calimero-primitives`,
  `calimero-context-config` and `calimero-account`, is deleted with those
  dependencies. What replaces each removed name:
  - `create_connection`, `ConnectionInfo` → `CloudClient(base_url, ...)`.
  - `create_client`, `Client` → `CloudClient`'s methods:
    `install_application`/`install_dev_application` → `upload_bundle`;
    `create_context` → `create_app`; `list_contexts`/`get_context` →
    `list_apps`/`get_app`; `execute_function` → `call` (writes) and `view`
    (reads); `update_context_application` → `upgrade_app`;
    `upload_blob`/`download_blob` → the same names, scoped to an app;
    namespaces → organizations (`create_org`, `get_org`, `set_member`,
    `remove_member`, `set_capabilities`, `invite`, `redeem_invite`); groups →
    teams (`create_team`, `get_team`, `update_team`, `delete_team`,
    `set_team_member`, `remove_team_member`); anything else → `get`, `post`,
    `delete` on the API path. Aliases, context identities, peer counts,
    syncing, device pairing, intent relays, group upgrades and migrations have
    no equivalent: they were properties of the peer-to-peer node.
  - `AuthMode`, `JwtToken`, `get_token_cache_path`, `get_token_cache_dir`,
    `save_token_cache` (merobox's `~/.merobox/auth_cache` tokens) →
    `sign_up`/`sign_in` with a passkey, `use_session`, `refresh`, or
    `use_api_key`. Sessions are not cached on disk.
  - `TeePolicy` (MRTD/RTMR pins, `from_releases`) → `CloudClient`'s
    `tee_profiles`, `min_release`, `allow_debug_profiles`, `allow_mock` and
    `app_hash`; the policy is a signed mero-tee release's profiles.
  - `sealed=True` → always on: every `CloudClient` request is sealed to the
    attested node.
  - `sign_warrant`, `sign_request` → none: v1's relayed execution and request
    proofs; a cluster authenticates the caller's session or API key.
  - `ClientError` → `CloudError(message, status, code, details)`.
  - The `calimero-client-py` command (`calimero.cli`) → none: it wrapped
    v1's `list-contexts`; `meroctl` is Calimero 1.0's command line.
  - `abi-codegen`'s generated client took a v1 `Client`, a `context_id` and an
    `executor_public_key` and called `execute_function`; it now takes a
    `CloudClient` and an `app_id` and calls `call`, letting `CloudError`
    through unchanged.
- **feat: `Client` is the client's name.** `Client` and `CloudClient` are
  the same class (`Client is CloudClient`); `Client` is the preferred name,
  `CloudClient` stays for code that imports it (mdma does). It is not v1's
  `Client`: it takes a node URL and a trust policy, as `CloudClient` does.
- **feat: the rest of core 1.0's client.** New methods, each over the
  `calimero-client` call of the same name: `session`, `usage`,
  `label_passkey`; aliases (`set_alias`, `alias`, `aliases`, `delete_alias`);
  packages (`upload_package`, `install_package`, `packages`, `package`,
  `uninstall_package`); apps (`delete_app`, `set_app_metadata`,
  `app_storage`, `app_abi`, `verify_app_cluster`, `presence_stream`);
  organizations (`delete_org`, `set_org_metadata`, `set_member_metadata`);
  `get_text`. `upgrade_org` takes `migrate=`. `call` and `view` results
  carry the node's signed `receipt`, which `verify_app_cluster`'s keys check.
- **feat: the Ops API.** With the node's ops token as the API key:
  `ops_status`, `ops_metrics`, `ops_register_node`, `ops_remove_node`,
  `ops_set_replicas`, `ops_backup`, `ops_quotas`, `ops_set_quotas`,
  `ops_approve_release`, `ops_approve_outbound`, `ops_upgrade_app`,
  `ops_upgrade_org`, `ops_packages`, `ops_upload_package`,
  `ops_install_package`, `ops_uninstall_package`. `ops_set_replicas` and
  `ops_remove_node` follow a `421 NOT_LEADER` to the leader the node names,
  attested under the client's own policy and sent the same key, so they can be
  sent to any node (mdma's own redirect keeps working; it no longer fires).
- **build: wheels for Linux, macOS and Windows.** PyPI used to get an sdist
  and a Windows wheel only. Each release now carries one `abi3` wheel (any
  CPython 3.9+) for manylinux2014 x86_64 and aarch64, macOS arm64 and x86_64,
  and Windows x86_64, built with maturin-action; the publish job refuses to
  publish if any of them is missing. `[tool.maturin]`'s `sdist = true` /
  `wheel = false`, which described source-only publishing, is removed.
- **chore(deps): core 1.0.0-rc.1.** `calimero-client` and
  `calimero-identity` are pinned to core's `1.0.0-rc.1` tag (before:
  `calimero-cloud-client` on the `release-v2-parity` branch; the crate was
  renamed, its API is unchanged). CI's drift check now builds against core
  `master`.

## 0.9.0

- **feat(warrant)!: `sign_warrant` names the executor device key and pins the
  release.** The signature is now `sign_warrant(context_id, executor,
  executor_key, release_bytecode_id, release_version, method, args, nonce,
  device_secret, credential, valid_for=300)`. A warrant is spendable only by
  the relay's device `executor_key` (#127), and only while the relay runs the
  release `release_bytecode_id`, whose semver `release_version` (at most 256
  bytes) is signed alongside it (#133). All four come from the relay's
  `get_intent_relay`, whose `data` now also carries `releaseBytecodeId` and
  `releaseVersion`. The signed bytes change, so warrants from this version
  verify only on nodes with core#4517, and older warrants are refused there.
- **feat: `create_group_in_namespace(namespace_id, group_name=None,
  visibility=None)`.** `visibility` is `"open"` or `"restricted"`; the node
  creates an open subgroup when it is omitted.
- **chore(deps): built on core `master` 6811e566**, which carries core#4517.

## 0.8.1

- **fix(deps): build on core 0.11.0-rc.79 (316e3835), so no request waits
  forever.** The manifest still follows core `master`; this moves the lock,
  which is what a release ships, from 5bd3ab40 to the rc.79 tag commit. The
  change a Python caller notices is core#4455: every request is now bounded.
  30 s by default; 5 min for calls the node answers only after network work
  of its own (installing an application, creating, joining or syncing a
  context, joining or syncing a group, upgrades, namespace joins, and
  `execute_function`/JSON-RPC); 1 h for blob uploads and downloads; 10 s to
  connect. Before, a node that accepted a request and never
  answered held the caller indefinitely. A timeout raises `RuntimeError` like
  any other transport failure; on a plain connection its message is
  `Client error: error sending request for url (...)` and does not say it timed
  out, on a sealed one it reads `the node did not answer within ...`. No
  binding changed.

## 0.8.0

- **fix!: a `TeePolicy` must pin the image, not just the MRTD.** On GCP the
  MRTD measures the platform's TD firmware, which every mero-tee image,
  profile and release shares; the image is in RTMR1–3. A policy pinning only
  the MRTD accepted a TD running anything. `TeePolicy` now requires
  `allowed_rtmr1`–`3`, and raises `ValueError` without them.
- **feat: `TeePolicy.from_releases(releases, profile)`** builds the policy from
  node releases' `published-mrtds.json`: one profile, every release given (for
  rollouts spanning two), and only the TCB statuses they all accept.

## 0.7.3

- **feat: sealed requests to a TEE node.** `create_connection` takes `tee` (a
  new `TeePolicy`) and `sealed`. A node's quote proves what runs in its TD and
  nothing about who reads the traffic, and TLS in front of a node usually ends
  at a proxy outside it. With `sealed=True` every request, token refreshes
  included, is encrypted to the node's attested transport key, exactly as
  mero-js does, so only the TD reads it. Quotes are verified here, against
  Intel's collateral, under a policy that must pin the image's MRTD; mock
  quotes are never accepted. `transport_public_key` takes a key verified some
  other way. Combinations that would protect nothing are refused rather than
  ignored. Built on `calimero-client`'s `tee` feature (core#4105).
- **fix: intent arguments are sorted explicitly.** Quote verification brings in
  `dcap-qvl`, which turns on `serde_json`'s `preserve_order` for the whole
  build, and features unify. The arguments `sign_warrant` hashes and
  `perform_intent` sends were sorted only because nothing had turned that
  feature on; they would have followed the caller's key order instead. Both now
  sort keys at every depth (`canonical_args`), so a warrant still verifies
  whatever order the caller wrote them in.


## 0.7.2

Entries for 0.6.33 through 0.7.1 were not written. Those releases are described
by their commits; these three are recorded because each changes what a Python
caller can do.

- **feat: a connection can sign every request with a device key.** `ConnectionInfo`
  and `create_connection` take `device_credential`, `device_secret` and an
  optional `device_session`. A session is a bearer token, obtained once and
  presented many times; a request-carried proof commits to one method, one path
  and one body, so it is minted per call and cannot be lifted onto another
  request. That is what lets a Python caller drive a relay it has no account on
  — nothing was ever issued to it. `device_session` decides which key
  `device_secret` is: with it a session key, without it the device key itself.
  Only the session link names a node, so a two-link chain is replayable at any
  node serving delegated access until it expires. Half a pair is refused rather
  than ignored, and so is a session that will not decode — silently falling back
  to the two-link chain is a downgrade to no node binding that the caller never
  asked for. The node honours any of this only when it runs with
  `--delegated-access`; one that does not answers 403, distinct from the 401 a
  bad proof gets (#118)
- **fix(deps): point the core crates back at master.** #116 merged carrying a
  temporary branch pin. All five core crates must move together or cargo
  resolves two copies of `calimero-primitives` and the errors name *types*
  rather than revisions; and the lock records a resolved revision, so a manifest
  saying `master` with a lock naming a branch keeps building the branch — which
  breaks every build, the publish job included, the moment that branch is
  deleted (#117)
- **feat(request): bind `RequestSig` so a Python caller can sign one request**
  (#116)


## 0.6.32

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
