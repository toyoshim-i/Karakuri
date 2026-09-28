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

The path to MVP proceeds through **M10 (Expressive Surface & Master Pipeline)** to the final release milestone **M11 (MVP Polish & Release Readiness)**.

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
│ M10: Expressive Surface & Master Pipeline (Active Milestone)            │
│ - Procedural Mesh & Topology Expansion (triangles, grid, ribbon)        │
│ - Rich L4 Material Presets (MatCap, Cyber Wire, Toon, Fresnel Hologram) │
│ - Master Chain Dynamic Slots, Insertion & Drag-and-Drop Reordering      │
│ - Curated Master Preset Chains (Clean Cyber, Retro Stage, Psychedelic)  │
│ - L5 Nested in a Set with Per-Set Frame Retention                       │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ M11: MVP Polish & Release Readiness                                     │
│ - In-App Dynamic MIDI Controller Map Editing & 14-Bit CC Support        │
│ - Wipe Mask Geometry & Edge Softness Control                            │
│ - Preset Library Store Management & Set Bundle Persistence              │
│ - Audio Spectrogram History Texture & Advanced Dynamics (Waterfall)     │
│ - Multi-Frame L5 History Target (Time-Delay Slit Scan, Echo)            │
│ - Procedural Sky & SDF Ambient Occlusion / Volumetric Extensions        │
│ - Mesh Group Lifecycle & Index-Shift Mesh Mode                          │
│ - Library Free-Text Search & Thumbnail Generation                       │
│ - Multi-Hour Continuous Rehearsal Stress Testing                        │
│ - Release Packaging, Verification & Documentation Audit                 │
└─────────────────────────────────────────────────────────────────────────┘
```

---

### M10 — Expressive Surface & Master Pipeline (Active Milestone)

**Objective**: Expand visual synthesis from 2D point/line sprites to expressive 3D procedural meshes, provide rich plug-and-play L4 material presets for LLM-driven generation, and furnish the master chain with essential live performance post-processing effects.

#### Key Deliverables:
1. **Procedural Mesh & Topology Expansion (`karakuri-ir`, `karakuri-codegen`)**:
   - `topology triangles`: Direct vertex-shader-art generation. A vertex is an element and is addressed by `seed`, never by a slot index (P-0007, ADR-0001; `id` stays reserved).
   - `topology grid`: Built-in tessellated grid topology with automated index buffering; allows LLMs to create reactive terrain, undulating cloth, and cyber-space meshes with simple displacement functions.
   - `topology ribbon`: Continuous connected quad strip topology for flowing light trails and vector field paths.
   - **Shared vertices on static sources only.** An index buffer names slot indices, and compaction moves them every frame (ir-spec *Dispatch*). A source with no `spawn` and no `kill()` never compacts and its `seed` is its slot index, so the three topologies are legal only where `Checked::is_static` holds — its third use after L2 edges and `morph.kir`.
   - **Spec revisions this owes (`docs/ir-spec.md`, written before the code)**:
     - *capacity / topology*: `topology` stops "constraining no renderer"; the index layout (e.g. grid W×H) becomes the L1's fact.
     - *How an L4 says what it draws*: per-element values reach `fragment` flat because a sprite and a segment have nothing to interpolate between. Under a shared-vertex topology they interpolate; the section already says it would need revisiting.
     - *Element lifecycle*: specify **group lifecycle** — elements spawn and die in units of k (alive flag and scan per group, `spawn` count and `capacity` multiples of k, the `min(spawn_count, capacity - survivors)` truncation made group-aligned). Order-preserving compaction then keeps each group contiguous and its indices valid, so spawning ribbons and debris can be meshes. Implementation is scheduled in M11 item 4.
   - **Depth buffer and `blend opaque` (decided 2026-09-28; lands before the topologies above)**: no pass in the engine attaches a depth buffer today (`depth_stencil_attachment: None` throughout), so an opaque polygon drawn later covers a nearer one drawn earlier. `weighted` keeps coverage right but averages colour by depth weight — by the ir-spec formula, a 1-unit-thick object 3 units away under near 0.1 / far 52 gets about 52/48 front/back — which breaks MatCap and toon.
     - `blend opaque`, a third value: writes and tests depth. A per-Set depth target (Depth32: ~8 MB at 1080p, ~33 MB at 4K), allocated only where a renderer declares `opaque` (P-0091). Stays inside the Set; the mixer is untouched, and opaque coverage is 1, so `over` works as is.
     - `additive` and `weighted` test depth and do not write it, so material behind an opaque surface is hidden and early-Z drops those fragments before shading.
     - Spec first: ir-spec *blend* and *WGSL lowering — L4*.
     - **ADR written with the implementation, not before** — the design may still drift. It carries what this paragraph would otherwise lose: why `weighted` cannot stand in (the averaging figure above), that wgpu attaches no depth implicitly as a GL context does, and how early-Z behaves for each blend mode.
     - Occlusion holds only where renderers overdraw one attachment. Under `merge` (renderers composited through an L5) each renderer has its own target and an opaque surface hides nothing in another. Between deck slots depth never applies; the mixer stacks layers.
     - **Draw order within a Set (decided 2026-09-28)**: renderers overdraw in declaration order, and that rule stays. A Set that declares an `opaque` renderer after a non-opaque one is refused at Set build (a new `SetError`, beside `PairingNotFirst`, which already refuses on position), and the refusal names the renderer to move first (P-0083). Reordering silently was rejected: it draws the same picture and takes the order out of the author's hands (P-0084). `check_set` reports it before a load; a refused Set never reaches the air (P-0094).
     - **Depth test is chosen where a renderer is placed, not in the `.kir` (decided 2026-09-28)**: a per-renderer Set setting, default *test*. *No test* draws over whatever came before regardless of depth — an overlay inside the Set, with no extra pass or target. The choice changes nothing a fragment writes, so it is the placement's answer, as `cut` is for `retains` (P-0086). `opaque` always tests and writes.
     - **A fullscreen L4 that writes no depth is tested at the far plane (decided 2026-09-28)**, so a marched sky drawn after an opaque mesh sits behind it. With *no test* the same field is an overlay. Superseded per field by M11 item 4's depth from fullscreen fields.
2. **Rich L4 Material Presets & Shading Helpers**:
   - Built-in screen-space flat normal derivation (`normalize(cross(dpdx(p), dpdy(p)))`) in L4 lowering, granting faceted low-poly shading to any procedural geometry with zero authoring overhead. Depends on item 1's interpolation revision: under today's flat rule `p` is constant across a primitive and the derivative is zero.
   - Core L4 material preset suite (`examples/` & Library):
     - `matcap_chrome.kir`, `matcap_clay.kir`, `matcap_iridescent.kir`: Procedural texture-free MatCap reflections and shading.
     - `cyber_wire.kir`: Barycentric / UV-edge dual rendering (filled polygon face + emissive wireframe edge).
     - `fresnel_holo.kir`: Edge-emitting translucent hologram / X-ray styling.
     - `toon_step.kir`: Quantized stepped lighting for cel-shaded comic aesthetics.
3. **Master Chain (L5) Integration & Performance Effects Suite**:
   - **Complementary L5 Performance Procedures (Filling gaps in the 23-procedure library)**:
     - `glitch_slice.kir`: Beat-synced horizontal band slicing and pseudo-random block displacement for drop punctuation (distinct from continuous `roll_panels`).
     - `negative_strobe.kir`: High-impact 1-frame inverted/negative-flash strobe triggered on transients.
     - `chroma_burst.kir`: Audio-transient-reactive radial dispersion that explodes outwards on kicks and snaps back (unlike static `rgb_shift`).
     - `film_grain.kir`: High-frequency organic film stock grain/noise to eliminate digital plastic look without heavy blur.
     - `slit_scan.kir`: Scan-line slit scan — `retains` with the `exit` cut, one line of `src` taken each frame and the held frame shifted along, so time runs across the frame. Written on today's one-frame retention. Known limit: the shift is a `frame_step` fraction and not whole texels, so `tap` resamples every frame and older parts soften. The per-position time-delay form needs a multi-frame history (M11 item 4).
   - **What the effects above owe**:
     - `beats` is readable in an L5, so beat sync needs nothing new. `onset` reaches a procedure only through a `bind`, and `bind` lives in a Set file; no binding for a master chain slot's params was found. Verify first; if absent, binding chain slot params to signals is part of this item.
   - **Dynamic Slot Manipulation & Operational Usability**:
     - Arbitrary slot insertion (`+ add`), deletion (`— remove`), and drag-and-drop reordering with live GPU cost budget tracking.
     - Curated master chain presets saved to and recalled from the Library (the chain-preset store lives here; M11 item 3 carries Set bundles only):
       - *Clean Cyber*: `bloom` + `chroma_burst` (the tone map is the fixed stage after the chain, set by `look`, not an L5)
       - *Retro Stage*: `crt_screen` + `analog_tv` + `film_grain`
       - *Psychedelic Echo*: `feedback` + `chroma_echo` + `slit_scan`
       - *Drop Assault*: `glitch_slice` + `negative_strobe` + `bloom`
   - The default chain stays empty (ir-spec *L5's chain*, ADR-0340: the default look is bit-identical for free); a preset is one Library recall away.
4. **L5 Nested in a Set, and What `retains` Answers There**:
   - Build ADR-0098's second role: an L5 inside a Set, folding its renderers' `uses … : Texture` slots through `edge`s. Specified in ir-spec *The two roles*; `compile::sort_compiled` still refuses it.
   - Specify which frame `held` is for a nested L5 (candidate: the Set's own previous output), after reading ADR-0238, which records the question open. This is what makes per-deck-slot reaction-diffusion, fluid ink and layer trails writable; today frame memory exists only in the master chain.
   - Replace the refusal text at `crates/karakuri-environment/src/compile.rs` (`kind L5` arm): it still says the master chain "is still three fixed passes".
5. **Authoring Corpus for the Exit Condition (`karakuri-mcp`)**:
   - A checked-in set of fixed prompts and the `.kir` an agent produced for each, run through `check_procedure`, asserting every file passes and stays within 30 lines.

**Exit Condition**:
- `cargo test -p karakuri-mcp --test authoring_corpus` passes (the test is item 5's).
- Live hotswapping across all L4 material presets and L5 master chains maintains a rock-solid 60 FPS under active budget governance.

---

### M11 — MVP Polish & Release Readiness

**Objective**: Final integration, usability refinement, advanced visual dynamics extensions, multi-hour stress testing, and packaging for initial 1.0 release.

#### Key Deliverables:
1. **Live MIDI Surface Mapping & Profile Management** *(Carried from M8)*:
   - Provide an in-app interface to load, edit, and persist MIDI controller maps (`.map` files) dynamically during performance.
   - Support 14-bit high-resolution MIDI CC mappings for ultra-smooth parameter sweeps.
2. **Wipe Mask Geometry Control** *(Carried from M8)*:
   - Expose explicit wipe front position and edge softness parameters on mixer strips.
3. **Library Management & Preset Store Refinements**:
   - Save and recall full Set bundles to user presets (master chain presets are M10 item 3's).
   - Tag taxonomy and category filtering across the preset library store.
4. **Advanced Dynamics & Extended Shading**:
   - **Audio Spectrogram / Waterfall History Texture** (L0): Continuous multi-frame FFT buffer exposed as a 2D GPU texture for flowing terrain and historical frequency visualization.
   - **Multi-Frame History Target for L5** (designed together with the spectrogram history above — both are a ring written one frame at a time): an L5 declares a frame count as a compile-time constant (e.g. `retains 32`) and reads any of the last N frames, so its cost is known before it is paid (P-0091), as `amplify`'s is. Held at reduced resolution, and the reduction is part of the declaration: Rgba16Float is 16.6 MB a frame at 1080p full size (32 frames ≈ 531 MB), 4.1 MB at half (32 frames ≈ 133 MB). Buys per-position time-delay slit scan, multi-tap echo and time displacement. Spec section in `docs/ir-spec.md` first.
   - **SDF Ambient Occlusion & Soft Shadow Helpers**: Standardized raymarch occlusion functions for depth-rich field rendering.
   - **Procedural Volumetric Fog / Light Shaft Extension**: Single-pass depth-aware atmospheric fog integration.
   - **Depth from Fullscreen Fields**: a fullscreen L4 (raymarcher) writes `frag_depth` at its hit, so meshes and marched fields in one Set occlude each other. Writing depth disables early-Z for that pass, which a pass covering the frame once can afford. Follows M10 item 1's depth buffer.
   - **Group Lifecycle for Meshes** *(specified in M10 item 1)*: engine and codegen for k-element spawn/kill groups, so a spawning source can carry a shared-vertex topology — per-particle ribbon trails, faces that shatter as units.
   - **Index-Shift Mesh Mode**: a declared opt-in under which a shared-vertex topology stays indexed while its source compacts freely. Deaths pull faces onto neighbouring vertices — a net that tears and re-stitches. Reproducible, since compaction is order-preserving and bit-exact (P-0092). It must be declared because the same picture unasked-for is the silently wrong image P-0094 refuses; indices beyond the live range are clamped into it. Spec section in `docs/ir-spec.md` first, then the lowering.
5. **Library Search & Caching**:
   - Interactive free-text search filtering across set names, procedure types, and metadata tags.
   - Cached off-screen thumbnail previews for rapid visual identification in the library browser.
6. **Session Last-State Recall & Slot Startup Initialization**:
   - Persist and recall each slot's last-played set/procedure across sessions so the performer re-opens into their exact live setup.
   - First-launch default starts with clean state, loading demo visuals per slot through the standard Load path, with Decks B–D muted under the unified Solo/Mute mixer architecture.
7. **Comprehensive Live Rehearsal Stress Test**:
   - 4-hour continuous burn-in test running multi-slot decks, active audio input, periodic set hotswaps, and concurrent MCP generation.
   - Memory leak audit (`#[global_allocator]` allocation tracking) confirming zero unbounded heap growth.
8. **Documentation Audit & Release Distribution**:
   - Complete synchronization of `docs/manual/` with all implemented operations.
   - Production build packaging for macOS and Linux.

#### Carried from M6 (closed 2026-09-14) and M8 (closed 2026-09-27), status:
- In-app dynamic MIDI map editing and 14-bit CC resolution deferred to M11; basic MIDI input binding and live learned maps are active.
- Mixer wipe edge softness and front position control deferred to M11; baseline wipe transitions and shape selection are fully operational.
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
- **External Video Texture Ingest**: Syphon, Spout, and UVC camera input capture surfaced as reactive texture sources for hybrid procedural visuals.
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
