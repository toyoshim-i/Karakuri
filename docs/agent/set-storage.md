---
id: set-storage
title: Set Storage, Sandbox, and Library Management
summary: Structure and separation between operator library sets, session sandbox files, and revision history.
---

# Set Storage, Sandbox, and Library Management

Karakuri maintains strict separation between operator libraries, session sandbox snapshots, and automated revision history in `karakuri-store`.

## Storage Tiers

1. **Operator Library (`<store>/`)**:
   - Contains curated `.kset` files managed directly by human operators.
   - Read-only to MCP agents via `read_set` and `list_sets`.
   - Never modified directly by automated agent tools.
2. **Session Sandbox (`<store>/sandbox/`)**:
   - Destination for agent snapshots created via `save_set`.
   - Files are timestamped and immutable (`YYYYMMDD-HHMMSS-mmm_<id>.kset`).
   - Saving a set never overwrites previous snapshots.
   - Sandbox files do not appear in `list_sets` or `--load-set` unless moved into the library by the operator.
3. **Revision History (`<store>/history/`)**:
   - Every procedure successfully compiled by the engine is archived by content hash.
   - Accessible via `walk_history` for rolling back nodes to previous working revisions.
