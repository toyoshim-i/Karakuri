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
- **[M7: Autonomous Agent Control & MCP Integration](history/m7.md)** (Foundations closed 2026-09-16, usability & ergonomics active): Core slot policy (`Auto` / `On` / `Off`; ADR-0362), live mixer tracking, and workflow tools (`get_permissions`, `read_slot`, `copy_slot`; ADR-0363) completed and archived in history. Remaining usability, HUD cards, mixer terminology, focus redesign, and documentation remain active in Section 3 below.

---

## 3. The Path to MVP

The remaining open work is structured into three sequential milestones: completing agent and console usability (M7), anchoring musical synchronization and hardware (M8), and final release polish (M9).

```
┌─────────────────────────────────────────────────────────────────────────┐
│ M7: Autonomous Agent Control & MCP Integration (Active / In Progress)  │
│ - Tooltip HUD Visual Overhaul & Literature Copy Streamlining            │
│ - Dedicated MCP Client Documentation & Connection Guides               │
│ - Mixer Channel Strip Controls: SOLO / MUTE Migration                   │
│ - Console Visual Parity & Active Focus (.wfocus) Redesign               │
│ - MCP Policy Persistence & Programmatic Refusal Codes                   │
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

### M7 — Autonomous Agent Control & MCP Integration (Remaining Deliverables)

**Objective**: Complete the human-AI co-performance ergonomics, console readability, and comprehensive developer/agent documentation. Foundational engine tracking and core tools are archived in [history/m7.md](history/m7.md).

#### Key Deliverables:
1. **Tooltip HUD Visual Overhaul & Copy Cleanup**:
   - Migrate all 39 interactive probe tooltips from stream-of-consciousness literature strings to the concise, structured HUD card format (`eyebrow`, `title`, `state`, `summary`, `body`, `midi`, `mcp`).
   - Implement Dark mode and Fancy/Day mode palette preview and runtime toggling.
2. **Dedicated MCP Client Documentation**:
   - Provide an official guide in `docs/manual/` (`mcp.html` / `mcp.md`) detailing agent connection setup, safety conventions, slot policies, and the M7 co-performance tool suite (`get_permissions`, `read_slot`, `copy_slot`).
   - Update `karakuri-mcp/README.md` and architecture references from the legacy 10-tool to the current 13-tool catalog.
3. **Mixer Channel Strip Controls (SOLO / MUTE Migration)**:
   - Transition channel strip primary controls from internal engine pipeline states (`live`, `prim`, `alloc`) to conventional mixer terminology (`SOLO`, `MUTE`), decoupling operator performance intent from residency tallies.
4. **Console Visual Parity & Focus Redesign**:
   - Revisit the `.wfocus` active bay indicator: replace the prominent dashed amber outline on the top transport bar with a cleaner, subtle indicator (e.g. subtle glow or header accent).
   - Restore visual decorations from `console.html`: vertical divider grip decoration (`⋮` on `.divider-v`).
5. **Session Policy Persistence & Programmatic Refusals**:
   - Persist per-slot MCP policies (`Auto`, `On`, `Off`) across sessions/restarts.
   - Standardize agent refusal responses into machine-readable error codes.

**Exit Condition**: 100% of console probes render structured HUD cards; dedicated MCP manual published; mixer channels expose SOLO/MUTE; active focus indicator polished; test suite green.

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
5. **Output Plugin Sinks**:
   - Syphon on macOS, Spout on Windows and NDI as out-of-process sinks beside the window, as [plugins.md](plugins.md) specifies ([ADR-0358](adr/0358-the-projector-is-fullscreened-by-the-operating-system-on-the-display-it-is-on-and-another-application-is-reached-through-a-plugin.md)).
6. **Wipe Mask Geometry Control**:
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

#### Carried from M6 (closed 2026-09-14), each still owed:
- Nothing on the console says a chain build is in flight; `ChainSwap::building()` is the reading a badge would draw from. `MasterChain::resize` still frees on the render thread. A `SlotError` arrives a frame later as `ChainEvent::Refused`.
- A value ridden on a non-head slot before the press is not in the session head (needs a per-slot parameter table `karakuri-cli` does not hold); the GUI records the canvas at the press and never again.
- The build worker's two rungs are taken on a host clock while the render thread draws, so an estimate can refuse under load and the slot falls to its measurement; a param write does not invalidate the estimate.
- A channel fader does not say who is holding it (rule 02); lanes cannot be reordered; `Hover::owed` is asked before `paint` on the frame a card comes down.
- `cargo clippy --workspace --all-targets -- -D warnings` is red at HEAD on `doc_lazy_continuation` in files untouched today (karakuri-ir, karakuri-store, karakuri-console); the pre-push hook runs it on tag pushes.

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
