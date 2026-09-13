# Architectural Refactoring & Modernization Roadmap

This document records the architectural refactoring history and roadmap for **Karakuri**.

---

## 1. Completed Phases (P1–P41)

All prior refactoring phases are complete, verified with full workspace tests, and documented across crate-level `README.md` files:

- **Phases 1 & 2 (P1–P20)**: Transient render graph DAG, WGSL AST & pass fusion, machine-readable AI diagnostics, zero-allocation stream replay, and keymap convergence.
- **Phase 3 (P21–P26)**: Subsystem decomposition, monolith extraction, layer type centralization, full crate-level `README.md` documentation, and Google Style comment standardization across all 16 crates.
- **Phase 4 (P30–P33)**: Console UI componentization (`card`, `chip`, `track`, `field`), bay monolith decomposition (`mixer/`, `transport/`, `library/`, `inspector/`), unified `ControlDescriptor` registry in `karakuri-console::control`, and `Readout` event dispatch decoupling into `readout/` (`costs`, `hud`, `dispatch`, `mod`).
- **Phase 5 (P34–P37)**: 3-tier hierarchical architecture documentation (`docs/architecture/`), GUI application event loop modularization (`karakuri::app`), control-aware dynamic tooltip shortcut badges in `karakuri-console::hover`, and engine Set execution decomposition (`karakuri-engine::set`).
- **Phase 6 (P38–P41)**: Console glyph componentization (`glyph.rs`), popup card row text deduplication (`card_row_text`), root view modularization (`draw.rs`, `regions.rs`), and GUI milestone M5 exit condition verification.

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
| **P34** | `docs/architecture/` | Established 3-tier hierarchical architecture documentation linked with crate READMEs | **COMPLETED** |
| **P35** | `karakuri::app` | Decomposed `app.rs` into `handler.rs`, `operations.rs`, `audio_midi.rs`, and `mod.rs` | **COMPLETED** |
| **P36** | `karakuri-console::hover` | Integrated `ControlDescriptor` keyboard shortcut badges into dynamic tooltips | **COMPLETED** |
| **P37** | `karakuri-engine::set` | Decomposed `set.rs` into `schedule.rs`, `layers.rs`, `types.rs`, and `mod.rs` | **COMPLETED** |
| **P38** | `karakuri-console::view::widgets::glyph` | Componentize `chevron_down`, `arrow_mark`, `CHEVRON_W`, `CHEVRON_H`; deduplicate convex polygon glyphs across 6 bay files | **COMPLETED** |
| **P39** | `karakuri-console::view::widgets::card` | Standardize popup card row text layout using `card_row_text` across `audio_in`, `arrangement`, and `wiring` | **COMPLETED** |
| **P40** | `karakuri-console::view` | Decompose `view/mod.rs` (3,001 lines) by extracting `draw.rs` (`draw`, `cursor`) and `regions.rs` (`Region`, `REGIONS`, `Kind`) | **COMPLETED** |
| **P41** | `docs/roadmap.md` & Console Verification | Verify M5 exit conditions (`grep -c 'rt plan">panel' docs/manual/operations.html` == 0) and full workspace test suite | **COMPLETED** |

---

## 2. Planned Phase: Phase 7 — GUI Interaction & Event Dispatch Modernization

Based on architectural analysis of the GUI event handling pipeline (`karakuri-console::input`, `karakuri::readout::dispatch`, `karakuri::app::handler`), Phase 7 modernizes event routing, eliminates hit-testing duplication, unifies modal overlay states (including new M5.16 Master Chain Add Chooser and Sequencer Lane Chooser cards), and abstracts text input handling:

| Initiative | Target Subsystem | Actionable Deliverable | Status |
|---|---|---|:---:|
| **P42** | `karakuri::app` / Text Input | Abstract inline naming modes (`arrangement.naming` & `view.naming_set`) into a unified `TextInputSession` handler | **PENDING** |
| **P43** | `karakuri-console::view` | Unify popup card and modal chooser open/close states into a type-safe `ModalOverlay` enum to guarantee Rule 2 mutual exclusion | **PENDING** |
| **P44** | `karakuri-console::hover` | Connect `ModalOverlay` to the hover layer to suppress tooltips on background controls beneath open cards and choosers | **PENDING** |
| **P45** | `karakuri::readout::dispatch` | Leverage `ControlId` in `input::claim` to eliminate double hit-testing and decompose 1,200-line pointer dispatch into per-bay handlers | **PENDING** |
| **P46** | `karakuri::app::handler` | Decouple post-event side-effects (I/O, set saving, projector routing) out of `handler.rs` into `operations.rs` | **PENDING** |

---

## 3. Planned Phase: Phase 8 — Core Subsystem Monolith Decomposition

Targets the largest remaining production monoliths across the repository to bring all files comfortably below 2,000 lines without breaking cross-crate invariants:

| Initiative | Target Subsystem | Actionable Deliverable | Status |
|---|---|---|:---:|
| **P47** | `karakuri-ir::check` (4,061 lines) | Decomposed into `check/` submodules (`contracts.rs`, `coverage.rs`, `context.rs`, `eval.rs`, `mod.rs`) | **COMPLETED** |
| **P48** | `karakuri::bridge::handlers` (3,896 lines) | Decompose console-to-engine state updates into per-bay modules (`mixer.rs`, `transport.rs`, `inspector.rs`, `sequencer.rs`, `master.rs`, `mod.rs`) | **PENDING** |
| **P49** | `karakuri-store::record` (3,222 lines) | Decompose ndjson schema & serialization into `record/` submodules (`types.rs`, `variants.rs`, `serde.rs`, `helpers.rs`, `mod.rs`) | **PENDING** |
| **P50** | `karakuri-cli::live` (3,056 lines) | Decompose interactive runtime controller into `live/` submodules (`interactive.rs`, `demo.rs`, `audio.rs`, `mod.rs`) | **PENDING** |
| **P51** | `karakuri-console::focus` (3,664 lines) | Decompose 2D spatial focus navigation and deduplicate card/chooser traversals into `focus/` submodules (`model.rs`, `card.rs`, `chooser.rs`, `ladder.rs`, `mod.rs`) | **PENDING** |

---

## 4. Secondary Monolith Candidates (Phase 9 Backlog)

Future candidates for modularization after Phase 8:
- `karakuri-operation::lib.rs` (2,587 lines) & `karakuri-operation-record::lib.rs` (2,148 lines): Vocabulary and serialized record definitions
- `karakuri::bridge::filesystem` (2,540 lines): Store synchronization, directory watcher event loop, and snapshot pipelines
- `karakuri::app::handler` (2,533 lines): Console event post-processing and side-effect coordination
- `karakuri-console::hover` (2,516 lines): Tooltip manual citation parsing and dynamic probe resolution
- `karakuri::readout::dispatch` (2,319 lines): Pointer translation and bay event dispatch
- `karakuri-mcp::spelled` (2,141 lines): Schema definitions and MCP protocol stringification
- `karakuri-midi::map` (2,038 lines): MIDI device map file parser, encoder, and hardware bindings

---

## 5. Future Initiatives

Future refactoring and architectural enhancements will be recorded here as new requirements emerge.
