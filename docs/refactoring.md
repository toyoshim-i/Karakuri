# Architectural Refactoring & Design Smells Report (Phase 3 & Phase 4)

This document records the architectural smells, structural friction, and design distortions identified in the **Karakuri** codebase, along with the completion record for **Phase 3 (P21–P26)** and the implementation roadmap for **Phase 4: UI Componentization, Unified Control Descriptors & Decoupled Interaction (P30–P33)**.

---

## 1. Executive Context & Ground-Truth Baseline

### The Transition from Phase 2 to Phase 3
- **Phase 1 & Phase 2 (P1–P20) [COMPLETED]**:
  - Addressed critical hot-path and simulation constraints: transient render graph DAG allocation (P15), structured WGSL AST & pass fusion (P16), machine-readable AI diagnostics (P17), zero-allocation stream replay (P19), and unified keymap convergence (P20).
  - Executed emergency first-pass triage: dismantled the original 33.1k-line `karakuri/src/main.rs` into submodules (`engine_bridge.rs`, `app.rs`, `readout.rs`, etc.), split `karakuri-console/src/view.rs` (24.2k lines) into bay modules, and extracted `karakuri-mcp` into its own package.
- **The Limit of Raw File Decomposition**:
  - The first-pass triage made file boundaries visible, but several monolithic modules and procedural adapters remain:
    - `crates/karakuri-cli/src/main.rs`: **12,300 lines** (untouched scaffolding monolith)
    - `crates/karakuri-mcp/src/lib.rs`: **10,888 lines** (6,254 lines of production code, 4,634 lines of tests)
    - `crates/karakuri/src/engine_bridge.rs`: **8,309 lines** (procedural bridge combining 4 distinct responsibilities)
    - `crates/karakuri-console/src/view/mod.rs`: **7,493 lines** (bay orchestrator coupled with shared widget drawing)
  - [ADR-0345](adr/0345-a-file-that-crosses-1000-lines-gets-a-nudge-not-a-gate.md) established that crossing 1,000 lines is a *nudge, not a gate*. The size is a measurable indicator; **the work of Phase 3 is resolving the structural coupling, duplicated orchestration, and cohesive module boundaries.**

---

## 2. Analysis of Core Structural Smells (Ground-Truth Corrected)

### Smell 1: Layer Primitive Duplication & Conversion Scattering
- **Phenomenon**: The 6-variant pipeline layer (`L1`, `L2`, `L3`, `L4`, `Field`, `L5`) is independently defined as an enum across three crates:
  1. `karakuri_ir::ast::Kind` (internal AST/IR type)
  2. `karakuri_operation::Layer` (strictly zero-dependency vocabulary leaf; derives `Debug, Clone, Copy, PartialEq, Eq`)
  3. `karakuri_store::record::Layer` (persistent `.ndjson` schema; derives `Hash, Serialize, Deserialize`)
- **Impact**: Because derive sets and dependency requirements deliberately differ (per `karakuri-operation`'s charter of having zero dependencies), conversions between them are duplicated across:
  - `karakuri-environment/src/meta.rs`: 4 functions (`kind_of`, `layer_of`, `kind_name`, `layer_named`)
  - `karakuri/src/engine_bridge.rs`: `asked_layer`, `ir_layer`, and string parser `kind_of`
  - `karakuri-mcp/src/lib.rs`: `layer_of`, `kind_of`, `layer_name`, `layer_named`
  - `karakuri-environment/src/midi.rs`: `layer_of`
  - `karakuri-engine/src/set.rs`: `kind_of_layer`
- **Key Insight**: String parsing (`FromStr` / `layer_named`) is functionally distinct from enum-to-enum mapping. Any unification must preserve `karakuri-operation`'s zero-dependency invariant and maintain strict serialized JSON compatibility for `record::Layer`.

### Smell 2: Dual Orchestration Duplication (CLI vs. GUI)
- **Phenomenon**: `karakuri-cli` remains a 12,300-line single-file binary. Both `karakuri-cli` (via its `Live` struct and event loop) and `karakuri` GUI (via `app.rs` and `engine_bridge.rs`) independently coordinate:
  - Multi-slot deck state, residency switching, and Set building.
  - Background `HotSwap` compilation worker channels and atomic swap.
  - Preset directory scanning, scratch space synchronisation, and Set file loading.
  - Session recording (`ndjson`) and replay driving.
- **Impact**: Maintenance overhead is doubled; capabilities divergence (e.g. headless session replay working only in CLI) persists.

### Smell 3: Multi-Responsibility Congestion in `engine_bridge.rs` (8,309 lines)
- **Phenomenon**: `karakuri-console` keeps `wgpu::Device` out of its dependencies so that arrangement and panel logic can be tested on machines without a GPU adapter (ADR-0156). To wire this console to the actual runtime device, `engine_bridge.rs` was created in `karakuri`. However, it has accumulated **four entirely different responsibilities**:
  1. **Render Targets & egui Sinks**: Managing `Presented`, `wgpu::TextureView`, resize handling, and egui user texture registrations (`Sink` implementation).
  2. **Engine Lifecycle & Coordination**: Deck instantiation, step advancement, HotSwap polling, and Governor budget verification.
  3. **Filesystem & Asset Discovery**: `Taking`, Presets directory scanning, folder drag-and-drop ingestion, and Set loading.
  4. **Operation Dispatch & Reconciliation**: Transforming UI interaction outcomes into `Operation` / `Record` batches and updating panel state.
- **Key Insight**: The ~8,300 lines cannot be eliminated by introducing intermediate snapshots (which would re-introduce render-thread heap allocations via `Arc`). Instead, this monolithic file must be decomposed by responsibility into dedicated submodules.

### Smell 4: `karakuri-mcp` Monolithic Packaging (10,888 lines)
- **Phenomenon**: `crates/karakuri-mcp/src/lib.rs` contains the entire MCP subsystem in a single file:
  - **Production Code (6,254 lines)**:
    - Transport & Server: JSON-RPC parsing, TCP stream handling, HTTP header checks, client session state (~2,430 lines).
    - Declarative Operation Schema (`SPELLED` array): JSON schema definitions, sample constructors, and argument parsers for all 64 operations (~1,940 lines).
    - Tool Implementations & Handlers: 10 tools (`check_procedure`, `check_set`, `read_procedure`, `write_procedure`, `wire_input`, `swap_outcome`, `read_set`, `list_sets`, `walk_history`, `save_set`, `operate`) and resource endpoints (`vocabulary`) (~1,880 lines).
  - **Test Suites (4,634 lines)**:
    - `mod wire_tests` (lines 6,255–9,194): Socket-level integration test suite (**2,940 lines**).
    - `mod tests` (lines 9,195–10,888): In-memory unit test suite (**1,694 lines**).

### Smell 5: Console UI Procedural Spaghetti & Three Abandoned Bays (`view/mod.rs`)
- **Phenomenon**: While `karakuri-console`'s underlying model layer (`Layout`, `Panel`, `Focus`, `Input`, `Hover`) is rigorously structured as toolkit-independent, deterministic state machines, its view presentation layer (`view/` totaling 26,000 lines) abandons componentization in favor of hundreds of procedural, low-level painter routines (`*_into(ui, pal, ...)`).
- **The Critical Oversight in First-Pass Triage**: When `view.rs` (24.2k lines) was originally split, 5 bay files were extracted (`inspector`, `library`, `transport`, `mixer`, `program`), but **three entire bays were left abandoned inside `view/mod.rs`**:
  1. **Sequencer Bay** (lines 3,130–4,134, **~1,000 lines**): `Sequencer`, `LaneCard`, `SeqRow`, `Choices`, `lane_card_into`, `sequencer_into`.
  2. **Staging Bay** (lines 4,135–4,836, **~700 lines**): `Stage`, `Candidate`, `StagingBay`, `staging_into`.
  3. **Master Bay** (lines 2,497–3,129, **~630 lines**): `MasterRow`, `FxRow`, `Chain`, `master_into`, `fx_into`.
- **Shared Widget Inlining**: Reusable UI controls (Head bars, Capsules, McpPills, FoldGrips, Outputs, Fader/Meter primitives) and geometry solvers (`plan_into`, `track`, `held_inside`) remain inlined in `mod.rs` (lines 800–2,496, **~1,700 lines**), preventing modular testing and creating a 7,493-line bottleneck.

---

## 3. Phase 3 Modernization Initiatives

```mermaid
graph TD
    P26["<b>P26: Componentize Console UI & Extract 3 Bays</b><br/>Extract sequencer, staging, master, and widgets [COMPLETED]"]
    P25["<b>P25: Decompose karakuri-mcp Monolith</b><br/>Separate server, spelled, tools, and test suites [COMPLETED]"]
    P24["<b>P24: Decompose engine_bridge.rs by Responsibility</b><br/>Split sinks, engine, filesystem, and handlers [COMPLETED]"]
    P22["<b>P22: Extract Headless Runtime Orchestrator</b><br/>karakuri-runtime / slim CLI scaffolding [COMPLETED]"]
    P21["<b>P21: Consolidate Layer Conversions & Contracts</b><br/>Centralize mappings; preserve zero-dep & serde compatibility [COMPLETED]"]
    P23["<b>P23: Rationalize karakuri-environment Boundaries</b><br/>Decompose monoliths, resolve couplings, and isolate tests [COMPLETED]"]

    P26 --> P24
    P25 --> P21
    P24 --> P22
    P21 --> P22
```

---

### P26. Componentize Console UI, Extract 3 Bays & Shared Widgets [COMPLETED]

#### Phenomenon
`crates/karakuri-console/src/view/mod.rs` (7,493 lines) remains a massive bottleneck because three major bays (Sequencer, Staging, Master), shared interactive widgets, and layout placement math were never extracted.

#### Refactoring Plan
1. **Extract the 3 Abandoned Bays into Dedicated Modules**:
   - `crates/karakuri-console/src/view/sequencer.rs`: `Sequencer`, `LaneCard`, `SeqRow`, `Choices`, `lane_card_into`, `sequencer_into` (~1,000 lines).
   - `crates/karakuri-console/src/view/staging.rs`: `Stage`, `Candidate`, `StagingBay`, `staging_into`, `staging_box` (~700 lines).
   - `crates/karakuri-console/src/view/master.rs`: `MasterRow`, `FxRow`, `Chain`, `master_into`, `fx_into` (~630 lines).
2. **Extract Shared Interactive Widgets into `crates/karakuri-console/src/view/widgets/`**:
   - `head.rs`: `Head`, `HeadWords`, `head_capsule`, `bank_capsules` (~750 lines).
   - `pills.rs`: `McpPill`, `SinkChip`, status indicators (`pill_into`, `pill_at`) (~650 lines).
   - `fold_grip.rs`: `FoldGrip`, `bay_grip`, `grip_dots` (~500 lines).
   - `outputs.rs`: `Outputs`, `OutputsRow`, master level monitors, routing selectors (~500 lines).
   - `fader.rs`: Common fader / meter drawing primitives (`fader_into`, `meter_into`, `fader`, `grabbed`) (~400 lines).
3. **Extract Geometry Placement into `crates/karakuri-console/src/view/layout.rs`**:
   - Layout helpers: `plan_into`, `track`, `held_inside`, `positive`, `filled` (~400 lines).
4. **Retain `src/view/mod.rs` as Clean Orchestrator**:
   - Top-level bay dispatch, `View` struct definition, and public re-exports (~600–800 lines).

---

### P25. Subsystem Decomposition of `karakuri-mcp` [COMPLETED]

#### Phenomenon
`crates/karakuri-mcp/src/lib.rs` (10,888 lines) houses server plumbing, 1,940 lines of `SPELLED` operation tables, 10 distinct tools, and 4,634 lines of test suites in one file.

#### Refactoring Plan
1. **Relocate Test Suites to Dedicated Integration Test Files**:
   - Move `mod wire_tests` (lines 6,255–9,194, 2,940 lines) to `crates/karakuri-mcp/tests/wire.rs`.
   - Move `mod tests` (lines 9,195–10,888, 1,694 lines) to `crates/karakuri-mcp/tests/unit.rs`.
   - *Immediate impact*: Reduces `lib.rs` from 10,888 lines to 6,254 lines with zero production code changes.
2. **Decompose Production Architecture**:
   - `src/lib.rs`: Public API (`serve`, `check_procedure`, `check_set_configuration`, `Reporter`, `Slots`) (~250 lines).
   - `src/protocol.rs`: JSON-RPC 2.0 framing, error codes, HTTP header parsing (`read_capped`, `is_local_origin`, `respond`) (~500 lines).
   - `src/server.rs`: TCP listener loop, thread handling, dispatch (~450 lines).
   - `src/state.rs`: Session state, permission gating, audit verification (~400 lines).
   - `src/spelled.rs`: `Spelled` struct, helper parsing combinators (`number_of`, `word_of`, etc.), and the 1,940-line `SPELLED` constant table.
   - `src/tools/`: Implementation of the 10 MCP tools:
     - `tools/mod.rs`: `call_tool` router and `tools()` schema catalog.
     - `tools/procedure.rs`: `check_procedure`, `read_procedure`, `write_procedure`.
     - `tools/set.rs`: `check_set`, `read_set`, `list_sets`, `save_set`.
     - `tools/wire.rs`: `wire_input`.
     - `tools/history.rs`: `walk_history`, `swap_outcome`.
     - `tools/operate.rs`: `operate` bridge.
   - `src/resources.rs`: `vocabulary` resource endpoint and checker reflection tables.

---

### P24. Decompose `engine_bridge.rs` by Concrete Responsibility [COMPLETED]

#### Phenomenon
`crates/karakuri/src/engine_bridge.rs` (8,309 lines) couples presentation sinks, engine lifecycle, filesystem scanning, and operation dispatch in a single monolithic file.

#### Refactoring Plan
Decompose into cohesive modules under `crates/karakuri/src/bridge/`:
1. **`bridge/sinks.rs`**:
   - `Presented` struct, `Sink` trait implementation, `Aiming`, texture format constants (`PICTURE_FORMAT`, `PICTURE_SAMPLED_FORMAT`), and egui user texture registrations (~900 lines).
2. **`bridge/engine.rs`**:
   - `Engine` struct, lifecycle initialization (`new`), frame advancement (`compose`), and HotSwap synchronization (~1,500 lines).
3. **`bridge/filesystem.rs`**:
   - `Taking` enum, `presets_listing`, `folder_files`, drag-and-drop directory scanning, and Set file resolution (~1,200 lines).
4. **`bridge/handlers.rs`**:
   - Transformation of panel UI events into `Operation` / `Record` batches and deck mutators (~1,500 lines).
5. **`bridge/mod.rs`**:
   - Facade re-exporting the bridge interface to `app.rs` and `main.rs`.

---

### P22. Extract Headless Runtime & Orchestrator (`karakuri-runtime`) [COMPLETED]

#### Phenomenon
`karakuri-cli/src/main.rs` (12,300 lines) and `karakuri` GUI (`app.rs` + `engine_bridge.rs`) independently duplicate deck management, HotSwap compilation worker channels, session recording, and transport clock logic.

#### Refactoring Plan
1. **Extract `karakuri-runtime`**:
   - Headless session controller: owns `Deck`, manages residency transitions, applies `Operation` batches, and drives step execution.
   - `HotSwapCoordinator`: manages background compilation worker threads, staging queues, budget probe verification, and atomic commit.
   - `TransportClock`: centralizes oscillator synchronisation, audio tempo corrections, and latency offset adjustments.
2. **Slim Down `karakuri-cli`**:
   - Transform `karakuri-cli` into a lightweight CLI harness (~800 lines) parsing CLI arguments, instantiating `SessionController`, and driving the event loop.

---

### P21. Consolidate Layer Conversions & Preserve Invariants [COMPLETED]

#### Phenomenon
Layer enums are duplicated across `karakuri-operation`, `karakuri-ir`, and `karakuri-store::record`. Conversion functions are scattered across `meta.rs`, `engine_bridge.rs`, and `mcp/lib.rs`.

#### Refactoring Constraints & Plan
1. **Preserve Invariants**:
   - `karakuri-operation::Layer` must remain **strictly zero-dependency** (`std` only). It cannot take dependencies on `serde` or other crates.
   - `karakuri-store::record::Layer` must retain exact JSON serialization names (`L1`, `L2`, `L3`, `L4`, `Field`, `L5`) to avoid breaking existing session streams.
2. **Refactoring Strategy**:
   - Centralize conversion utilities between `ir::ast::Kind` and `operation::Layer` into `meta.rs` as the authoritative mapping module.
   - Standardize string parsing via `FromStr` implementations rather than ad-hoc `kind_of` / `layer_named` functions.
   - Add automated schema consistency tests ensuring `record::Layer` serialization matches `operation::Layer` string representations.

---

### P23. Rationalize `karakuri-environment` Module Boundaries [COMPLETED]

#### Phenomenon
`karakuri-environment` held 14 modules totaling 20,041 lines, with heavy monoliths (`setfile.rs` at 5,156 lines, `mix.rs` at 2,810 lines) and nearly 9,000 lines of inline unit tests intertwined with production code. Furthermore, ambiguous re-exports caused external callers to bypass canonical sources (`meta.rs`).

#### Refactoring Implementation
1. **Canonical Type Routing**:
   - Routed layer conversions in `mix.rs`, `karakuri-cli`, `karakuri-mcp`, and `karakuri::bridge` directly to `karakuri-environment::meta`.
   - Added canonical `layer_name` and preserved backward-compatible re-exports with documentation directives.
2. **Decomposition of Monoliths**:
   - Decomposed `setfile/` into:
     - `types.rs` (314 lines): data representations (`Loaded`, `Node`, `Saving`, `Owned`)
     - `binding.rs` (225 lines): binding decoder, encoder, and parameter ordinals
     - `bundle.rs` (604 lines): packaging, bundling, unbundling, and `.kset` resolution
     - `summary.rs` (317 lines): store summarization, listing, and `Sources`
     - `codec.rs` (958 lines): save, load, and Set serialization pipeline
     - `tests.rs` (2,677 lines): isolated test suite
     - `mod.rs` (96 lines): slim public facade
   - Decomposed `mix/` into:
     - `shipped.rs` (57 lines): embedded example presets and addresses
     - `mod.rs` (1,333 lines): performance mix state and record change translation
     - `tests.rs` (1,415 lines): isolated test suite
3. **Test Suite Isolation**:
   - Extracted dedicated `tests.rs` submodules for `history` (945 lines), `watch` (966 lines), and `midi` (841 lines), reducing all production modules below or near ~1,000 lines.

---

---

## 4. Phase 3 Modernization Completion (P21–P26)

All initiatives of Phase 3 are 100% complete, verified with full test suites, and documented in `crates/README.md`:

| Initiative | Target Subsystem | Actionable Deliverable | Status |
|---|---|---|:---:|
| **P26** | `karakuri-console` | Extracted 3 bays (`sequencer`, `staging`, `master`), `widgets/`, and `layout.rs` | **COMPLETED** |
| **P25** | `karakuri-mcp` | Extracted tests to `tests/wire.rs` & `src/tests.rs`; decomposed `lib.rs` into `protocol`, `server`, `spelled`, `tools/` | **COMPLETED** |
| **P24** | `karakuri` (GUI) | Decomposed `engine_bridge.rs` into `bridge/` (`sinks`, `engine`, `filesystem`, `handlers`) | **COMPLETED** |
| **P22** | `karakuri-cli` / GUI | Extracted headless runtime controller; modularized `karakuri-cli` into `app`, `args`, `live`, `aiming`, `replay`, `save` | **COMPLETED** |
| **P21** | Type Conversions | Centralized layer conversions in `karakuri-environment::meta` as single source of truth | **COMPLETED** |
| **P23** | `karakuri-environment` | Decomposed monoliths (`setfile`, `mix`), resolved couplings, and isolated test suites | **COMPLETED** |

---

## 5. Phase 4: UI Componentization, Unified Control Descriptors & Decoupled Interaction (P30–P33)

### Analysis of New Structural Smells

#### Smell 6: The Quadruple-Dispatch & Fragile Hit/Tooltip Seam
- **Phenomenon**: Defining or modifying an interactive control requires updating up to 6 separate places across crates:
  1. Drawing routine in `view/<bay>.rs` (raw painter calls)
  2. Hit test probe in `karakuri-console::input::PROBES` (38 manual entries)
  3. Pointer event translation in `karakuri::readout::Readout::pointer` (1,300 lines of manual sequential dispatch)
  4. Operation translation & execution in `app.rs` / `handlers.rs`
  5. Tooltip lookup in `karakuri-console::hover::TIPS` (mapping probes to HTML CSS classes in `console.html`)
  6. Keyboard shortcut binding in `karakuri::keymap::KEY_BINDINGS` / `live.rs`
- **Impact**: Heavy duplication of control metadata; adding tooltips, keyboard hotkeys, or MCP inspection requires touching disjointed arrays with risk of subtle drift or missed synchronisation.

#### Smell 7: Console Bay Monoliths and Raw egui Painter Scattering
- **Phenomenon**: Individual bay files in `karakuri-console/src/view/` remain massive:
  - `view/inspector.rs`: **4,955 lines**
  - `view/library.rs`: **4,593 lines**
  - `view/transport.rs`: **4,130 lines**
  - `view/mixer.rs`: **2,907 lines**
  - `view/program.rs`: **1,579 lines**
- **Impact**: UI elements (chips, badges, context cards, slider tracks, search fields) are reimplemented with low-level `egui::Painter` primitives in each bay rather than using componentized, reusable widgets.

#### Smell 8: Monolithic Event Accumulators (`readout.rs` & `app.rs`)
- **Phenomenon**:
  - `karakuri/src/readout.rs` (**4,754 lines**) combines memory allocation profiling, frame time sampling, HUD text generation, layout solving, and a 1,300-line `Readout::pointer` match block.
  - `karakuri/src/app.rs` (**5,067 lines**) bundles window event loops, key routing, file drop ingestion, audio monitoring, and session persistence threads.
- **Impact**: High cognitive load, merge conflict friction, and difficulty in testing event handling in isolation.

---

### Phase 4 Modernization Initiatives

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

#### P30. Expand Componentized Widget Library (`karakuri-console::view::widgets`)
Extract recurring visual and interactive elements out of bay modules into `view/widgets/`:
1. `chip.rs`: Standardized chips and badges (Tally residency chips, Blend mode chips, Scope tabs, Sync source chips).
2. `card.rs`: Reusable modal card and dropdown popup containers with drop shadow, border, and dismiss-on-outside-click logic (Audio-in card, Arrangement card, Row context menu).
3. `track.rs`: Continuous slider tracks and scrubbers (Exposure track, Latency offset track, Transport scrubber).
4. `field.rs`: Reusable interactive text and filter input fields with focus indicators and clear buttons.

#### P31. Modularize Bay Monoliths into Subdirectories
Decompose bay files exceeding 2,500 lines into focused submodules:
1. `view/inspector/` (from `inspector.rs`, 4,955 lines):
   - `mod.rs`: Bay container and layout
   - `header.rs`: Node header, residency chips, fold state
   - `params.rs`: Parameter row listing and value faders
   - `curves.rs`: Modulation curve graph painter
   - `wiring.rs`: Node connection input/output edges
2. `view/library/` (from `library.rs`, 4,593 lines):
   - `mod.rs`: Bay container and layout
   - `scopes.rs`: Scope tabs (Presets, Sets, History)
   - `search.rs`: Filter input and query matching
   - `rows.rs`: File and version rows, star actions
   - `menu.rs`: Context actions and row menus
3. `view/transport/` (from `transport.rs`, 4,130 lines):
   - `mod.rs`: Bay container and layout
   - `tempo.rs`: Tempo figure and BPM nudging
   - `grid.rs`: Beat light dots and quantization display
   - `audio.rs`: Audio input monitoring pill and popup card
   - `readouts.rs`: Budget and system health status capsules
4. `view/mixer/` (from `mixer.rs`, 2,907 lines):
   - `mod.rs`: Bay container and layout
   - `strip.rs`: Per-deck channel strip layout
   - `crossfader.rs`: Transition slider and master blend
   - `tally.rs`: Residency indicators and controls

#### P32. Unified Control Descriptor & Declarative Registry
Establish a single source of truth for interactive controls:
1. Define `ControlDescriptor`:
   - `id: ControlId`: Strongly typed control identifier
   - `label: &'static str`: Human-readable label / legend
   - `default_key: Option<BoundKey>`: Default keyboard accelerator
   - `tooltip: &'static str`: Factual documentation text
   - `operation: Option<fn(...) -> Operation>`: Target action
   - `safety_class: Option<Class>`: Operator permission gate
2. Unify consumers:
   - `hover.rs`: Render tooltips directly with dynamically embedded shortcut badges (e.g., `"[G] Fold Bay"`).
   - `keymap.rs`: Derive keyboard legend tables directly from control descriptors.
   - `karakuri-mcp`: Provide consistent parameter descriptions and action schemas.

#### P33. Decouple `Readout` and Event Dispatch in `karakuri`
Dismantle `karakuri/src/readout.rs` (4,754 lines) into `karakuri/src/readout/`:
1. `mod.rs`: `Readout` struct and state container
2. `costs.rs`: Metric / frame timing / memory allocation measurement (`Costs`, `STILL`, `SAMPLE_CAP`)
3. `hud.rs`: Legend formatting and status string generation
4. `dispatch.rs`: Delegate pointer event handling to modular bay dispatchers, replacing the monolithic 1,300-line match block in `Readout::pointer`.

---

## 6. Execution Matrix (Phase 4)

| Initiative | Target Subsystem | Actionable Deliverable | Readiness |
|---|---|---|:---:|
| **P30** | `karakuri-console` | Extract reusable widgets: `chip.rs`, `card.rs`, `track.rs`, `field.rs` | **Ready to Execute** |
| **P31** | `karakuri-console` | Decompose bay monoliths (`inspector/`, `library/`, `transport/`, `mixer/`) | **Ready to Execute** |
| **P32** | `karakuri-operation` / `console` | Implement `ControlDescriptor` registry linking UI, keys, tooltips, and MCP | **Ready to Execute** |
| **P33** | `karakuri` (GUI) | Decompose `readout.rs` into `costs`, `hud`, and modular pointer `dispatch` | **Ready to Execute** |

