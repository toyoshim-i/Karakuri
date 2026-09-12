# `karakuri-console`

Immediate-mode graphical performance console (egui) for real-time visual operators and VJs.

---

## 1. Role in Architecture

`karakuri-console` sits at **Layer 5 (Applications & Presentation)** in the Karakuri workspace:

- **VJ Performance Console**: Implements an immediate-mode UI interface designed for live stage operations with low-latency responsiveness and tactile controls.
- **Pure Presentation & Interaction Model**: Decoupled from GPU rendering pipelines; consumes view models and produces strongly typed commands from `karakuri-operation`.
- **Layout & Bay Arrangement**: Coordinates dynamic split arrangements, collapsible bays, and persistent console geometry via `karakuri-layout`.

---

## 2. Key Console Bays & Subsystems

1. **Program & Staging Bays (`view/program.rs`, `view/staging.rs`)**:
   - Displays live program output previews and staging preview cells.
   - Houses shader compilation diagnostics, compilation cards, and hot-swap controls.
2. **Mixer & Master Bays (`view/mixer.rs`, `view/master.rs`)**:
   - Interactive crossfaders, parameter strips, and deck gain/opacity controls.
   - Tonemapping operators and compositing blend mode selectors.
3. **Sequencer & Transport (`view/sequencer.rs`, `view/transport.rs`)**:
   - Beat grid visualizer, tempo readout, and quantization step triggers.
   - Latency offset nudging and external synchronization indicators.
4. **Library Bay (`view/library.rs`)**:
   - Catalogs stored Sets, presets, and authored `.kset` files.
   - Supports text filtering, favorite tags, and drag-and-drop loading into deck slots.
5. **Inspector & Wiring (`view/inspector.rs`)**:
   - Parameter curves, modulation routing, and node input edge wiring.
6. **Shared Widget Primitives (`view/widgets/`)**:
   - `fader.rs`: Mini faders, gradient fills, value displays, and focus rings.
   - `fold_grip.rs`: Collapsible panel handles and pane dividers.
   - `head.rs`: Capsule headers, deck indicators, and operator permission pills.
   - `pills.rs`: Status badges, toggle capsules, and MCP authority chips.

---

## 3. Testing & Verification

Run the test suite:

```sh
cargo test -p karakuri-console
```
