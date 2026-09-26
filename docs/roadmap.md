# Karakuri — Vision and Roadmap

**This document is the operational ledger of open engineering work for Karakuri.**
It tracks the milestone path required to deliver a rock-solid, production-grade Minimum Viable Product (MVP) for live performance.

- Core invariants: [docs/principles/](principles/)
- Contributing guidelines & ADR lifecycle: [docs/contributing.md](contributing.md)
- System architecture: [docs/architecture.md](architecture.md) & [docs/architecture/](architecture/)
- Procedural shading DSL: [docs/ir-spec.md](ir-spec.md)
- Console manual & operation directory: [docs/manual/](manual/) and [docs/manual.md](manual.md)
- Historical decision records: [docs/adr/](adr/)
- Completed milestones archive: [docs/history/](history/)

---

## 1. System Vision & Core MVP Goals

Karakuri is a real-time visual performance system where human performers and autonomous AI agents collaborate through a unified control architecture. During a live set, the system generates, warms up, and introduces procedural visual material aligned to musical tempo, while the human operator maintains absolute authority to override, reshape, or take manual control of any parameter at any instant.

### Core Properties Defining MVP Success

1. **GPU-Native Procedural Synthesis**: Visuals are compiled into GPU compute and render pipelines executing every frame, rather than pre-rendered video clips. What is stored and recalled is an instrument with live parameters.
2. **Unified Control Mechanism**: Human performers (via keyboard, mouse, and MIDI) and AI agents (via Model Context Protocol / MCP) interact through the identical `Operation` vocabulary. Node-level authority defines control precedence.
3. **Deterministic Reproducibility**: Given the same event journal and initial seeds, a recorded performance reproduces identically on replay.
4. **Stage Safety & Real-Time Stability**: Audio-visual rendering never stalls, crashes, or drops frames during live execution. Compilation and I/O occur strictly off the render thread.

### Explicit Non-Goals for MVP
- General-purpose non-linear video editing or video file playback (except as external camera/capture inputs).
- Complex 3D mesh modeling or timeline animation keyframing.
- Cloud-hosted rendering services (the engine runs 100% locally on dedicated hardware).

---

## 2. Completed Milestones (Foundations)

- **[M1: Closing the Loop](history/m1.md)**: End-to-end procedural compiler pipeline from `.kir` DSL to rendered GPU pixels.
- **[M2: Playable Engine](history/m2.md)**: 4-slot deck execution graph, atomic hot-swapping at frame boundaries, and basic CLI runner.
- **[M3: Expressive Depth](history/m3.md)**: Multi-layer compositing, deformations, procedural cameras, and vector field generators.
- **[M4: Library at Scale](history/m4.md)**: Content-addressed artifact storage, self-contained `.kbset` bundles, and metadata card indexing.
- **[M5: VJ Console Interface](history/m5.md)** (Closed 2026-09-14):
  - Modular panel layout (Program, Staging, Mixer, Transport, Library, Inspector, Master, and Sequencer bays).
  - Componentized widget system (cards, chips, slider tracks, input fields, and glyphs).
  - Unified `ControlId` and `ControlDescriptor` registry across all 38 interactive probes.
  - Dynamic Master Chain UI and pipeline (`kind L5` effects with GPU cost budgeting).
  - Multi-level keyboard navigation and focus ladder.
  - Verified exit condition: 100% of panel operations implemented (`grep -c 'rt plan">panel' docs/manual/operations.html` returns 0).
- **[M6: Live Performance Hardening & Runtime Safety](history/m6.md)** (Closed 2026-09-14): the chain compiles on a worker (ADR-0354), a swapped-in Set is estimated (ADR-0356), a session head names the deck (ADR-0355), ADR-0323's refusal is called, a lane can be removed (ADR-0357); the projector, the transport figure and `over_budget` settled by ADR-0358..0360.
- **[M7: Autonomous Agent Control & MCP Integration](history/m7.md)** (Closed 2026-09-20, Hardened 2026-09-22):
  - Unified Console Bay 1:1 gate architecture (Inspector bay gate as per-slot policy `Auto`/`On`/`Off`, Mixer/Master/Outputs/Program bay gate capsules).
  - Slot access policies (`Auto`/`On`/`Off`) and live mixer contribution tracking.
  - Complete 14-tool matrix, machine-readable refusal codes, and recovery flows in `docs/manual/mcp.html`.
  - Workflow and validation tools (`get_permissions`, `read_slot`, `copy_slot`, `check_procedure`, `check_set`) published in `tools/list` schema.
  - Enforced slot policy write checks across all deck-mutating operations in `operate`.
  - Atomic file staging, session policy persistence, Master Chain in-flight build indicator, and structured Tooltip HUD cards.
- **[M8: Musical Synchronization & Hardware Integration](history/m8.md)** (Closed 2026-09-27):
  - Low-latency real-time FFT audio bus with 8 log-spaced semantic spectral bands and transient onset detection.
  - Decoupled GPU output plugin sinks (Syphon on macOS, Spout on Windows) with self-reporting discovery protocol (`Hello { name, kind, surfaces }`) and dynamic Outputs row integration.
  - Hardened bay-scoped keyboard navigation model with interactive tooltip key learning and global promotion.
  - Unified bay header interaction: retained 27px header bars on fold, 6-dot menu dice reservation, and double-click folding across all 7 bays (ADR-0364).
  - Dynamic MIDI controller map editing and wipe mask geometry carried forward to M10.
- **[M9: Integrated Agent Terminal & Prompt Bay](history/m9.md)** (Closed 2026-09-27):
  - Left-pane Prompt bay layout integration beneath Library and Staging with flexible height budgeting and 27px retained header bar on fold (ADR-0343, ADR-0364, ADR-0365).
  - Multi-session detached PTY background process multiplexer with 1024-line scrollback buffers and automatic lifecycle cleanup on process exit.
  - 18 sorted AI CLI presets (`agy`, `aider`, `claude`, `ollama`, ...) plus `custom...` defaulting to system interactive shell (`sh`/`powershell`).
  - Terminal cursor-anchored borderless multiline input with native macOS/Windows IME candidate positioning for Japanese and multilingual prompt drafting.
  - ANSI VT100 / xterm sequence emulation (24-bit Truecolor, relative cursor positioning, alternate screen buffer, raw LF/CR semantics, and cursor visibility toggles).
  - Smooth console focus ladder with keyboard capture mode forwarding arrow keys and control characters to PTY stdin while preserving `Tab` bay navigation and `Esc` release.

---

## 3. The Path to MVP

The path to MVP has reached its final release milestone: **M10 (MVP Polish & Release Readiness)**.

```
┌─────────────────────────────────────────────────────────────────────────┐
│ M9: Integrated Agent Terminal & Prompt Bay (Closed 2026-09-27)          │
│ - Left-Pane Prompt Bay & Layout Integration (Foldable, 27px Bar)        │
│ - Multi-Session Detached PTY Multiplexer & Process Lifecycle Manager   │
│ - Alphabetical CLI Selector (agy, aider, claude, codex, deepseek, ...)  │
│ - Cursor-Anchored Native Multiline Input with Full Japanese IME Support │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ M10: MVP Polish & Release Readiness (Active Milestone)                  │
│ - In-App Dynamic MIDI Controller Map Editing & 14-Bit CC Support         │
│ - Wipe Mask Geometry & Edge Softness Control                            │
│ - Master Chain Reordering & Custom Chain Persistence                    │
│ - Library Free-Text Search & Thumbnail Generation                       │
│ - Multi-Hour Continuous Rehearsal Stress Testing                        │
│ - Release Packaging, Verification & Documentation Audit                 │
└─────────────────────────────────────────────────────────────────────────┘
```

---

### M10 — MVP Polish & Release Readiness (Active Milestone)

**Objective**: Final integration, usability refinement, multi-hour stress testing, and packaging for initial 1.0 release.

#### Key Deliverables:
1. **Live MIDI Surface Mapping & Profile Management** *(Carried from M8)*:
   - Provide an in-app interface to load, edit, and persist MIDI controller maps (`.map` files) dynamically during performance.
   - Support 14-bit high-resolution MIDI CC mappings for ultra-smooth parameter sweeps.
2. **Wipe Mask Geometry Control** *(Carried from M8)*:
   - Expose explicit wipe front position and edge softness parameters on mixer strips.
3. **Master Chain & Library Refinements**:
   - Master chain slot reordering (drag-and-drop handles) and arbitrary slot insertion.
   - Named master chain presets saved to and loaded from the Library store.
4. **Library Search & Caching**:
   - Interactive free-text search filtering across set names, procedure types, and metadata tags.
   - Cached off-screen thumbnail previews for rapid visual identification in the library browser.
5. **Session Last-State Recall & Slot Startup Initialization**:
   - Persist and recall each slot's last-played set/procedure across sessions so the performer re-opens into their exact live setup.
   - First-launch default starts with clean state, loading demo visuals per slot through the standard Load path, with Decks B–D muted under the unified Solo/Mute mixer architecture.
6. **Comprehensive Live Rehearsal Stress Test**:
   - 4-hour continuous burn-in test running multi-slot decks, active audio input, periodic set hotswaps, and concurrent MCP generation.
   - Memory leak audit (`#[global_allocator]` allocation tracking) confirming zero unbounded heap growth.
7. **Documentation Audit & Release Distribution**:
   - Complete synchronization of `docs/manual/` with all implemented operations.
   - Production build packaging for macOS and Linux.

#### Carried from M6 (closed 2026-09-14) and M8 (closed 2026-09-27), status:
- In-app dynamic MIDI map editing and 14-bit CC resolution deferred to M10; basic MIDI input binding and live learned maps are active.
- Mixer wipe edge softness and front position control deferred to M10; baseline wipe transitions and shape selection are fully operational.
- Master Chain in-flight build badge was landed in M7 (`Head::building` pill). `MasterChain::resize` still frees on the render thread. A `SlotError` arrives a frame later as `ChainEvent::Refused`.
- A value ridden on a non-head slot before the press is not in the session head (needs a per-slot parameter table `karakuri-cli` does not hold); the GUI records the canvas at the press and never again.
- The build worker's two rungs are taken on a host clock while the render thread draws, so an estimate can refuse under load and the slot falls to its measurement; a param write does not invalidate the estimate.
- A channel fader does not say who is holding it (rule 02); lanes cannot be reordered; `Hover::owed` is asked before `paint` on the frame a card comes down.
- Mixer channel control unified under Solo/Mute: prototype-era `Residency::Allocated` startup hack retired; Decks B–D default to muted on first launch with clean load-path participation.
- Keyboard navigation model hardened: global scope restricted strictly to navigation (`Tab`, `Shift-Tab`, `Esc`, `Enter` descent) to prevent misoperation in live performance; operational shortcuts scoped to bays with customizable keymaps (`<store>/keymaps/default.keymap`), explicit globalization flag, and collision detection.
- Workspace clippy warnings (`doc_lazy_continuation` etc.) resolved; clean build across all targets with `-D warnings`.

**Exit Condition**: Clean 4-hour rehearsal without crashes, memory leaks, or unhandled errors; 100% operation coverage across all implemented surfaces; clean workspace lint and test suite.

---

## 4. Continuous Invariants

These invariants must be defended continuously across all milestones:

- **Live Performance Safety ([P-0094](principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md))**:
  Rendering never halts. Uncompilable shaders, missing artifacts, or network dropouts fall back gracefully without interrupting active output.
- **Strict Determinism ([P-0092](principles/0092-the-same-inputs-produce-the-same-frame.md))**:
  The same event journal and initial seeds produce the exact same visual frame sequence.
- **Predictable Cost Allocation ([P-0091](principles/0091-cost-is-known-before-it-is-paid.md))**:
  GPU and CPU costs are evaluated and budgeted prior to activating a procedure on air.

---

## 5. Deferred by Decision (Post-MVP Backlog)

The following capabilities are architecturally anticipated but explicitly deferred until after MVP:

- **Procedural Shader Fusion**: Folding multi-stage stateless pipelines into single combined WGSL shaders to minimize buffer round-trips.
- **Automated ShaderToy Import**: AST extraction of single-pass fragment shaders into composable `Field` procedures.
- **Hardware Lighting Protocols**: DMX512 and Art-Net lighting universe output synchronized with visual energy.
- **Advanced Projection Mapping**: Multi-projector edge blending, geometric warping, and multi-display spatial stitching.
- **Networked Multi-Operator Sessions**: Distributed multi-user collaborative performance over WebRTC/WebSockets.

---

## 6. Project Instrumentation & Verification

Track implementation progress objectively using the following automated tools:

```sh
# Measure operational surface coverage
for col in panel key MIDI MCP CLI; do
  echo -n "$col: "
  grep -o "class=\"rt [a-z]*\">$col " docs/manual/operations.html |
    sed 's/.*rt //;s/">.*//' | sort | uniq -c | tr '\n' ' '; echo
done

# Count remaining unimplemented panel controls
grep -o 'rt plan">panel <b>[^<]*' docs/manual/operations.html | sed 's/.*<b>//' |
  sort | uniq -c | sort -rn

# Run full workspace test suite (CPU only)
cargo test --workspace -- --skip gpu::

# Verify operation vocabulary synchronization
cargo test -p karakuri-operation --test the_manual_and_the_vocabulary_agree
```
