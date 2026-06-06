# Changelog

## 0.6.18

- feat(client): add `abort_migration(namespace_id)` binding — logically abort an in-flight namespace migration (flips the pending target back to the pre-migration app id and drops the marker, cascading to descendants; idempotent). Wraps `POST admin-api/groups/{namespace_id}/migration/abort` (depends on calimero-network/core#2681)
- fix(client): drop the removed `Coordinated` upgrade-policy variant — rejected upstream (deadline was inert; migrate converges only under LazyOnAccess) — to track core master

## 0.6.17

- feat(client): add `get_cascade_status(namespace_id)` binding — per-descendant cascade migration status across a namespace subtree (depends on calimero-network/core#2524)

## 0.6.16

- feat(client): pick up `app_key` field on `SignedGroupOpenInvitation` (depends on calimero-network/core#2507)
