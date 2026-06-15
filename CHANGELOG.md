# Changelog

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
