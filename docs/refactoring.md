# Architectural Refactoring & Modernization Roadmap

This document records the architectural refactoring history and roadmap for **Karakuri**.

---

## 1. Completed Phases Summary (P1–P155)

All refactoring phases through Phase 20 (P1–P155) are complete, verified with full workspace tests and Clippy quality gates, and documented across crate-level `README.md` files:

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
- **Phase 17 (P88–P98)**: Near-monolith modularization (1,500–1,800 lines target) across 11 files (`karakuri::tests::view_interaction`, `karakuri-ir::ast`, `karakuri-cli::src::tests::parse`, etc.), lowering pre-commit hard gate to 1,500 lines.
- **Phase 18 (P99–P116)**: Intermediate reduction (1,300–1,500 lines target) across 18 files (`karakuri-environment::mix::tests`, `karakuri-cli::live::interactive`, `karakuri-engine::tests::binding`, `karakuri-console::tests::mixer`, etc.), lowering pre-commit hard gate to 1,300 lines.
- **Phase 19 (P117–P155)**: Final monolith elimination across all 39 files in the 1,000–1,300 line cohort across all crates (`karakuri-cli`, `karakuri`, `karakuri-console`, `karakuri-engine`, `karakuri-environment`, `karakuri-mcp`, `karakuri-midi`). All files across the entire workspace brought strictly under 1,000 lines.
- **Phase 20**: Pre-commit hard gate lowered to 1,000 lines (`hard_limit=1000`) and 800-line warning threshold established in `.githooks/pre-commit`, achieving 100% compliance with ADR-0345 (1,000-line ceiling).

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
│ Stage 3: Target <1,300 Lines (Phase 18) (COMPLETED)                     │
│ - All 18 files brought under 1,300 lines (P99–P116).                    │
│ - Hard gate lowered to 1,300 lines in .githooks/pre-commit.             │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Stage 4: Monolith Elimination Below 1,000 Lines (Phase 19) (COMPLETED)  │
│ - All 39 files in 1,000–1,300 line range decomposed (P117–P155).        │
│ - Exactly 0 files >= 1,000 lines remain across the entire codebase.     │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Stage 5: Final Architectural Ceiling (Phase 20) (COMPLETED)             │
│ - Lower pre-commit hard gate to 1,000 lines (hard_limit=1000).          │
│ - Establish 800-line warning threshold in .githooks/pre-commit.         │
│ - Complete compliance with ADR-0345 (1,000 lines single-file ceiling).  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Post-Ceiling Architecture & Maintenance

Now that all source and test files across all 16 crates are strictly under 1,000 lines and enforced by the git pre-commit hook:

### 3.1 Maintenance Guidelines
- **Ceiling Enforcement**: Any commit introducing a Rust file exceeding 1,000 lines is blocked by `.githooks/pre-commit`.
- **Early Nudge Threshold**: Any commit expanding a file past 800 lines emits a warning nudge to encourage proactive modularization before reaching the hard ceiling.
- **Submodule Organization**: New functionality should be structured into focused domain submodules (`common.rs`, `geometry.rs`, `controls.rs`, etc.) rather than appended onto existing files.

### 3.2 Future Initiatives
- **Workspace Test Fixture & Helper Consolidation**: Consolidate repetitive test boilerplate (`compile`, `render`, `console`, `point`/`at`, `f16`) into shared test utilities.
- **Dynamic Module Hot-Reloading Ergonomics**: Extend `.kir` hot-reloading abstractions across non-shader resource bundles.
- **Unified Event Journal Introspection**: Standardize tooling for offline inspection and diffing of `.ndjson` session streams.
