# Karakuri Architecture Guide

This document describes the source code structure, multi-crate architecture, compilation and execution pipeline, threading model, and contributor guide for **Karakuri**, a GPU-native, AI-native real-time visual generation system.

---

## 1. Core Principles & Design Goals

1. **Zero Allocations & Zero Shader Compilations on the Render Thread**
   - The real-time frame render loop never allocates on the heap or compiles WGSL shaders synchronously. Pipeline compilation and buffer allocation are handled asynchronously on background threads or at initialization.
2. **Deterministic Time & Step-Based Simulation (`tick` and `dt`)**
   - The engine does not read a wall clock directly. Time advances by deterministic integer `tick` records and a fixed time step `dt` (`1.0 / 60.0`s). This guarantees bit-exact reproduction and session replays.
3. **Double-Buffered HotSwapping & Governor Rollbacks**
   - Dynamic pipeline updates (`HotSwap`) swap atomically at frame boundaries. If a newly compiled pipeline exceeds the frame budget (default 16ms), the `Governor` detects the budget overrun and automatically rolls back to the previous stable pipeline.
4. **Set Immutability & Structural Forking**
   - Live `Set` structures are never mutated in place when node topology or procedures change (with the sole exception of parameter uniform updates). Any structural change forks a new `Set`.

---

## 2. Workspace & Crate Architecture

Karakuri is structured as a Cargo workspace with 8 dedicated crates under [crates/](../crates):

```mermaid
graph TD
    CLI[karakuri-cli] --> ENGINE[karakuri-engine]
    CLI --> AUDIO[karakuri-audio]
    CLI --> MIDI[karakuri-midi]
    CLI --> STORE[karakuri-store]
    CLI --> IR[karakuri-ir]
    CLI --> SIGNAL[karakuri-signal]

    ENGINE --> CODEGEN[karakuri-codegen]
    ENGINE --> IR
    ENGINE --> SIGNAL

    CODEGEN --> IR

    AUDIO --> SIGNAL
```

### Crate Breakdown

| Crate | Path | Responsibility |
|---|---|---|
| [karakuri-ir](../crates/karakuri-ir) | `crates/karakuri-ir` | DSL (`.kir`) parsing, lexing, type checking, contract verification, and static cost estimation |
| [karakuri-codegen](../crates/karakuri-codegen) | `crates/karakuri-codegen` | WGSL shader code generation from typed AST (`Checked`), uniform struct layout computation |
| [karakuri-engine](../crates/karakuri-engine) | `crates/karakuri-engine` | `wgpu` pipeline management, `Deck`/`Set` composition, `HotSwap`, `Governor`, OIT, `Present` |
| [karakuri-signal](../crates/karakuri-signal) | `crates/karakuri-signal` | Complete signal bus (`SignalBus`), signal `confidence` tracking, local `Oscillator` |
| [karakuri-audio](../crates/karakuri-audio) | `crates/karakuri-audio` | Real-time audio capture (`cpal`), FFT band analysis, beat tracking, latency offset management |
| [karakuri-midi](../crates/karakuri-midi) | `crates/karakuri-midi` | MIDI input event parsing and signal/parameter binding |
| [karakuri-store](../crates/karakuri-store) | `crates/karakuri-store` | Content-addressed artifact storage (keyed by `.kir` hash), `.set` files, `.ndjson` session logs |
| [karakuri-cli](../crates/karakuri-cli) | `crates/karakuri-cli` | Application entry point, `winit` event loop, hot-reloading watcher, MCP server integration |

---

## 3. Compilation & Execution Pipeline

The journey from a `.kir` file to rendered GPU pixels spans 8 explicit stages:

```mermaid
sequenceDiagram
    autonumber
    participant DSL as .kir file
    participant IR as karakuri-ir
    participant CG as karakuri-codegen
    participant ENG as karakuri-engine
    participant GPU as GPU (wgpu)

    DSL->>IR: 1. parse()
    IR->>IR: 2. type check (AST -> Checked)
    IR->>IR: 3. contract check
    IR->>IR: 4. cost estimation
    IR->>CG: 5. generate() [WGSL lower]
    CG->>ENG: WGSL Source & Uniform Layout
    ENG->>ENG: 6. HotSwap (Background compilation)
    ENG->>ENG: 7. Governor (Budget check)
    ENG->>GPU: 8. Compute & Render Passes (Deck Composite)
```

1. **Parse Stage** (`karakuri-ir::parse`): Lexical tokenization and recursive-descent parsing into an unchecked AST.
2. **Type Check Stage** (`karakuri-ir::typed`): Type inference and type annotation verification. Produces a typed `Checked` AST.
3. **Contract Check Stage** (`karakuri-ir::check`): Mandated attribute assignment check (`position`, etc.), lifespan constraints (`spawn`/`kill`), contract compliance.
4. **Cost Estimation Stage** (`karakuri-ir::cost`): Static analysis of execution cost (element counts, loop bounds, field call counts).
5. **WGSL Lowering Stage** (`karakuri-codegen`): Lowering `Checked` trees to WebGPU compute, vertex, and fragment WGSL text, ensuring 16-byte uniform alignment padding.
6. **HotSwap & Compilation Stage** (`karakuri-engine::swap`): Asynchronous background `wgpu` pipeline creation and atomic swap on frame boundaries.
7. **Governor Verification Stage** (`karakuri-engine::governor`): Frame budget monitoring against `Probe` timing measurements, initiating rollback upon budget overrun.
8. **Execution Stage** (`karakuri-engine::deck` / `present`): GPU compute pass dispatch (L1/L3) and render pass execution (L4, Weighted Blended OIT, Tone Mapping).

---

## 4. Crate Deep Dives

### 4.1 karakuri-ir

- **Purpose**: Parsing and static verification of `.kir` DSL files.
- **Key Modules**:
  - `ast.rs` / `typed.rs`: AST node definitions (`Kind::L1` [geometry/elements], `Kind::L2` [deformations/filters], `Kind::L3` [camera], `Kind::L4` [renderers], `Kind::Field` [spatial data fields]) and the `Checked` tree.
  - `check.rs`: Enforces language contracts (e.g., all emitted attributes assigned on every branch).
  - `cost.rs`: Static cost model and field evaluation call counter.

### 4.2 karakuri-codegen

- **Purpose**: Translating checked IR trees into WGSL shader code and uniform buffer layouts.
- **Key Modules**:
  - `l1.rs`: Lowers `spawn`/`element` blocks to compute shaders with double-buffered `prev`/`next` state arrays.
  - `l3.rs`: Lowers camera state procedures to single-invocation compute passes.
  - `l4.rs`: Lowers vertex and fragment procedures. Expands `topology points` to quad primitives for billboard rendering.
  - `field.rs`: Splices spatial field function bodies into caller modules.
  - `layout.rs`: Computes uniform struct member offsets and trailing 16-byte alignment padding.

### 4.3 karakuri-engine

- **Purpose**: `wgpu` pipeline management, render graph execution, and compositing.
- **Key Modules**:
  - `deck.rs`: Manages up to 4 deck slots and composites them using `add`, `over`, or `max` blend modes.
  - `set.rs`: Manages a node graph (`Set`) representing a complete visual scene.
  - `swap.rs`: `HotSwap` state machine for safe pipeline transitions.
  - `governor.rs`: Tracks frame execution budget (`budget_ms`) and enforces rollbacks.
  - `oit.rs`: Weighted Blended Order-Independent Transparency for alpha blending.
  - `present.rs`: Tone mapping operators (AgX, ACES, Filmic, etc.) and color space conversions.

### 4.4 karakuri-signal, karakuri-audio, karakuri-midi

- **Purpose**: Signal propagation, audio capture, and MIDI controller integration.
- **Key Design Principles**:
  - **Complete Bus**: The `SignalBus` never fails or returns an `Option`. Querying an unprovided signal name yields a `Sample::synthesized(0.0)` with a low `confidence` score (0.0..1.0).
  - **Local Oscillator**: Rendering reads from a local `Oscillator` rather than external system clocks. Audio beat tracking supplies a `Correction` delta to adjust oscillator drift.
  - **Latency Offset**: Audio-visual latency is adjusted via `--latency-offset-ms` (signed), compensating for external PA/projector pipeline delays.

### 4.5 karakuri-store

- **Purpose**: Content-addressed artifact storage keyed by SHA-256 of `.kir` source text, `.set` configuration files, and `.ndjson` session recordings.

### 4.6 karakuri-cli

- **Purpose**: Main entry point and runtime driver.
- **Key Modules**:
  - `main.rs`: `winit` application event loop, VJ keyboard controls, status line formatting.
  - `watch.rs`: Hot-reloading file system watcher for live editing.
  - `mcp.rs`: Model Context Protocol (MCP) server for dynamic LLM agent interaction.
  - `session.rs` / `render.rs`: Session recording/replay and offscreen headless PNG rendering.

---

## 5. Threading & Concurrency Model

Karakuri maintains high real-time performance through a decoupled multi-threaded architecture:

```mermaid
graph LR
    subgraph Threads
        MAIN[Main / Render Thread<br>winit & wgpu Encoder]
        AUDIO[Audio Processing Thread<br>cpal Input & FFT Analyzer]
        WATCH[Watch Worker Thread<br>notify File Watcher]
        MCP[MCP Server Threads<br>JSON-RPC / Agent Interface]
        SAVE[Save Thread<br>one per live save]
    end

    AUDIO -- "Lock-free Queue<br>(AudioFrame & Correction)" --> MAIN
    WATCH -- "File Change Events" --> MAIN
    MCP -- "Commands & Sets" --> MAIN
    MAIN -- "Material to keep" --> SAVE
    SAVE -- "Outcome" --> MAIN
```

- **Main / Render Thread**: Drives the `winit` event loop and encodes `wgpu` render passes. Must never block or allocate during frame rendering.
- **Audio Thread**: Runs `cpal` callbacks, FFT band analysis, and beat tracking, sending measurements across a lock-free channel.
- **Watch Worker Thread**: Monitors `.kir` files via `notify`, triggering background parsing and lowering upon edits.
- **MCP Server Threads**: Handles incoming Model Context Protocol connections from external AI agents.
- **Save Thread**: One per live save (the `k` key), spawned and detached. It writes the Set file into the store — which may not happen on a frame. **Nothing here reads a `.kir`**: the run keeps the text it compiled, so what a slot is running is a content hash from launch onward and this thread puts those bytes in the store as it writes the file that names them. That is what makes a save follow the picture rather than the disk, and it is also why a windowed run that saves nothing creates no store at all — only `--record-session`, whose `procedure` records a replay has to resolve, needs the launch sources on disk before the first frame. The outcome comes back over a channel the frame loop drains, and the `save` record is written there rather than at the key press, so a stream never claims a file the disk refused. No frame waits for one; the *run* waits once, at exit, bounded, so a save pressed in the last second is still recorded and a hung disk still cannot prevent quitting. A pool would be machinery for a rate of a few an hour.

---

## 6. Contributor & Extension Guide

### 6.1 Adding a New IR Builtin Function

1. **`karakuri-ir`**:
   - Add the variant to `Builtin` enum in `src/builtin.rs`.
   - Add type checking and arity validation in `src/check.rs`.
   - Assign cost in `src/cost.rs`.
2. **`karakuri-codegen`**:
   - Implement lowering logic / WGSL string generation in `src/lower.rs` (or appropriate sub-module).
3. **Testing**:
   - Add an end-to-end test in `karakuri-codegen/tests/naga_test.rs` to verify Naga validation passes.

### 6.2 Adding a New Tone Mapper / Composite Mode

1. **`karakuri-engine`**:
   - Add variant to `TonemapOp` or `Blend` in `src/present.rs` / `src/deck.rs`.
   - Add shader logic in the corresponding WGSL presentation shader.
2. **`karakuri-cli`**:
   - Map key binding in `src/main.rs` and update `--help` output.

---

## 7. Related Documents

- [README.md](../README.md): Quickstart and CLI flag reference
- [ir-spec.md](ir-spec.md): `.kir` DSL language specification
- [roadmap.md](roadmap.md): Architectural vision and implementation roadmap
- [manual.md](manual.md): VJ operator manual
