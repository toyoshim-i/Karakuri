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

---

## 3. The Path to MVP

The remaining open work is structured into four sequential milestones focused on runtime hardening, agent autonomy, musical synchronization, and final release polish.

```
┌─────────────────────────────────────────────────────────────────────────┐
│ M6: Live Performance Hardening & Runtime Safety (Immediate Priority)   │
│ - Async Master Chain Compilation (Worker Thread)                        │
│ - Dedicated Fullscreen Projector Window Output                          │
│ - Unified Multi-Slot Budget Governor & Telemetry                        │
│ - Multi-Slot Session Recording & Replay Fidelity                        │
│ - Mixer & Sequencer Safeguards (Lane Conflicts, Keybindings)            │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ M7: Autonomous Agent Control & MCP Integration                          │
│ - Asynchronous Background Generation Queue                              │
│ - Tri-State Authority Protocol (Autonomous / Suggest / Takeover)        │
│ - Coordinated Multi-Layer Set Generation Tools                          │
│ - Candidate Set Priming & Variant Pool                                  │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ M8: Musical Synchronization & Hardware Integration                      │
│ - Ableton Link Out-of-Process Synchronization                           │
│ - Live MIDI Surface Mapping & Profile Persistence                       │
│ - Low-Latency Audio FFT & Onset Signal Pipeline                         │
│ - Bar- and Beat-Quantized Transition Scheduling                         │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ M9: MVP Polish & Release Readiness                                      │
│ - Master Chain Reordering & Custom Chain Persistence                    │
│ - Library Free-Text Search & Thumbnail Generation                       │
│ - Multi-Hour Continuous Rehearsal Stress Testing                        │
│ - Release Packaging, Verification & Documentation Audit                 │
└─────────────────────────────────────────────────────────────────────────┘
```

---

### M6 — Live Performance Hardening & Runtime Safety

**Objective**: Resolve all critical operational and architectural debts identified during M5 testing to guarantee uninterrupted, drop-free 60+ FPS live performance.

#### Key Deliverables:
1. **Asynchronous Master Chain Compilation**:
   - Move `mix::apply_chain` shader compilation and target texture allocation off the main/render thread to a background worker thread (enforcing Principle [P-0091](principles/0091-cost-is-known-before-it-is-paid.md)).
   - Swap compiled chain effects atomically at frame boundaries without stalling ongoing rendering.
2. **Dedicated Fullscreen Projector Output**:
   - Enable borderless fullscreen presentation on an operator-selected display/projector using `winit` monitor enumeration.
   - Decouple projector display refresh from console UI rendering passes.
3. **Deck-Wide Budget Governor & Telemetry**:
   - Wire `karakuri-engine::estimate` into `Deck::govern` to evaluate procedural GPU cost before activation.
   - Extend budget accounting across all four active deck slots plus the master chain, replacing single-slot assumptions with holistic frame period monitoring ([ADR-0313](adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)).
   - Update Transport frame telemetry to clearly display total frame period and GPU headroom.
4. **Complete Session Recording & Replay Fidelity**:
   - Expand `Record::MasterChain` and session stream headers to capture the complete multi-slot state of the deck at session start.
   - Ensure session journal replay reproduces all multi-slot assignments and parameter overrides byte-for-byte.
5. **Mixer & Sequencer Conflict Safeguards**:
   - Enforce [ADR-0323](adr/0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md): reject manual scheduled fades on controls currently driven by an active sequencer lane.
   - Implement `RemoveLane` operation and UI control to allow removing modulation lanes from the sequencer.
   - Finalize keybindings and panel cells for Mixer fade and crossfade operations.
6. **Modal Overlay Interaction & Tooltip Suppression**:
   - Connect modal card state to the hover layer to suppress underlying bay tooltips when a popup card is open.

**Exit Condition**: Zero frame drops or stalls during continuous chain edits, clean multi-slot session replay, and zero remaining `plan` badges in the key column of `docs/manual/operations.html`.

---

### M7 — Autonomous Agent Control & MCP Integration

**Objective**: Turn the Model Context Protocol (MCP) server into an active co-performer capable of autonomously authoring, auditioning, and proposing visual sets during a performance.

#### Key Deliverables:
1. **Asynchronous Generation Queue**:
   - Priority-based generation queue managing LLM requests in the background without blocking interactive response.
   - Pre-validation of generated `.kir` code against IR grammar, type safety, and GPU cost ceilings before warm-up.
2. **Tri-State Node Authority Protocol**:
   - Implement node-level authority states:
     - `Autonomous`: Agent drives parameter modulation and swaps.
     - `Suggest`: Agent proposals are staged as candidate cards for operator confirmation.
     - `Takeover`: Operator manual manipulation immediately demotes agent control ([P-0094](principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
3. **Coordinated Multi-Layer Set Authoring**:
   - Provide high-level MCP tools allowing agents to generate coordinated multi-layer Sets (Geometry + Deform + Renderer) with declared parameter ranges and matching uniforms.
   - Expose active store procedures and metadata cards to MCP clients for context-aware code generation.
4. **Offline Priming & Variant Pool**:
   - Support background priming of candidate Sets on off-air deck slots, enabling instantaneous, glitch-free swaps when visual themes shift.
5. **Agent Telemetry & Real-Time Feedback**:
   - Provide MCP tools with live governor feedback, available frame headroom, and audio energy descriptors.

**Exit Condition**: An AI agent running over loopback MCP can autonomously generate, prime, and crossfade a sequence of valid procedural Sets in response to simulated music cues without manual intervention.

---

### M8 — Musical Synchronization & Hardware Integration

**Objective**: Anchor Karakuri's procedural animation and transitions to live musical structure and professional DJ/VJ hardware.

#### Key Deliverables:
1. **Ableton Link Integration**:
   - Connect the out-of-process `--tempo-source` plugin interface to Ableton Link, synchronizing tempo, beat phase, and quantum downbeats with external DJ software (Traktor, Serato, Ableton Live).
2. **Quantized Transition Engine**:
   - Implement precise bar- and phrase-quantized execution for wipes, fades, and deck swaps, ensuring visual changes lock to musical drops and phrase boundaries.
3. **Live MIDI Surface Mapping & Profile Management**:
   - Provide an in-app interface to load, edit, and persist MIDI controller maps (`.map` files) dynamically during performance.
   - Support 14-bit high-resolution MIDI CC mappings for ultra-smooth parameter sweeps.
4. **Low-Latency Audio Signal Bus**:
   - Low-latency real-time FFT processing with multi-band energy extraction (sub, bass, mid, air) and onset detection feeding the internal signal oscillator.
5. **Wipe Mask Geometry Control**:
   - Expose explicit wipe front position and edge softness parameters on mixer strips.

**Exit Condition**: The console locks beat grids to an external Ableton Link peer, responds to hot-plugged MIDI hardware, and triggers bar-quantized transitions synchronized to incoming audio beats.

---

### M9 — MVP Polish & Release Readiness

**Objective**: Final integration, usability refinement, multi-hour stress testing, and packaging for initial 1.0 release.

#### Key Deliverables:
1. **Master Chain & Library Refinements**:
   - Master chain slot reordering (drag-and-drop handles) and arbitrary slot insertion.
   - Named master chain presets saved to and loaded from the Library store.
2. **Library Search & Caching**:
   - Interactive free-text search filtering across set names, procedure types, and metadata tags.
   - Cached off-screen thumbnail previews for rapid visual identification in the library browser.
3. **Comprehensive Live Rehearsal Stress Test**:
   - 4-hour continuous burn-in test running multi-slot decks, active audio input, periodic set hotswaps, and concurrent MCP generation.
   - Memory leak audit (`#[global_allocator]` allocation tracking) confirming zero unbounded heap growth.
4. **Documentation Audit & Release Distribution**:
   - Complete synchronization of `docs/manual/` with all implemented operations.
   - Production build packaging for macOS and Linux.

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
