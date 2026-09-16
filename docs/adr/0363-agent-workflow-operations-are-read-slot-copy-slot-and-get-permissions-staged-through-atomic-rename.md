---
id: 0363
title: Agent workflow operations are read_slot, copy_slot, and get_permissions, staged through atomic rename
status: accepted
date: 2026-09-16
supersedes: []
superseded_by: []
principles: [0083, 0085, 0094]
tags: [mcp, scratch, tools, safety, m7]
---

# Agent workflow operations are read_slot, copy_slot, and get_permissions, staged through atomic rename

## Context

Collaborative visual generation during a performance requires an agent to quickly inspect what is
playing, verify what actions are permissible, and replicate or modify entire slot setups.

Previous MCP tooling was limited to individual low-level operations (`operate`, `write_procedure`).
This posed three operational hazards during live execution:
1. **Blind attempts and opaque errors**: An agent had no means of discovering which bays or slots
   were locked without attempting mutations and failing.
2. **Multi-roundtrip assembly**: Duplicating or inspecting a slot required querying each layer and
   reading multiple files separately over JSON-RPC.
3. **File watcher torn reads**: In `karakuri-environment`, `scratch::place` and `scratch::materialise`
   performed direct `std::fs::write` to `.kir` files in the watch directory. When an agent or
   operator edited a procedure, the background file watcher often fired mid-write, presenting the
   compiler worker with a truncated or half-written shader. Furthermore, multi-file edits (such as
   an L1 geometry updating its declared interface alongside an L4 renderer) triggered two distinct
   watcher events, frequently generating transient contract-mismatch compilation errors.

## Decision

**1. High-level agent workflow tools in `karakuri-mcp`:**
- `get_permissions`: Returns the open/closed status of all bay classes, per-slot MCP policies (`Auto`,
  `On`, `Off`), mixer live-contribution status (`in_mix`), and whether each slot is currently writable.
- `read_slot`: Returns the procedural definitions for all layers (`L1`..`L5`) active in a slot in a
  single round trip.
- `copy_slot`: Copies all active procedures from a source slot to a destination slot with optional
  `layer` filtering, protected by the destination slot's access policy.

**2. Enforce atomic writes through temporary files (`scratch::write_atomic`):**
All disk mutations in the scratch workspace now write first to an adjacent temporary file (`.kir.tmp`)
and replace the target via `std::fs::rename`. Because `fs::rename` is atomic on POSIX filesystems,
the compiler worker and file watcher never observe a partial file or truncated AST.

In `copy_slot`, all destination files are written to `.tmp` paths before any target is replaced.
The replacement happens in immediate succession, eliminating the window where mismatched layer
interfaces can trigger compile errors.

**3. Standard MCP error envelopes:**
Tool refusals (e.g. attempting to write to a protected slot or copying an unassigned slot) return
`Ok(Called::Answered(Err(reason)))`, serializing as standard JSON-RPC tool errors:
```json
{
  "result": {
    "isError": true,
    "content": [{"type": "text", "text": "deck B is protected by mcp policy: auto (currently in live mix)"}]
  }
}
```
This preserves the protocol invariant that an application-level rejection is a tool result rather than
a transport-level protocol fault ([P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).

## Alternatives rejected

- **In-process AST transfer without disk**:
  Bypassing the file system for MCP edits breaks the invariant that the scratch directory holds the
  truth of live edits ([P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)). The
  disk watcher must remain the single synchronization mechanism.
- **Transport JSON-RPC errors for write guards**:
  Returning a JSON-RPC error envelope (`{"error": {"code": ...}}`) causes standard MCP client
  libraries to raise unhandled exceptions rather than feeding the rejection back to the model as a
  re-tryable tool output.
