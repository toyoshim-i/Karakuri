# Architectural Refactoring & Modernization Roadmap

This document records the architectural refactoring history and roadmap for **Karakuri**.

---

## 1. Summary Roadmap & Completed Phases (P1–P71)

All prior refactoring phases through Phase 11 are complete, verified with full workspace tests, and documented across crate-level `README.md` files:

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
| **P60** | `karakuri-ir/tests/check.rs` (4,835 lines) | Decomposed into `check/` (`canonical.rs`, `layers.rs`, `slots.rs`, `effects.rs`, `common.rs`), reducing `check.rs` to 19 lines | **COMPLETED** |
| **P61** | `karakuri-engine/tests/deck.rs` (4,571 lines) | Decomposed into `deck/` submodules (`common.rs`, `masks_and_transitions.rs`, `blending_and_master.rs`, `lifecycle_and_swaps.rs`, `measurements_and_edge_cases.rs`), reducing `deck.rs` to 18 lines | **COMPLETED** |
| **P62** | `karakuri-cli/src/tests.rs` (4,523 lines) | Decomposed into `src/tests/` submodules (`mod.rs`, `parse.rs`, `interactive.rs`, `live_save.rs`, `wire.rs`), reducing `mod.rs` to 6 lines | **COMPLETED** |
| **P63** | `karakuri-console/tests/library.rs` (4,408 lines) | Decomposed into `tests/library/` submodules (`geometry.rs`, `scope_and_filter.rs`, `params_and_history.rs`, `menus_and_scroll.rs`, `common.rs`), reducing `library.rs` to 18 lines | **COMPLETED** |
| **P64** | `karakuri::tests::gpu` (3,990 lines) | Decomposed into `src/tests/gpu/` submodules (`common.rs`, `mcp_and_storage.rs`, `frame_and_textures.rs`, `decks_and_simulation.rs`, `transitions_and_sinks.rs`), reducing `gpu.rs` to 8 lines | **COMPLETED** |
| **P65** | `karakuri-mcp/tests/wire.rs` (3,123 lines) | Decomposed into `tests/wire/` submodules (`common.rs`, `protocol_and_http.rs`, `tools_and_save.rs`, `library_and_sets.rs`, `routing_and_operations.rs`), reducing `wire.rs` to 16 lines | **COMPLETED** |
| **P66** | `karakuri-codegen/tests/naga_test.rs` (2,784 lines) | Decomposed into `tests/naga/` submodules (`common.rs`, `primitives_and_blends.rs`, `camera_and_amplify.rs`, `fields_and_layouts.rs`, `post_and_textures.rs`), reducing `naga_test.rs` to 16 lines | **COMPLETED** |
| **P67** | `karakuri-environment::setfile::tests` (2,779 lines) | Decomposed into `src/setfile/tests/` submodules (`common.rs`, `edges_and_params.rs`, `chain_and_composition.rs`, `camera_and_noise.rs`, `bundle_and_resolution.rs`), reducing `tests.rs` to 6 lines | **COMPLETED** |
| **P68** | `karakuri-console/tests/grammar.rs` (2,387 lines) | Decomposed into `tests/grammar/` submodules (`common.rs`, `navigation_and_dispatch.rs`, `space_and_enter.rs`, `cards_and_choosers.rs`, `chain_and_dismissal.rs`), reducing `grammar.rs` to 19 lines | **COMPLETED** |
| **P69** | `karakuri-engine/tests/hot_swap.rs` (2,195 lines) | Decomposed into `tests/hot_swap/` submodules (`common.rs`, `lifecycle_and_swaps.rs`, `budgets_and_estimates.rs`, `workers_and_performance.rs`, `rewind_and_macros.rs`), reducing `hot_swap.rs` to 19 lines | **COMPLETED** |
| **P70** | `karakuri-console/src/view/library/listing.rs` (2,167 lines) | Decomposed into `listing/` submodules (`reading.rs`, `menu.rs`, `rows.rs`, `layout.rs`, `render.rs`, `mod.rs`) | **COMPLETED** |
| **P71** | `karakuri-console/src/view/mod.rs` (2,073 lines) | Decomposed root view into submodules (`types.rs`, `nav.rs`, `budget.rs`, `choices.rs`, `mod.rs`) | **COMPLETED** |
| **P72** | Clippy Warnings & Lints | Eliminate all 255 `clippy::doc_lazy_continuation` errors, resolve `too_many_arguments` with Parameter Objects, and resolve `type_complexity` | **PLANNED** |
| **P73** | CI & Git Hooks | Move workspace Clippy enforcement (`cargo clippy --workspace --all-targets -- -D warnings`) into `.githooks/pre-commit` | **PLANNED** |
| **P74** | `karakuri-ir::check::eval` (1,923 lines) | Decompose IR type-checking & evaluation monolith into `check/eval/` (`stmt.rs`, `expr.rs`, `call.rs`, `mod.rs`) | **PLANNED** |
| **P75** | `karakuri-console::input` (1,903 lines) | Decompose console input handling into `input/` (`probes.rs`, `claim.rs`, `wheel.rs`, `mod.rs`) | **PLANNED** |
| **P76** | `karakuri-console::view::inspector` (1,892 lines) | Decompose inspector bay view monolith into modular subcomponents | **PLANNED** |
| **P77** | `karakuri-cli::args` (1,765 lines) | Decompose CLI argument parsing, validation, and usage help into `args/` submodules | **PLANNED** |
| **P78** | `karakuri::keymap` (1,714 lines) | Decompose keybinding dispatch and action routines into `keymap/` submodules | **PLANNED** |
| **P79** | `karakuri::tests::operations` (1,893 lines) | Subdivide integration operations test suite by domain categories | **PLANNED** |
| **P80** | `karakuri-store::tests::store` (1,789 lines) | Decompose store integration tests into CAS, journal, and concurrency submodules | **PLANNED** |
| **P81** | `karakuri::tests::arrangement` (1,781 lines) | Decompose arrangement and session integration test suite | **PLANNED** |
| **P82** | `karakuri-cli::src::tests::live_save` (1,773 lines) | Decompose interactive runtime save/replay test suite | **PLANNED** |

---

## 2. Phase 10: Test Suite Monolith Decomposition (P59–P68) — COMPLETED

All top 10 largest test files (>2,300 lines down to <2,000 lines, target <1,500 lines) across the workspace are now fully decomposed into modular, isolated submodules:
- `karakuri::tests` (8,782 lines -> 56 lines) -> **P59 COMPLETED**
- `karakuri-ir/tests/check.rs` (4,835 lines -> 19 lines) -> **P60 COMPLETED**
- `karakuri-engine/tests/deck.rs` (4,571 lines -> 18 lines) -> **P61 COMPLETED**
- `karakuri-cli/src/tests.rs` (4,523 lines -> 6 lines) -> **P62 COMPLETED**
- `karakuri-console/tests/library.rs` (4,408 lines -> 18 lines) -> **P63 COMPLETED**
- `karakuri::tests::gpu` (3,990 lines -> 8 lines) -> **P64 COMPLETED**
- `karakuri-mcp/tests/wire.rs` (3,123 lines -> 16 lines) -> **P65 COMPLETED**
- `karakuri-codegen/tests/naga_test.rs` (2,784 lines -> 16 lines) -> **P66 COMPLETED**
- `karakuri-environment::setfile::tests` (2,779 lines -> 6 lines) -> **P67 COMPLETED**
- `karakuri-console/tests/grammar.rs` (2,387 lines -> 19 lines) -> **P68 COMPLETED**

---

## 3. Phase 11: Final Monolith Elimination (<1,000 Lines Target) (P69–P71) — COMPLETED

Decompose the final three files across the entire workspace exceeding 2,000 lines to ensure every file is under 1,000 lines (or comfortably below 1,500 lines):
- `karakuri-engine/tests/hot_swap.rs` (2,195 lines -> 19 lines) -> **P69 COMPLETED**
- `karakuri-console/src/view/library/listing.rs` (2,167 lines -> decomposed into 6 submodules: `layout.rs` 771 lines, `render.rs` 441 lines, `menu.rs` 230 lines, `reading.rs` 149 lines, `rows.rs` 106 lines, `mod.rs` 48 lines) -> **P70 COMPLETED**
- `karakuri-console/src/view/mod.rs` (2,073 lines -> decomposed into `types.rs` 839 lines, `mod.rs` 726 lines, `nav.rs` 205 lines, `budget.rs` 212 lines, `choices.rs` 138 lines) -> **P71 COMPLETED**
- Enforcement: Pre-commit hook (`.githooks/pre-commit`) strictly prohibits committing any Rust file exceeding 2,000 lines.

---

## 4. Phase 12: CI / Git Hook Modernization & Clippy Quality Gate (P72–P73) — PLANNED

Clean up existing linter warnings and shift quality enforcement earlier in the developer workflow by moving Clippy from `pre-push` into `pre-commit`:

- **P72: Clippy Warnings Elimination & Doc Formatting Standardization**:
  - Fix 255 instances of `clippy::doc_lazy_continuation` (markdown list indentation in doc comments) across `karakuri-ir`, `karakuri-store`, `karakuri-mcp`, `karakuri`, and `karakuri-environment`.
  - Introduce Parameter Objects / Context structs for functions triggering `clippy::too_many_arguments`:
    - `karakuri-engine::set::schedule::plan_sources` (8 arguments)
    - `karakuri-console::view::inspector::header::pane_target` (8 arguments)
    - `karakuri-console::view::inspector::inspector_into` (8 arguments)
  - Refactor `clippy::type_complexity` complex return tuple in `karakuri-engine::set::schedule`.
  - Fix idiom warnings: `clippy::needless_borrow` in `listing/render.rs`, `clippy::manual_map` in `modal.rs`, `clippy::single_match` in `app/handler/key.rs`, and unused imports in tests.
- **P73: Pre-Commit Clippy Integration**:
  - Update `.githooks/pre-commit` to execute `cargo clippy --quiet --workspace --all-targets -- -D warnings`.
  - Update `docs/contributing.md` to document that `pre-commit` verifies formatting, 2,000-line limit, and strict Clippy cleanliness.

---

## 5. Phase 13: Proactive Decomposition of Near-Monoliths (1,500–1,950 Lines) (P74–P78) — PLANNED

Target files in the "danger zone" (1,500 to 1,950 lines) to prevent accidental pre-commit gate rejections and maintain high cognitive readability:

- **P74: `karakuri-ir::check::eval.rs` (1,923 lines)**:
  - Decompose into `crates/karakuri-ir/src/check/eval/`:
    - `stmt.rs`: statement type checking (`check_stmt`, `check_stmts`, `check_let`, `check_assign`, `check_if`)
    - `expr.rs`: expression evaluation, binary/unary operators, literal coercion
    - `call.rs`: builtin and constructor invocations, domain validations
    - `mod.rs`: entry point and shared evaluation context
- **P75: `karakuri-console::src/input.rs` (1,903 lines)**:
  - Decompose into `crates/karakuri-console/src/input/`:
    - `probes.rs`: ~800 lines of `on_*` UI control hit-test derivations (`on_step`, `on_strip`, `on_mcp`, etc.)
    - `claim.rs`: pointer event ownership arbitration and Rule 1-4 enforcement
    - `wheel.rs`: scroll wheel interaction logic
    - `mod.rs`: re-exports, constants (`PROBES`, `CONTROLS`), and interface definitions
- **P76: `karakuri-console::src/view/inspector/mod.rs` (1,892 lines)**:
  - Decompose pane layout rendering and inspector body dispatch into focused submodules alongside existing `header.rs`, `params.rs`, and `wiring.rs`.
- **P77: `karakuri-cli::src/args.rs` (1,765 lines)**:
  - Decompose manual argument parsing into `crates/karakuri-cli/src/args/`:
    - `parser.rs`: command-line token consumption and flag parsing
    - `validate.rs`: argument semantic constraints and consistency checks
    - `help.rs`: usage strings and manual generation
    - `mod.rs`: `Args` struct definition and public API
- **P78: `karakuri::src/keymap.rs` (1,714 lines)**:
  - Decompose into `crates/karakuri/src/keymap/`:
    - `table.rs`: `KEY_BINDINGS` table definition and documentation cross-checks
    - `actions.rs`: individual `KeyAction` execution implementations
    - `mod.rs`: `KeyCtx` and dispatch entry point

---

## 6. Phase 14: Secondary Test Monolith Decomposition (P79–P82) — PLANNED

Decompose test suites that have accumulated beyond 1,700 lines:

- **P79: `karakuri::tests::operations` (1,893 lines)**: Decompose into operations category submodules (`transport.rs`, `mixer.rs`, `mcp.rs`, `surfaces.rs`).
- **P80: `karakuri-store::tests::store` (1,789 lines)**: Decompose into `store/` submodules (`cas.rs`, `journal.rs`, `concurrency.rs`, `compaction.rs`).
- **P81: `karakuri::tests::arrangement` (1,781 lines)**: Decompose arrangement lifecycle and session serialization tests.
- **P82: `karakuri-cli::src::tests::live_save` (1,773 lines)**: Decompose interactive runtime state recording and replay test cases.

---

## 7. Future Initiatives

- **Parameter Object Standardization**: Systematically introduce Options/Context patterns for functions with growing parameter lists.
- **Error Type Ergonomics**: Standardize error reporting across CLI and bridge boundaries using structured error enums.



