# Architectural Refactoring & Modernization Roadmap

This document records the architectural refactoring history and roadmap for **Karakuri**.

---

## 1. Completed Phases (P1–P37)

All prior refactoring phases are complete, verified with full workspace tests, and documented across crate-level `README.md` files:

- **Phases 1 & 2 (P1–P20)**: Transient render graph DAG, WGSL AST & pass fusion, machine-readable AI diagnostics, zero-allocation stream replay, and keymap convergence.
- **Phase 3 (P21–P26)**: Subsystem decomposition, monolith extraction, layer type centralization, full crate-level `README.md` documentation, and Google Style comment standardization across all 16 crates.
- **Phase 4 (P30–P33)**: Console UI componentization (`card`, `chip`, `track`, `field`), bay monolith decomposition (`mixer/`, `transport/`, `library/`, `inspector/`), unified `ControlDescriptor` registry in `karakuri-console::control`, and `Readout` event dispatch decoupling into `readout/` (`costs`, `hud`, `dispatch`, `mod`).
- **Phase 5 (P34–P37)**: 3-tier hierarchical architecture documentation (`docs/architecture/`), GUI application event loop modularization (`karakuri::app`), control-aware dynamic tooltip shortcut badges in `karakuri-console::hover`, and engine Set execution decomposition (`karakuri-engine::set`).

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

---

## 2. Future Initiatives

Future refactoring and architectural enhancements will be recorded here as new requirements emerge.
