# Architectural Refactoring & Modernization Roadmap

This document records the architectural refactoring history and roadmap for **Karakuri**.

---

## 1. Completed Phases (P1–P58)

All prior refactoring phases are complete, verified with full workspace tests, and documented across crate-level `README.md` files:

- **Phases 1 & 2 (P1–P20)**: Transient render graph DAG, WGSL AST & pass fusion, machine-readable AI diagnostics, zero-allocation stream replay, and keymap convergence.
- **Phase 3 (P21–P26)**: Subsystem decomposition, monolith extraction, layer type centralization, full crate-level `README.md` documentation, and Google Style comment standardization across all 16 crates.
- **Phase 4 (P30–P33)**: Console UI componentization (`card`, `chip`, `track`, `field`), bay monolith decomposition (`mixer/`, `transport/`, `library/`, `inspector/`), unified `ControlDescriptor` registry in `karakuri-console::control`, and `Readout` event dispatch decoupling into `readout/` (`costs`, `hud`, `dispatch`, `mod`).
- **Phase 5 (P34–P37)**: 3-tier hierarchical architecture documentation (`docs/architecture/`), GUI application event loop modularization (`karakuri::app`), control-aware dynamic tooltip shortcut badges in `karakuri-console::hover`, and engine Set execution decomposition (`karakuri-engine::set`).
- **Phase 6 (P38–P41)**: Console glyph componentization (`glyph.rs`), popup card row text deduplication (`card_row_text`), root view modularization (`draw.rs`, `regions.rs`), and GUI milestone M5 exit condition verification.
- **Phase 7 (P42–P46)**: GUI interaction & event dispatch modernization: unified modal overlays (`ModalOverlay`), tooltip suppression under open cards, text input session abstraction (`TextInputSession`), post-event side-effect decoupling (`handle_post_event_side_effects`), and per-bay pointer dispatch decomposition in `readout::dispatch`.
- **Phase 8 (P47–P51)**: Core subsystem monolith decomposition: `karakuri-ir::check` (P47), `karakuri::bridge::handlers` (P48), `karakuri-store::record` (P49), `karakuri-cli::live` (P50), and `karakuri-console::focus` (P51) bringing all core files comfortably below 2,000 lines.
- **Phase 9 (P52–P58)**: Secondary monolith modularization: `karakuri-console::hover` (P52), `karakuri-operation` & `karakuri-operation-record` (P53), `karakuri::app::handler` (P54), `karakuri::bridge::filesystem` (P55), `karakuri-mcp::spelled` (P56), `karakuri-midi::map` (P57), and `karakuri::readout::dispatch` (P58). All secondary monoliths (>1,900 lines) across the workspace are now modularized.

| Initiative | Target Subsystem | Actionable Deliverable | Status |
|---|---|---|:---:|
| **P21** | Type Conversions | Centralized layer conversions in `karakuri-environment::meta` as single source of truth | **COMPLETED** |
| **P22** | `karakuri-cli` / GUI | Extracted headless runtime controller; modularized `karakuri-cli` | **COMPLETED** |
| **P23** | `karakuri-environment` | Decomposed monoliths (`setfile`, `mix`), resolved couplings, isolated test suites | **COMPLETED** |
| **P24** | `karakuri` (GUI) | Decomposed `engine_bridge.rs` into `bridge/` (`sinks`, `engine`, `filesystem`, `handlers`) | **COMPLETED** |
| **P25** | `karakuri-mcp` | Extracted test suites to `tests/`; decomposed `lib.rs` into `protocol`, `server`, `spelled`, `tools/` | **COMPLETED** |
| **P26** | `karakuri-console` | Extracted 3 bays (`sequencer`, `staging`), `widgets/`, and `layout.rs` | **COMPLETED** |
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
| **P42** | `karakuri::app` / Text Input | Abstract inline naming modes (`arrangement.naming` & `view.naming_set`) into a unified `TextInputSession` handler | **COMPLETED** |
| **P43** | `karakuri-console::view` | Unify popup card and modal chooser open/close states into a type-safe `ModalOverlay` enum to guarantee Rule 2 mutual exclusion | **COMPLETED** |
| **P44** | `karakuri-console::hover` | Connect `ModalOverlay` to the hover layer to suppress tooltips on background controls beneath open cards and choosers | **COMPLETED** |
| **P45** | `karakuri::readout::dispatch` | Leverage `ControlId` in `input::claim` and decompose 1,100-line pointer dispatch into per-bay handlers | **COMPLETED** |
| **P46** | `karakuri::app::handler` | Decouple post-event side-effects (I/O, set saving, projector routing) out of `handler.rs` into `operations.rs` | **COMPLETED** |
| **P47** | `karakuri-ir::check` (4,061 lines) | Decomposed into `check/` submodules (`contracts.rs`, `coverage.rs`, `context.rs`, `eval.rs`, `mod.rs`) | **COMPLETED** |
| **P48** | `karakuri::bridge::handlers` (3,896 lines) | Decomposed console-to-engine state updates into per-bay modules (`mixer.rs`, `transport.rs`, `inspector.rs`, `staging.rs`, `sequencer.rs`, `master.rs`, `apply.rs`, `mod.rs`) | **COMPLETED** |
| **P49** | `karakuri-store::record` (3,222 lines) | Decomposed into `record/` submodules (`types.rs`, `variants.rs`, `helpers.rs`, `tests.rs`, `mod.rs`) | **COMPLETED** |
| **P50** | `karakuri-cli::live` (3,056 lines) | Decomposed interactive runtime controller into `live/` submodules (`interactive.rs`, `demo.rs`, `audio.rs`, `mod.rs`) | **COMPLETED** |
| **P51** | `karakuri-console::focus` (3,664 lines) | Decomposed 2D spatial focus navigation and deduplicate card/chooser traversals into `focus/` submodules (`model.rs`, `card.rs`, `chooser.rs`, `ladder.rs`, `mod.rs`) | **COMPLETED** |
| **P52** | `karakuri-console::hover` (2,710 lines) | Decomposed hover layer into `hover/` submodules (`citation.rs`, `probes.rs`, `view.rs`, `mod.rs`) | **COMPLETED** |
| **P53** | `karakuri-operation` (2,702 lines) & `karakuri-operation-record` (2,466 lines) | Decomposed operation vocabulary into `types.rs`/`op.rs` and record translator into `state.rs`/`tests.rs` | **COMPLETED** |
| **P54** | `karakuri::app::handler` (2,591 lines) | Decomposed application event loop & window management into `handler/` (`key.rs`, `pointer.rs`, `projector.rs`, `redraw.rs`, `mod.rs`) | **COMPLETED** |
| **P55** | `karakuri::bridge::filesystem` (2,540 lines) | Decomposed filesystem store bridge into `filesystem/` (`arrangement.rs`, `transfer.rs`, `reading.rs`, `folder.rs`, `listing.rs`, `mod.rs`) | **COMPLETED** |
| **P56** | `karakuri-mcp::spelled` (2,333 lines) | Decomposed MCP operation spelling into `spelled/` (`schema.rs`, `table.rs`, `dispatch.rs`, `mod.rs`) | **COMPLETED** |
| **P57** | `karakuri-midi::map` (2,038 lines) | Decomposed MIDI map parsing and controller bindings into `map/` (`tests.rs`, `parse.rs`, `types.rs`, `mod.rs`) | **COMPLETED** |
| **P58** | `karakuri::readout::dispatch` (1,915 lines) | Decomposed pointer routing and bay event dispatch into `dispatch/` (`types.rs`, `press.rs`, `actions.rs`, `mod.rs`) | **COMPLETED** |
| **P59** | `karakuri::tests` (8,782 lines) | Decomposed into submodules (`keeps.rs`, `history.rs`, `arrangement.rs`, `frames.rs`, `view_interaction.rs`, `operations.rs`, `mod.rs`), reducing `mod.rs` to 56 lines | **COMPLETED** |

---

## 2. Phase 10 Backlog: Test Suite Monolith Decomposition (P59–P68)

Targeting the top 10 largest test files (>2,300 lines down to <2,000 lines, target <1,500 lines):
- `karakuri::tests` (8,782 lines) -> **P59 COMPLETED**
- `karakuri-ir/tests/check.rs` (4,835 lines) -> **P60 IN PROGRESS**
- `karakuri-engine/tests/deck.rs` (4,571 lines) -> **P61**
- `karakuri-cli/src/tests.rs` (4,523 lines) -> **P62**
- `karakuri-console/tests/library.rs` (4,408 lines) -> **P63**
- `karakuri::tests::gpu` (3,990 lines) -> **P64**
- `karakuri-mcp/tests/wire.rs` (3,123 lines) -> **P65**
- `karakuri-codegen/tests/naga_test.rs` (2,784 lines) -> **P66**
- `karakuri-environment::setfile::tests` (2,779 lines) -> **P67**
- `karakuri-console/tests/grammar.rs` (2,387 lines) -> **P68**

---

## 3. Future Initiatives

Future refactoring and architectural enhancements will be recorded here as new requirements emerge.

