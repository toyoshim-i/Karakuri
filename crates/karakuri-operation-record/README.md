# `karakuri-operation-record`

Operation-to-Record translation layer: converts high-level, surface-agnostic [`karakuri_operation::Operation`] commands into content-addressed session journal [`karakuri_store::record::Record`] entries.

---

## 1. Role in Architecture

`karakuri-operation-record` sits at **Layer 2 (Domain Extensions & Storage)** in the Karakuri workspace:

- **Inputs**:
  - An [`Operation`](../karakuri-operation/README.md) describing user intent (e.g. `SetGain { deck: 1, gain: 0.8 }`, `SetExposure { exposure: 1.2 }`).
  - A [`Current`](src/lib.rs) context snapshot capturing existing engine parameters (e.g., current tone mapper, mask angle/softness) needed to produce complete, self-contained record representations.
- **Output**: A [`Written`] classification:
  - `Written::Records(Vec<Record>)`: Emits one or more journal records to append to the session history.
  - `Written::Silent(Silent)`: Acknowledges operations that intentionally emit no journal record (e.g. transient UI focus, queries).
  - `Written::Owed(Owed)`: Identifies unimplemented or future operations with explanatory diagnostics.
- **Consumer**: Engine mutation handlers, headless replay runners, and MCP mutation pipelines.

---

## 2. Key Invariants

1. **Contextual Completeness**:
   - Operations express surface intent (e.g. changing exposure), while records must be replayable in isolation without depending on external state.
   - The translation layer combines incoming operations with [`Current`] state to construct complete record structs (such as [`Record::Look`]).
2. **Deterministic Session History**:
   - All interactive controls (GUI, CLI, MIDI, MCP) produce identical journal records when given the same operations and initial state.
   - Recording an operation stream and replaying it offscreen yields byte-identical state progressions.
3. **No Engine/GPU Dependency**:
   - `karakuri-operation-record` depends only on `karakuri-operation` and `karakuri-store`. It contains no graphics or GPU (`wgpu`) dependencies.

---

## 3. Testing & Verification

Run unit tests:

```sh
cargo test -p karakuri-operation-record
```
