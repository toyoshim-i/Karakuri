# Architectural Refactoring & Modernization Roadmap

This document records the architectural roadmap for **Karakuri**, focusing on the active refactoring plan: **Phase 5 (P34–P37)**.

---

## 1. Completed Phases (P1–P33)

All prior refactoring phases are complete, verified with full workspace tests, and documented across crate-level `README.md` files:

- **Phases 1 & 2 (P1–P20)**: Transient render graph DAG, WGSL AST & pass fusion, machine-readable AI diagnostics, zero-allocation stream replay, and keymap convergence.
- **Phase 3 (P21–P26)**: Subsystem decomposition, monolith extraction, layer type centralization, full crate-level `README.md` documentation, and Google Style comment standardization across all 16 crates.
- **Phase 4 (P30–P33)**: Console UI componentization (`card`, `chip`, `track`, `field`), bay monolith decomposition (`mixer/`, `transport/`, `library/`, `inspector/`), unified `ControlDescriptor` registry in `karakuri-console::control`, and `Readout` event dispatch decoupling into `readout/` (`costs`, `hud`, `dispatch`, `mod`).

| Initiative | Target Subsystem | Actionable Deliverable | Status |
|---|---|---|:---:|
| **P21** | Type Conversions | Centralized layer conversions in `karakuri-environment::meta` as single source of truth | **COMPLETED** |
| **P22** | `karakuri-cli` / GUI | Extracted headless runtime controller; modularized `karakuri-cli` | **COMPLETED** |
| **P23** | `karakuri-environment` | Decomposed monoliths (`setfile`, `mix`), resolved couplings, isolated test suites | **COMPLETED** |
| **P24** | `karakuri` (GUI) | Decomposed `engine_bridge.rs` into `bridge/` (`sinks`, `engine`, `filesystem`, `handlers`) | **COMPLETED** |
| **P25** | `karakuri-mcp` | Extracted test suites to `tests/`; decomposed `lib.rs` into `protocol`, `server`, `spelled`, `tools/` | **COMPLETED** |
| **P26** | `karakuri-console` | Extracted 3 bays (`sequencer`, `staging`, `master`), `widgets/`, and `layout.rs` | **COMPLETED** |
| **P30** | `karakuri-console::view::widgets` | Componentized reusable widgets: `card.rs`, `chip.rs`, `track.rs`, `field.rs` | **COMPLETED** |
| **P31** | `karakuri-console::view` | Decomposed bay monoliths into subdirectories (`mixer/`, `transport/`, `library/`, `inspector/`) | **COMPLETED** |
| **P32** | `karakuri-console::control` | Implemented `ControlDescriptor` registry linking UI probes, hotkeys, and operations | **COMPLETED** |
| **P33** | `karakuri::readout` | Decomposed `readout.rs` into `costs.rs`, `hud.rs`, `dispatch.rs`, and `mod.rs` | **COMPLETED** |

---

## 2. Phase 5: Architecture Documentation Hierarchy, App Event Loop & Control-Aware Tooltips (P34–P37)

### Core Structural Opportunities

#### Opportunity 1: Hierarchical Architecture Documentation
- **Phenomenon**: `docs/architecture.md` is a 500-line monolithic document written early in the project. While `docs/principles/` and `docs/adr/` provide micro-level rules and historical decisions, there is no mid-level subsystem architectural documentation explaining how major crates interact. Recent major architectural refactorings (modular bays, widgets, control registry, bridge, readout) are not yet reflected.
- **Solution**: Establish a clean 3-level documentation hierarchy under `docs/architecture/` (`README.md`, `console.md`, `engine.md`, `runtime.md`, `operations.md`), keeping `docs/architecture.md` as the high-level portal.

#### Opportunity 2: Monolithic GUI Event Loop (`karakuri/src/app.rs`)
- **Phenomenon**: `crates/karakuri/src/app.rs` is **5,067 lines**, combining the `winit` application event loop (`ApplicationHandler`), operation dispatch and routing, audio device capture and measurement, MIDI mapping, and window rendering.
- **Solution**: Modularize into `crates/karakuri/src/app/` (`handler.rs`, `operations.rs`, `audio_midi.rs`, `mod.rs`), mirroring the successful CLI modularization of P22.

#### Opportunity 3: Control-Aware Dynamic Tooltips (`karakuri-console::hover`)
- **Phenomenon**: `hover.rs` paints tooltips parsed from `docs/manual/console.html`, but currently renders plain text without displaying keyboard shortcut badges. The newly introduced `ControlDescriptor` registry in `karakuri-console::control` provides hotkeys and operation titles that are not yet displayed in the hover layer.
- **Solution**: Connect `hover.rs` to `ControlDescriptor`, dynamically rendering shortcut badges (e.g. `[Space]`, `[K]`, `[G]`) inside tooltip popups.

#### Opportunity 4: Engine Set Execution Monolith (`karakuri-engine::set.rs`)
- **Phenomenon**: `crates/karakuri-engine/src/set.rs` is **3,705 lines**, managing compilation, scheduling, layer rendering, and buffer lifecycle in a single file.
- **Solution**: Modularize `set.rs` into `set/` (`compiler.rs`, `schedule.rs`, `layers.rs`, `mod.rs`).

---

### Phase 5 Initiatives

#### P34. Hierarchical Architecture Documentation (`docs/architecture/`)
Establish hierarchical architecture documentation:
1. `docs/architecture/README.md`: Subsystem architectural map, dataflow, and navigation index.
2. `docs/architecture/console.md`: UI components, bay subdirectories, `ControlDescriptor` registry, hover layer, and focus graph.
3. `docs/architecture/engine.md`: Rendering engine pipeline, Deck/Set execution graph, residency model, HotSwap, and GPU buffers.
4. `docs/architecture/runtime.md`: Application lifecycle, Bridge, Readout event dispatch, Environment services, and Store.
5. `docs/architecture/operations.md`: Single vocabulary, authority model, operator permission gates, MCP tools, and session streams.
6. `docs/architecture.md`: Updated top-level portal pointing to subsystem documents.

#### P35. Modularize GUI App & Event Loop (`karakuri::app`)
Decompose `crates/karakuri/src/app.rs` (5,067 lines) into `crates/karakuri/src/app/`:
1. `mod.rs`: `App` state struct container and lifecycle initialization.
2. `handler.rs`: `impl ApplicationHandler for App` (`window_event`, `about_to_wait`, `resumed`, `suspended`).
3. `operations.rs`: Operation execution and routing (`routed`, `answered`, `played`, `overlaid`, `restored`, `aimed_set`).
4. `audio_midi.rs`: Audio capture device management, MIDI learn mapping, audio measurement, tracking, and tap tempo.

#### P36. Control-Aware Dynamic Tooltips (`karakuri-console::hover`)
Integrate `ControlDescriptor` registry with the hover layer:
1. Link `TIPS` / probe resolution to `ControlDescriptor` via `ControlId`.
2. Dynamically format tooltips with visual hotkey badges (e.g. `"[Space] Wipe next deck"`, `"[K] Keep"`).
3. Verify tooltip formatting and probe alignment with automated tests.

#### P37. Modularize Engine Set Execution (`karakuri-engine::set`)
Decompose `crates/karakuri-engine/src/set.rs` (3,705 lines) into `crates/karakuri-engine/src/set/`:
1. `types.rs`: `Set`, `Layering`, and `Published` data structures.
2. `schedule.rs`: Execution ordering and dependency graph resolution.
3. `layers.rs`: Layer execution passes and buffer transitions.
4. `mod.rs`: Core coordinator and re-exports.

---

## 3. Execution Matrix (Phase 5)

| Initiative | Target Subsystem | Actionable Deliverable | Readiness |
|---|---|---|:---:|
| **P34** | `docs/architecture/` | Establish hierarchical architecture documentation (`console`, `engine`, `runtime`, `operations`) | **Ready to Execute** |
| **P35** | `karakuri::app` | Decompose `app.rs` into `handler.rs`, `operations.rs`, `audio_midi.rs`, `mod.rs` | **Ready to Execute** |
| **P36** | `karakuri-console::hover` | Integrate `ControlDescriptor` hotkey badges into tooltips | **Ready to Execute** |
| **P37** | `karakuri-engine::set` | Modularize `set.rs` into `schedule.rs`, `layers.rs`, `types.rs`, `mod.rs` | **Ready to Execute** |
