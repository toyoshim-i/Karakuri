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
│ Stage 1: Current Hard Gate — 1,800 Lines (Enforced in .githooks)       │
│ - Zero existing violations (max file: 1,768 lines). Regressions barred.│
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Stage 2: Target <1,500 Lines (Phase 17)                                 │
│ - 16 files currently in 1,500–1,800 range to be modularized.           │
│ - Once complete, lower hard gate in pre-commit to 1,500 lines.          │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Stage 3: Target <1,300 Lines (Phase 18)                                 │
│ - 23 files currently in 1,300–1,500 range to be modularized.           │
│ - Once complete, lower hard gate in pre-commit to 1,300 lines.          │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Stage 4: Ultimate Architectural Target <1,000 Lines (Phase 19)          │
│ - 41 files in 1,000–1,300 range to be modularized.                      │
│ - Aligns fully with ADR-0345 (1,000 lines as single-file ceiling).     │
└─────────────────────────────────────────────────────────────────────────┘
```

### Phase 17: Near-Monolith Modularization (1,500–1,800 Lines Target) — PLANNED

Modularize the 16 remaining files exceeding 1,500 lines:

| Initiative | Subsystem / File | Lines | Scope & Approach | Status |
|:---:|---|:---:|---|:---:|
| **P88** | `karakuri::tests::view_interaction` | 1,768 | Partition UI view interaction tests into focused modal, hover, and bay submodules | **COMPLETED** |
| **P89** | `karakuri-ir::ast` | 1,726 | Decompose AST definitions into `types.rs`, `attr.rs`, `decl.rs`, `expr.rs`, and `stmt.rs` | **COMPLETED** |
| **P90** | `karakuri-cli::src::tests::parse` | 1,708 | Split CLI argument and flag parser tests into category submodules | **COMPLETED** |
| **P91** | `karakuri-operation-record::tests` | 1,704 | Partition operation record serialization and backward-compatibility tests | **COMPLETED** |
| **P92** | `karakuri-mcp::src::tests` | 1,686 | Decompose MCP server and tool handler unit test suites | **PLANNED** |
| **P93** | `karakuri-console::hover::probes` | 1,667 | Separate static TIPS definitions from runtime probe hit-testing algorithms | **PLANNED** |
| **P94** | `karakuri-mcp::spelled::table` | 1,661 | Modularize schema dictionary and spelling lookup tables | **PLANNED** |
| **P95** | `karakuri-engine::tests::governor` | 1,653 | Partition frame pacing, timing governor, and rate-limiting test suites | **PLANNED** |
| **P96** | `karakuri-console::panel` | 1,644 | Separate panel layout geometry arithmetic from coordinate transformations | **PLANNED** |
| **P97** | `karakuri::bridge::handlers::apply` | 1,636 | Subdivide operation dispatch and refusal emission into domain handlers | **PLANNED** |
| **P98** | `karakuri::session` | 1,602 | Decompose session lifecycle, state streams, and persistence coordination | **PLANNED** |
| **P99** | `karakuri::bridge::engine` | 1,586 | Separate engine command channel management from frame telemetry collection | **PLANNED** |
| **P100** | `karakuri-console::view::program` | 1,579 | Split program bay rendering into canvas display, stats, and overlays | **PLANNED** |
| **P101** | `karakuri-cli::live::interactive` | 1,562 | Modularize CLI interactive terminal event handling and render loop | **PLANNED** |
| **P102** | `karakuri-engine::tests::sources` | 1,540 | Decompose shader source loading, preprocessing, and error recovery tests | **PLANNED** |
| **P103** | `karakuri-console::view::inspector::header` | 1,518 | Separate inspector header title rendering from chip buttons and target badges | **PLANNED** |

---

## 3. Future Initiatives

- **Stage 3 Gate Lowering (<1,300 Lines)**: Target secondary cohort of 23 files (Phase 18).
- **Stage 4 Architectural Ceiling (<1,000 Lines)**: Target final 41 files to achieve full ADR-0345 alignment (Phase 19).
- **Dynamic Module Hot-Reloading Ergonomics**: Extend `.kir` hot-reloading abstractions across non-shader resource bundles.
- **Unified Event Journal Introspection**: Standardize tooling for offline inspection and diffing of `.ndjson` session streams.
