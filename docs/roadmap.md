# Karakuri — Vision and Roadmap

Companion to `README.md` (invariants and V1 scope) and `docs/ir-spec.md` (the IR).

This document exists so that decisions made during V1 do not foreclose later milestones.
Where a later milestone imposes a constraint on earlier code, that constraint is stated
under **Demands on earlier work**. Those are the parts worth reading during V1
implementation even though the milestone itself is far off.

---

## The end state

A performer opens an empty session and types a prompt. Over the course of a set, the
system generates visual material, warms it up out of sight, and offers it at musically
sensible moments. The performer takes control of anything they want and leaves the rest to
the system. Everything generated is saved, searchable, and reusable in later sessions.
Nothing stops working when the network drops or the DJ gear changes.

Four properties define whether this succeeded:

1. **Procedural, not generated frames.** The AI writes procedures that run on the GPU
   every frame, not images or video. What gets saved is an instrument with parameters, not
   a recording.
2. **Manual and autonomous on the same mechanism.** A human and an agent affect the system
   through the identical interface. Authority is a per-layer setting, not a global mode.
3. **Reproducible.** The same record stream and the same seeds produce the same show.
4. **Unbreakable on stage.** Nothing stops working when the network drops or the DJ gear
   changes. Defended continuously rather than built once — see Continuous concerns.

### What this is not

- Not a general-purpose compositor. Video file playback and editing are out of scope
  except as an external source node.
- Not a 3D content creation tool. There is no modelling, no asset import pipeline, no
  animation timeline.
- Not primarily a live-coding environment for humans. The IR is written by LLMs and read
  by humans, not the other way round.
- Not a cloud service. The engine runs locally and keeps running when generation is
  unavailable.

### The reference point

TouchDesigner is the closest existing thing, and the deliberate divergence is worth
stating. TouchDesigner is largely one operator per GPU pass, connected by texture
ping-pong, driven from the CPU. Karakuri compiles a subgraph into a minimal set of passes
and fuses procedural chains into single shaders. The cost is a compilation step and less
immediate feedback; the gain is that primitive-centric generation at high element counts
becomes affordable.

---

## Architecture beyond V1

V1 implements L1 and L4 inside a single Set. The full model:

| Layer | Role | Milestone |
|---|---|---|
| L0 | Signal bus. Synthesized values from M1; audio and tempo landed in M2, MIDI has not | M1 / M2 |
| L1 | Geometry generation. Vertices, particles, SDF builtins used inline. The first-class `Field` type is M3 | M1 |
| L2 | Deformation and motion. Time-axis modulation, physics | M3 |
| L3 | Camera and space. Viewpoint, motion grammars | M3 |
| L4 | Render and material. Raster, raymarch, splatting | M1 |
| L5 | Composite. Set mixing, transitions, post, output routing | M2 |

Orthogonal to the layers:

- **Control plane** — node agents, director, mix agent, generation worker (M6)
- **Library** — search, genealogy, embeddings, previews (M4), on top of the content
  addressing that exists from M1. The separate metadata file it also wants does not exist
  yet — see M4's demands

### Three clocks

The single most load-bearing idea in the design. These must never collapse into each other.

| Clock | Period | What happens |
|---|---|---|
| Frame | 8–16 ms | GPU execution and parameter evaluation only. No allocation, no compilation |
| Beat / bar | 0.5–4 s | Variant switching, parameter morphs, transitions. Selection among precompiled options only |
| Generation | seconds to minutes | LLM writes IR, validation, shader compilation. Background worker |

The consequence: **AI works ahead of time; the runtime only selects.** An agent reacting to
music is choosing from a pool it prepared earlier and queueing generation for material it
expects to need later. No LLM call is ever on a path that a frame waits for.

### Slot interface contracts

What makes procedures reusable across arbitrary combinations. Introduced properly in M3.

```
L1 : ()                  -> Geometry | Field
L2 : Geometry            -> Geometry      // endomorphism, therefore stackable
L3 : ()                  -> Camera
L4 : (Geometry, Camera)  -> Texture + AOVs
```

`Geometry` is not monolithic. It declares topology, an attribute set, a count mode, and a
spatial domain. Compatibility is a subset check on attributes, with automatic adapters
where a derivation rule exists. This declaration-plus-adapter layer is what lets almost
any L2 in the library sit on almost any L1.

### Multiplicity within a layer

Each layer composes differently, and each needs its own semantics.

- **L1 multiple** — merge. Several geometry sources in one Set, which raises a question
  with no answer yet: **how two sources avoid colliding identities.** Identity is `seed`, a
  monotone spawn ordinal from a counter that is engine state and resets at Set start — one
  counter, described in the singular. Two sources either share it and interleave, or hold
  one each and collide immediately. The `{"t":"seed"}` record salts per *layer*, not per
  source, so it does not settle this either. Decide before L1 multiplicity is built.
- **L2 multiple** — chain. Order matters. Each modulator carries a weight and a mask, and
  masks are attribute-based: only elements where `seed % 3 == 0`, only inside a region,
  only where `age > 0.7`. This is the largest single source of expressive range in the
  system, because the same modulator becomes a different thing under a different mask.
- **L3 multiple** — weighted blend or cut. Blending interpolates trajectories, so an orbit
  and a handheld rig can be mixed at 0.3.
- **L4 multiple** — overdraw on shared geometry. The same point cloud drawn as points, and
  as lines between neighbours, and as an SDF raymarch. Geometry is shared, so the marginal
  cost is one draw pass. This is the cheapest visual variety per unit of GPU time in the
  whole system, and it is the payoff for being primitive-centric.

---

## Milestones

Sizes are rough and relative — a sense of which milestone is larger than which, not an
estimate of anyone's calendar.

### M1 — Closing the loop *(= V1, scoped in `README.md`)*

**Proves:** an LLM can generate constrained IR, we can validate it, compile it to WGSL,
and hot-swap it without dropping a frame.

One Set, hardcoded slots, no UI, synthesized signals only, L1 and L4 only. If this loop
does not close, the whole concept needs rethinking. Everything after this is engineering.

**Closed.** All three clauses hold; what remains unbuilt inside V1's scope is tracked in
`README.md`, not here.

What it cost, since it is the only evidence about how the later estimates should be read:
the loop closed, and then four separate things that had checked clean were found to come
up short at runtime — an unimplemented derivation rule, a Set-composition check that was
specified and written nowhere, a `spawn_rate` requirement no pass enforced, and a clock
that stood still across a frame's substeps. None was found by a test that existed.
Three were found by reading the code against the specification, and the fourth by
disbelieving a comment. Budget for that in every milestone below: the implementation is
not the expensive half.

~2–3 weeks.

---

### M2 — Playable

**Goal:** play a real one-hour set on this and survive.

**Landed so far**, in order: the deck with an L5 mix and a per-slot level meter; tone
mapping once after the mix; signal binding; priming, the budget governor and `closed_form`;
audio input with beat tracking; a record vocabulary for the mix. Each bullet below says
what it cost against what it promised — the ones marked **Done** are worth reading for
where the promise was wrong, not only for the fact that it is kept.

**Still open**, and the shape of the rest of this milestone: transport, transitions, blend
modes and masks, MIDI, output routing, Ableton Link, a panic key, per-slot preview, and
loading a Set file into the engine at all.

Five things the milestone has taught, all worth carrying:

- **A fader at zero must mean zero.** It did not: `0.0 * NaN` is NaN, and one NaN in one
  slot took the whole mix with it. A fader is the operator's last way out of broken
  material, and material generated by an LLM will be broken sometimes.
- **The watchdog judged Sets that were not on screen**, in both directions. The
  false-reject is the worse one — a tight budget plus busy neighbours discards good
  generated material in silence — and it is the shape every automatic decision in this
  milestone can take. Whatever the governor measures, it has to be measuring the thing it
  is judging.
- **A confidently wrong automatic decision is worse than no decision.** Tempo's octave was
  chosen by heuristics that read plausible and were wrong above 160 bpm; replacing them with
  a fold into a window centred on the grid deleted the judgement entirely and handed the
  residual case to the operator. Prefer arithmetic that cannot be subtly wrong, plus a key,
  over a rule that is usually right.
- **A test can pass against the exact defect it is named for.** Eight of those were found by
  review across this milestone — including one asserting that a callback allocates nothing,
  which fed the analyser a signal so flat that it returned before reaching the code under
  test. Watch a test fail before trusting it.
- **Two runs of the same path agree with each other under a wrong implementation.** The
  test for the priming fix compared two *warming* runs, so it pinned the rate out of the
  arithmetic and said nothing about which instant was right — a clock a whole frame early
  passed it, and so did one at the wrong scale entirely. It took a comparison against the
  *on-air* path to find that the fix had made full-rate warming diverge from Live within a
  second. **An invariance test needs a reference run, not a second run of the thing under
  test.**

Getting on stage early is not a vanity milestone. Live use surfaces failure modes that no
amount of desk testing finds — thermal throttling, a laptop lid closing, a set that looked
fine alone and is unreadable next to lights.

**Adds**

- Multiple Sets with a lifecycle. This bullet named five states — Cold / Warming / Priming /
  Live / Cooling — and three were built: **Live**, **Priming**, **Allocated**. Warming and
  Cooling turned out to be transitions rather than states: a Set arrives compiled from the
  build worker with nothing to warm into, and a slot taken off air simply stops being
  stepped. If a real Cooling appears it will be because something needs to fade rather than
  cut, which is the Transitions bullet below and not a residency level
- **The deck** — the one place prepared-but-not-showing material lives, at Set granularity.
  Priming Sets, the staging lane M5 draws, the pool an agent fills in M6, and whatever M7
  schedules ahead of a phrase are all this, seen from different angles. Naming it once here
  stops each of those milestones inventing its own waiting area. One to four members are
  Live and composited; the rest are resident
- Residency has three levels: **Live**, **Priming** (stepping hidden, at reduced rate), and
  **Allocated** (compiled, buffers held, not stepping). **The governor does not move slots
  between them** — it computes an *effective* level each pass from the operator's *requested*
  one, which it never writes. A refusal is therefore a deferral: a slot parked for lack of
  budget primes again by itself when there is room. Built, and the same shape as M5's
  greyed-out sync toggles — a surface shows what was asked for and, separately, what is
  happening. Two instances now; do not invent a third vocabulary for it.

  Allocated keeps its state, so returning to Priming resumes where it stopped rather than
  starting over — `t` is simulation time and does not advance while parked. Going straight
  from Allocated to Live shows an unwarmed image, which is the operator's call to make.
- Live preview of any slot's output, not just a Set's. Auditioning candidates is the basic
  workflow and it cannot be done blind, so this is a prerequisite rather than a convenience.
  It needs a **default renderer per topology** — a built-in L4 that draws raw geometry — so
  that a generated L1 can be seen directly rather than through whatever L4 happens to be
  paired with it. This is also how a broken L1 becomes diagnosable
- Priming — **stepping** hidden so stateful simulations reach their attractor before
  becoming visible. Without this, every fade-in shows particles being born, which usually
  looks bad. Built, and this bullet's original wording — "rendering hidden at reduced rate
  and resolution" — was superseded on contact: L1 owns every piece of per-element state and
  L4 is stateless, so warming is the compute passes and the render is skipped outright.
  There is no resolution to reduce, and skipping the draw makes "primed then Live"
  *identical* to "always Live" rather than close to it. Reduced rate survives and means
  stepping on one frame in *n*.

  **That identity cost one repair to be true of bound material.** A warming slot was
  handed the session's oscillator at the frame's phase, so a slot stepping one frame in
  *n* had its bound params swept *n* times as fast per step as the same Set warming at
  full rate — the governor's rate, chosen from frame budget and shown to nobody, deciding
  what the Set warmed into. It reads the same grid held back by the steps it has not
  taken instead. Two things that does not reach, both narrow and both recorded in the
  code: a tempo *change* during a slow warm-up, and measured audio, which has no past
  value to be read at
- Budget governor. Built, with two corrections to this bullet's original wording. Not
  per-Set **GPU timestamps** — a per-Set measurement taken by the probe on the build worker,
  calibrated and labelled with its method, because timestamps are advertised, enabled and
  unreliable here. And not priming **resolution** — step rate only, since priming does not
  render. A Set is measured before it goes on air, because an unmeasured Live slot is
  unbudgetable rather than free, and treating unknown as headroom is the same mistake as
  treating an unmeasured candidate as costless.

  **VRAM budgeting was not built**, and the original wording expected it: the deck was to
  have two budgets, compute bounding how many slots can step and VRAM bounding how many can
  be Allocated at all, with deck size an output rather than a layout constant. Only the
  compute half exists. VRAM needs an allocator's cooperation this engine does not have, and
  `MAX_SLOTS` is still the constant it was supposed to replace.
- ~~Closed-form procedures skip priming entirely.~~ **Done**, and it takes three conditions
  rather than the one this bullet named: no read of an emitted attribute, no `spawn` block,
  and no `kill()`. Spawning disqualifies however pure the `element` block is, because *which
  elements exist* is history — jumping to `t=30` gives one frame of newborns, not thirty
  seconds of population. The check pass decides it and the artifact carries it; the governor
  refuses to prime a closed-form slot because there is nothing to warm.

  Its larger payoff is not priming but **scrubbing** — see Transport above. Because the
  classification produces no diagnostic it can only ever be wrong silently, which is why it
  is deliberately conservative: strict-wrong costs a warm-up nobody needed, permissive-wrong
  is a scrub that renders garbage.
- L5 mixer. Per-slot linear gain and opacity are built and the composite reads them as one
  weight — under additive blending they are indistinguishable, and blend modes and masks are
  what would separate them. Those, and the layer stack, are not built
- Tone mapping, once, after the mix. Three different things get called exposure and only
  one of them belongs to the artifact: the `param exposure` inside a procedure is how bright
  that material is, the per-Set gain at L5 is how it balances against the others, and tone
  mapping is the transfer from unbounded linear HDR to something displayable. Today the
  first is standing in for the third, which is why an artifact authored at high element
  counts carries an exposure far below 1.0 — and that breaks the second, because a mixer
  fader means nothing if each Set arrives at a different nominal level. Semi-automatic gain
  needs a measured level per Set, which is the same per-Set measurement hook the budget
  governor needs.

  **Done, and it read true.** The tone mapper landed, the example's exposure went back to
  1.0, and the per-slot meter is the measurement semi-automatic gain would need — which is
  deliberately not built: the meter shows the number and nothing acts on it, because an
  exposure that moves by itself is the worst thing that can happen on stage.
- **Transport, per slot.** The mapping from session time to a slot's `t`, so that material
  can be run at a rate, held, or scrubbed — tape-style fast-forward and rewind, locked to
  the beat grid. Driven by a **position** rather than by a tempo and a phase, because a
  position can reverse and jump and a tempo cannot: audio analysis supplies a position that
  only ever moves forward, and a deck link (M7) supplies one that does not. Same entry
  point, so the downstream is not rebuilt when the better source arrives
- **What the transport can do depends on `closed_form`**, which the check pass now decides
  and the artifact carries. A closed-form procedure evaluates at any `t`, so scrubbing it is
  free. An accumulating one can only be run forward — but "cannot rewind" is too strong:
  because the engine is bit-exactly reproducible, restarting it and running to the target is
  *the same state*, not an approximation. So a rewind is priced rather than impossible, and
  the governor's per-Set measurement is what prices it. That is a second payoff of the
  determinism invariant, alongside undo, replay, A/B and session recording
- Transitions as first-class objects, not just crossfade
- `VideoSource` interface with `color` required and AOVs optional
- ~~Audio input. Spectrum, energy, onset detection, tempo estimation~~ **Done**, plus beat
  tracking with latency compensation. Analysis runs in the driver's callback rather than on
  the frame path. Two records carry it — `audio` per frame and `tempo` for corrections — on
  `tick`'s terms, so replay reads them back and opens no device. `tempo` carries the
  **correction, not the estimate**, so a session recorded today replays the same after the
  analyser improves
- ~~Wiring the signal bus into parameters.~~ **Done**, except for the decoder: a binding
  samples, curves, maps and blends by confidence into the uniform write, and the spawn
  accumulator reads the same value. What is missing is that a `Record::Bind` never becomes
  one, because nothing loads a Set file into the engine — bindings arrive by CLI flag,
  shaped like the record so that replacing the flag is deleting a parser. **The two
  diagnostics that guard the flag** — a `bpm` binding, which saturates because a tempo is
  not a `[0, 1]` signal, and a `noise.octaves` on a kind that has none — **live only in the
  flag today, and the decoder owes them too**
- **A record vocabulary for the mix.** `gain`, `residency` and `look`, built on every key
  that moves them, decoded back, and only then applied — the same arrangement `audio` and
  `tempo` have. Without it a session would replay the material and not the *performance*:
  the same Sets, on the same beat, all at whatever gain they happened to start at, with
  nothing ever going on or off air. `residency` carries the **request** and never the
  effective level, because the governor recomputes that from the budget of whatever machine
  is running, and replaying one machine's budget onto another's is not replaying a
  performance. `opacity` deliberately has no record: the deck has the control and nothing
  reaches it — no key, no flag — and a record for a control the operator cannot move is one
  more record nobody writes, which is the condition this closes rather than extends
- **Parameter values keyed by `(layer, name)`.** The record format already does it; the
  engine holds one flat map, so an L1 and an L4 declaring the same parameter name shared a
  value, with the L4 default silently winning. `Set::build` now refuses the collision
  rather than resolving it, which is correct and is not the answer — a Set whose two
  procedures both want a `hue` is not an error
- ~~PLL correction of the local oscillator against external tempo~~ **Done**, and shaped by
  what it is for: tempo is stable except at a track change, so the grid is **predicted, not
  chased**. A locked grid free-runs and takes a slow trim; a single disagreeing estimate
  moves it not at all; re-acquisition needs eight consecutive consistent revisions. The
  correction **leads** by the analysis lag plus the output lag, because a loop that merely
  tracks shows its beat late by both, consistently — which reads as wrong rather than as
  noise
- Ableton Link as a passive peer that never proposes a tempo
- MIDI control surface on a dedicated controller, not the DJ controller
- Output routing: Syphon / Spout / NDI
- Panic key to a known-good Set

**Demands on earlier work**

- Set must already be the compilation and lifecycle unit in M1, even with one Set
- `VideoSource` must exist as a type in M1
- **One `begin_frame` per encoder has to become structural before there are several Sets.**
  M1's hot swap replaces the live Set only at the top of `begin_frame`, which is correct but
  is a convention rather than something the types enforce: a caller can open one encoder,
  end the borrow, take a second Set, and record both into it. With one Set and one frame
  loop that is a comment; with a deck compositing up to four and priming the rest it is a
  frame built from two different simulations. The fix is a guard owning both the `&mut Set`
  and the encoder, submitting on drop — cheap now, a change to every call site later.
  **Discharged**: `Deck::begin_frame` owns the encoder and a second frame while one is open
  is `E0499`. What it does *not* stop is named in `deck.rs` rather than left implied —
  `mem::forget` on a frame desynchronises a Set permanently, and no guard closes that
- Every signal consumer must branch on confidence, never on provider presence
- The fork-and-swap invariant must hold from M1, because it is how Sets get edited live

**Decide before building**

- ~~Whether GPU timestamps work on the performing machine.~~ **Decided: do not steer on
  them.** They are advertised, enabled and unreliable here, and flaky is worse than absent
  because it quietly returns a usable-looking number. The governor budgets against a
  calibrated, method-labelled probe measurement taken on the build worker instead — off the
  frame path, comparable between slots, and honest about being a reference figure rather
  than a prediction of frame time. What it cannot see is written down where a reader of the
  report will look: one frame of a cold Set at a fixed resolution, never re-measured.
- ~~Whether noise rate stays tempo-relative once tempo is corrected.~~ **Decided: it
  stays.** A noise binding follows a *tempo* correction and ignores a *phase* one — the
  oscillator carries two anchored beat counts so that a rate is a rate while a grid
  realignment does not re-hash the lattice. A seconds-relative mode and a freeze-at-bind-time
  were both rejected for making two kinds of noise and leaving every existing `bind` record
  ambiguous. The question was live rather than hypothetical the moment correction landed,
  which is when it was answered.

~6–8 weeks.

---

### M3 — Expressive depth

**Goal:** the combinatorial range that makes the library worth having.

**Adds**

- L2 and L3 as IR `kind`s with their own slots
- Multiple L1 sources, and the `source` attribute that keeps their identities apart. Per
  source `seed` counters starting at zero so structured layouts survive, and a per-source
  hash salt so randomness differs without structure differing
- **L2 amplification** — a second kind of L2 whose output count differs from its input.
  Kaleidoscopes, instancing, trails and subdivision are all unwriteable today because
  `Geometry -> Geometry` is an endomorphism. The factor is declared and constant, amplifying
  stages multiply rather than compose, and the output is derived and rebuilt each frame so
  it needs neither double buffering nor compaction. This is the geometry-side version of the
  argument for drawing one point cloud several ways
- Cross-source interpolation, restricted at first to static sources where `seed` is the slot
  index and the paired read is a direct one
- Slot interface contracts, attribute declarations, automatic adapters
- L2 stacking with weights and attribute-based masks
- Multiple L4 renderers over shared geometry
- The `Field` type — a spatial function represented as code rather than data
- Graph compiler. Node graph as authoring representation, render graph as execution
  representation, with fusion of `Field` chains into single shaders
- `blend weighted` (weighted blended OIT) alongside `blend additive`

**Demands on earlier work**

- The `blend` declaration must exist in the L4 header from M1, even with one legal value
- Identity must already be per element and carried, not a slot index, or multiple sources
  cannot be told apart at all
- IR must be a real IR from M1, not a thin wrapper over WGSL, or fusion has nothing to work
  with
- `capacity` must already be a Set-level value rather than baked into the artifact

**Clear before building**

- ~~Pack attributes into one storage buffer per direction.~~ **Done in M1**, because
  compaction had to be written against the packed layout or written twice. The compute
  stage binds 4 buffers now and stops growing with the attribute count, so L2 stacking no
  longer walks into the WebGPU default limit of 8. What was *not* done is narrowing each
  slot to its attribute's natural width: every slot is still a padded 16 bytes, which
  doubles VRAM per element against what it needs. That bill comes due at M2's deck, where
  the constraint is how many Sets fit resident, and it is one function in `layout.rs`.
- **An L4 is now compiled against a specific L1's element layout**, since both declare the
  same struct over the same buffer. That is the slot interface contract arriving early and
  informally. When the contract becomes a real declaration, it should subsume this rather
  than sit beside it.
- **Give an L4 procedure a way to say what it renders.** Quad expansion is justified by
  `topology points`, but `topology` is declared on the L1 header and a checked L4 tree has
  no field for it. One topology and one blend mode hide the problem; M3 adds a second of
  each.

~8–10 weeks.

---

### M4 — Library at scale

**Goal:** finding the right thing among two thousand artifacts is faster than generating a
new one.

**Adds**

- Library thumbnails. Every artifact gets a short loop and a still at promotion time, for
  browsing. Distinct from the live slot preview built in M2 — that one renders a running
  instance, this one is a stored asset
- Dual embeddings. Text embedding of prompt and tags, plus a visual embedding of the
  preview. Visual search matters more than it sounds — under stage conditions people
  search by look, not by words
- Genealogy. Derivation graph via `parent`, enabling "make ten variations of this"
  evolutionary workflows
- Variant pools. A slot holds several compiled alternatives, selectable at beat resolution.
  Mechanically these are pre-forked Sets sharing every other slot, so they are deck members
  rather than a separate structure — and they cost what their differing slot costs. Three
  alternatives that differ only in L4 share one simulation; three that differ in L1 are
  three simulations. Selection is close to free for closed-form material and expensive for
  accumulating material, which is the same distinction priming turns on
- Bundle and unbundle for sharing single self-contained patch files

**Demands on earlier work**

- Content addressing and separate metadata files from M1. **Half done.** The store is
  content-addressed and tested; the metadata file is specified and does not exist — no
  record type for it is in the vocabulary `karakuri-store` decodes. See "Metadata file
  format" under Beyond v0.2 in `docs/ir-spec.md`
- `parent` recorded from the first generated artifact, or the genealogy has a hole at the
  root. Nothing is recorded yet, and the first generated artifact is close — this is the
  demand most likely to be missed by simply arriving late

~4–6 weeks.

---

### M5 — Interface

**Goal:** a surface where a human can see what the system is about to do and disagree with it.

This comes before agents deliberately. An autonomous system that cannot be observed and
overridden is not usable on stage, and building the observation surface afterwards means
retrofitting it into decisions already made.

**Adds**

- Node editor for the graph model introduced in M3
- Parameter surfaces with MIDI learn and signal binding UI
- Set browser with live previews of priming Sets
- Staging lane — where candidates appear before they go live. Its first producer is the
  operator's own regeneration of a slot, which needs no agents and makes the lane useful
  and testable as soon as it exists; M6's agents write to the same place rather than
  inventing one. Because a rejected candidate costs nothing — the previous artifact is
  still in the library and the slot record still points at it — this is an A/B between two
  versions, not a merge tool
- Per-layer mode control: Manual / Suggest / Auto
- **Per-slot tempo-sync and beat-sync toggles, with beat-sync greyed out for accumulating
  material.** The two are different requests and only one of them is always available:
  tempo-sync changes the *rate*, which any procedure can follow because it only ever means
  stepping more or fewer times; beat-sync locks the *position*, which needs the ability to
  jump and reverse. Continuous beat-sync is therefore closed-form only — an accumulating
  slot re-running its whole history on every correction is not a feature. A *one-off* scrub
  is available to anything that fits the budget, so what the surface shows there is a price
  ("8 bars back — 340 ms") rather than a disabled control.

  Worth noting what makes the greying-out possible: `closed_form` is decided statically in
  the check pass, travels with the artifact, and arrives at the interface as a fact. Had it
  been left for the engine to infer at runtime, the surface could not have said anything
  until the Set was built and run.

~8–10 weeks.

---

### M6 — Autonomy

**Goal:** the system prepares material without being asked and is safe to let run.

**Adds**

- Slot agents, one per slot rather than per graph node. Each owns a prompt, watches
  designated inputs, selects from its prepared variants, and queues generation for material
  it expects to need. Autonomous selection means choosing among what already exists, never
  generating on a path the frame waits for
- Director. Cross-layer coherence — whether the camera behaviour suits the geometry,
  whether L4 is killing the shape L1 produced
- Mix agent. Set-level arc, energy trajectory, monitoring for staleness
- Authority model. Budget constraints ("camera changes at most once per four bars"),
  and manual intervention instantly demoting that layer's agent to `Suggest`
- Generation queue with priority and cost awareness

**Demands on earlier work**

- The record stream must be the sole mutation path, so that an agent is structurally
  incapable of doing anything a human could not do through the same interface
- Parameter schemas with ranges on every procedure, from M1 — an agent with no declared
  ranges has nothing to search over

~8–10 weeks.

---

### M7 — Musical foresight

**Goal:** transitions land on phrase boundaries because the system knew they were coming.

Audio analysis is reactive; it reports what already happened. Track analysis is
anticipatory. This is what makes generation-time latency compatible with live performance:
knowing a chorus arrives in 32 bars is enough time to generate, compile, and prime.

**Adds**

- rekordbox integration. Beatgrid and transport position, current track identification,
  and phrase structure (`PSSI`) from the local collection database
- Lookahead timeline as a queryable structure, not a signal — "the first Chorus within 16
  bars"
- Backward scheduling. Transition point fixes priming start, which fixes generation start.
  "Will not make it" becomes a decision to postpone rather than a dropped frame
- Track metadata into visual direction: key to palette, colour tags, and a convention for
  writing VJ hints in the rekordbox comment field
- Optionally PRO DJ LINK via beat-link and crate-digger, if playing on club CDJs

**Demands on earlier work**

- The Set lifecycle must already be schedulable by absolute time, not just triggered
- Signal confidence must already drive transition policy, since these sources are
  unofficial reverse-engineered integrations that will break

~6–8 weeks.

---

## Continuous concerns

Not milestones. These degrade silently if not defended at every step.

### Live safety

- No allocation or compilation on the render thread, ever
- Watchdog on new pipelines with automatic rollback
- The show continues when the generation API is unavailable. This becomes meaningful once
  there is a pool to fall back on, so it is a promise from M4 onward rather than from M1
- Graceful degradation for every input source, with confidence driving how conservative
  transition scheduling becomes

### Determinism

Reproducibility is not a nice-to-have; it is what makes the record stream meaningful.
Undo, replay, A/B comparison, and session recording all follow from it. Anything
introducing run-to-run variance — atomic append ordering, real frame delta, wall-clock
seeds — breaks all four at once.

### Performance discipline

Per-element cost is the artifact's intrinsic property; total cost is a function of capacity.
Metadata records the former. Any change touching the frame path comes with a GPU timestamp
measurement.

---

## Deferred by decision

Designed, understood, deliberately not scheduled.

**External shader source compatibility.** Shadertoy-style material does not fit the
primitive-centric model directly: it is a single fullscreen fragment shader, with no
geometry stage and no parameters. Two import modes were designed — wholesale, as an opaque
L4 fullscreen node, and extraction, where an LLM pulls out the SDF or noise function and
discards the raymarch loop so it becomes a composable `Field`. The real value in both is
parameter lifting: hardcoded constants become declared `param`s, turning a frozen image
into an instrument.

This is deferred rather than dropped because `VideoSource` makes it cheap later. A foreign
source only needs to produce a colour texture. It slots in beside a Set without touching
the Set model.

Note that imported material is often extremely expensive. Allowing it makes the budget
governor mandatory rather than advisory.

**Also deferred:** projection mapping and multi-output warping, DMX and Art-Net lighting
sync, collaborative multi-operator sessions, video file playback, and any browser or wasm
target. The engine is native for Syphon, NDI, low-latency audio, MIDI, and GPU timestamp
queries.

---

## Settled decisions

Reference, not history. These are in force, they constrain what later milestones can
choose, and changing one is a redesign rather than an edit. The reasoning behind each is in
`docs/ir-spec.md`; what follows is the shape.

**Identity and randomness**

- Element identity is `seed`, a monotone spawn ordinal carried per element. There is no
  `id`: a buffer slot index stops being an identity the moment compaction moves elements.
- `seed` is an ordinal, not a random number. Randomness comes from the hash builtins, which
  are salted per layer from the seed stream — so re-seeding a Set changes its randomness
  without touching anything structural.

**Time**

- `t` is simulation time, never wall clock. It advances by `steps * dt` where `steps` comes
  from a `tick` record: emitted from real time when live, read back verbatim on replay.
  Nothing in the engine measures anything. **The emission is not built** — `steps` reaches
  the engine as a number and no `Record::Tick` is constructed outside tests. What the
  invariant rests on is already true: the engine advances by a step count and never by a
  duration, so the record serialises a quantity that exists rather than one that has to be
  invented. `README.md`'s Invariants list what else is in this position.
- Two tick histories reaching the same elapsed time are the same point in the session.
  Substepping exists so that state at a given time does not depend on frame rate, so
  nothing downstream may distinguish them.
- A **Set file** is a state projection with no time in it. A **session stream** is the
  timeline. Keeping them apart is what stops saving a Set from saving a performance.

**Ordering**

- Compaction is order preserving. Bit-exact reproduction depends on elements being combined
  in the same order every run — floating-point addition is not associative, so this holds
  under additive blending as much as anything else. Atomic allocation is cheaper and
  forfeits it.

**The IR surface**

- The signal bus is unreachable from IR. External values arrive as a `param` with a `bind`
  record, which is what keeps every external coupling declarative and adjustable.
- Every name the language gives meaning to is reserved against params and locals:
  attributes, ambients, stage outputs, `id`. Generated WGSL additionally mangles all
  IR-derived identifiers, so a procedure cannot capture a generated name whatever it is
  called.
- One `t` value means one record shape, across every file. A decoder dispatches on `t`
  alone, and every ndjson decoder does.
- `capacity` is a Set-level dial with a range declared by the artifact, passed as a uniform.
  It is not part of a procedure's identity — otherwise the library multiplies by every size
  anyone wanted.
- Spawn timing is engine-side. No ambient exposes the birth fraction: an exposed one is
  forgotten by half the generators that need it and applied twice by the other half.

**Editing and revision**

- A Set **value** is immutable and content-addressed; a **compiled instance** is not. Every
  structural change produces a new value and a record, which is what makes replay work.
  Whether the engine rebuilds the instance or updates it in place is an implementation
  choice, and it may update in place whenever the change needs no reallocation and no
  recompilation. Editing in the background is therefore fast without punching a hole in the
  record stream. "Never mutate a live Set in place" reads as a weaker claim than the rule
  it belongs to; the rule is about values.
- The cheapest correct answer to a revision request is usually not regeneration. Try the
  existing parameter range first — a uniform write, effective within the frame — then a
  range change, which needs a fork but no compile, and only then new code. This is what
  makes mandatory parameter ranges pay off a fourth time.
- Regeneration is destructive at slot granularity and that is fine. The other slots are
  untouched by construction, the previous artifact is still in the library, and a slot
  record points back at it. What guards against a bad generation is not caution in the
  prompt but the validation pipeline, the probe, and automatic rollback.
- Reference material handed to a generator is part of what produced an artifact and belongs
  in `origin` with the prompt. Two artifacts from the same words that differ because
  different examples were supplied are otherwise unexplainable.

**Craft knowledge**

- What a user saves is the prompt that worked, alongside the thing it produced. That is a
  view over the library — favourites, their `origin.prompt`, and their thumbnails — rather
  than a subsystem. Generation assembles a fixed preamble the system owns (the language,
  the viewing conditions, the reserved names) plus whatever the operator supplied.
- Such a palette expresses taste and nothing else. The moment an entry is compensating for
  a missing engine feature — an exposure value that only works at one element count, say —
  it has become a tone mapper written in prose and shipped to every prompt forever. Fix the
  feature instead.

**Diagnostics and cost**

- One severity. No warnings, because the response to a rejection is to regenerate rather
  than proceed with a caveat. That puts the burden on the diagnostic: a cost rejection
  states the estimate, the ceiling, and what dominated, since "over budget" gives a repair
  prompt nothing to aim at.
- Cost is three separate figures, each counted against a different unit — the mistake to
  avoid is calling them all "per element". `element` scales with live population,
  `spawn` with spawn rate, `fragment` with covered pixels. They share a unit and must never
  be summed.
- An artifact records cost per element; total cost is a function of capacity and therefore
  belongs to the Set. For L4 the published figure is a measurement at stated resolution and
  parameters, because fill-rate-bound cost has no meaningful per-element form.

---

## Document map

| | |
|---|---|
| `README.md` | What exists, how to run it, and the invariants in force |
| `docs/ir-spec.md` | The IR: grammar, semantics, lowering, record formats |
| This file | Milestones, their demands on earlier work, and the settled decisions above |

Neither of the other two should restate this one, and this one should not restate them.

---

## Reading order for implementation

1. `README.md` — invariants, then V1 scope
2. `docs/ir-spec.md` — the whole thing before writing any parser code
3. This document — the **Demands on earlier work** sections only, during M1
