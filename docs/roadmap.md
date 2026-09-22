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

---

## 3. The Path to MVP

The remaining open work is structured into two sequential milestones: anchoring musical synchronization and hardware (M8), and final release polish (M9).

```
┌─────────────────────────────────────────────────────────────────────────┐
│ M8: Musical Synchronization & Hardware Integration (Active / In Prog)  │
│ - Low-Latency Audio Signal Bus & Multi-Band Procedural Modulation (Done)│
│ - External Output Plugin Sinks (Syphon completed on macOS, Spout/NDI)   │
│ - Live MIDI Surface Mapping & Profile Persistence                       │
│ - Bar- and Beat-Quantized Transition Scheduling                         │
│ - Wipe Mask Geometry & Edge Softness Control                            │
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

### M8 — Musical Synchronization & Hardware Integration (Active Milestone)

**Objective**: Anchor Karakuri's procedural animation and transitions to live musical structure and professional DJ/VJ hardware.

#### Key Deliverables:
1. **Low-Latency Audio Signal Bus & Multi-Band Procedural Modulation** *(Completed — M8-4)*:
   - Low-latency real-time FFT processing with multi-band energy extraction (8 log-spaced semantic bands: `sub`, `bass`, `low_mid`, `mid`, `high_mid`, `presence`, `brilliance`, `air`) and transient `onset` detection feeding `.kir` shader parameter bindings.
2. **Output Plugin Sinks**:
   - Syphon on macOS implemented via standalone `karakuri-syphon` plugin using zero-copy `IOSurfaceID` IPC protocol (M8-2 completed for macOS; Spout/NDI pending beside the window, as [plugins.md](plugins.md) specifies ([ADR-0358](adr/0358-the-projector-is-fullscreened-by-the-operating-system-on-the-display-it-is-on-and-another-application-is-reached-through-a-plugin.md))).
3. **Quantized Transition Engine**:
   - Implement precise bar- and phrase-quantized execution for wipes, fades, and deck swaps, ensuring visual changes lock to musical drops and phrase boundaries.
4. **Live MIDI Surface Mapping & Profile Management**:
   - Provide an in-app interface to load, edit, and persist MIDI controller maps (`.map` files) dynamically during performance.
   - Support 14-bit high-resolution MIDI CC mappings for ultra-smooth parameter sweeps.
5. **Wipe Mask Geometry Control**:
   - Expose explicit wipe front position and edge softness parameters on mixer strips.
6. *(Cancelled)* **Ableton Link Out-of-Process Synchronization**:
   - Cancelled for MVP: Live DJ testing confirmed Pioneer rekordbox does not publish deck BPM over Ableton Link (link fader is independent and does not follow the playing track; see [manual.md](manual.md#with-rekordbox-this-is-much-less-useful-than-it-sounds-and-the-reason-is-rekordboxs)), making Link ineffective for unattended DJ tempo tracking without manual intervention. Real-time audio spectral/beat tracking is already operational and serves as the primary live tempo follower.

**Exit Condition**: The console responds to hot-plugged MIDI hardware, modulates visuals via live audio spectral bus, outputs video to external sinks (Syphon/Spout), and triggers bar-quantized transitions synchronized to incoming audio beats.

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

#### Carried from M6 (closed 2026-09-14), status:
- Master Chain in-flight build badge was landed in M7 (`Head::building` pill). `MasterChain::resize` still frees on the render thread. A `SlotError` arrives a frame later as `ChainEvent::Refused`.
- A value ridden on a non-head slot before the press is not in the session head (needs a per-slot parameter table `karakuri-cli` does not hold); the GUI records the canvas at the press and never again.
- The build worker's two rungs are taken on a host clock while the render thread draws, so an estimate can refuse under load and the slot falls to its measurement; a param write does not invalidate the estimate.
- A channel fader does not say who is holding it (rule 02); lanes cannot be reordered; `Hover::owed` is asked before `paint` on the frame a card comes down.
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
