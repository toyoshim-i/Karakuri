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

## 2. Future Initiatives

- **Dynamic Module Hot-Reloading Ergonomics**: Extend `.kir` hot-reloading abstractions across non-shader resource bundles.
- **Unified Event Journal Introspection**: Standardize tooling for offline inspection and diffing of `.ndjson` session streams.
