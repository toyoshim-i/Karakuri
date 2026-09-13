# Architectural Refactoring & Modernization Roadmap

This document records the architectural roadmap for **Karakuri**, focusing on the active refactoring plan: **Phase 4 (P30–P33)**.

---

## 1. Completed Phases (P1–P26)

All prior refactoring phases are complete, verified with full workspace tests, and documented in [`crates/README.md`](../crates/README.md):

- **Phase 1 & 2 (P1–P20)**: Transient render graph DAG, WGSL AST & pass fusion, machine-readable AI diagnostics, zero-allocation stream replay, and keymap convergence.
- **Phase 3 (P21–P26)**: Subsystem decomposition, monolith extraction, layer type centralization, full crate-level `README.md` documentation, and Google Style comment standardization across all 16 crates.

| Initiative | Target Subsystem | Actionable Deliverable | Status |
|---|---|---|:---:|
| **P21** | Type Conversions | Centralized layer conversions in `karakuri-environment::meta` as single source of truth | **COMPLETED** |
| **P22** | `karakuri-cli` / GUI | Extracted headless runtime controller; modularized `karakuri-cli` | **COMPLETED** |
| **P23** | `karakuri-environment` | Decomposed monoliths (`setfile`, `mix`), resolved couplings, isolated test suites | **COMPLETED** |
| **P24** | `karakuri` (GUI) | Decomposed `engine_bridge.rs` into `bridge/` (`sinks`, `engine`, `filesystem`, `handlers`) | **COMPLETED** |
| **P25** | `karakuri-mcp` | Extracted test suites to `tests/`; decomposed `lib.rs` into `protocol`, `server`, `spelled`, `tools/` | **COMPLETED** |
| **P26** | `karakuri-console` | Extracted 3 bays (`sequencer`, `staging`, `master`), `widgets/`, and `layout.rs` | **COMPLETED** |
| **P30** | `karakuri-console::view::widgets` | Componentized `card`, `chip`, `track`, `field` in `view/widgets/` | **COMPLETED** |
| **P31** | `karakuri-console::view` | Decomposed bay monoliths into subdirectories (`mixer/`, `transport/`, `library/`, `inspector/`) | **COMPLETED** |

---

## 2. Phase 4: UI Componentization, Unified Control Descriptors & Decoupled Interaction (P30–P33)

### Core Structural Smells

#### Smell 1: Quadruple-Dispatch & Fragile Hit/Tooltip Seams
- **Phenomenon**: Defining or modifying an interactive control requires updating up to 6 separate places across crates:
  1. Drawing routine in `view/<bay>.rs` (raw painter calls)
  2. Hit test probe in `karakuri-console::input::PROBES` (38 manual entries)
  3. Pointer event translation in `karakuri::readout::Readout::pointer` (1,300 lines of manual sequential dispatch)
  4. Operation translation & execution in `app.rs` / `handlers.rs`
  5. Tooltip lookup in `karakuri-console::hover::TIPS` (mapping probes to HTML CSS classes in `console.html`)
  6. Keyboard shortcut binding in `karakuri::keymap::KEY_BINDINGS` / `live.rs`
- **Impact**: Heavy duplication of control metadata; adding tooltips, keyboard hotkeys, or MCP inspection requires touching disjointed arrays with risk of subtle drift or missed synchronisation.

#### Smell 2: Console Bay Monoliths and Raw egui Painter Scattering
- **Phenomenon**: Individual bay files in `karakuri-console/src/view/` remain massive:
  - `view/inspector.rs`: **4,955 lines**
  - `view/library.rs`: **4,593 lines**
  - `view/transport.rs`: **4,130 lines**
  - `view/mixer.rs`: **2,907 lines**
  - `view/program.rs`: **1,579 lines**
- **Impact**: UI elements (chips, badges, context cards, slider tracks, search fields) are reimplemented with low-level `egui::Painter` primitives in each bay rather than using componentized, reusable widgets.

#### Smell 3: Monolithic Event Accumulators (`readout.rs` & `app.rs`)
- **Phenomenon**:
  - `karakuri/src/readout.rs` (**4,754 lines**) combines memory allocation profiling, frame time sampling, HUD text generation, layout solving, and a 1,300-line `Readout::pointer` match block.
  - `karakuri/src/app.rs` (**5,067 lines**) bundles window event loops, key routing, file drop ingestion, audio monitoring, and session persistence threads.
- **Impact**: High cognitive load, merge conflict friction, and difficulty in testing event handling in isolation.

---

### Phase 4 Architecture & Initiatives

```mermaid
graph TD
    subgraph Controls ["Unified Control & Action Layer"]
        Registry["ControlRegistry / Descriptor<br/><i>ID, Legend, Default Key, Tooltip, Operation</i>"]
    end

    subgraph ConsoleWidgets ["karakuri-console::view::widgets"]
        Chip["chip.rs<br/><i>Tally, Blend, Scope, Kind chips</i>"]
        Card["card.rs<br/><i>Popup cards & Context menus</i>"]
        Track["track.rs<br/><i>Value tracks, Scrub, Exposure</i>"]
        Field["field.rs<br/><i>Search & Filter input fields</i>"]
        Existing["fader.rs, head.rs, pills.rs, fold_grip.rs"]
    end

    subgraph ConsoleBays ["Modular Bay Subdirectories"]
        InspBay["view/inspector/<br/><i>curves, wiring, params</i>"]
        LibBay["view/library/<br/><i>listing, search, scopes</i>"]
        TransBay["view/transport/<br/><i>tempo, grid, readout</i>"]
        MixBay["view/mixer/<br/><i>strips, crossfader</i>"]
    end

    subgraph Integration ["Cross-Cutting Consumers"]
        Hover["hover.rs<br/><i>Direct tooltip lookup from Registry</i>"]
        Keymap["keymap.rs<br/><i>Declarative bindings from Registry</i>"]
        MCP["karakuri-mcp<br/><i>Schema & operations from Registry</i>"]
    end

    Registry --> ConsoleWidgets
    Registry --> Hover
    Registry --> Keymap
    Registry --> MCP
    ConsoleWidgets --> ConsoleBays
```

#### P30. Expand Componentized Widget Library (`karakuri-console::view::widgets`) — **COMPLETED**
Extracted recurring visual and interactive elements out of bay modules into `view/widgets/`:
1. `card.rs`: `bay_card`, `popup_card`, `card_row_text`.
2. `chip.rs`: `Tally` residency enum, `badge`, `toggle_chip`, `scope_tab`, `centre_galley`.
3. `track.rs`: `slider_track_into` continuous slider tracks.
4. `field.rs`: `filter_field`, `editable_text_field`, `CARET`.

#### P31. Modularize Bay Monoliths into Subdirectories — **COMPLETED**
Decomposed all four massive bay files into cohesive modular subdirectories:
1. `view/mixer/`: `strip.rs`, `transition.rs`, `mod.rs`
2. `view/transport/`: `tempo.rs`, `audio_in.rs`, `tracker.rs`, `arrangement.rs`, `look.rs`, `mod.rs`
3. `view/library/`: `scopes.rs`, `filters.rs`, `listing.rs`, `mod.rs`
4. `view/inspector/`: `header.rs`, `params.rs`, `wiring.rs`, `mod.rs`

#### P32. Unified Control Descriptor & Declarative Registry — **COMPLETED**
Established `karakuri-console::control` single source of truth:
1. `id.rs`: `ControlId` enum covering all 38 interactive console probes.
2. `descriptor.rs` & `registry.rs`: `ControlDescriptor` and static `DESCRIPTORS` array linking probe names, human labels, hotkeys, and operation titles.
3. Aligned with `PROBES`, `TIPS`, and `keymap::KEY_BINDINGS` with automated verification tests.

#### P33. Decouple `Readout` and Event Dispatch in `karakuri` — **COMPLETED**
Dismantled `karakuri/src/readout.rs` into modular submodules under `readout/`:
1. `costs.rs`: Metric / frame timing / memory allocation measurement (`Costs`, `STILL`, `SAMPLE`, `Counting`).
2. `hud.rs`: Legend formatting, logging, and status string generation (`print_legend`, `say_drag`, `say_op`).
3. `dispatch.rs`: Pointer event translation (`Readout::pointer`) and bay event dispatch methods (`Pointer`, `Acted`).
4. `mod.rs`: `Readout` state struct and foundational panel operations.

---

## 3. Execution Matrix (Phase 4)

| Initiative | Target Subsystem | Actionable Deliverable | Readiness |
|---|---|---|:---:|
| **P30** | `karakuri-console` | Extract reusable widgets: `chip.rs`, `card.rs`, `track.rs`, `field.rs` | **COMPLETED** |
| **P31** | `karakuri-console` | Decompose bay monoliths (`inspector/`, `library/`, `transport/`, `mixer/`) | **COMPLETED** |
| **P32** | `karakuri-console` / `karakuri` | Implement `ControlDescriptor` registry linking UI, keys, tooltips, and MCP | **COMPLETED** |
| **P33** | `karakuri` (GUI) | Decompose `readout.rs` into `costs`, `hud`, and modular pointer `dispatch` | **COMPLETED** |
