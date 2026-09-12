# `karakuri-engine`

Real-time GPU execution runtime (`wgpu 30`): Decks, Sets, HotSwap, Governor, and multi-sink frame presentation.

---

## 1. Role in Architecture

`karakuri-engine` sits at **Layer 3 (Core GPU Runtime)** in the Karakuri workspace:

- **Hardware Layer**: Direct owner of the `wgpu::Device`, `wgpu::Queue`, compute pipelines, and render passes.
- **Upstream Dependencies**: `karakuri-ir`, `karakuri-codegen`, `karakuri-signal`, and `karakuri-store`.
- **Downstream Consumers**: Applications (`karakuri`, `karakuri-cli`) and coordination services (`karakuri-environment`).
- **Core Entities**:
  - [`Deck`](src/deck.rs): Manages up to four simultaneous performance slots with residency, blend modes, transitions, and audio sync.
  - [`Set`](src/set.rs): An instantiated visual preset containing active GPU buffers and shader pipelines for `L1`–`L5` stages.
  - [`HotSwap`](src/swap.rs): Non-blocking shader and procedure swaps executed strictly on frame boundaries.
  - [`Governor`](src/governor.rs): Dynamic frame budget enforcement preventing GPU hangs and frame drops.
  - [`Frame`](src/frame.rs): Frame progression coordinator dispatching output to multiple [`Sink`](src/frame.rs) presentation targets.

---

## 2. Key Invariants

1. **Non-Blocking Render Thread**:
   - Pipelines and large GPU buffers are created on background worker threads or during initialization.
   - The render loop never invokes blocking compile or disk I/O routines.
2. **Boundary-Synchronized HotSwap**:
   - Procedure changes and Set re-aiming take effect exclusively at the start of a frame, ensuring glitch-free visual transitions.
3. **Multi-Sink Independence**:
   - Presentation targets (GUI texture, offscreen buffer, physical displays) operate independently. A slow or failing sink does not block simulation progression on the deck.

---

## 3. Testing & Verification

Run unit tests:

```sh
cargo test -p karakuri-engine --lib
```
