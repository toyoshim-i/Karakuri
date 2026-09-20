# Architectural Refactoring & Modernization Roadmap

This document records the architectural refactoring history and roadmap for **Karakuri**.

---

## 1. Completed Phases Summary (P1–P71)

All prior refactoring phases through Phase 11 are complete, verified with full workspace tests, and documented across crate-level `README.md` files:

- **Phases 1 & 2 (P1–P20)**: Transient render graph DAG, WGSL AST & pass fusion, machine-readable AI diagnostics, zero-allocation stream replay, and keymap convergence.
- **Phase 3 (P21–P26)**: Subsystem decomposition, monolith extraction, layer type centralization, full crate-level `README.md` documentation, and Google Style comment standardization across all 16 crates.
- **Phase 4 (P30–P33)**: Console UI componentization (`card`, `chip`, `track`, `field`), bay monolith decomposition (`mixer/`, `transport/`, `library/`, `inspector/`), unified `ControlDescriptor` registry in `karakuri-console::control`, and `Readout` event dispatch decoupling into `readout/` (`costs`, `hud`, `dispatch`, `mod`).
- **Phase 5 (P34–P37)**: 3-tier hierarchical architecture documentation (`docs/architecture/`), GUI application event loop modularization (`karakuri::app`), control-aware dynamic tooltip shortcut badges in `karakuri-console::hover`, and engine Set execution decomposition (`karakuri-engine::set`).
- **Phase 6 (P38–P41)**: Console glyph componentization (`glyph.rs`), popup card row text deduplication (`card_row_text`), root view modularization (`draw.rs`, `regions.rs`), and GUI milestone M5 exit condition verification.
- **Phase 7 (P42–P46)**: GUI interaction & event dispatch modernization: unified modal overlays (`ModalOverlay`), tooltip suppression under open cards, text input session abstraction (`TextInputSession`), post-event side-effect decoupling (`handle_post_event_side_effects`), and per-bay pointer dispatch decomposition in `readout::dispatch`.
- **Phase 8 (P47–P51)**: Core subsystem monolith decomposition: `karakuri-ir::check` (P47), `karakuri::bridge::handlers` (P48), `karakuri-store::record` (P49), `karakuri-cli::live` (P50), and `karakuri-console::focus` (P51) bringing all core files comfortably below 2,000 lines.
- **Phase 9 (P52–P58)**: Secondary monolith modularization: `karakuri-console::hover` (P52), `karakuri-operation` & `karakuri-operation-record` (P53), `karakuri::app::handler` (P54), `karakuri::bridge::filesystem` (P55), `karakuri-mcp::spelled` (P56), `karakuri-midi::map` (P57), and `karakuri::readout::dispatch` (P58).
- **Phase 10 (P59–P68)**: Workspace-wide test suite monolith decomposition across the 10 largest test files (>2,300 lines decomposed into modular subdirectories).
- **Phase 11 (P69–P71)**: Final 2,000+ line monolith elimination (`hot_swap.rs`, `listing.rs`, `view/mod.rs`) and pre-commit 2,000-line gate enforcement.

---

## 2. Active & Planned Refactoring Roadmap (P72–P87)

| Initiative | Target Subsystem | Actionable Deliverable | Status |
|:---:|---|---|:---:|
| **P72** | Clippy Warnings & Lints | Eliminate all `clippy::doc_lazy_continuation` errors, resolve `too_many_arguments` with Parameter Objects, and resolve `type_complexity` | **COMPLETED** |
| **P73** | CI & Git Hooks | Move workspace Clippy enforcement (`cargo clippy --workspace --all-targets -- -D warnings`) into `.githooks/pre-commit` | **COMPLETED** |
| **P74** | `karakuri-ir::check::eval` (1,923 lines) | Decompose IR type-checking & evaluation monolith into `check/eval/` (`stmt.rs`, `expr.rs`, `call.rs`, `mod.rs`) | **COMPLETED** |
| **P75** | `karakuri-console::input` (1,903 lines) | Decompose console input handling into `input/` (`probes.rs`, `claim.rs`, `wheel.rs`, `mod.rs`) | **PLANNED** |
| **P76** | `karakuri-console::view::inspector` (1,892 lines) | Decompose inspector bay view monolith into modular subcomponents | **PLANNED** |
| **P77** | `karakuri-cli::args` (1,765 lines) | Decompose CLI argument parsing, validation, and usage help into `args/` submodules | **PLANNED** |
| **P78** | `karakuri::keymap` (1,714 lines) | Decompose keybinding dispatch and action routines into `keymap/` submodules | **PLANNED** |
| **P79** | `karakuri::tests::operations` (1,893 lines) | Subdivide integration operations test suite by domain categories | **PLANNED** |
| **P80** | `karakuri-store::tests::store` (1,789 lines) | Decompose store integration tests into CAS, journal, and concurrency submodules | **PLANNED** |
| **P81** | `karakuri::tests::arrangement` (1,781 lines) | Decompose arrangement and session integration test suite | **PLANNED** |
| **P82** | `karakuri-cli::src::tests::live_save` (1,773 lines) | Decompose interactive runtime save/replay test suite | **PLANNED** |
| **P83** | Vocabulary Destination Purity | Purge toggle/step operations (`ToggleSolo`, `ToggleMute`) in favor of destination operations (`SetSolo`, `SetMute`) across all surfaces (P-0090, ADR-0180) | **PLANNED** |
| **P84** | Structured Refusal Unification | Standardize `RefusalDetail` structured type and guarantee bit-exact error wording across GUI, CLI, and MCP (P-0083, ADR-0131) | **PLANNED** |
| **P85** | Context & Parameter Objects | Replace 8-argument cascades with dedicated Context structs (`PlanSourcesCtx`, `InspectorRenderCtx`) (P-0091, ADR-0210) | **PLANNED** |
| **P86** | Sandbox-Safe Test Partitioning | Partition socket-dependent MCP integration tests from offline CPU/GPU tests to guarantee 100% deterministic test execution in sandboxes (ADR-0017, ADR-0242) | **PLANNED** |
| **P87** | Documentation Modernization & Style | Replace verbose poetic commentary with technical RustDoc and restore strict `clippy::doc_lazy_continuation` enforcement | **PLANNED** |

---

## 3. Phase 10: Test Suite Monolith Decomposition (P59–P68) — COMPLETED

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

## 4. Phase 11: Final Monolith Elimination (<1,000 Lines Target) (P69–P71) — COMPLETED

Decompose the final three files across the entire workspace exceeding 2,000 lines to ensure every file is under 1,000 lines (or comfortably below 1,500 lines):
- `karakuri-engine/tests/hot_swap.rs` (2,195 lines -> 19 lines) -> **P69 COMPLETED**
- `karakuri-console/src/view/library/listing.rs` (2,167 lines -> decomposed into 6 submodules: `layout.rs` 771 lines, `render.rs` 441 lines, `menu.rs` 230 lines, `reading.rs` 149 lines, `rows.rs` 106 lines, `mod.rs` 48 lines) -> **P70 COMPLETED**
- `karakuri-console/src/view/mod.rs` (2,073 lines -> decomposed into `types.rs` 839 lines, `mod.rs` 726 lines, `nav.rs` 205 lines, `budget.rs` 212 lines, `choices.rs` 138 lines) -> **P71 COMPLETED**
- Enforcement: Pre-commit hook (`.githooks/pre-commit`) strictly prohibits committing any Rust file exceeding 2,000 lines.

---

## 5. Phase 12: CI / Git Hook Modernization & Clippy Quality Gate (P72–P73) — PLANNED

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

## 6. Phase 13: Proactive Decomposition of Near-Monoliths (1,500–1,950 Lines) (P74–P78) — PLANNED

Target files in the "danger zone" (1,500 to 1,950 lines) to prevent accidental pre-commit gate rejections and maintain high cognitive readability:

- **P74: `karakuri-ir::check::eval.rs` (1,923 lines -> `check/eval/`) — COMPLETED**:
  - Decomposed into `crates/karakuri-ir/src/check/eval/`:
    - `stmt.rs` (495 lines): statement type checking (`check_stmt`, `check_stmts`, `check_let`, `check_assign`, `check_if`, `check_for`, `check_kill`)
    - `expr.rs` (719 lines): expression evaluation, binary/unary operators, literal coercion, swizzling
    - `call.rs` (654 lines): builtin and constructor invocations, texture sampling, slot access
    - `mod.rs` (79 lines): entry point and shared evaluation context, constants, and recovery helpers
- **P75: `karakuri-console::src/input.rs` (1,903 lines -> `input/`) — COMPLETED**:
  - Decomposed into `crates/karakuri-console/src/input/`:
    - `handlers.rs` (760 lines): all 39 `on_*` UI control hit-test derivation functions
    - `probes.rs` (421 lines): `Probe` struct, `PROBES` table, `CONTROLS` count and summation
    - `claim.rs` (104 lines): `Claim` enum and pointer event claim arbitration (`claim`)
    - `wheel.rs` (81 lines): scroll wheel interaction logic and `Turned` enum (`wheeled`)
    - `mod.rs` (561 lines): module documentation, submodules, and public API re-exports
- **P76: `karakuri-console::src/view/inspector/mod.rs` (1,899 lines -> submodules) — COMPLETED**:
  - Decomposed `crates/karakuri-console/src/view/inspector/` into focused submodules:
    - `pane.rs` (712 lines): `InspectorPane` struct and spatial hit-test/geometry methods
    - `layout.rs` (149 lines): `inspector` layout calculation and `pane_box` arithmetic
    - `render.rs` (231 lines): `inspector_into` UI rendering and `InspectorIntoCtx`
    - `dispatch.rs` (322 lines): `View` inspector methods (naming, wiring, scrolling, deck targeting)
    - `tests.rs` (342 lines): inspector pane layout and authority unit tests
    - `mod.rs` (165 lines): module definitions, constants (`PANES`, `PANE_NAMES`, `PANE_DECKS`), and re-exports
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

## 7. Phase 14: Secondary Test Monolith Decomposition (P79–P82) — PLANNED

Decompose test suites that have accumulated beyond 1,700 lines:

- **P79: `karakuri::tests::operations` (1,893 lines)**: Decompose into operations category submodules (`transport.rs`, `mixer.rs`, `mcp.rs`, `surfaces.rs`).
- **P80: `karakuri-store::tests::store` (1,789 lines)**: Decompose into `store/` submodules (`cas.rs`, `journal.rs`, `concurrency.rs`, `compaction.rs`).
- **P81: `karakuri::tests::arrangement` (1,781 lines)**: Decompose arrangement lifecycle and session serialization tests.
- **P82: `karakuri-cli::src::tests::live_save` (1,773 lines)**: Decompose interactive runtime state recording and replay test cases.

---

## 8. Phase 15: Architectural Principle Realization & Vocabulary Modernization (P83–P86) — PLANNED

Ground system design in Karakuri's standing principles (`docs/principles/`) and Architectural Decision Records (`docs/adr/`), turning implicit design conventions into enforceable compiler contracts:

- **P83: Principle P-0090 Vocabulary Convergence & Destination Purity**:
  - *Principle P-0090 (A surface offers; it never decides)* & *ADR-0180 (Vocabulary crate with no dependencies)*: Every operation variant must name an absolute destination rather than an affordance or relative motion.
  - Fully purge remaining toggle operations (`ToggleSolo`, `ToggleMute`) from `karakuri-operation::Operation`.
  - Migrate all UI, keymap, and MIDI handlers to explicit destination operations (`SetSolo { deck, solo }`, `SetMute { deck, mute }`).
  - Ensure surfaces maintain their own local toggle/cycle state and submit destination values, preventing state drift across GUI, MIDI, and MCP.
- **P84: Principle P-0083 & ADR-0131 Structured Refusal Unification**:
  - *Principle P-0083 (A refusal carries what the next attempt needs)* & *ADR-0131 (One refusal sentence per mistake across all surfaces)*: Refusals must carry explicit numbers and constraint bounds without guessing fixes.
  - Replace ad-hoc string formatting in `karakuri::bridge::handlers::apply::unwritten` and `karakuri-operation::gate` with a strongly-typed `RefusalDetail` struct (capturing constraint identifier, active value, requested value, and gating class).
  - Guarantee bit-exact identical refusal messages across GUI status lines, CLI stderr, and MCP JSON-RPC error responses.
- **P85: Principle P-0091 Context & Parameter Object Standardization**:
  - *Principle P-0091 (Cost is known before it is paid)* & *ADR-0210 (Declared cost is one panel pass)*: UI repaint schedules and layout measurements must remain decoupled from deep function call hierarchies.
  - Replace 8-argument parameter cascades with cohesive Context structures:
    - `karakuri-engine::set::schedule::plan_sources` -> `PlanSourcesCtx<'a>`
    - `karakuri-console::view::inspector::header::pane_target` -> `PaneTargetCtx<'a>`
    - `karakuri-console::view::inspector::inspector_into` -> `InspectorRenderCtx<'a>`
  - Encapsulate declared costs, theme palettes, and layout caches to insulate signatures against future feature additions.
- **P86: ADR-0017 & Sandbox-Safe Test Suite Partitioning**:
  - *ADR-0017 (An invariant that can be tested is a test)* & *ADR-0242 (Command line is test tooling)*:
  - Formally partition the test suite into:
    1. *Offline unit & IR tests*: CPU-only AST/type-check, formatting, layout, store CAS.
    2. *Headless GPU render tests*: Dedicated `mod gpu` suites verifying bit-exact shader execution.
    3. *Socket-dependent integration tests*: Loopback MCP tests requiring external socket privileges.
  - Ensure local container CI and restricted sandboxes execute 100% of offline and GPU suites deterministically without failing on socket permissions.

---

## 9. Phase 16: Documentation Modernization & Technical Clarity (P87) — PLANNED

**Goal**:
Eliminate verbose, speculative, and philosophical prose comments left behind by prior model self-talk (e.g. Opus prose). Replace them with concise, factual, engineering-oriented RustDoc (documenting *What*, *Why*, preconditions, and invariants). Restore strict CommonMark markdown indentation across all doc comments and re-enable `clippy::doc_lazy_continuation` enforcement.

**Key Actions**:
- **P87: Prose Comment Pruning & Clippy Doc Lint Re-enforcement**:
  - *Prune Philosophical Ramblings*: Replace stream-of-consciousness, metaphor-heavy commentary (e.g., philosophical musings on keys, focus, and state transitions) with clean, factual descriptions of mechanics, invariants, and side-effects.
  - *Normalize CommonMark Indentation*: Re-indent all multi-line lists, blockquotes, and nested items according to CommonMark / rustdoc guidelines to eliminate lazy continuation ambiguity.
  - *Re-enable Clippy Enforcement*: Remove `doc_lazy_continuation = "allow"` from root `Cargo.toml` (restoring it to `warn`/`deny`) so that future documentation style regressions are prevented at pre-commit time.

---

## 10. Future Initiatives

- **Dynamic Module Hot-Reloading Ergonomics**: Extend `.kir` hot-reloading abstractions across non-shader resource bundles.
- **Unified Event Journal Introspection**: Standardize tooling for offline inspection and diffing of `.ndjson` session streams.




