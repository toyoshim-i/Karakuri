---
id: BAY_CLOSED
aliases: ["bay-closed"]
title: Functional bay is closed to MCP operations
summary: Requested operation targets a functional bay that is currently closed to MCP access.
---

# Functional bay is closed to MCP operations

## Behavior

The gate audit rejected the operation because the corresponding functional bay is closed to automated control. Each bay maintains a capsule header toggle governing MCP authority.

## Remediation

- Call `get_permissions` to inspect which bays are currently open (`mcp · on`) or closed (`mcp · off`).
- Inform the operator which specific bay capsule needs to be opened (e.g. *"Please open MixFaders in the Mixer bay"*).
