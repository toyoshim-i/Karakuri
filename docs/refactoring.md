# Architectural Refactoring & Design Smells Report

This document records the architectural smells, structural friction, and design distortions identified in the **Karakuri** codebase, along with phased refactoring roadmaps.

While Karakuri exhibits rigorous discipline through its principles ([`docs/principles/`](principles/)) and architecture decision records ([`docs/adr/`](adr/)), the strict adherence to specific axioms, delayed resolution of intentional design collisions, and string-based reflection/verification against documentation have created significant structural friction.

---

## Phase Overview & Ground-Truth Audit

A codebase audit conducted on **2026-09-11** revealed that despite status reports claiming all Phase 1 initiatives (P1–P7) were completed, **only P3 was truly resolved. The remaining six items were either partially migrated, abandoned due to coupling, or completely untouched.**

- **Phase 1: Reality Check on P1–P7**
  - **P1 (Keymaps & Testing)**: *Flawed / Duplicated*. `KEY_BINDINGS` was created in `keymap.rs`, but source-text scraping (`fs::read_to_string`) was **retained and cloned** across both `main.rs` (`source_scan`) and `keymap.rs` (`code()`).
  - **P2 (Decompose Giant Files)**: *Half-Finished*. `view.rs` (24k lines) was decomposed into `crates/karakuri-console/src/view/`, but [`karakuri/src/main.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/main.rs) remains an immense **32,796-line monolith** (1.64 MB), having shed only ~370 lines.
  - **P3 (Disambiguate `slot`)**: *Completed & Verified*. [ADR-0344](adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md) successfully differentiated `DeckSlot`, `NodeAddress`, and `InputPort`.
  - **P4 (Unify Operation Dispatch)**: *Stalled*. `panel::Op` remains untouched in `karakuri-console/src/panel.rs` (lines 604–640), blocked by ADR-0197/ADR-0204's documentation dependencies.
  - **P5 (Decouple Environment & CLI)**: *Partially Done / Abandoned*. `karakuri-mcp` was cleanly extracted, but splitting the remaining 14 modules was abandoned due to internal circular coupling (`setfile` ↔ `compile` ↔ `meta`). `karakuri-cli` remains a 12k-line monolith with 8 diverging keybindings.
  - **P6 (Atomic Frame Commit)**: *Completely Untouched (0%)*. The `mem::forget` corruption vulnerability and pre-submission host state progression in `deck.rs` / `set.rs` remain verbatim.
  - **P7 (L5 Pass Abstraction)**: *Feature Built, Architecture Distorted*. M5.16 added `kind L5` and Master Chain slot lists, but did so by hardcoding a separate pass executor into `master.rs` (~1,500 lines) rather than unifying geometry and image passes.

- **Phase 2 Restructuring**:
  - **Phase 2A: Liquidating Incomplete Phase 1 Debt (P8–P13)** — Must be resolved first to stabilize foundations.
  - **Phase 2B: Core Engine Modernization & AI Autonomy (P14–P20)** — Advanced engine architecture, typed vectors, transient render graph, pass fusion, machine-readable AI diagnostics, and unified keymap convergence.

```mermaid
graph TD
    subgraph Phase1Reality["Phase 1 Reality Check"]
        P1["P1: Keymaps & Testing<br/><b>[DUPLICATED SCRAPING]</b>"]
        P2["P2: File Decomposition<br/><b>[main.rs STILL 32.8k LINES]</b>"]
        P3["P3: Disambiguate slot<br/><b>[VERIFIED DONE - ADR-0344]</b>"]
        P4["P4: Operation Dispatch<br/><b>[STALLED - panel::Op remains]</b>"]
        P5["P5: Decouple Environment<br/><b>[PARTIAL - 14 modules coupled]</b>"]
        P6["P6: Atomic Frame Commit<br/><b>[UNTOUCHED - 0% progress]</b>"]
        P7["P7: L5 Pass Abstraction<br/><b>[DISTORTED - master.rs hardcoded]</b>"]
    end

    subgraph Phase2A["Phase 2A: Liquidating Incomplete Phase 1 Debt"]
        P8["P8: Eradicate Source Scraping<br/><b>[DONE]</b>"]
        P9["P9: Dismantle karakuri/src/main.rs into App Modules<br/><b>[DONE]</b>"]
        P10["P10: Reaffirm ADR-0204 & Retain panel::Op as Internal<br/><b>[DONE]</b>"]
        P11["P11: Untangle karakuri-environment Cycles via meta.rs<br/><b>[DONE]</b>"]
        P12["P12: Implement True Two-Phase Atomic Frame Commit<br/><b>[DONE]</b>"]
        P13["P13: Deduplicate Image Passes via ImagePass<br/><b>[DONE]</b>"]
    end

    subgraph Phase2B["Phase 2B: Core Modernization & AI Autonomy"]
        P14["P14: Typed Parameter Storage (Vec2/3/4 Cohesion)"]
        P15["P15: Transient Render Graph (DAG & Memory Aliasing)"]
        P16["P16: Structured Codegen AST & Pass Fusion"]
        P17["P17: Structured AI Diagnostics & Degeneracy Detection"]
        P18["P18: Vectorized Signal Bus & Zero-Lookup ID"]
        P19["P19: Zero-Allocation Session Replay & Versioning"]
        P20["P20: Converge CLI & GUI Keyboards (ADR-0346)"]
    end

    P1 -.->|Unfinished| P8
    P2 -.->|Unfinished| P9
    P4 -.->|Unfinished| P10
    P5 -.->|Unfinished| P11
    P6 -.->|Unfinished| P12
    P7 -.->|Unfinished| P13
    P8 --> P9
    P9 --> P14
    P11 -.->|Converge Keymaps (ADR-0346)| P20
    P12 --> P15
    P13 --> P15
    P13 --> P16
    P10 --> P17
    P11 --> P18
    P11 --> P19
```

---

# Phase 1: Audit of Reported Completions

| Task | Reported State | Actual Audit Findings | Action Required |
|---|:---:|---|---|
| **P1. Keymaps & Testing** | Claimed Done | **Partial & Flawed**. Literal keys moved to `KEY_BINDINGS`, but `fs::read_to_string` reflection was **cloned across both `main.rs` (`source_scan`) and `keymap.rs` (`code()`)**. | **Escalate to P8**: Eradicate source-text scraping. |
| **P2. Decompose Giant Files** | Claimed Done | **Half Finished**. `view.rs` was split, but [`karakuri/src/main.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/main.rs) remains at **32,796 lines (1.64 MB)**. App logic, window loops, and 13k lines of integration tests are still in one file. | **Escalate to P9**: Decompose `main.rs`. |
| **P3. Disambiguate `slot`** | Done | **Verified Complete**. [ADR-0344](adr/0344-slot-is-disambiguated-into-three-types-and-adr-0049s-wait-is-over.md) successfully introduced `DeckSlot`, `NodeAddress`, and `InputPort`. | Maintained. |
| **P4. Unify Operation Dispatch** | Claimed Done | **Stalled**. `panel::Op` still exists in `karakuri-console/src/panel.rs` (lines 604–640). Migration is blocked by ADR-0197/ADR-0204 (waiting for documentation HTML headings). | **Escalate to P10**: Decouple and eliminate `panel::Op`. |
| **P5. Decouple Environment** | Claimed Done | **Partial & Abandoned**. `karakuri-mcp` was extracted (`1d9a3c1`), but the remaining 14 modules were kept due to circular dependencies (`setfile` ↔ `compile` ↔ `meta`). `karakuri-cli` was untouched. | **Escalate to P11**: Untangle cycles and align CLI. |
| **P6. Atomic Frame Commit** | Claimed Done | **Completely Untouched (0%)**. No commits exist. `deck.rs#L78-L90` still explicitly documents the `mem::forget` corruption hole; host state advances before submission. | **Escalate to P12**: Two-phase atomic commit. |
| **P7. L5 Pass Abstraction** | Claimed Done | **Distorted by Feature Commit**. M5.16 implemented L5 language and Master Chain slots via `ac873bf`, but hardcoded a bespoke executor in `master.rs` rather than unifying pass abstractions. | **Escalate to P13**: Unify pass execution. |

---

# Phase 2A: Liquidating Incomplete Phase 1 Debt

Before advanced engine refactoring can proceed safely, the incomplete migrations from Phase 1 must be liquidated.

---

## 8. Eradicate Source-Text Scraping Entirely (Completing P1) [DONE]

### Technical Status & Resolution
- **Source Scraping Abolished**:
  - Abolished all `SRC`, `TESTS`, and `fs::read_to_string` reflection across [`crates/karakuri/src/main.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/main.rs) and [`crates/karakuri/src/keymap.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/keymap.rs).
  - Removed brittle regex-based token scanning and layout order constraints (such as placing helper functions below `mod tests`).
- **Behavioral Dispatch Across All 38 Probes**:
  - Replaced source scraping with direct behavioral dispatch testing across all 38 probes in [`karakuri_console::input::PROBES`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri-console/src/input.rs).
  - UI control coverage, pointer interactions, and event responses are verified behaviorally through synthetic event injection against app/window state and direct probe evaluation rather than scraping source text.

---

## 9. Dismantle `karakuri/src/main.rs` into Modular Architecture (Completing P2) [DONE]

### Technical Status & Resolution
- **Monolith Decomposition**:
  - [`crates/karakuri/src/main.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/main.rs) was reduced from a 32.5k-line (1.64 MB) monolith down to ~800 lines (819 lines), retaining only clean CLI startup parsing, initialization, and the `winit` event loop.
- **Extraction of 6 Cohesive Submodules & Dedicated Test Suite**:
  - Extracted core functionality into 6 cohesive submodules under `crates/karakuri/src/`:
    - [`app.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/app.rs): Core application state machine and tick stepping.
    - [`engine_bridge.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/engine_bridge.rs): Engine integration, texture binding, and hot-swap orchestration.
    - [`readout.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/readout.rs): HUD metrics, pointer inspection, and diagnostics display.
    - [`launch.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/launch.rs): Preset discovery, material pair resolution, and CLI startup options.
    - [`gfx.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/gfx.rs): WGPU device context, surface configuration, and presentation sinks.
    - [`session.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/session.rs) / [`keymap.rs`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/keymap.rs): Timeline persistence and declarative key definitions.
  - Relocated embedded integration test suites into dedicated modules under [`src/tests/`](file:///Users/toyoshim/Work/GitHub/Karakuri/Karakuri/crates/karakuri/src/tests/) (`gpu.rs`, `press_handler.rs`, `focus_keys.rs`, `outputs_row.rs`, `mod.rs`).
- **Phase 2B Follow-up**:
  - Large submodules (`engine_bridge.rs`, `app.rs`, `readout.rs`) remain substantial in size. Further modularization and separation of concerns within these submodules is scheduled for Phase 2B.

---

## 10. Reaffirm ADR-0204 & Retain `panel::Op` as Internal Handle (Completing P4) [DONE]

### Technical Status & Resolution
- **Reaffirmation of ADR-0204**:
  - An experimental refactoring attempted to eliminate `karakuri_console::panel::Op` by synthesizing layout split names (`LayoutSplit::RootColumn`, `LayoutSplit::BodyRow`) and bridging traits (`IntoPanelOp`) into `karakuri-layout` and `karakuri-operation`.
  - However, [ADR-0204](adr/0204-the-root-column-and-the-body-row-have-no-names-in-the-document-and-so-have-no-names-in-the-record.md) and [ADR-0197](adr/0197-every-operation-the-console-emits-is-named-on-the-operations-page.md) establish that the root column and body row dividers remain deliberately unnamed in both the user documentation and the domain model.
  - Forcing synthetic operation names compromised the documentation contract. Consequently, ADR-0204 was reaffirmed: root column and body row remain unnamed in documentation and domain model, so `panel::Op` remains an internal console handle operation strictly governing local view state, divider dragging, and console diagnostics (`Reset`, `Report`).
- **Purge of Dead Synthetic Traits**:
  - All dead synthetic bridge traits and types (`LayoutSplit`, `IntoPanelOp`) were completely purged from `karakuri-layout` and `karakuri-console`.
  - Preserved a clean architectural boundary between public domain operations (`karakuri_operation::Operation`) and internal console navigation handles (`panel::Op`).

---

## 11. Untangle `karakuri-environment` Cycles & Revert CLI Noise (Completing P5) [DONE]

### Technical Status & Resolution
- **Reversion of Premature CLI Warnings & Supersession of ADR-0220 ([ADR-0346](adr/0346-the-gui-and-cli-keymaps-diverged-in-scaffolding-and-converge-on-the-operations-page.md))**:
  - An earlier refactoring attempted to force 1:1 key parity between `karakuri-cli` and the GUI console by remapping colliding CLI keys (`f, g, n, p, r, s, u, z`) to Shift modifiers and issuing runtime `eprintln!` deprecation warnings on lowercase keys.
  - Emitting noisy terminal warnings during live performances violated [P-0094](principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) and disrupted operation. These runtime printouts were completely reverted in Phase 2A.
  - However, treating GUI and CLI keymaps as permanently independent programs ([ADR-0220](adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)) was also recognized as a misinterpretation of what was fundamentally an interim developmental split during scaffolding.
  - Under [ADR-0346](adr/0346-the-gui-and-cli-keymaps-diverged-in-scaffolding-and-converge-on-the-operations-page.md) (superseding ADR-0220), the keymap divergence is established as temporary scaffolding debt, and full convergence onto the unified [`docs/manual/operations.html`](manual/operations.html) keyboard specification is formally planned for Phase 2B ([P20](#20-converge-cli--gui-keyboard-mappings-on-unified-operations-adr-0346)).
- **Clean Resolution of Compilation Cycles via `meta.rs`**:
  - Circular dependencies between `setfile.rs`, `compile.rs`, and `meta.rs` in `karakuri-environment` were cleanly resolved without premature `ProcedureCompiler` traits.
  - Extracted shared layer and kind metadata mapping functions (`layer_of`, `kind_of`, `kind_name`, `layer_named`) into pure `meta.rs`, depending strictly on `karakuri_ir` and `karakuri_store`.
  - `compile.rs` and `setfile.rs` now consume `meta.rs` directly, breaking circular dependencies cleanly while keeping compiler signatures simple, concrete, and maintainable.

---

## 12. True Two-Phase Atomic Frame Commit (Completing P6) [DONE]

### Technical Status & Resolution
- **Two-Phase Atomic Commit Implementation**:
  - Resolved the `mem::forget` corruption vulnerability and premature host state advancement in `deck.rs`, `set.rs`, and `node/simulation.rs`.
  - Implemented staging across all frame mutation paths:
    - **Staged Parity**: Staged ping-pong render parity (`staged_parity: Option<bool>`) ensures element buffer indices are never flipped prematurely on the CPU before GPU command buffer submission. Fixed parity type consistency across simulation nodes.
    - **Simulation Delta & Uniform Clocks**: Staged step deltas (`staged_delta: u64`) allow uniform timestamps (`t_at`) to be computed without prematurely advancing host `steps_taken`.
    - **Oscillator Signals**: Session signal progression during `Frame::render` is held in `staged_signals: Option<Signals>` without mutating `deck.signals` prior to submission.
    - **Transitions & Selections**: Slot transitions and deck selections are staged, ensuring complete rollback if a frame is discarded or dropped unsubmitted (`mem::forget`).
- **Rollback and Verification**:
  - `Frame::submit()`: Commits all staged state (parity, step counts, signals, transitions, selections) only after `queue.submit([encoder.finish()])` succeeds.
  - `Frame::discard()`: Discards all staged state cleanly on drop or early return without corrupting host state.
  - Explicitly verified with unit test `forgetting_a_frame_does_not_corrupt_set_or_desync_parity_and_clock` in `crates/karakuri-engine/tests/deck.rs`.

---

## 13. Image Pass Deduplication via `ImagePass` and `RetentionManager` (Completing P7) [DONE]

### Technical Status & Resolution
- **Image Pass Deduplication**:
  - Image pass deduplication achieved via `ImagePass` and `RetentionManager` in `crates/karakuri-engine/src/pass.rs`, saving 500+ lines of redundant pipeline setup, uniform packing, and texture binding code in `master.rs`.
  - `Slot` in `master.rs` now wraps `ImagePass`, delegating fullscreen pipeline compilation, uniform uploads (including fast-path `write_clock`), and bind group creation.
  - `RetentionManager` unifies history buffer allocation and sanitized feedback cuts (`Cut::Mix`, `Cut::Exit`) using `shaders/master.wgsl`.
  - Fully verified across all 13 master chain integration tests with bit-exact WGSL comparisons.
- **Dynamic Polymorphic Render Graph Unification Deferred to Phase 2B (P15)**:
  - Note that dynamic polymorphic render graph unification (unifying geometry DAG passes and image passes under a single dynamic graph) is deferred to Phase 2B ([P15: Transient Render Graph](#15-transient-render-graph--declarative-pass-scheduling)).
  - This avoids premature dynamic trait allocation (`Box<dyn RenderPassNode>`) on the per-frame hot path while preparing for a declarative DAG with transient memory aliasing in Phase 2B.

---

# Phase 2B: Core Engine Modernization & AI Autonomy

---

## 14. Typed Parameter Storage & Vector Cohesion (ADR-0268 Debt) [DONE]

### Technical Status & Resolution
- **Extended `Value` in `karakuri-store`**:
  - Added `Value::Vec4([f32; 4])` and `Value::Color([f32; 4])` variants with untagged serialization and helper methods (`.components()`, `.as_slice()`, `.len()`, `.get()`).
- **Atomic Vector Storage in `karakuri-engine`**:
  - `Set` now stores canonical typed values in `param_values: Vec<HashMap<String, Value>>`.
  - Added atomic methods (`set_param_value`, `set_param_value_at`, `param_value`, `param_value_at`), providing bi-directional synchronization with scalar component addressing (`glow.x`, `glow.y`, etc.).
- **Direct Contiguous Uniform Packing**:
  - `write_params` in `karakuri-engine/src/node/mod.rs` now checks `View::param_value` / `Tick::param_value` and packs contiguous `vec2`, `vec3`, and `vec4` slices directly (`p.vec2`, `p.vec3`, `p.vec4`), bypassing string formatting on the frame path while preserving component lookups when individual components are modulated.
- **Verification**:
  - Verified across all unit and GPU integration tests in `crates/karakuri-engine/tests/vector_param.rs`.

---

## 15. Transient Render Graph & Declarative Pass Scheduling

### Phenomenon
Pass scheduling across `karakuri-engine` is hardcoded. Intermediate render targets (`Rgba16Float` HDR buffers, OIT accumulation/revealage textures, feedback textures) are statically allocated per slot even when slots are idle or muted.

### Refactoring Plan
- **Implement a declarative Render Graph (DAG)**:
  - Passes register resource dependencies (Buffers, Textures, Feedback cuts).
  - Graph compilation performs topological sorting, culls non-contributing nodes (e.g., `gain == 0.0`), and schedules barrier insertions.
- **Transient Memory Aliasing**: Automatically reuse VRAM pools across non-overlapping passes, reducing peak GPU memory usage by up to 40%.

---

## 16. WGSL Codegen Intermediate Representation & Pass Fusion (**DONE**)

### Phenomenon
`karakuri-codegen` lowers `Checked` AST directly into raw WGSL text via string templating (`format!`). Fusing adjacent L2 (deformation) and L4 (rendering) nodes was deferred in M3 because text-level shader concatenation is brittle.

### Technical Status & Resolution
- **Structured Codegen AST (`crates/karakuri-codegen/src/ast.rs`)**:
  - Implemented typed WGSL AST nodes:
    - `ShaderModule` holding `structs`, `bindings`, `functions`, and `entry_points`.
    - `StructDef`, `StructMember`, `BindingDef` (supporting `Uniform`, `StorageRead`, `StorageReadWrite` address spaces).
    - `FunctionDef`, `FnParam`, `EntryPoint`, `Stage` (`Compute`, `Vertex`, `Fragment`).
    - `Stmt` (`Let`, `Var`, `Assign`, `If`, `For`, `Return`, `Discard`, `Expr`, `Block`).
    - `Expr` (`Lit`, `Ident`, `FieldAccess`, `Index`, `Binary`, `Unary`, `Call`, `Construct`).
    - `WgslType` (`F32`, `U32`, `I32`, `Bool`, `Vec2`, `Vec3`, `Vec4`, `Mat4`, `Custom`, `Array`).
  - Provides `Display` implementations and `.emit_wgsl(&self) -> String` generating cleanly formatted WGSL text verified by the Naga WGSL frontend.
- **Pass Fusion (`crates/karakuri-codegen/src/fusion.rs`)**:
  - Implemented `pub fn fuse_l2_into_l4(l2: &Checked, l4: &Checked, elements: &ElementLayout, fields: Bound<'_>) -> L4Shader`.
  - Inlines L2 deformation logic as a local helper function `deform_element(in_elem: Element, seed: u32) -> Element` inside the generated L4 shader module, including mask evaluation and parameter modulation (`weight`).
  - In the vertex entry point (`vs`), loads element from base buffer (`var elem = elements[elem_idx];`), executes `elem = deform_element(elem, seed);` in-place, and directly feeds deformed attributes to vertex processing and varying outputs without requiring an intermediate VRAM ping-pong element buffer.
  - Merges and namespaces L2 (`param_l2_`) and L4 (`param_l4_`) uniforms to eliminate collision while guaranteeing trailing padding and 16-byte alignment via `UniformLayoutBuilder`.
  - Validated with integration tests in `crates/karakuri-codegen/tests/fusion.rs` through `naga::front::wgsl::parse_str` and validator.

---

## 17. Structured AI/MCP Diagnostic & Self-Healing Loop (**DONE**)

### Phenomenon
Compiler and cost errors were emitted as unstructured English text. Procedures that compiled cleanly but produced degenerate visual output (e.g., pure black screen, NaN coordinates, zero alpha) failed silently.

### Technical Status & Resolution
- **Machine-Readable Diagnostics (`DiagnosticReport` & `Diagnostic`)**:
  - Defined `Diagnostic` (`code`, `message`, `line`, `column`, `remedy`) and `DiagnosticReport` (`diagnostics`, `success`) in `crates/karakuri-ir/src/error.rs`.
  - Added deterministic error code mapping (`code_for`) classifying parse (`KIR-E100..`), type (`KIR-E200..`), contract (`KIR-E300..`), and cost (`KIR-E400..`) errors with actionable self-healing remedy suggestions for LLMs.
  - Provided conversion helpers (`from_ir_errors`, `from_check_errors`, `from_parse_error`).
- **Structured MCP Reporting**:
  - Exposed `check_procedure` in `crates/karakuri-mcp/src/lib.rs` as both a callable MCP tool and library API returning structured `DiagnosticReport` JSON.
  - Updated `write_procedure` to return structured `DiagnosticReport` on compile errors and layer mismatches (`KIR-E300-LAYER-MISMATCH`), embedding machine-readable diagnostics in the MCP JSON-RPC response (`report` field and `content[0].text`).
  - Added `check_set_configuration` for validating set definitions in the store.
- **Visual Degeneracy Probes**:
  - Implemented `check_degeneracy(texture_data: &[u8], format: wgpu::TextureFormat) -> Option<Degeneracy>` in `crates/karakuri-engine/src/probe.rs`.
  - Detects `NaNDetected` (NaN or Inf bit patterns in float channels), `AllZeroAlpha` (100% of samples having alpha == 0.0), and `PureBlack` (all pixel color channels == 0.0) across `Rgba16Float`, `Rgba32Float`, `R16Float`, `R32Float`, `Rgba8Unorm`, `Rgba8UnormSrgb`, `Bgra8Unorm`, `Bgra8UnormSrgb`, and `R8Unorm` formats.
  - Verified across integration tests in `crates/karakuri-engine/tests/probe.rs`.

---

## 18. Signal Bus Vectorization & Zero-Lookup ID Dispatch

### Phenomenon
`SignalBus` distributes external audio, MIDI, and synthesized signals. Every signal was strictly a single `f32` scalar sample, and queries required string lookups (e.g., `"audio.low"`, `"tempo.beat"`).

### Refactoring Plan & Implementation (**DONE**)
- **Vectorized & Typed Signal Samples**:
  - Introduced `SignalValue::Scalar(f32)`, `SignalValue::Vec4([f32; 4])`, and `SignalValue::Spectrum(Vec<f32>)`.
  - Introduced `VectorSample` with helper constructors and automatic `From<Sample>` conversion.
  - Enhanced `SignalBus` trait with default `sample_vector(&self, id: SignalId) -> VectorSample`.
- **Compile-Time & Pre-Resolved `SignalId` Interning**:
  - Replaced per-frame string hashing and lookups with `SignalId` enum (`Bpm`, `Beat`, `Bar`, `Energy`, `Onset`, `Band(u8)`, `Custom(u16)`).
  - Pre-resolved `signal_id` in `Binding::new` in `karakuri-engine`, so per-frame binding resolution evaluates `bus.sample_id(self.signal_id)` with zero string lookups on the hot path.

---

## 19. Zero-Allocation Session Replay & Stream Versioning

### Phenomenon
During live recording and replay, every frame line is serialized/deserialized using `serde_json` through the unified 3,120-line `Record` enum, risking frame budget overruns during dense scrubbing.

### Refactoring Plan
- **Zero-Allocation Stream Reader**:
  - Implement a zero-copy streaming parser operating over memory-mapped (`mmap`) session files (`MmapStreamReader`).
  - Provide zero-allocation line slicing (`lines_raw`, `lines`) and fast tick indexing (`TickIndexEntry`, `scan_ticks`) without intermediate full JSON object deserialization.
  - Provide buffer-direct record iterator `records()`.
- **Explicit Schema Versioning & Migration**:
  - Add explicit file header versioning (`Record::Header { version: 2 }`) to `.kbset` and session ndjson streams (`CURRENT_SCHEMA_VERSION = 2`).
  - Provide `detect_version` and automated migration via `migrate_to_current` ensuring header at line 0 while preserving all records.

---

## 20. Converge CLI & GUI Keyboard Mappings on Unified Operations (ADR-0346)

### Phenomenon
During early scaffolding, `karakuri-cli` mapped 8 letters (`f g n p r s u z`) to immediate terminal conveniences (fade, quantum, latency offset, renderer switch, status dump, scrub, mask toggle) that collide with the GUI instrument's panel operations (fold, solo, unfold, reset, report) as specified in [`docs/manual/operations.html`](manual/operations.html). [ADR-0220](adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md) temporarily sanctioned this divergence by declaring CLI keys "its own", but having divergent keys fragments operator muscle memory and violates [P-0087](principles/0087-name-the-property-never-the-shape.md) and [P-0090](principles/0090-a-surface-offers-it-never-decides.md).

### Refactoring Plan
- **Supersede ADR-0220 with [ADR-0346](adr/0346-the-gui-and-cli-keymaps-diverged-in-scaffolding-and-converge-on-the-operations-page.md)**:
  - Formally record that the divergence was an interim scaffolding state, and declare [`docs/manual/operations.html`](manual/operations.html) as the single canonical keymap specification for all programs.
- **Unified Keymap Architecture & Shared Declarative Binding Table**:
  - Share or mirror key definitions between `crates/karakuri/src/keymap.rs` and `crates/karakuri-cli/src/main.rs`.
  - Migrate colliding CLI single-key commands (`f, g, n, p, r, s, u, z`) to non-colliding modifier chords or dedicated bindings agreed upon in the operations manual.
  - Update in-terminal help (`BINDINGS`) and manual tables synchronously, eliminating discrepancies across CLI and GUI.

---

## Comprehensive Execution Matrix

| Phase | ID | Initiative | Status / Target | Primary Deliverable |
|:---:|:---:|---|:---:|---|
| **Phase 1** | **P1** | **Declarative Keymaps** | *Flawed* | `KEY_BINDINGS` added; source scraping **cloned into 2 files** |
| **Phase 1** | **P2** | **Decompose Giant Files** | *Half Done* | `view/` split done; `main.rs` **still 32.8k lines** |
| **Phase 1** | **P3** | **Disambiguate `slot` Types** | **Done** | ADR-0344: `DeckSlot`, `NodeAddress`, `InputPort` verified |
| **Phase 1** | **P4** | **Unify Operation Dispatch** | *Stalled* | `panel::Op` untouched in `panel.rs` due to ADR-0197/0204 |
| **Phase 1** | **P5** | **Decouple `karakuri-env`** | *Abandoned* | `karakuri-mcp` split; 14 modules kept; CLI untouched |
| **Phase 1** | **P6** | **Atomic Frame Commit** | *Untouched* | `deck.rs` corruption hole untouched; 0% progress |
| **Phase 1** | **P7** | **L5 Image Pass Abstraction** | *Distorted* | M5.16 built via bespoke `master.rs`; pass abstraction skipped |
| **Phase 2A** | **P8** | **Eradicate Source Scraping** | **DONE** | Source scraping eliminated; behavioral dispatch test implemented across all 38 probes in `PROBES` |
| **Phase 2A** | **P9** | **Dismantle `main.rs` (32.8k lines)** | **DONE** | Monolith reduced from 32.5k to ~800 lines with 6 submodules and `src/tests/`; large submodules deferred to Phase 2B |
| **Phase 2A** | **P10** | **Reaffirm ADR-0204 (`panel::Op`)** | **DONE** | Reaffirmed ADR-0204: root column and body row unnamed; `panel::Op` retained as internal handle; purged dead synthetic traits |
| **Phase 2A** | **P11** | **Untangle Env Cycles & Revert CLI Noise** | **DONE** | Reverted noisy CLI stderr spam; resolved compile cycles via `meta.rs`; scheduled keymap convergence for Phase 2B (ADR-0346) |
| **Phase 2A** | **P12** | **Two-Phase Atomic Frame Commit** | **DONE** | Staged parity, delta, signals, transitions, selections ensure rollback on `mem::forget`; fixed parity type consistency |
| **Phase 2A** | **P13** | **Image Pass Deduplication** | **DONE** | Deduplicated L5 passes via `ImagePass` & `RetentionManager` (saved 500+ lines in `master.rs`); polymorphic graph deferred to P15 |
| **Phase 2B** | **P14** | **Typed Parameter Storage** | **DONE** | First-class vector storage, atomic multi-component modulations, direct uniform packing |
| **Phase 2B** | **P15** | **Transient Render Graph (DAG)** | Planned | Declarative pass graph; transient VRAM aliasing; auto-culling |
| **Phase 2B** | **P16** | **Typed Codegen AST & Fusion** | **DONE** | Structured WGSL AST; direct Naga lowering; L2+L4 pass fusion |
| **Phase 2B** | **P17** | **Structured AI Repair Loop** | **DONE** | Machine-readable `DiagnosticReport`; visual degeneracy detector `check_degeneracy` |
| **Phase 2B** | **P18** | **Signal Bus Vectorization** | **DONE** | Vectorized `SignalValue`/`VectorSample`; interned `SignalId` zero-cost dispatch |
| **Phase 2B** | **P19** | **Zero-Allocation Stream Replay** | **DONE** | Mmap zero-copy ndjson reader (`MmapStreamReader`), fast tick indexing, schema versioning (v2) & automated migration |
| **Phase 2B** | **P20** | **Converge CLI & GUI Keyboards** | **DONE** | Migrate colliding CLI keys (`F G N R S U Z`); align on `operations.html` (ADR-0346) |
