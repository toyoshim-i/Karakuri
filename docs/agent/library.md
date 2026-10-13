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

When called with no parameters, `list_library` provides an overview catalog showing distribution statistics by layer, topology, and common categories.

## Reading Source Code

- **`read_procedure`**: Read the full `.kir` source code of any procedure by `name` (e.g. `{"name": "bloom"}`) without needing to mount it onto an active deck slot. Can still read active slot nodes via `{"slot": 0, "layer": "L1"}`.
- **`read_set`**: Read the complete structure and full `.kir` source code of all constituent nodes for any Set by `id` (e.g. `{"id": "beat_cloud"}`).
