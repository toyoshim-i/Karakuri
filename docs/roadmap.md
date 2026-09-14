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
│ - Chain compiled on a worker, installed at a frame boundary (closed)   │
│ - Session head names the whole deck; replay builds it (closed)         │
│ - Lane/hand refusal called; a lane can be removed (closed)             │
│ - over_budget as the deck's total (maintainer's)                       │
│ - Chain-build badge, ridden values in the head, clippy baseline (open) │
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

**Objective**: the render thread never compiles, allocates or frees; a session replays the deck it was recorded on; a hand and a lane never fight over one control silently.

#### Closed (2026-09-14)

- **The master chain is compiled on a thread of its own and lands at a frame boundary.** [ADR-0354](adr/0354-a-chain-is-compiled-on-a-thread-of-its-own-and-lands-at-a-frame-boundary.md); `karakuri-engine::chain_swap`; held by `crates/karakuri-engine/tests/chain_swap.rs`.
- **A swapped-in Set arrives with its own estimate, and a resize re-reads it rather than dropping it.** [ADR-0356](adr/0356-the-worker-estimates-what-it-built-and-an-estimate-is-a-fit-rather-than-a-number-at-one-size.md).
- **A session head says what the deck held.** [ADR-0355](adr/0355-a-session-head-says-what-the-deck-held-in-the-records-that-already-say-it.md); `--replay` builds the deck the head names; [ir-spec.md](ir-spec.md) *The head — what the deck held at frame 0*.
- **ADR-0323's refusal is called**: a scheduled fade, crossfade or wipe on a fader a lane holds is refused before any record is written, in one sentence naming the lane.
- **A lane can be taken out of a pattern**: `Operation::RemoveLane`, a minus glyph on the lane's row, `enter` on the lane. [ADR-0357](adr/0357-a-lane-is-taken-out-by-a-glyph-on-its-row-and-enter-on-the-lane-is-the-key.md).
- **A tip already up goes on the frame a card comes down** (`Hover::paint` asks `View::has_modal_overlay`; all nine overlays tested).

#### Settled by decision, not built

- **Fade and crossfade keybindings and panel cells**: both rows read `gap` in the panel and key columns by [ADR-0353](adr/0353-a-scheduled-fade-is-not-drawn-on-a-real-time-surface-so-the-two-rows-read-gap-in-both-hand-columns.md). What is open on those rows is MIDI only, and it waits on a map-layer helper that carries time (ADR-0236's third job), unscheduled.
- **A candidate judged against the deck's period or a share of the budget**: refused by [ADR-0313](adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md) — a verdict is the candidate's own cost against one frame; the period is a deck-level alarm that warns and never acts.
- **Fullscreen on a chosen display, and a monitor list**: [ADR-0358](adr/0358-the-projector-is-fullscreened-by-the-operating-system-on-the-display-it-is-on-and-another-application-is-reached-through-a-plugin.md) — the projector is a window, the operating system's own gesture makes it fullscreen on the display it is on, and a destination that is not a display in the room is a plugin sink's ([plugins.md](plugins.md)), scheduled under M8.
- **The transport row's frame figure**: stays the CPU's and says `cpu` ([ADR-0359](adr/0359-the-transport-rows-frame-figure-stays-the-cpus-and-says-so.md)); the period would read full on every frame that is fine, and the GPU's time is not measurable per frame on Metal or Vulkan (ADR-0169) — `Cost::drained` is the once-per-500 ms flush in the readout.
- **A projector frame loop decoupled from the console's**: refused by [ADR-0166](adr/0166-the-engines-frame-and-the-panels-are-one-submission.md) — one encoder, one submission; a second submission over the deck's targets is the race that record is about.

#### Open — the maintainer's

- **What `over_budget` says as the deck's total.** `committed_ms` sums Live slots only; ADR-0313 supplied `Report::deck_over_period` and took none of this decision; nothing acts on or draws it. Two single-slot readings survive in code and are facts for the decision: `Deck::frame_period_ms` reads slot 0's watchdog alone (`deck.rs:801`), and the GUI calibrates the compute budget from slots 0 and 1 only (`bridge/engine.rs:1239`).

#### Open — owed by today's work, unscheduled

- Nothing on the console says a chain build is in flight; `ChainSwap::building()` is the reading a badge would draw from. `MasterChain::resize` still frees on the render thread. A `SlotError` arrives a frame later as `ChainEvent::Refused`.
- A value ridden on a non-head slot before the press is not in the session head (needs a per-slot parameter table `karakuri-cli` does not hold); the GUI records the canvas at the press and never again.
- The build worker's two rungs are taken on a host clock while the render thread draws, so an estimate can refuse under load and the slot falls to its measurement; a param write does not invalidate the estimate.
- The GUI's MCP reply on a refused, owed or silent operation — in progress 2026-09-14.
- A channel fader does not say who is holding it (rule 02); lanes cannot be reordered; `Hover::owed` is asked before `paint` on the frame a card comes down.
- `cargo clippy --workspace --all-targets -- -D warnings` is red at HEAD on `doc_lazy_continuation` in files untouched today (karakuri-ir, karakuri-store, karakuri-console); the pre-push hook runs it on tag pushes.

**Exit**: the three commands below are green, and the maintainer's item above is decided (built or struck with the ADR that struck it).

```sh
cargo test -p karakuri-engine --test chain_swap --test master      # no chain build on a render thread; worker bytes == synchronous bytes
cargo test -p karakuri-cli --test replay                           # a two-slot head replays both slots; same session, same bytes
test "$(grep -c 'rt plan">key' docs/manual/operations.html)" = 0   # no key route left as a plan
```

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
