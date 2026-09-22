# Architectural Refactoring & Modernization Roadmap

This document records the architectural refactoring history and roadmap for **Karakuri**.

---

## 1. Completed Phases Summary (P1–P87)

All refactoring phases through Phase 16 (P1–P87) are complete, verified with full workspace tests and Clippy quality gates, and documented across crate-level `README.md` files:

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
- **Phase 12 (P72–P73)**: Pre-commit Clippy gate integration and initial lint cleanup across the workspace.
- **Phase 13 (P74–P78)**: Proactive modularization of danger-zone source monoliths (1,700–1,950 lines) into submodules strictly < 1,000 lines.
- **Phase 14 (P79–P82)**: Secondary test monolith decomposition across integration suites (>1,700 lines).
- **Phase 15 (P83–P86)**: Architectural principles realization (destination purity, structured refusal unification, parameter objects, and sandbox-safe test partitioning).
- **Phase 16 (P87)**: Documentation modernization, elimination of speculative/philosophical prose comments, CommonMark indentation restoration, and strict `clippy::doc_lazy_continuation` re-enforcement.

---

## 2. Staged Line Limit Reduction Roadmap (Target: <1,000 Lines)

To maximize cognitive readability, prevent "God module" accumulation, and optimize pair programming with LLMs (keeping files within the ~500–800 line sweet spot), the pre-commit gate and code organization follow a staged reduction roadmap:

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Stage 1: Initial Hard Gate — 1,800 Lines (COMPLETED)                    │
│ - Zero existing violations. Baseline established.                       │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Stage 2: Target <1,500 Lines (Phase 17) (COMPLETED)                     │
│ - All files brought under 1,500 lines (P88–P98).                        │
│ - Hard gate lowered to 1,500 lines in .githooks/pre-commit.             │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Stage 3: Target <1,300 Lines (Phase 18) (ACTIVE)                        │
│ - 18 files currently in 1,300–1,500 range to be modularized (P99–P116). │
│ - Once complete, lower hard gate in pre-commit to 1,300 lines.          │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Stage 4: Ultimate Architectural Target <1,000 Lines (Phase 19)          │
│ - Files in 1,000–1,300 range to be modularized.                         │
│ - Aligns fully with ADR-0345 (1,000 lines as single-file ceiling).     │
└─────────────────────────────────────────────────────────────────────────┘
```

### Phase 17: Near-Monolith Modularization (1,500–1,800 Lines Target) — COMPLETED

All 11 initiatives targeting files exceeding 1,500 lines are complete:

| Initiative | Subsystem / File | Lines | Scope & Approach | Status |
|:---:|---|:---:|---|:---:|
| **P88** | `karakuri::tests::view_interaction` | 1,768 | Partition UI view interaction tests into focused modal, hover, and bay submodules | **COMPLETED** |
| **P89** | `karakuri-ir::ast` | 1,726 | Decompose AST definitions into `types.rs`, `attr.rs`, `decl.rs`, `expr.rs`, and `stmt.rs` | **COMPLETED** |
| **P90** | `karakuri-cli::src::tests::parse` | 1,708 | Split CLI argument and flag parser tests into category submodules | **COMPLETED** |
| **P91** | `karakuri-operation-record::tests` | 1,704 | Partition operation record serialization and backward-compatibility tests | **COMPLETED** |
| **P92** | `karakuri-mcp::src::tests` | 1,686 | Decompose MCP server and tool handler unit test suites | **COMPLETED** |
| **P93** | `karakuri-console::hover::probes` | 1,667 | Separate static TIPS definitions from runtime probe hit-testing algorithms | **COMPLETED** |
| **P94** | `karakuri-mcp::spelled::table` | 1,652 | Modularize schema dictionary and spelling lookup tables into `table/` submodules | **COMPLETED** |
| **P95** | `karakuri-engine::tests::governor` | 1,617 | Partition frame pacing and timing governor tests into `pure.rs` and `deck.rs` | **COMPLETED** |
| **P96** | `karakuri-console::panel` | 1,541 | Separate panel layout geometry arithmetic and types into `panel/` submodules | **COMPLETED** |
| **P97** | `karakuri-engine::tests::sources` | 1,540 | Decompose multiple geometry sources and deformation tests into `sources/` submodules | **COMPLETED** |
| **P98** | `karakuri::bridge::engine` | 1,508 | Separate engine command channel management from frame telemetry collection | **COMPLETED** |

### Phase 18: Intermediate Reduction (1,300–1,500 Lines Target) — ACTIVE

Modularize the 18 remaining files exceeding 1,300 lines down below 1,000 lines:

| Initiative | Subsystem / File | Lines | Scope & Approach | Status |
|:---:|---|:---:|---|:---:|
| **P99** | `karakuri-environment::mix::tests` | 1,485 | Partition mixer state persistence and audio synchronization test cases | **COMPLETED** |
| **P100** | `karakuri-cli::live::interactive` | 1,472 | Modularize CLI interactive terminal event handling and render loop | **COMPLETED** |
| **P101** | `karakuri-engine::tests::binding` | 1,452 | Split pipeline resource binding and bind group layout integration tests | **COMPLETED** |
| **P102** | `karakuri-console::tests::mixer` | 1,449 | Partition mixer bay fader, balance, and solo/mute test suites | **COMPLETED** |
| **P103** | `karakuri-console::view::inspector::header` | 1,429 | Separate inspector header title rendering from chip buttons and target badges | **PLANNED** |
| **P104** | `karakuri-engine::set::layers` | 1,412 | Separate layer binding management from uniform buffer assignment | **PLANNED** |
| **P105** | `karakuri-engine::tests::master` | 1,402 | Partition master chain GPU pipeline and pass fusion tests | **PLANNED** |
| **P106** | `karakuri-console::view::program` | 1,397 | Decompose program bay monitor rendering and aspect ratio calculations | **PLANNED** |
| **P107** | `karakuri::bridge::handlers::apply` | 1,387 | Extract operation execution match arms into focused handler functions | **PLANNED** |
| **P108** | `karakuri-layout::layout` | 1,373 | Decompose layout constraint solver and rect partitioning utilities | **PLANNED** |
| **P109** | `karakuri-mcp::tools::mod` | 1,371 | Extract tool dispatch registry and argument schemas into submodules | **PLANNED** |
| **P110** | `karakuri::session` | 1,370 | Separate session state persistence from event log playback | **PLANNED** |
| **P111** | `karakuri-engine::deck` | 1,364 | Partition deck slot execution and texture lifecycle management | **PLANNED** |
| **P112** | `karakuri::readout::costs` | 1,354 | Separate frame cost tracking from telemetry aggregation | **PLANNED** |
| **P113** | `karakuri-console::tests::library::geometry` | 1,334 | Partition library geometry browser and card loading tests | **PLANNED** |
| **P114** | `karakuri-ir::tests::check::layers` | 1,333 | Partition IR layer type checking and diagnostic emission tests | **PLANNED** |
| **P115** | `karakuri-operation::gate` | 1,330 | Modularize operation validation gating rules and authority checks | **PLANNED** |
| **P116** | `karakuri-engine::frame` | 1,308 | Decompose frame synchronization and render target binding lifecycle | **PLANNED** |

---

## 3. Structural & Semantic Modernization Initiatives (Beyond File Splitting)

While module splitting enforces file length constraints, true architectural clarity and maintainability require semantic refactorings. The following initiatives are queued for implementation across subsequent phases:

### 3.1 Workspace Test Fixture & Helper Consolidation
Integration and unit tests currently duplicate significant setup boilerplate across multiple files:
- **`compile(&str)`**: Duplicated across 28 distinct test files in `karakuri-engine::tests`.
- **`render(&[IrError], &str)`**: Duplicated across 25 test files.
- **`console(viewport: Rect)` / `console()`**: Repeated across 36 integration test files in `karakuri-console::tests`.
- **`at(Pos2)` / `point(Pos2)`**: Coordinate transformation helpers duplicated in 28 test suites.
- **`f16(bits: u16)`**: Bitcast conversion helpers copied into 15 test files.

**Action Plan**: Consolidate into per-crate `tests/common/` modules or a shared workspace test fixture crate, eliminating thousands of lines of duplicated test infrastructure.

### 3.2 Massive Match Expression Function Extraction (Dispatch Clarity)
Several central dispatcher functions contain monolithic `match` expressions where individual arms span hundreds of lines:
- **`karakuri::app::mod::App::act` (L526)**: 379-line match with inline operation execution, journal logging, and error conversion.
- **`karakuri::app::handler::pointer::dispatch` (L22)**: 303-line pointer event match.
- **`karakuri::app::handler::key::dispatch` (L108)**: 291-line keyboard event match.
- **`karakuri::bridge::handlers::apply::apply_operation` (L797)**: 234-line operation execution match.

**Action Plan**: Extract arm logic into dedicated, named private functions (e.g. `handle_emitted_operation`, `dispatch_pointer_down`). Reduce top-level matches to clean, 20–40 line dispatch tables.

### 3.3 Domain Substate Decomposition (Fat Struct Remediation)
Several monolithic structs aggregate fields across unrelated UI or storage concerns:
- **`karakuri_console::view::View` (835 lines)**: Holds flat state for mixer, inspector, library, transport, and modals simultaneously.
- **`karakuri_store::record::Record` (1,006 lines)**: Monolithic record enumeration.

**Action Plan**: Decompose `View` into domain substate structs (`MixerState`, `InspectorState`, `LibraryState`, `TransportState`) composed under `View`. Improves borrow checker ergonomics and enforces clear component boundaries.

### 3.4 Structured Error Handling & Declarative Formatting
- **`karakuri_engine::set::SetError` (239 lines)**: Contains manual string concatenation and formatting logic embedded inside error declarations.

**Action Plan**: Modernize domain error hierarchies with declarative traits and structured metadata, eliminating manual string building logic.

---

## 4. Future Initiatives

- **Stage 3 Gate Lowering (<1,300 Lines)**: Target secondary cohort of files (Phase 18).
- **Stage 4 Architectural Ceiling (<1,000 Lines)**: Target remaining files to achieve full ADR-0345 alignment (Phase 19).
- **Dynamic Module Hot-Reloading Ergonomics**: Extend `.kir` hot-reloading abstractions across non-shader resource bundles.
- **Unified Event Journal Introspection**: Standardize tooling for offline inspection and diffing of `.ndjson` session streams.
