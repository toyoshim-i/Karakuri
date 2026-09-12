# `karakuri-operation`

Unified command vocabulary and action grammar for the Karakuri visual performance system.

---

## 1. Role in Architecture

`karakuri-operation` sits at **Layer 1 (Pure Foundations)** in the Karakuri workspace:

- **Single Source of Truth**: Defines every operation ([`Operation`]) Karakuri can execute.
- **Surface Neutrality**: GUI panels (clicks/drags), keyboard shortcuts, MIDI mappings, and AI pairing tools (MCP) translate into identical `Operation` variants.
- **Zero Dependencies**: Pure leaf crate with zero workspace dependencies and zero heavy external libraries. Any component across the system can depend on `karakuri-operation` without pulling in UI, audio, MIDI, or GPU code.
- **Gate & Permission Checking**: Provides [`gate::Gate`] and [`gate::Class`] to audit whether an operation is allowed given current console focus and deck lifecycle state.

---

## 2. Key Invariants

1. **Explicit Targeting**:
   - Decks, nodes, and parameters are always addressed explicitly (e.g. `deck: u8`).
   - Operations never imply "the focused deck" or "the selected item", preserving deterministic replay and surface independence.
2. **Absolute States Over Toggles/Cycles**:
   - Continuous operations set absolute values rather than nudging increments (e.g. `SetGain`, `SetOpacity`).
   - Multistate controls specify destination states explicitly (e.g. `SetResidency { residency: Residency::Live }`, `SetBlendMode { blend: BlendMode::Over }`) rather than toggling or cycling.
3. **Pure Grammar**:
   - Operations carry only what the action acts upon, never how it was reached (no keys, MIDI CC numbers, or screen coordinates).
   - Operations perform no mutations themselves; they are decoded and applied by downstream crates (`karakuri-operation-record`, `karakuri-engine`, `karakuri-mcp`).

---

## 3. Testing & Verification

Run tests to ensure synchronization with the system manual:

```sh
cargo test -p karakuri-operation
```
