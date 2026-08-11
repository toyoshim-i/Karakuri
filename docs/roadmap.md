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
audio input with beat tracking; a record vocabulary for the mix; the `beats` ambient;
per-slot transport; Set files, saved and loaded; a session writer and a replay driver; the
L5 blend modes. Each bullet below says
what it cost against what it promised — the ones marked **Done** are worth reading for
where the promise was wrong, not only for the fact that it is kept.

**Still open**, and the shape of the rest of this milestone: masks, output routing, and
Ableton Link. The panic key is **decided against** rather than pending —
see its bullet.

The one debt this milestone was carrying — **`Deck::set_opacity` unreachable**, no key and
no flag, and therefore no `opacity` record — **is paid**, and it was paid by building the
thing that made a second fader mean something rather than by wiring a key to it. See the
L5 mixer bullet.

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
- ~~Live preview of any slot's output, not just a Set's.~~ **Done for what a slot can hold
  today**, and the rule it turned out to need is one line: **an audition adds a draw and
  never a step.** Stepping a slot because somebody looked at it would break the property
  every residency level rests on — `t` advances through `Set::prepare` and nowhere else, so
  a slot taken off air and put back resumes where it stopped — in the least visible way
  available: the operator watches a running image, puts it on air, and it is somewhere other
  than where they left it.

  It is **not a second pass**. The mix runs as it always does with the previewed slot's
  terms forced to unity and every other slot skipped, so `0.0 + 1.0 * src` puts that slot's
  own texels in the target and the tone mapper, the present pass and a readback are all
  unchanged. An operator judges the material through the transfer curve it will be shown
  through. The faders are deliberately ignored: what is being judged is the level the
  material arrives at, which is the input to setting a fader rather than the output of
  having set one — the same ordering the meter measures in. The previewed slot is metered
  for that reason, which is what makes an audition an audition rather than a look.

  **Priming and preview compose, and neither had to learn about the other.** A Set that has
  never been stepped has no element state to draw, so auditioning a candidate that arrived
  into an off-air slot shows black — and the level that exists for exactly this steps it out
  of sight. Park it, prime it, look at it.

  Three things it costs, all named where the code is rather than only here. **The extra
  pass is unbudgeted** — the governor charges an Allocated slot nothing and a Priming one a
  fraction, and an audition makes both pay a full L4 pass; it also lands inside the frame
  interval the hot-swap watchdog judges candidates on, so auditioning something heavy can
  roll back an unrelated slot's build. **An audition is unfiltered**, so it bypasses the
  fader's skip and material that has gone NaN reaches the output — right for what an
  audition is *for*, and the one place "a fader at zero means zero" does not hold. And
  **auditioning a warming slot shows it at its own grid position**, which is behind the
  room's, so material that reads `beats` moves when it goes on air. That last one was
  invisible while priming did not draw, and `Set::prepare_warming` argued the split was safe
  precisely because "the frame before was not drawn" — an audition is the case where it is.

  What is **not** built is the other half this bullet named: a **default renderer per
  topology**, a built-in L4 that draws raw geometry so a generated L1 can be seen without
  whatever L4 it happens to be paired with. It is not built because it cannot yet be needed:
  `Set::build` takes an L1 and an L4, so there is no such thing as a slot holding geometry
  with nothing to draw it. That arrives with M5's staging lane and M6's generation, and it
  should be built then rather than guessed at now
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
- ~~L5 mixer.~~ **Done for gain, opacity and blend; masks are not built.** This bullet said
  gain and opacity were "indistinguishable under additive blending, and blend modes and
  masks are what would separate them", and blend modes duly separated them: every mode
  composites as `acc <- mix(acc, f(acc, gain * src), opacity)`, so gain is the level the
  material arrives at and opacity is the fader across the blend. It also **discharged the
  debt this milestone was carrying** — `Deck::set_opacity` had no key, no flag and no record
  precisely because there was nothing for a second number to mean.

  **The modes are `add`, `over` and `max`, and the set was chosen by the pipeline rather
  than by the vocabulary.** A VJ mixer lists `screen` and `multiply`; both are
  display-referred, defined on `[0, 1]`, and this mix is unbounded linear HDR with no tone
  map upstream of it — `screen` of two 2.0s is 0.0, which is not a blend mode but a bug with
  a familiar name. They belong after the transfer curve or not at all. That is worth
  carrying past M2: **an operator's vocabulary is not automatically well defined in the
  space the engine works in**, and importing it unexamined is how a control comes to mean
  something the operator did not ask for.

  `over` cost one change outside L5, and it is the interesting one: it needs to know what a
  layer *covers*, and the alpha channel of a slot target was written by nothing. The L4 pass
  now accumulates coverage there — colour adds, alpha composes as `over` — so the mix reads
  premultiplied colour. Sparse material barely covers, which makes `over` on a thin point
  cloud read close to `add`: correct rather than a defect, and worth saying because it looks
  like one.

  **What that channel is not is bounded**, and finding out cost a review pass. Nothing in
  the pipeline clamps what a `fragment` block assigns to alpha — the IR says values above
  1.0 are expected and means it — so an L4 writing `1.5` gives `over` a coverage of 1.5,
  and `A*(1 - 1.5)` is a hiding layer that *subtracts*. At 2.0 and beyond it amplifies with
  the sign flipped. The mix saturates on the way in rather than L4 clamping on the way out,
  because clamping the fragment would change the colour too: additive blending multiplies
  colour by that same alpha.

  The general shape is worth more than the fix: **a channel nothing wrote and nothing read
  was a free variable, and giving it a reader made every value the material could put there
  into an input.** It had been out of range all along and there was nothing to notice

  The layer stack is still slot order, which is what determinism wanted anyway, and there is
  no reordering control. Masks are the remaining half of this bullet
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
- ~~**`beats`, the tempo grid readable from IR.**~~ **Done.** Before it a procedure had `t`
  and nothing else, so the picture ran at wall time whatever the music did — a tempo change
  moved the grid every binding is sampled on and moved nothing that was drawn. A `bind`
  could not close it: a binding writes one `param`, and what follows a tempo is not a
  parameter value but the passage of time. Per substep like `t`, the same instant `t` names,
  and continuous across a correction, which matters because a trim arrives several times a
  second and a grid that recomputed history on each one would shimmer.

  **This is following, not seeking.** A procedure reading `beats` knows where the room is;
  it still cannot be evaluated at another time, and that is the whole of what the transport
  below adds. What `beats` did settle is the question underneath it: a procedure that reads
  the grid is a function of the grid as well as of `t`, and the grid at a past `t` is not a
  function of `t` — so a scrub answers against the grid **as it stands**, which is what
  seeking to bar 32 should mean, rather than against the grid that was running then, which
  nothing keeps
- ~~**Transport, per slot.**~~ **Done**, and driven by a position exactly as this bullet
  asked. Three modes: `free` is wall time, `tempo` scales the rate, `beat` locks the slot's
  clock to the room's musical position and therefore jumps and reverses. The scrub is on a
  key so that the reversing path runs today rather than waiting for a source that can
  reverse — audio only ever moves forward, and a deck link (M7) arrives at the same entry
  point.

  **It turned out to be two mechanisms rather than one**, and `closed_form` is the seam:
  a seek can place a closed-form clock in one element pass, and an accumulating one can
  only be given a rate. So `beat` is refused on accumulating material — where the operator
  asks, not where it would misbehave, because seeking an accumulating Set evaluates it once
  from wherever it happened to be and that is garbage rather than an error.

  **A second refusal was not foreseen and is the more interesting one.** Material that reads
  `beats` already follows the room; scaling its clock by the tempo as well makes it follow
  twice, at roughly the square of the tempo ratio. The check pass records whether a
  procedure reads the ambient, and `tempo` is refused on material that does. Two controls
  that each look right and compound into nonsense is a shape worth watching for elsewhere.

  The anchor — what tempo this material calls 1× — is a per-slot dial because **material has
  no intrinsic tempo**: a `.kir` declares parameters and a capacity, not a bar length. It
  defaults to the tempo at the moment sync is engaged, so engaging it moves nothing
- **What the transport can do depends on `closed_form`**, which the check pass now decides
  and the artifact carries. A closed-form procedure evaluates at any `t`, so scrubbing it is
  free. An accumulating one can only be run forward — but "cannot rewind" is too strong:
  because the engine is bit-exactly reproducible, restarting it and running to the target is
  *the same state*, not an approximation. So a rewind is priced rather than impossible, and
  the governor's per-Set measurement is what prices it. That is a second payoff of the
  determinism invariant, alongside undo, replay, A/B and session recording
- ~~Transitions as first-class objects, not just crossfade.~~ **Done, and the object it
  wanted turned out to be smaller than a crossfade rather than larger**: one control, one
  destination, one musical duration, one curve. A crossfade is two of them issued together,
  a fade-in is one, and a cut on the bar is one with a duration of zero — so what is
  first-class is the *scheduled move*, and every gesture anyone names is made of those. A
  `Crossfade` type would have been one gesture with the other three left out.

  **It is the first thing in the engine that *schedules* on the middle clock.** The
  three-clock table has said since M1 that beat and bar are for "variant switching,
  parameter morphs, transitions"; `Sync::Beat` already follows the grid continuously, and
  this is the first thing to arrange to happen at a named instant on it. A transition is a
  function of the
  beat count and of nothing else — not wall time, not frames — so the same records produce
  the same fade on a machine running at a different rate, and a tempo correction mid-fade is
  *correct* rather than a glitch. Eight beats is eight beats.

  Where a fade starts *from* is read when it is **scheduled**, not when it begins, and the
  reason is the paragraph above: the start is a musical instant, so "the value at the start"
  would be captured on the first *frame* at or after it and a machine at a different rate
  would capture it at a different beat. Nothing can move a control in between anyway — a
  hand cancels, another transition replaces. What a scheduled move does *not* do is touch
  its control before it starts, which would freeze a fader for up to a bar between the press
  and the music.

  Two rules it needed, both of which state something larger than themselves. **The values a
  fade produces are not recorded** — one record schedules it and the grid determines the
  rest, which is `tick`'s shape seen from the other end and is what the three-clock model
  means by the runtime selecting rather than computing. And **the operator wins**: a hand on
  a control cancels whatever was moving it, because the one place an operator reaches when
  something is wrong must not be the one place an automatic thing is writing. That is M6's
  rule for agents — manual intervention demoting a layer to `Suggest` — arriving early
  because the first automatic writer arrived early.

  What is not built: a transition that is not a fade. A wipe wants a mask and a stutter
  wants a clock the transport does not offer, so both are the next bullet's and the
  transport's rather than this one's
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
  shaped like the record so that replacing the flag is deleting a parser. **The decoder
  has landed and the debt is paid** — the two diagnostics that guarded the flag, a `bpm`
  binding and a `noise.octaves` on a kind that has none, live in the decoder now and the
  flag reaches them by *building the record and decoding it*. One rule rather than two
  copies of it, so a command line and a Set file cannot mean different things by the same
  fields. The one check that stayed with the flag is the one a record cannot express:
  `BindNoise::octaves` has a serde default, deliberately, so a record cannot say whether
  `octaves` was named and only the flag knows
- **A record vocabulary for the mix.** `gain`, `opacity`, `blend`, `residency` and `look`,
  built on every key
  that moves them, decoded back, and only then applied — the same arrangement `audio` and
  `tempo` have. Without it a session would replay the material and not the *performance*:
  the same Sets, on the same beat, all at whatever gain they happened to start at, with
  nothing ever going on or off air. `residency` carries the **request** and never the
  effective level, because the governor recomputes that from the budget of whatever machine
  is running, and replaying one machine's budget onto another's is not replaying a
  performance. `opacity` deliberately had no record here — the deck had the control and
  nothing reached it, and a record for a control the operator cannot move is one more record
  nobody writes, which is the condition this closes rather than extends. **It got a record
  when it got a control**, which is the rule working rather than an exception to it
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
- ~~MIDI control surface on a dedicated controller, not the DJ controller.~~ **Done for
  input**, and it turned out to be the first real test of an invariant rather than a feature
  of its own. `README.md` says the record stream is the sole mutation path, and the reason
  M6's agents will be safe to run is that they can do nothing a human could not do through
  the same interface — a surface is the first thing to put that claim under load, because it
  is the first thing that is not the keyboard. Every arm of the connection ends in the
  method a key press ends in, so it holds by construction: **a session recorded from a
  controller replays with neither controller nor map attached.**

  Building it is what found the one arm where it was already false: the beat tap moved the
  grid and dropped the `tempo` record it had just built, so a session replayed on a
  different phase from the one it was played on. Two keys had been doing that; the claim is
  what made it visible. Where it is still short is the replay driver rather than the
  surface: a session stream has no way to say what a deck *held*, so `--replay` builds a
  deck of one and reports every record naming another slot — and `examples/surface.map` is
  four slots wide, so most of a surface's moves are skipped on the way back. That gap is
  the format's and it is named in `docs/ir-spec.md` where the records are.

  The map is a **file and is not in the stream**, which is the same argument `residency`
  makes from the other end: which knob is which belongs to the hardware in the room, and
  replaying one room's wiring in another is replaying the furniture. With no map, every
  message prints the line that would map it — a learn mode with no UI to learn into, which
  is what M5 arrives to replace.

  Two things not built and named rather than implied: **MIDI out**, so a surface's LEDs and
  motorised faders do not follow the deck, which matters the moment two things can move a
  fader; and **14-bit control changes**, so a fader is 128 positions, about 0.8% of its
  range per step
- Output routing: Syphon / Spout / NDI
- ~~Panic key to a known-good Set.~~ **Decided against, and the reasoning is worth more
  than the key would have been.**

  Everything a panic key would undo is already manually recoverable: `space` takes a slot
  off air, `\` returns the gain, `'` walks the fader back to 1.0 in ten presses, `` ` ``
  returns the exposure, and the status line says what each of them currently is. So the key
  would not add a *capability* — it would add doing all of it at once, without choosing.

  What killed it is what "without choosing" costs. **An anomaly is usually one frame and
  usually harmless**: a shader dividing by a value that reaches zero is one of the most
  ordinary things a shader does, and what it produces is a blown-out pixel nobody notices.
  A control that resets a performance in response to that does far more damage than the
  thing it is responding to — and an operator who has learned that the panic key is
  sometimes the wrong answer has a control they hesitate over, which is the one property a
  panic key must not have.

  It also could not have done the job it was named for. "A known-good Set" implies rolling
  a slot back to what it was showing, and there is nothing to roll back to:
  `HotSwap::previous` exists only inside a judging window and is retired the moment a build
  is accepted. A standing rollback target would mean holding a whole Set resident per slot,
  which is the VRAM budget this milestone did not build.

  **What was missing was not a recovery action but a way to see.** The meter now reports
  the texels it left out of a reading — the bullet directly below — and decides nothing.
  That is the third time this milestone has landed on the same answer, after the tempo
  octave and semi-automatic gain: *show the number, act on nothing.*
- **The meter no longer loses a slot's reading to one bad texel.** Luminance that is not a
  finite number is counted and excluded rather than summed, because one NaN admitted to a
  sum makes the mean NaN — so a routine shader artifact used to take away the number an
  operator sets faders by. The count is reported beside the mean and the peak and is
  deliberately **not** a fault indicator: being ordinary and mostly harmless is exactly what
  makes it a bad proxy for "this material is wrong", and a warning that fires on normal
  material teaches an operator to ignore warnings

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
