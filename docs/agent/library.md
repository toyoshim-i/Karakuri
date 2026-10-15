---
id: library
title: Library & Preset Exploration
summary: Discovering, filtering, and reading procedural shaders and sets across built-in presets and user stores.
---

# Library & Preset Exploration

Karakuri provides a comprehensive library containing both built-in reference examples (`presets`) and operator-authored files (`store`).

## Structure

1. **Presets (`examples/`)**:
   - 260+ standalone `.kir` procedures across layers L1 through L5.
   - 23 complete `.kset` configurations demonstrating multi-layer composition, wiring, and cameras.
   - Verified end-to-end by test suites and available in both native and Web environments.
2. **Store (`.karakuri/`)**:
   - Saved operator Sets (`sets/*.kbset`), procedures (`procedures/*.kir`), and content-addressed artifacts (`artifacts/*.kir`).

## Discovery & Multi-Dimensional Filtering

Use `list_library` to search and explore available procedures and sets:

- **Layer Filter (`layer`)**: Restrict results to a specific layer (`L1`, `L2`, `L3`, `L4`, `Field`, `L5`, or `Set`).
- **Topology Filter (`topology`)**: For geometry (`L1`), filter by primitive type (`points`, `lines`, `triangles`).
- **Scope Filter (`scope`)**: Choose between built-in presets (`presets`), saved store items (`store`), or everything (`all`).
- **Text Search (`query`)**: Matches keywords across file names, identifiers, and leading comment documentation (e.g. `"audio"`, `"particle"`, `"fractal"`, `"bloom"`).
- **Pagination (`offset`, `limit`)**: Page deterministically through large result sets to inspect every item in the library.

## Permissions & Privacy Gate

Library access is governed by the operator's Library bay MCP gate (`LibraryGate`):

- **`preset` (Default)**: Grants access exclusively to public reference examples (`presets`). All user store items (`.karakuri/`) are withheld from listings, and requests specifying `scope: "store"` are refused with an explanation directing the operator to open the gate if access is intended.
- **`all`**: Operator has authorized full library access. Both presets and private store sets and procedures can be discovered and read.
- **`off`**: The operator has closed the Library bay. All library listing and procedure/set reading calls are refused.

When an operation is refused due to the gate, the response includes actionable instructions naming the Library bay header pill.

## Reading Source Code

- **`read_procedure`**: Read the full `.kir` source code of any procedure by `name` (e.g. `{"name": "bloom"}`) without needing to mount it onto an active deck slot. Can still read active slot nodes via `{"slot": 0, "layer": "L1"}`.
- **`read_set`**: Read the complete structure and full `.kir` source code of all constituent nodes for any Set by `id` (e.g. `{"id": "beat_cloud"}`). Stored Sets require the Library gate set to `all`.

## Content Addresses & Master Chain Effects

Every procedure has a canonical SHA-256 content address (`sha256:<64 hex digits>`):
- **`list_library`**: Exposes the `hash` field on each `.kir` item so clients know the procedure's exact content address.
- **`check_procedure`**: When static checks succeed, returns `hash` in the diagnostic report alongside `success: true`.
- **`operate("Add an effect to the master chain")`**: Accepts either a library procedure name (e.g. `"bloom"`, `"glitch_slice"`, `"zoom_blur"`) or a full SHA-256 content address (`"sha256:..."`).

