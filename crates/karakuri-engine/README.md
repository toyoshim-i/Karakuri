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
  - [`Governor`](src/governor.rs): Dynamic frame budget enforcement preventing GPU hangs and frame drops. The master chain's cost is reserved out of the budget ahead of every deck slot (`Governor::set_chain_ms`, `Report::chain_ms`).
  - [`Chain`](src/master.rs): The master chain, an ordered list of `kind L5` slots between the mix's write and the tone map. Installed on a [`Present`](src/present.rs); its clock and its price are handed over by [`compose`](src/frame.rs) once a frame.
  - [`ChainSwap`](src/chain_swap.rs): The master chain's worker, named `karakuri-chain`. Compiles a chain's slots and allocates its targets off the render thread, installs the newest finished build at a frame boundary, and frees the chain it displaced on the same thread.
  - [`Frame`](src/frame.rs): Frame progression coordinator dispatching output to multiple [`Sink`](src/frame.rs) presentation targets.

---

## 2. Key Invariants

1. **Non-Blocking Render Thread**:
   - Pipelines and large GPU buffers are created on background worker threads or during initialization.
   - The render loop never invokes blocking compile or disk I/O routines.
2. **Boundary-Synchronized HotSwap**:
   - Procedure changes and Set re-aiming take effect exclusively at the start of a frame, ensuring glitch-free visual transitions.
   - A master chain is the same: compiled on `karakuri-chain` and installed by `ChainSwap::begin_frame`. The chain that is running draws every frame until then. `mix::install_chain` is the synchronous path, for runs with no frame waiting on a clock.
3. **Multi-Sink Independence**:
   - Presentation targets (GUI texture, offscreen buffer, physical displays) operate independently. A slow or failing sink does not block simulation progression on the deck.
4. **One Frame, One Clock and One Price for the Master Chain**:
   - `compose` writes the chain's clock (`Present::set_chain_clock`) and charges the deck what the chain costs (`Deck::set_chain_ops_per_fragment`) on every frame. No host does either.
   - The clock is `Deck::chain_clock(steps)`: the session clock after this frame's `steps`, with `dt` the fixed simulation step. It comes from the frame's `tick` and never from a wall clock.
   - The price is `Deck::chain_ms()`: `estimate::chain_ms` over the chain's summed `ops_per_fragment` against the deck's current size, so a resize moves it with no second write.

---

## 3. Testing & Verification

Run unit tests:

```sh
cargo test -p karakuri-engine --lib
```
