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
| L0 | Signal bus. Synthesized values from M1; audio, tempo and MIDI landed in M2 | M1 / M2 |
| L1 | Geometry generation. Vertices, particles, SDF builtins used inline. Built, and `kind Field` made the SDF builtins reachable: `examples/melt_blob.kir` is a shape and nothing else, `examples/field_march.kir` marches one | M1 / M3 |
| L2 | Deformation and motion. Time-axis modulation, physics | M3 |
| L3 | Camera and space. Viewpoint, motion grammars | M3 |
| L4 | Render and material. Raster, raymarch, splatting | M1 for sprites, M3 for segments and for raymarch — a fullscreen L4 with `eye` and `ray`. Splatting is unbuilt |
| L5 | Composite. Set mixing, transitions, post, output routing | M2 |

Orthogonal to the layers:

- **Control plane** — node agents, director, mix agent, generation worker (M6). **MCP
  arrived early and took a bite out of this**: an external model driving the system through
  the same records a key press writes is M6's stated demand, arriving from outside the
  process. What it does not cover is the autonomous half — an agent that prepares material
  unasked has no chat window to be asked in
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
L1    : ()                  -> Geometry
L2    : Geometry            -> Geometry   // and two variants that break this, below
L3    : ()                  -> Camera
L4    : (Geometry, Camera)  -> Texture
Field : vec3                -> float      // a file and no node: spliced into its callers
```

`Geometry` is not monolithic. It declares topology, an attribute set, a count mode, and a
spatial domain. Compatibility is a subset check on attributes, with automatic adapters
where a derivation rule exists — `age` and `velocity` are the two that exist. This
declaration-plus-adapter layer is what lets almost any L2 in the library sit on almost any
L1.

**Two declarations break L2's endomorphism, on its two different axes**, and that is why the
line above no longer states it flatly. `amplify N` breaks it on the *count* axis — one
element in, `N` out — and `pairs` breaks it on the *arity* axis, taking two geometries and
returning one. Both stay in the same slot position, because what a `deform` writes is decided
before it runs. Stackability survives for the plain kind, which is the kind a library is
mostly made of.

`Field` is the mirror of L5's argument: a `kind` says what a procedure *lowers to*, an L5 has
no `kind` because the compositing is fixed and there is nothing to lower, and a field has
*only* code to lower — so it has a file and no node. AOVs were once written into L4's return
and are not built; nothing in the workspace produces one.

### Multiplicity within a layer

Each layer composes differently, and each needs its own semantics.

- **L1 multiple** — merge. Several geometry sources in one Set, which raised the question
  this document carried from before there was a compiler: **how two sources avoid colliding
  identities.** Identity is `seed`, a monotone spawn ordinal from one counter that resets at
  Set start, so two sources either share it and interleave — and the second one's `seed` no
  longer starts at zero, which breaks every structured layout — or hold one each and collide.
  **Answered, and half built**: `docs/ir-spec.md`, "Multiple L1 sources". Each source counts
  from zero, and the hash salt moved from per *layer* to per *source*, so two identical grids
  differ in colour by default. `--set a.kir,b.kir,renderer.kir` builds two sources in one
  Set, each at its own capacity, each with its own chain.

  **What is not built is the identity.** There is no `source` attribute — nothing carries
  one, so nothing downstream can mask on one. And the salt is **derived** from the source's
  ordinal where the design says it must be **assigned and recorded**, because every
  derivation fails on a case this system has: graph edits move a position, a `.kir` edit
  moves a content hash under `--watch`, and a declared procedure name collides for the same
  lattice used twice. Today reordering `--set` changes which geometry gets which randomness.
  Both wait on the same thing — a source has no name. See "Naming what a Set holds".
- **L2 multiple** — chain. Order matters. Each modulator carries a weight and a mask, and
  masks are attribute-based: only elements where `seed % 3 == 0`, only inside a region,
  only where `age > 0.7`. This is the largest single source of expressive range in the
  system, because the same modulator becomes a different thing under a different mask.
  **Built.** `weight` is a declared `param` and `mask` is a block writing a `strength` in
  `[0, 1]`; the two multiply into one `mix` between what reached the node and what the
  `deform` wrote. Keeping them separate is what keeps the operator's control a `param` —
  faders, bindings, transitions and Set files all reach a `param` and none of them reaches
  an expression. The mask is a **scalar** and not a predicate, which is the mixer's mask one
  layer down: a predicate is expressible as one and a soft boundary is not expressible as a
  predicate.
- **L3 multiple** — weighted blend or cut. Blending interpolates trajectories, so an orbit
  and a handheld rig can be mixed at 0.3.
- **L4 multiple** — overdraw on shared geometry. The same point cloud drawn as points, and
  as segments, and as an SDF raymarch. Geometry is shared, so the marginal cost is one draw
  pass. This is the cheapest visual variety per unit of GPU time in the whole system, and it
  is the payoff for being primitive-centric.

  **Built.** A Set holds a list of renderers over one geometry — `--set L1.kir,L4.kir,L4.kir`
  — the first clears the target and the rest load it, and the marginal cost is one draw pass
  as promised. `drift_shell + soft_points + drift_streaks` is one cloud drawn as sprites and
  as segments, from one simulation. What is *not* shared is geometry across **slots**: the
  same L1 in two slots is still two simulations, because a Set owns its element buffers. That
  is a different want — two independently faded copies of one cloud — and nothing has needed
  it yet.

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
per-slot transport; Set files, saved and loaded; a session writer and a replay driver; L5
blend modes; slot preview; MIDI in; transitions; masks; window output as a preview. Each bullet below says what it cost
against what it promised — the ones marked **Done** are worth reading for where the promise
was wrong, not only for the fact that it is kept.

**Ableton Link is built and verified**, and the verification changed what it is for.

`--tempo-source` works — a helper joins a Link session and Karakuri follows the shared beat,
checked end to end through a real peer, a proposed tempo change, and into a recorded
session. What does not work is the pairing. rekordbox joins the session and its decks can be
made to *follow* Link, but it never publishes the deck's BPM: Pioneer's own answer is that
Link has no master mechanism, so a tempo fader cannot be the thing that drives a session
tempo. The Link tempo is entered by hand in a subscreen, or from a MIDI-mapped knob, and
nothing connects it to the track that is playing.

So with rekordbox, Link does not make the picture follow the music by itself. It was chosen
on the expectation that it would, and it does not.

**That is a narrower failure than it first looked, and the first draft of this paragraph
got it wrong.** A set is not a rehearsed timeline played back; matching the timing on the
night is the craft, and an operator is riding controls all evening anyway. "It has to be set
by hand" is the same answer this milestone gave to semi-automatic gain, the tempo octave and
the panic key — and calling it a failure here contradicted the position taken everywhere
else.

What Link gives even when it is set by hand is that the setting is **shared**. `b` taps a
grid on this machine; a tap in the Link subscreen puts every peer on it at once, which is
worth having the moment there is more than one machine or more than one application in the
rig. The downbeat is only as good as the hand that set it — and it is that good on every
peer simultaneously.

Where it does deliver what was hoped for is a peer that *drives* Link, such as Ableton
Live.

What that does *not* invalidate is the seam. `--tempo-source` is an interface for tempo
sources in general and was deliberately not Link's, so the next candidate — a MIDI clock
from the controller, which does follow the deck — arrives as another implementation rather
than another design. Whether a MIDI clock can carry a *downbeat* is the open question there,
and the honest answer today is probably not: it carries tempo and beat phase, which fixes
the octave problem and leaves the bar where it was.

**MCP landed inside this milestone rather than M5 or M6**, and it is worth saying why it
was cheap. `--mcp` serves three tools — read a procedure, write one, ask what the swap
machinery did — and every hard part was already built for something else. The hot-swap
watchdog was written so a *human* editing a file could not wreck a set; it now catches a
model doing the same, unchanged. The record invariant was written so a session would replay;
it now means an AI-driven set replays with no AI attached. The IR was designed so a model
could write it from the specification alone, which was tested in M1 with three models and is
the reason "rewrite this procedure" is a plausible instruction at all.

What it needed of its own was the loop: `write_procedure` hands back the checker's
diagnostics rather than a bare failure, and `swap_outcome` says whether the result reached
the screen. A model that cannot see the compiler's answer is guessing; one that can is
iterating.

There is deliberately **no undo tool**, and the argument for that needed correcting once it
was reviewed. When a client reads before it writes, the previous version is in the
conversation and undo is context rather than API — a property a prompt box could not have
had. But nothing *makes* a client read first, and a write replaces the operator's file with
no backup, so the claim describes a well-behaved client rather than the interface.

**The one that mattered most was the replay claim, and it was simply false** — and it is
now true, because it was worth closing rather than qualifying. A procedure rewrite was a
file and not a record, so an AI-driven set replayed with the procedures it started with. The
hole predated MCP by as long as `--watch` has existed, but until this feature nothing had
claimed otherwise, and the claim was the whole reason the feature looked safe.

`Record::Procedure` closes it, and the shape is worth recording because it needed something
of the engine. The record is one line naming a slot, a layer and a content address; the
source goes in the store where a Set file's already does. What was missing was a way to say
*which build landed*: `label` is for a human and repeats on every rebuild of the same pair,
and the queue collapses superseded builds, so counting does not work either. So a build
request carries an **id** now and every event about it echoes it back — which is a thing an
asynchronous build queue should have had anyway, and which `Request`'s own documentation was
already reaching for when it said a request must not depend on what happens to be live.

One infidelity is named rather than hidden: a rollback restores the outgoing Set at the `t`
it was parked at, and a replay meeting these records builds afresh, so `t` restarts there. A
swap *in* is defined to start cold and so replays exactly. A rollback means the candidate
was over budget, which is an exceptional frame already.

**It connects to a real client.** `claude mcp add --transport http karakuri
http://127.0.0.1:8737/` and Claude Code drives it — read a procedure, rewrite it, ask what
happened. That was genuinely uncertain until it was tried: what is implemented here is the
minimum of the Streamable HTTP transport, POST with a JSON reply and 405 on anything else,
and no real client had ever spoken to it. The caveat worth keeping is that the same model
wrote both ends, so shared assumptions may be carrying more of the weight than the
specification is.

**What the first real session revealed is worth more than the feature.** Asked for line
art, the model could not find an example to copy — the deck had one slot — and fell back on
the compiler as its only source of truth, deliberately: *"compilation failing means the file
is not written, so probing is safe."* The check-before-write ordering exists so a model can
see the checker's answer; it turns out to double as a **free, safe probe**, and the model
found that on its own.

Then the diagnostics taught it the language. `hint: v0.2 defines 'points' only` closed off
line primitives in one try. And this one changed its whole approach rather than its syntax:

> `hint: loop bounds cannot reference a param, an ambient, or any other expression — cost
> estimation needs them fixed at parse time`

A hint that only corrected the syntax would have produced a working `for` loop and a
procedure that could not be afforded. Because the hint carried the *why*, the model
concluded that integrating a streamline per element was incompatible with the cost model at
all, and switched to an analytic curve with curl displacement — same per-element cost,
continuous strands. **Hints were written to be kind to a human and turned out to be the
specification an LLM reads.** Seventy-two of them exist; that number should grow, and every
one should say why rather than what.

The result also moderates this milestone's own complaint about a single topology. Denied a
line primitive, the model decomposed `seed` into a strand id and a position along the
strand, laid points densely along an analytic curve, moved hue and width from the element to
the strand — *"per-element hue would give each sample along a line its own colour and the
line would read as noise"* — and dropped exposure by a factor of four because additive
samples along a strand overlap. It got line art out of a point cloud. So the limit is real
and it is softer than "everything looks alike": what it costs is a halved element budget and
a technique nobody would find without being pushed into it.

One thing it wanted and did not have: **an example to read.** It looked for another slot's
procedure first. A library of procedures exposed as MCP resources would have answered in one
call what four failed compiles answered slowly — see M4, where a slice of that library is
now scheduled ahead of the search it was originally about.

The review also found the surface was a network service written like a local one: an
attacker-supplied `Content-Length` was allocated before it was believed, and a fifty-six byte
request aborted the render process. Loopback was treated as a boundary and is not one — a
page on any site can POST to `127.0.0.1`, and a write needs no readable reply to have
happened. Both are fixed, and the second is a reminder that **"it is only on loopback" is a
sentence to distrust**.

**M2 is closed.** What it delivered, in one sentence each: a deck with four slots, an L5
mix with blend modes and masks, transitions on the beat clock, a per-slot meter, tone
mapping once after the mix, priming with a budget governor, audio in with beat tracking,
per-slot transport, Set files and session recording and replay, MIDI in, the window as a
preview with the canvas as a recorded property of the run, a tempo source, and MCP.

**What it taught, beyond the bullets above**, is that the two invariants stated at the start
paid for themselves in ways nobody planned. "Everything an operator moves goes through a
record" is why a session driven by a model can be replayed without one. "No allocation on
the frame path" is why the frame loop could be handed to a `Sink` and a test double at all.
Both were also *wrong* in places nobody had checked until something new leaned on them —
which is the argument for stating an invariant as a claim rather than keeping it as a habit,
made four times over.

**What M2 did not touch, and should have worried about sooner**, is what any of this draws.
See the next milestone. Output routing is **out of this
milestone** — the window is a preview and an OBS capture of it covers the ordinary case, so
Syphon changes where the pixels go and not what the system does.

Link is the opposite, and the reason is sharper than "a better tempo source", which is how
it was described here for a long time. Link carries a shared beat **number**. The beat
tracker can find how fast beats go and where they are; it cannot find *which one is beat
one*, because downbeat detection is a separate and harder problem that this project does not
attempt. So `bar` in a binding, "the next bar" on `n`, and an 8-beat fade are today on the
right grid at an offset decided by whenever the session happened to start — musical in
length, arbitrary in alignment by up to three beats. That is not a precision improvement
waiting to happen; it is information the system does not have and cannot get from audio.

There is also a licensing fact that decides the shape: Ableton Link is **GPL-2.0-or-later**
and this workspace is MIT. That does not forbid the combination, but it does mean a binary
linking it is distributed under the GPL. A separate process is the clean boundary, and it is
the same boundary `docs/plugins.md` already requires for stability reasons — the constraint
that the interface may pass only what survives a process boundary turns out to have two
independent justifications.

The seam's in-repo half — one frame loop, and a `Sink` trait with the window and the PNG
writer behind it — is built. What is not built is the interface a plugin crosses, deliberately, because an interface designed before its first implementation is a guess. Both bring a non-Rust toolchain into a workspace
that is otherwise cleanly closed — Syphon wants Objective-C, Link wants cmake and a C++
compiler — and both are outside the deterministic path, which is what makes them safe to
put behind an interface rather than into the build. See `docs/plugins.md`. The panic key is
**decided against** rather than pending — see its bullet.

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
- ~~L5 mixer.~~ **Done for gain, opacity, blend and masks; a mask read from a texture is
  not built.** This bullet said
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
  no reordering control.

  **Masks landed too, and they cost less than the bullet implied.** A mask multiplies a
  layer's opacity per texel — everything the fader does, done to part of the frame — so it
  needed no new place in the composite and no second notion of what a layer contributes.
  Two shapes: a straight front at an angle, and an iris. Both ends of the front are exact,
  0 revealing nothing anywhere and 1 revealing everything everywhere, which is what lets a
  layer masked to nothing be *skipped* — a third escape from broken material beside
  residency and the fader — and what makes a wipe actually finish.

  **A wipe turned out to be a mask and one scheduled move**, and neither half knows about
  the other: the transition carries a number and the mask reads one. `docs/ir-spec.md` says
  a crossfade is two `transition`s and there is no `crossfade` record; the same is true here
  and for the same reason. What is not built is a mask read from a **texture** — an
  arbitrary shape, or another layer's luminance — because that needs somewhere for the shape
  to come from, and the answer is M3's `Field` rather than a third kind
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
- ~~`VideoSource` interface with `color` required~~ — **built**, and it is the seam the probe
  measures through. AOVs are not built and nothing produces one
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
- ~~**Parameter values keyed by `(layer, name)`.**~~ **Done**, and by node rather than by
  layer, since a Set holds several renderers. The engine held one flat map, so an L1 and an
  L4 declaring one name shared a value; `Set::build` refused the collision, which was
  correct and was not the answer — a Set whose two procedures both want a `hue` is not an
  error. The *external* half closed in M3: `--param L4:1:exposure=2.0` and `--bind index=1`
  address one node, and both records carry an optional `index`. A bare name still reaches
  every node declaring it, which is the useful default rather than the remaining gap
- ~~PLL correction of the local oscillator against external tempo~~ **Done**, and shaped by
  what it is for: tempo is stable except at a track change, so the grid is **predicted, not
  chased**. A locked grid free-runs and takes a slow trim; a single disagreeing estimate
  moves it not at all; re-acquisition needs eight consecutive consistent revisions. The
  correction **leads** by the analysis lag plus the output lag, because a loop that merely
  tracks shows its beat late by both, consistently — which reads as wrong rather than as
  noise
- ~~Ableton Link as a passive peer that never proposes a tempo~~ — **built and verified**,
  out of process behind `--tempo-source`. See `docs/plugins.md`
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
- ~~Output routing: Syphon / Spout / NDI.~~ **Moved outside the repository, and the line
  is drawn in `docs/plugins.md`.**

  The window is the default output and it landed as a **preview**: `--canvas` is what a run
  renders at and `--size` is only how big the window is, with the canvas fitted into it and
  the leftover black. Splitting them paid three times. The canvas became a **record**, so a
  replay renders at the size the performance ran at — before, a session played in a small
  window and replayed with a large `--size` produced different pixels and nothing in the
  stream said which was the performance. A window resize stopped reallocating every slot's
  target, which was the **last GPU allocation on the frame path** and it fired once per
  frame of a drag. And `present_mode` became a choice rather than `caps.present_modes[0]`,
  which had made frame pacing a property of whatever order the driver listed.

  A fourth thing came out of looking at that path at all, and it belongs with the beat-tap
  hole rather than here: the frame loop recorded a `tick` **before** discovering the
  swapchain had no texture, so an abandoned frame told the stream it had simulated steps the
  deck never took. `Outdated` arrives on every resize.

  What is *not* built is fullscreen and display selection, and that is deliberate rather
  than deferred: the window is a preview, an OBS capture of it covers the ordinary case, and
  anything past that is the plugin seam's job rather than a bigger window's
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

**Closed.** Every `Ln` in the algebra is a node with a `kind` behind it, and a Set holds a
chain of them: one or more L1 geometries, a list of L2 deformations, an optional L3 camera,
a list of L4 renderers, an optional L5 folding those, and a `kind Field` any of them may
call. `--set` reaches all of it with no new syntax — files are sorted by the `kind` each
declares. Parameters, bindings, the edit history and the MCP surface address a *node* rather
than a layer, and a Set can say which of them a console sees.

Every item on the Adds list below is struck. Four things that were inside those items are
**not** built, and each is struck through to where it went rather than left implied:

- The **graph compiler**, split in two. Its authoring half opens M4 as "Naming what a Set
  holds"; its fusion half is in "Deferred by decision" with a trigger.
- A source's **salt is derived rather than assigned**, so `--set` order decides which
  geometry gets which randomness — M4, same item.
- **How a mask names a source**, and the `source` attribute it would read — M4, same item.
- **What an L3 points at**: a reduction, or element zero. Decided, refused by name in the
  checker, and waiting on the same naming — M4, same item.

That is one piece of work under four headings, which is why it is scheduled once.

**What it cost**, on M1's terms, since that section promised this accounting would be worth
more than the estimates: 64 commits, 21000 lines, 9 test files, 9 examples, 844 tests. The
implementation was again not the expensive half.

**Six `.kir` files that checked clean took the process down**, one of them found on the last
day of the milestone by an audit rather than by anything running. Every one is the same
shape — a feature widened what the checker accepts, and some position the widened thing can
now reach had nothing to emit — and every one was a *different* position: a fullscreen L4
has no element, a `spawn` block has no derived attribute yet, an L3 has no field, a field
cannot evaluate itself, an amplified chain outgrows the device, and a field has no `seed`.
Being able to say that shape out loud did not stop the sixth.

**Three review passes returned about fifteen real defects**, and a documentation audit at
the close returned five more — including that sixth process death, a second geometry running
at the first one's capacity, and a Set file discarding the capacity it had just recorded. A
claim in prose is a test nobody runs, and roughly a third of the time the prose was the half
that was right.

**The recurring defect has a name now: one fact derived in two places.** Slot arithmetic
that a constant `1` was right about until a Set could hold two L1s. A layer list built twice.
`l1_count` against `sources.len()`. A comment stating what a loop does beside a loop that
does something else, since the day the loop was written. None of these is a hard problem;
all of them are invisible until the second place moves.

**And the tests learned three things about their own material.** A fixture below the
workgroup size cannot see a wrong dispatch count — 64 elements is the floor. A freshly
allocated buffer is zeroed, so "never written" and "written correctly" are the same picture.
And a measurement has to be able to see the defect: a band count cannot see overlapping
elements where additive light can, and a light total cannot see a position where a centroid
can.

---

**A validation error is now a diagnostic rather than a dead process**, and that is worth
recording here because of how it was learned. wgpu answers a validation error by reporting it
to an uncaptured handler that *panics the thread that made the call*: on the swap worker that
is a `SetError::Panicked` and the running Set survives, and at startup there is no
`catch_unwind` above it and the process exits. `Set::build_many` now runs inside a validation
error scope, and per the WebGPU rules an error a scope captures is not reported to the
uncaptured handler — so it arrives as a value.

**Five process deaths in one milestone is what prompted it**, and they were not all the same
shape: `seed` in a fullscreen L4, a derived attribute read in `spawn`, an L3 evaluating a
field the Set has none of, a field evaluating itself, and an amplified chain past the device's
buffer limit. Four are generated WGSL that naga refuses; **the fifth is a resource limit and
not a shader at all**, which is why the scope is the right net and validating with naga before
the call would only have caught four of five. Each was injected with its own refusal disabled
and each came back as a message.

**The sixth arrived after this paragraph was written**, which is the paragraph earning its
keep: `seed` in a `field` block, found by an audit on the last day of the milestone. It is
refused now, and it would have been a message rather than a crash if it had not been.

**Rust's memory safety does not reach any of this**, and it is worth saying why: the failure
is in a *different language*, generated at run time, whose type system runs after this
compiler has already shipped its answer. That is the whole reason the check pass carries the
burden it does, and the reason every widening of what checks clean opens a hole — six of
them, here, in one milestone.

**The net is a net and not a plan.** Everything it catches is something the check pass should
have refused with a sentence about the `.kir`, and `SetError::Invalid` says exactly that: an
author who sees it has found a compiler bug rather than made a mistake. What it buys is that
finding the sixth hole costs a message instead of a crash.

What it does **not** reach: device loss, a GPU hang, a driver crash. Those are the cases a
separate process would rescue, and none of the six was one — a compile is not where a device
dies. Rescuing those means the *render* crossing a process boundary, which is a frame-path
cost, or `Device::set_device_lost_callback` and rebuilding, which the deck already has the
material for since it can rebuild a Set from records.

**How it went, item by item.** Cross-source interpolation was the last to land,
as a pairing L2 — `pairs` on the header, `other.<attr>` for the paired element, and a Set
that holds exactly two sources when the chain begins with one. Multiple L1 sources landed
before it, and the nested L5's motivating case landed with those — "two pipelines, one knob"
was waiting on nothing but a Set able to hold two geometries, and it now renders and is
tested: two sources, two renderers, two composited targets, and one published control that
moves all four instances.

**Slot interface contracts landed as the attribute derivation** and **`Field` landed as a
kind**; both are struck below.

**A correction to a sentence this replaces.** An earlier revision called multiple L1 sources
*"decided in full"*, and they are not: what shipped is the merging, not the identity. Another
said every Adds item was built, written from the list of things that had just landed rather
than from the list this section names — which is the kind of claim a milestone summary exists
to prevent rather than to make.

**L2 amplification is built**, and so is the rendering front this milestone owed —
`topology lines`, a fullscreen L4 that marches, and `blend weighted`.

**L3 is built, edge first.** `L4 : (Geometry, Camera) -> Texture` makes a camera an input
*edge*, and it is one: the six numbers a camera *is* go into a GPU buffer, a compute pass
derives the `view_proj`, ray basis and `depth_range` a renderer reads, and every L4 binds the
result. Nothing about a camera is *derived* on the host any more — the six numbers still
live there and are uploaded, and everything computed from them moved onto the GPU. The producer is either the built-in
orbit, host-written, or a `.kir` with a `camera` block, which is a second compute pass writing
the same buffer.

**Building the edge before the producer was the load-bearing decision.** The tempting
increment is a `kind L3` the host evaluates, leaving the L4 uniform alone; it works for every
camera expressible today and has to be undone by the first one that follows an element, since
that reads a buffer and reading it back is the one thing this architecture is built to avoid.
Deciding *where the camera lives* is what the geometry-reading case forces, and it forces it
whether or not anything reads geometry yet.

What is left on this front is what an L3 can point at — a reduction, or element zero — which
is addressing the language does not have and which the checker refuses by name; **scheduled,
at the head of M4, as one item with the other three things that want a node to have a name.**
And state, which the layer is allowed to hold and which is what damping a follow needs —
nothing has needed it, so nothing has built it. `examples/beat_jump.kir` is the half that needs neither:
a cut chosen by hashing the beat number is a pure function of the clock, so it is seekable.

**L5 is a node kind, and that is two words this project had been using as one.** A `.kir`'s
`kind` says what a procedure lowers to; an L5 has no code to lower, since the compositing is
fixed. So there is no `kind L5` file and there should not be — `crate::node::Merge` is the
node, `crate::mix` is the shader and the per-input controls both roles share, and `--merge
<slot>` is how a Set asks for one.

**The motivating case has landed**, and it needed no change to the L5 at all. "Two pipelines
merged by a nested L5, published as one control" was waiting on nothing but a Set able to
hold two geometries: a merge composites *renderers*, and every source draws into the target
its renderer owns, so "first onto this attachment clears it" carried the whole of it. Two
sources, two renderers, two targets, four instances and one knob.

**What a Set publishes** landed alongside it, and is struck in the list below: an interface
names which controls a console sees, under what name, over what part of their declared
range.

**First, and before any of the list below: more than one way to draw.**

`Topology` had one value and `Blend` had one value. Every frame this project had ever
rendered was an additive point sprite, and every example looked like a relative of every
other for that reason and no other. No amount of L2 amplification, cross-source
interpolation or graph compilation changes what a pixel can be while that is true — those
multiply the ways geometry can be *arranged*, over a single way it can be *seen*.

**`Topology::Lines` is built**, and it is the cheapest thing in this milestone by a wide
margin. Three things about how it landed are worth carrying.

**It cost the engine nothing.** A line primitive in WebGPU is one pixel wide, exactly as a
point primitive is, so `lines` is the *same quad* the sprite path already expands: six
vertices per instance, a `TriangleList` pipeline, the same indirect draw arguments,
`VERTICES_PER_ELEMENT` unchanged. Only where the six corners land differs. Nothing in
`karakuri-engine` was touched to add a rendering mode — which is the bet the enum was
declared with one variant to make, paying off exactly as stated.

**The connectivity question answered itself in the opposite direction from the plan.** This
bullet used to read "drawn between elements a procedure nominates", and nomination is
precisely what compaction forbids: an element naming another is an index into the element
buffer, and the scan moves elements between steps, so the one shape that expresses a shared
vertex is the one that spawning material invalidates. **A segment whose two ends both belong
to one element has no such dependency.** A polyline is *n* segments, a trail is one segment
per particle, and both survive spawning and killing because neither refers to anything
outside itself. It costs the duplication of shared endpoints, which is the price of the
primitive being smaller than the gesture — the same answer transitions gave in M2, arrived
at from a different direction.

**It closed the "give an L4 a way to say what it renders" question below, without adding a
declaration.** An L4 draws segments by assigning `clip_b`, a second clip-space endpoint, and
sprites by not assigning it; the check pass infers the topology from that and records it in
`Checked::topology`, which was previously an L1-only field. A `topology` line on an L4 header
would have been a second place for one fact to be stated and a first place for a file to
contradict its own `vertex` block. **`blend` is not the same shape** and should not be given
the same treatment: nothing an L4 writes could imply `weighted` over `additive`, because the
two differ in how identical assignments are combined.

**And a check that looked principled turned out to be wrong.** With a topology on both sides
it is natural to refuse a pair that disagrees, and `Set::build` briefly did. It would have
forbidden the one thing this milestone exists for: the same cloud drawn as sprites by one L4
and as strokes by another. A segment gets both ends from attributes the L4 already
`consumes`, so a renderer needs *nothing* from the geometry that the composition check does
not already cover, and requiring agreement invents a dependency the lowering does not have.
The declaration on the L1 side says what the geometry is meant to read as; it constrains no
renderer.

**Nothing remains.** The vocabulary no longer runs ahead of the engine: `sd_sphere`,
`sd_box`, `sd_torus`, `sd_plane` and the four CSG operators have a renderer now, and
`examples/field_march.kir` uses five of the eight. `Topology` has three values, `Blend` has
two, and neither enum is a promise any more.

- ~~**A fullscreen L4.**~~ **Built.** An L4 with no `vertex` block covers the frame and
  marches instead, and `examples/field_march.kir` is the first picture here made of no
  elements — a sphere, a box smoothly unioned into it, a torus through both, all from
  builtins that had passed the checker since M1 with nothing able to draw them.

  Four things it turned out to need, and the last two are the ones worth carrying.

  **The declaration is the absence of a `vertex` block**, exactly as `clip_b`'s presence is
  the declaration for lines. One place for the fact.

  **The ray is given, not derived.** `Orbit::basis` hands a fragment the eye and three
  pre-scaled vectors, so `ray` is an interpolation and a normalize. Handing a procedure an
  inverse camera instead would put the projection convention inside every shader that
  marches, and a convention restated is a convention that drifts.

  **`consumes` must be empty, and that is a rule rather than a consequence.** With no vertex
  block there is nowhere to read an element from — but *stating* it is what makes skipping
  the paired L1's entire simulation provable instead of plausible. The same fact left
  implicit would have been an optimisation resting on a reading of the language.

  **The fragment cost ceiling had to become two numbers**, and the reason generalises. 512
  ops was a stand-in for a quantity nobody knows: `capacity` sprites, times their area, times
  their overdraw. A fullscreen pass covers the canvas exactly once, so the number is known
  and the stand-in has nothing to stand in for — and applied anyway it forbids raymarching
  outright, because thirty-odd iterations of a distance function is what a marcher *is*.
  **A ceiling calibrated against one shape refuses the next one.** It was worth remembering
  before `blend weighted`, and the warning half landed: what bit there was a borrowed
  weight function's constant rather than this table.

  What it did *not* change: a Set always has an L1, so a fullscreen renderer is paired with
  geometry it ignores, and the operator has to choose it. Half-answered since, by the
  renderer stack — a fullscreen node beside a per-element one reads nothing and the geometry
  is not dead weight, because the other node reads it. A Set that can hold *no* geometry is
  still the graph model's.

- ~~**`blend weighted`.**~~ **Built.** The other half of the point-sprite monotony:
  additive is why everything glowed, and lines did not change that — a stroke glowed
  exactly as a sprite did. **Material in this project now occludes**, which nothing it had
  ever drawn did.

  Four things it turned out to need, and the last two are the ones worth carrying.

  **A second attachment, because a slot target has room for two of the three running
  quantities.** `sum(c * a * w)` and `sum(a * w)` fit in one `Rgba16Float`; `prod(1 - a)`
  composes by multiplication where those compose by addition, and a colour attachment has
  one blend state. So the revealage is its own `R16Float` target with `dst * (1 - src)` on
  it — `R16Float` rather than the published technique's `R8Unorm`, because 8 bits quantises
  coverage to 1/255 exactly where soft material is thinnest.

  **It cost L5 nothing.** The resolve writes premultiplied colour and coverage in alpha,
  which is precisely what the additive path writes, so `composite.wgsl` reads a weighted
  slot without knowing the mode exists. That was not luck — it is what picking the resolve's
  output convention to match the existing one buys, and it was worth checking before
  building anything, because the alternative was OIT knowledge in the mix.

  **The ceiling this section warned about did arrive, in a different shape.** The caution
  was that a number calibrated against one thing refuses the next; what actually bit was
  the published weight function's `3e3` factor, which assumes colours in `[0, 1]` and
  overflows `f16` immediately in a pipeline where colours of 20 are ordinary. The fix was to
  notice that **the resolve divides the weight out**, so its absolute scale means nothing
  and only the ratio survives — the constant could simply be dropped. A borrowed formula
  carries its author's assumptions in its constants, and the useful question is which of
  them the code around it still holds.

  **`Set::resize` takes a device now**, because a Set can own render targets. Twenty-odd
  call sites, all mechanical — and it is *joining* a convention rather than breaking one:
  `Present::resize`, `Deck::resize`, `Composite::rebind` and `Meters::rebind` all already
  take one, and `Set::resize` was the outlier only because it had nothing to allocate.

  What it does *not* do: sort. It is an approximation, and where it is coarse is stated in
  the spec rather than hidden — material occupying a thin slice of a wide frustum gets
  near-equal weights and the resolve approaches a plain alpha-weighted average. That
  degradation is graceful, because what still separates it from `additive` there is the
  occlusion rather than the ordering.

**How hard the old limit actually bit** was measured once, by accident, and the measurement
is now a fixed point to compare against: asked for line art, a model denied a line primitive
decomposed `seed` into a strand id and a position along a strand, laid points densely along
an analytic curve, and moved hue and width from the element to the strand. It works — it
reads as strokes. It cost roughly twice the elements and a technique nobody finds without
being forced into it. `examples/strand_shell.kir` is kept as written for that reason, beside
`examples/drift_streaks.kir`, which gets the same look out of one assignment.

**Next, and it is not on the list below because it is underneath three things that are: a
Set stops owning everything, and `Ln` becomes a node.**

Reading the Adds list after the rendering front closed, three of its nine items turn out to
be pushing on one fact rather than on three. **L2 and L3 as slots** needed a stage between
L1 and L4, and there was nowhere to put one because a Set *was* a pair. **L2 stacking and
amplification** need several such stages in a row. **Multiple L4 renderers over shared
geometry** needed the element buffers to outlive the renderer reading them — which is built,
and is what the tenses in this paragraph are now marking. This document
already says so in the L4-multiple bullet — *"it is the part that needs a Set to stop being
the unit that owns everything"* — and what is new is only that it is now true of most of the
milestone rather than of one bullet.

**The shape is nodes, and that came from Toyoshima rather than from this document.** My
first draft split a Set into a Geometry and a list of Renderers — a two-level hierarchy with
a star baked into it. The correction: *「ノード系ツールであるようにLnがノード、Setと読んでいる
のはノードグラフのグループ化みたいな緩い意味になるかもね」*. `Ln` is a node; a Set is a
grouping drawn around some nodes. That is not a different implementation of the same idea, it
is a different idea, and the Adds list already ends at it — **"node graph as authoring
representation, render graph as execution representation"**. A fixed one-geometry-many-
renderers type is a shape the graph compiler would have to demolish.

So: **the unit that owns GPU state is the node**, not the Set. An L1 node owns element
buffers and produces `Geometry`. An L2 node takes `Geometry` and produces `Geometry`. An L4
node takes `Geometry` and `Camera` and produces `Texture`. A Set is a list of nodes, the
edges between them, and one output; evaluation is dependency order, which for every shape
that exists today is list order with a check.

What this buys, stated as what stops being a special case. "Multiple L4 renderers over
shared geometry" is no longer a feature — it is two L4 nodes with an edge from the same L1,
and there is nothing to add for it beyond the edge existing. "L2 as a slot" is a node kind
and an insertion, not a third position in a fixed pipeline. And the render graph the graph
compiler is supposed to target exists in miniature from the start, rather than being what it
has to replace.

What it does not buy, and this is worth saying so nobody reads more into it: **today's graph
is a star with two node kinds in it.** There is no authoring representation, no fan-in, no
`Field` chain to fuse. Building a general graph runtime now would be one shape's worth of
machinery pretending to be a system. What is being built is the *ownership* — a node holds
its own buffers, pipelines and uniforms, and names its inputs — which is the part L2 and
multi-L4 both need and the part a graph compiler cannot be bolted onto later.

A Renderer node is not portable between Geometry nodes and is not meant to be: its bind
groups name specific buffers and its `Element` struct is compiled against a specific layout.
That edge is typed, in other words, and `Set::build`'s composition check is already the type
check for it — informally, and against a pair rather than against an edge.

**A fourth reason, from somewhere else, and the one an operator would name first.** A swap
transfers no state: `swap.rs` says so plainly, and it is correct for what it was written
for. The consequence nobody wrote down is that **editing an L4 restarts the simulation** —
change one number in a renderer under `--watch` and the cloud goes back to `t = 0`, because
the unit rebuilt was the pair. M2's priming answers "warm a Set before showing it"; it does
not answer "this node did not need rebuilding at all". Per node, the rule is one sentence:
**a swap costs what it invalidates.** Editing a renderer costs a pipeline. Editing geometry
costs the simulation, and every node downstream of it.

What falls out with no further work: `examples/drift_shell.kir` drawn as sprites *and* as
streaks *and* as a solid, for one simulation and three draw passes — the payoff the
primitive-centric bet was made for, which today costs three simulations.

**Built, and reachable from the command line.** `--set L1.kir,L4.kir,L4.kir` is one
geometry with two renderers, and there is genuinely no new syntax for it: one
comma-separated list, read as one L1 and however many L4s. A third path used to be refused
as a stray comma, which was right while a Set was a pair. `--demo lines` is now what it was
always demonstrating — it was two slots running `drift_shell` twice so a second L4 could
read it, and it is one slot with two renderers.

A Set file says a stack as **several `slot` records on `L4`**, in draw order, and needed no
new record to say it. A session stream says it as several `procedure` records carrying an
`index` — absent when 0, so a stream written before stacks existed replays byte for byte.

Both are closed: the edit history and the MCP surface are keyed by `(slot, layer, index)`,
which is the address the params grew at the same time. What is *not* addressable is a node
with no ordinal to be found by — a second geometry — which is the head of M4.

**Built: several renderers over one geometry.** `Set::build_many` takes the L4s in draw
order and they run in that order over the one attachment — the first clears, the rest load —
so `drift_shell` drawn as sprites *and* as streaks *and* as a solid is one simulation and
three draw passes, where it used to be three simulations. **Nothing about a renderer had to
change to be in a list**, which is the node split paying for itself: it was already a node
owning its pipeline, its uniform and its targets, reading the geometry across a typed edge.

Two rules moved to where their premises live. `SetError::WeightedFullscreen` said `blend
weighted` on a fullscreen procedure is the identity — true only while that procedure is the
*only* one drawing, since a later one composites `over` what is under it rather than
replacing a clear. It was refused inside `Renderer::build`, which cannot know the count; it is
now refused in `Set::build_many`, which can, and only for a lone renderer. And the fullscreen
simulation skip became a question about *every* renderer rather than the one: a node that
reads no attribute does not excuse the simulation if another reads them all.

**Built: both nodes own their state.** `crate::node` holds an L1 node and an L4 node —
element buffers, counts, compaction, the spawn accumulator, pipelines and bind groups on one
side; pipeline, uniform, accumulation targets and its own bind groups on the other — with
`Geometry` the edge between them, resolved once at build time from the node that offers it
rather than assembled by the Set out of buffers it reached into. `set.rs` halved on the day —
1557 lines to 927 — and what was left was the grouping: camera, parameter values and
bindings, viewport, clock, and which nodes run in what order. It is larger than either
number now, and what grew is the grouping's own business: lists, chains, sources and the
refusals that go with them. **One node of each kind still**, on the day, so nothing an author
could see had changed.

**Three places still reach into a node, and naming them is naming what the list changes.**
`Set::draw` reads the simulation's parity and counts buffer; `Set::step` asks the renderer
whether it is fullscreen before running a simulation nothing would read; `Set::bind` asks
both which params they declare. Only the first two are shape: with several renderers the
fullscreen skip becomes a question about *every* renderer, and parity-and-counts stops
wanting to be fetched once per reader. Which is the finding worth carrying: **`Geometry` is
only the build-time half of the edge from L1.** The per-frame half — which parity holds what
was last written, and where the instance count lives — is routed around it as two arguments,
so a `Renderer` cannot draw from a `Geometry` alone. Fine for one reader; the point at which
it wants a type is the same commit that makes the list.

**The clock deliberately did not move.** `t` is the grouping's — one clock serves every node
— so the simulation node is *handed* the instants its substeps land on rather than deriving
them from a step counter of its own. It was the one piece of the old `Set` that was tempting
to move along with the code that reads it, and moving it would have made two nodes in one Set
able to disagree about what this frame was.

**Two holes in the tests, both the same shape, and the shape is the finding.** Neither was a
defect in the code — both were things nothing was watching, which is the same thing one edit
later. They were found by injecting defects into the seams the split moved and noting which
injections *passed*.

This repository tests almost everything by **comparing one run against another**: same record
stream, same image; primed then live, same as always live; two frames of one step, same as
one frame of two. That is the right shape for an invariance claim, and it is structurally
blind to anything that moves *both* sides equally. Two such defects passed every suite then
in the repository:

- **Shift every substep's `t` by one whole `dt`** — every procedure in the system reads a
  clock a frame out, and every comparison shifts with it. Closed by
  `beats.rs::the_instants_a_substep_reads_are_the_sets_own_clock`: absolute arithmetic instead
  of a comparison, pinning the last substep's instant against `Set::time` and the sum over
  every substep against `dt * (1 + 2 + … + n)`, so where the sequence *starts* is asserted as
  well as where it ends.
- **Drop the spawn accumulator's fractional carry** — the mechanism a nineteen-line comment
  and the ir-spec's "Spawn timing" both rest on. Existing rates that do not divide evenly by
  60 did not help, because both sides of every comparison truncated identically. Closed by
  `lifecycle.rs::the_spawn_rate_is_exact_over_many_substeps_because_the_fraction_carries`,
  which asserts a *rate* — and does it at half an element per substep, where losing the carry
  is not an inaccuracy but a silent total failure: `floor(0.5)` is zero forever, so no rate
  under 60/s would ever spawn anything.

The rule this leaves: **a comparison test needs an anchor beside it.** Invariance says the
system agrees with itself; only arithmetic says it agrees with what it was asked for.

Three forks, **all three decided as recommended** (*「全部推奨通りで問題ないと思う」*). All
three read differently under the node framing than they did under the hierarchy one, which is
the reframe earning its keep.

**How several `Texture` nodes reach the Set's one output.** **Their passes run in order over
the one attachment — the first clears and the rest load.** Then order
among renderer nodes is the same kind of order slot order is in the mix, and each blend mode
already knows how to meet what is under it: `additive`'s blend state accumulates into
whatever is there, and a weighted node's resolve composites `over` instead of replacing,
which is the identical result on the cleared target it writes today. The alternative — a
texture per node and an implicit combining node — is L5 rebuilt inside a Set, at a render
target apiece. It was also written as the answer that becomes wrong first — *"a real graph has nodes
that consume textures, and then the combine is a node"* — and that turned out to be true the
same day rather than later. It is **the rule for one of two shapes**: several L4s straight to
the output is overdraw and costs one target, several L4s into an L5 is compositing and costs
a target apiece. See `docs/ir-spec.md`, "Overdraw and compositing are different operations".

**What a Set file and a command line look like.** **No new syntax at all.**
`--set drift_shell.kir,soft_points.kir,drift_streaks.kir` already parses; every file declares
its own `kind`, so the loader reads list order as order within a kind. The edges are
*inferred* while the graph is a star — every L4 reads the only L1 — and the moment that stops
being true they have to be spelled. Inferring them was not a shortcut being taken; it is that
there was exactly one edge set consistent with the nodes, so writing them down would have been
a second place for the same fact.

**That moment has passed.** A Set can hold two L1s, and a pairing L2 reads two of them, so
there is no longer exactly one consistent edge set — which two geometries a `pairs` node takes
is decided by `--set` order and written nowhere. This paragraph promised that "when fan-in
arrives it brings the notation with it"; fan-in arrived twice and brought none, and the debt
is now the first item of M4. See "Naming what a Set holds".

**How a param is addressed.** Under the hierarchy framing this was "layer and position";
under nodes it is simply **the node**, which is the same key with a name that will still be
right after the graph exists. Superseded in part by what a Set publishes, below: the node and
the name is the *internal* address, and the console sees whatever the Set called it. It is
forced rather than chosen, and it paid a debt recorded in `SetError::ParamCollision`:
`Set::params` was keyed by name alone across the whole Set, so two procedures declaring
`exposure` were refused — and every L4 in `examples/` declares `exposure`, so the check
forbade the feature it was guarding.

**Built, both halves.** The engine keys parameter values by node and the error is deleted;
`--param L4:1:exposure=2.0`, `--bind index=1`, and an `index` on the `param` and `bind`
records reach one node from outside. A bare name still means **every node that declares
it** — one knob moving both renderers, the useful default and the "one control driving both"
case an interface would offer — so the address is what closes the gap a wildcard cannot, and
neither replaces the other.

**The `index` is an `Option` on these two records and a plain number on `slot` and
`procedure`, and that is not an inconsistency.** Those name exactly one node and always did,
so absent is 0. These address a *value*, where absent is a **wildcard** — which is both what
a bare name has always meant and what keeps every Set file ever written reading the same way:
`layer` on a `param` record was a placeholder the loader ignored, so honouring it now would
have silently retargeted them. The address is `(layer, index)` present or absent as a unit,
which makes `layer` load-bearing exactly when an `index` appears beside it.

**The same address closed two more surfaces**, which is why it was worth doing first. The
edit history was keyed by (slot, layer), so every renderer of a stack shared one chain and
one memory of what it last wrote — two renderers recorded one snapshot per save, alternating
between procedures neither of which had changed. And the MCP `read_procedure` /
`write_procedure` pair handed back renderer 0 for any `L4`, so a model told to rewrite the
strokes of a slot that draws sprites *and* strokes would have rewritten the sprites. Both
now take an `index`, and an index past the end is refused with the range named rather than
folded to the first.

What this is *not*: L2. The split makes the space a stage goes into; putting one there is
the next thing after, and it should be built against a Set that already holds a list rather
than against one being taught to.

**Adds**

- ~~**L2 as an IR `kind` with its own node.**~~ **Built.** `kind L2`, a `deform`
  block, a compute pass between the simulation and the renderers, and a `Set` that
  holds a chain of them. What it cost was small because the node split had already
  paid for it: `Geometry -> Geometry` is the edge that existed, and an L2 is a node
  that reads one and offers another.

  **Statelessness is structural rather than checked**, which is the part worth
  keeping. Every generated `deform` opens by overwriting its output from its input,
  so there is no previous value left to accumulate onto and a drifting modulator
  cannot be written. That is what keeps the layer freely stackable and keeps
  `closed_form` an L1 question however long a chain gets.

  What it cannot do is `kill()`, and the diagnostic now says why in the layer's own
  terms rather than pointing at an `element` block an L2 does not have: compaction
  runs once, after L1, so liveness is settled before a deformation sees anything.

  The composition check became a **walk** rather than a comparison. An L2 may `emit`
  an attribute no L1 in the library produces, and everything below it can consume
  that — so `consumes ⊆ available at this position`, and the error names the
  position.

  **Reachable from the command line, with nothing new to spell.** `--set
  L1.kir,L2.kir,L4.kir` works because every `.kir` declares its own `kind`: the first path
  is the geometry and the rest are sorted by what they say they are, list order being chain
  order for L2s and draw order for L4s. `examples/swirl_warp.kir` is the first one — a twist
  that goes over any geometry emitting `position`, which is the argument for the layer
  existing rather than folding the same maths into the L1 and needing a second `.kir` to
  have it without.
- ~~**L3 as an IR `kind` with its own node.**~~ **Built, except what it can point at**, which
  is decided and refused rather than implemented: the checker rejects reading geometry from a
  `camera` block by name, with "that addressing is specified in `docs/ir-spec.md` but not
  built" as the hint. It needs the same naming M4 opens with — pointing at something is
  addressing it. A camera belongs
  to an **L4**, not to a Set or a deck: `L4 : (Geometry, Camera) -> Texture` makes it an input
  edge, so two renderers on one camera is one viewpoint drawn twice and two renderers on two
  cameras is a Set that composites two scenes. And an L3 is a `.kir` procedure rather than the
  `camera` record it is today, because what an author wants from one is dynamic — *follow an
  element, jump on the beat while facing the centre* — and two numbers in a record carry
  neither. The first of those expectations also broke a claim this project had held for a day:
  an L3 that follows an element **reads geometry**, which is in a GPU buffer that cannot be
  read back on the frame path, so the `Camera` edge is a GPU buffer written either by the host
  or by a small compute pass. **What an L3 points at is a reduction — centroid or bounds — or
  element zero**, and the second is where order-preserving compaction paid for itself a third
  time: a live element at index 0 has nothing alive before it, so it stays there until it
  dies, which makes index 0 *the oldest living element* rather than an arbitrary slot. Nothing
  is declared and nothing is checked; an L1 that expects to be looked at can make that element
  a leader, and one that does not still offers its oldest survivor. **That much is the
  design.** What exists is the camera as an edge, a `camera` block, and a refusal where the
  targeting would go
- ~~**A Set declares what it publishes.**~~ **Built**, and it is the other half of the console
  being a real layer. Today every `param` of every procedure reaches the desk, flat and by name — nine
  controls for a pair, twenty-five for a graph, most of them authoring decisions the author
  already made. So a Set names which of its controls appear, under what name, over what part
  of their declared range; the rest keep the values they were left at. **Publishing decides
  what is shown, never what is reachable** — a `param` record still addresses any control in
  any node, because a surface is a choice about attention and not about authority, which is
  this project's standing position everywhere else it has come up. Driving a control from
  something other than a knob is not a third mechanism: `bind` already maps a source through a
  curve and a range, so its source becomes "a signal, or a published control" and macros fall
  out of it. `SetError::ParamCollision` is already gone — the engine keys values by node —
  so what an interface adds here is the *external* name: two renderers both declaring
  `exposure` become two published controls or one driving both, which is an authoring
  decision rather than an error
- ~~**L5 becomes a `kind`, with two roles and one implementation.**~~ **Built — as a *node*
  kind, which is the distinction this bullet was eliding.** A `.kir`'s `kind` says what a
  procedure lowers to and an L5 has no code to lower, so there is no `kind L5` file:
  `crate::node::Merge` is the node and `--merge <slot>` asks for one. The console an operator
  mixes on is the top-level one; the same node nested inside a Set folds several L4s into one
  texture. What differs is only whether a surface is wired to it — which finally separates
  the *mix* (gain, opacity, blend, mask: properties of an edge into an L5) from the *deck*
  (residency, priming, hot swap, budget, transport, preview, meters: properties of a Set being
  played, which have nothing to do with mixing and sit beside L5 only because that is where a
  performance happens)
- ~~**Multiple L1 sources**~~ — **built, except the identity**, which is M4's. Per-source
  `seed` counters starting at zero so structured layouts survive, and a per-source hash salt
  so randomness differs without structure differing. A Set holds a *list* of sources, each
  one a simulation and the chain over it, and `--set a.kir,b.kir,renderer.kir` fills it. The
  `source` attribute is not built and has no value to carry — see the salt paragraph below.

  **The chain is per source rather than the geometry being concatenated**, and two things
  force that. Two sources kill independently, so compaction is each source's own and there is
  no shared live range to concatenate into. And with a chain instance per source, **two
  sources need not agree on what they `emit`** — each instance is compiled against the layout
  of the source it runs over, which is a question with no answer at all if one buffer has to
  hold both.

  What falls out is that **`source` need not be an element slot**, which the spec assumed it
  would be. A chain instance knows statically which source it belongs to, so what varies with
  the source is a *uniform* — and an element slot is sixteen bytes on every element of every
  merged Set. That is the third time in this milestone a per-element cost the spec took for
  granted turned out to be avoidable, after `velocity`'s third buffer and the always-on
  derivation storage.

  **Addressing is by procedure and not by instance.** `--param L2:0:x` names the first L2
  *procedure*, and the Set writes it into every source's instance of it — the same
  relationship a spliced field's params already have with their callers. "First onto this
  attachment clears it" needed no second rule for several sources, since it was already about
  the attachment rather than about the list.

  **The salt is derived and not assigned**, which is the provisional half. The spec calls for
  a value chosen when a source is added and *recorded*, so it survives the list being
  reordered; derived from the ordinal, it gives the picture the spec asks for — two identical
  grids in different colours by default — and not the stability. Reordering `--set` changes
  which colour is which. Assigning it wants a Set file able to carry sources, and one carries
  an L1 and its renderers. **Outstanding, and scheduled** — head of M4.

  Still open for the same reason: **how a mask names a source.** The spec says a name resolved
  where the Set is built, and every worked example writes an ordinal — which is the spelling
  the same section rejects. Nothing reads `source` yet, so nothing is wrong today; what is
  missing is the half that makes two sources treatable *differently*. **Outstanding, and
  scheduled** — same item
- ~~**L2 amplification**~~ — **built.** `amplify <factor>` on an L2 header, `copy` readable
  as the index of the copy being made, and `examples/kaleidoscope.kir` for the picture. Three
  things it turned out to need that the paragraph this replaces did not mention, and all
  three are about buffers rather than about the language.

  **An amplifier owns its liveness**, because its buffer is `factor` times as long as its
  input's and cannot be the same one. That is not the layer deciding liveness — still refused
  — but the L1's decision re-indexed. The flags go down **before** the dead-slot return: a
  killed element sits inside the live range for exactly one frame before the next scan
  compacts it, and copies whose flags were merely left alone still draw in that frame.

  **It owns a `Counts` too**, and a `Counts` is three numbers at once: the workgroups a pass
  is dispatched in, the range a pass bounds itself by, and the instances a renderer draws.
  Two of those are separately load-bearing for a stage *below* an amplifier — one is fixed at
  build, the other at record — so a chain can be right about one and wrong about the other.

  **A fixture smaller than one workgroup sees neither mistake.** Eight elements run in one
  workgroup whatever range they were told, so every test here that means to catch a wrong
  dispatch count needs more than 64 of them. Two injected defects survived a whole test file
  before that was noticed, which is the argument for injecting them
- ~~**Cross-source interpolation**~~ — **built**, restricted to static sources where `seed` is the slot index
  and the paired read is a direct one. **The language half is built**: `pairs` on an L2
  header, `other.<attr>` for the paired element. **And the engine half**: the pairing node binds
  the second geometry as a third input buffer and reads `other[i]` at the same slot index, and
  `examples/morph.kir` slides a lattice onto a sphere on one fader. Five preconditions are
  refused where the Set is built, because none of them is a property of one file — two
  sources, the pairing node first in the chain, both static, both the same size, and the far
  source able to supply what the chain derives.

  **It needed no new syntactic category.** `other.position` reaches the checker as a swizzle
  — `expr . ident` is the grammar — so deciding it there costs one header keyword and one
  base name, and nothing else in the language moves. That is the same shape `amplify` had,
  and the two now break the L2 endomorphism on its two different axes: count and arity.

  **A pairing Set is one source made of two simulations**, not two sources — which is what
  answers "is the far geometry also drawn?" by construction rather than by a rule: there is
  one chain and one set of renderers over the pair, and the second simulation feeds the
  pairing node and nothing else.

  **Two defects, and both were a second copy of one fact.** The paired simulation was built
  without the chain's derived-attribute list, so its element struct was a slot short of the
  one the pairing node addresses it with and `other[i].position` read from the middle of the
  element before it. And every L1 resolved its params against the *first* source's map, so a
  name the two did not share came back a miss and reached the shader as zero — a picture with
  a shape in it, drawn from a value nobody set. `examples/morph.kir` is the picture.

  **Which two geometries is the Set's**, in `--set` order, and a Set holding a pairing L2 has
  exactly two sources. This is the system's first fan-in and it deliberately brings no general
  notation for one
- ~~**Slot interface contracts, attribute declarations, automatic adapters**~~ — **built as
  the derivation, and the three words above turned out to name one thing.** An unmet
  `consumes` is no longer an unconditional error: `age` and `velocity` are synthesised where
  nothing emits them, so a renderer wanting a motion streak composes with an L1 that never
  thought to emit `velocity`. That coupling is the whole of what the contract existed to
  remove.

  **The contract is the element layout, subsumed rather than sat beside**, which is what the
  note further down asked for. `generate_element_layout` no longer takes `emit` alone: it
  takes what the *Set* decided, so a slot can exist that no procedure named, and an
  `ElementLayout` now answers what it holds — as a slot, or as a derivation — rather than only
  listing fields.

  **Three things the design paragraph got wrong, all in the same direction.** It said
  `velocity` needed a third buffer; it needs a slot, because the obstacle is compaction
  reordering rather than buffer count. It said the storage would be always-on; it is
  conditional, because the Set knows whether anything consumes the attribute. And it implied
  adapters would be *nodes*; both rules are pure functions of the element and the clock, so
  they are a substitution at the read site and a write in a pass that already runs. Nothing
  was inserted into the chain.

  What is **not** built from that bullet: user-declared attribute names, and the count-mode
  and spatial-domain fields the algebra above mentions. Neither was needed by anything, and
  `amplify` already carries the count mode where it matters
- ~~L2 stacking with weights and attribute-based masks~~ — **built.** `weight` is a declared
  `param` and `mask` is a block writing a `strength` in `[0, 1]`; keeping them separate is
  what keeps the operator's control a `param`, since faders, bindings, transitions and a
  published interface all reach a `param` and none of them reaches an expression
- ~~Multiple L4 renderers over shared geometry~~ **Built**, and it cost almost nothing once
  `Ln` was a node: `Set::build_many` takes the L4s in draw order, they run in that order over
  one attachment (first clears, rest load), and no renderer changed to be in a list.
  `--set L1.kir,L4.kir,L4.kir` reaches it. What it *did* cost was the parameter model — every
  L4 in `examples/` declares `exposure`, so the collision refusal had to go first
- ~~**The `Field` type**~~ — **built.** `kind Field`, a `field` block, `point` in and
  `distance` out; `field(p)` evaluates the Set's field from any procedure;
  `examples/melt_blob.kir` is a shape and `examples/field_lens.kir` is a marcher that
  contains no shape at all.

  **It turned out to need no new syntactic category, and therefore not to reopen the
  "user-defined functions" non-goal.** The shape is one more kind, one more block, one more
  ambient and one more output — exactly what L2 and L3 added. What made that possible is the
  L5 argument run backwards: a `kind` says what a procedure *lowers to*, an L5 has no `kind`
  because it has no code to lower, and a field has *only* code to lower, so it has a file and
  no node. A consumer evaluates it as `field(p)`, and one per Set is the same restriction the
  camera already has — several would need naming, and naming is fan-in.
- ~~Graph compiler~~. Node graph as authoring representation, render graph as execution
  representation, with fusion of `Field` chains into single shaders. **Split and rescheduled
  rather than built**: the authoring half opens M4 as "Naming what a Set holds", and fusion is
  deferred with a trigger. Neither is M3's any more
- ~~`blend weighted` (weighted blended OIT) alongside `blend additive`~~ — **built**

**Demands on earlier work**

- The `blend` declaration must exist in the L4 header from M1, even with one legal value
- Identity must already be per element and carried, not a slot index, or multiple sources
  cannot be told apart at all
- IR must be a real IR from M1, not a thin wrapper over WGSL, or fusion has nothing to work
  with
- `capacity` must already be a Set-level value rather than baked into the artifact.
  **Discharged, and then outgrown**: it is a uniform and not a constant, which is the half
  that mattered — the generated WGSL is the same at any capacity — but it is no longer
  Set-level. Each source runs at the default its own procedure declares. The demand was
  written when a Set held one geometry, and what it was really asking for was that capacity
  not be baked into a *compiled artifact*, which still holds

**Clear before building**

- ~~Pack attributes into one storage buffer per direction.~~ **Done in M1**, because
  compaction had to be written against the packed layout or written twice. The compute
  stage binds 4 buffers now and stops growing with the attribute count, so L2 stacking no
  longer walks into the WebGPU default limit of 8. What was *not* done is narrowing each
  slot to its attribute's natural width: every slot is still a padded 16 bytes, which
  doubles VRAM per element against what it needs. That bill comes due at M2's deck, where
  the constraint is how many Sets fit resident, and it is one function in `layout.rs`.

  **Two milestones later it is still 16 bytes, and both halves of that sentence need
  correcting.**

  *"Doubles VRAM"* overstates it. A `vec3` is 16-byte aligned in `std430` whatever the
  layout does, so the recoverable waste is the scalars: `seed`, `birth_frac` and `copy` take
  a full slot each to hold four bytes, and `size` and `age` could ride in the `.w` of the
  vector above them. For `drift_shell` — `emit position, velocity, tint` — that is 5 slots
  to 4, **80 bytes to 64, a fifth**. For a procedure emitting six attributes it is nearer
  two fifths. Worth having, and not a factor of two.

  *"The bill comes due at M2's deck"* was measured against the wrong machine, and this
  document has made that mistake once before — see **Performance discipline**, where the
  same development machine's GPU timestamps are already recorded as unrepresentative. The
  numbers, so that a target can be held against them rather than a feeling:

  | | |
  |---|---|
  | One slot, `drift_shell` at 262144, no chain | **57.8 MiB** — 40 element, 2 alive, 15.8 render target |
  | A deck of four of those | **231 MiB** |
  | The same L1 at its declared maximum, 1048576 | 160 MiB of element buffer alone |
  | One `amplify 6` stage on it | **+120 MiB**, single-buffered, per frame |
  | One `amplify 64` stage on it | **+1.25 GiB** |

  So the element layout is not what decides whether a deck fits at the default capacity;
  **amplification is**, because it multiplies the same stride the layout would narrow. A
  fifth off an amplified chain is a fifth off the largest allocation in the system, which is
  the argument for doing it that "four Sets fit here" hid.

  **And it is scheduled, on a principle rather than on a threshold** — see "What a machine's
  size is allowed to decide", below. Sixteen bytes holding four is not a trade that buys
  anything at any capacity on any machine; it is slack, and slack is tightened because it is
  slack. Waiting for a Set that does not fit would be waiting for a *rich* machine to notice
  something a small one pays for every frame. It is `generate_element_layout` and the offsets
  that read it, and it is the second item of M4.
- **An L4 is now compiled against a specific L1's element layout**, since both declare the
  same struct over the same buffer. That is the slot interface contract arriving early and
  informally. When the contract becomes a real declaration, it should subsume this rather
  than sit beside it.

  **It did.** `emit` and `consumes` are the declaration, `Set::build_many` is the check, and
  attribute derivation is the adapter — so `consumes ⊆ available at this position` is now
  stated and enforced rather than being a property of two files happening to agree. The
  layout compilation stayed, and is now the *mechanism* under a declaration rather than the
  whole of the arrangement. That is the shape this note asked for.
- ~~**Give an L4 procedure a way to say what it renders.**~~ **Done, and the answer was to
  add no declaration at all.** The problem was real: quad expansion was justified by
  `topology points` while `topology` was an L1 header field a checked L4 tree had no
  counterpart for. What closed it is *inference* — an L4 that assigns `clip_b` is drawing a
  segment and there is nothing else it could be doing, so the check pass reads it off the
  `vertex` block and fills in `Checked::topology`. One place for the fact, and no way for a
  file to disagree with itself. The same move does **not** work for `blend`, which is why it
  stays declared: two blend modes differ in how identical assignments are combined, so
  nothing an L4 writes distinguishes them.

~8–10 weeks.

---

### M4 — Library at scale

**Goal:** finding the right thing among two thousand artifacts is faster than generating a
new one.

#### Naming what a Set holds

**This is the graph compiler's authoring half, rescheduled out of M3, and it is the first
work of this milestone rather than a nicety inside it.** A library of Sets you cannot save is
not a library, and that is exactly where this lands.

**Its stated precondition has arrived and inverted into a debt.** The condition was "wait for
the fan-in that multiple sources bring". Fan-in arrived twice in M3 — two geometries in a
Set, and a pairing L2 that reads both — and neither brought a notation. So the edges of the
graph are no longer inferable: which two geometries a `pairs` node takes is `--set` order,
written nowhere, and reordering the command line silently changes the picture.

**What is capped, blocked or quietly broken today, all of it for the same reason — a node has
no name:**

| | Where it shows |
|---|---|
| A slot with two geometries cannot be rebuilt under `--watch` | It starts, then every save prints a refusal and changes nothing |
| A slot holding a chain or a second geometry cannot be saved as a Set file | `--save-set` refuses it, and `--record-session` with it |
| MCP reaches an L1 and the renderers and no other node | An L2, an L3, a field and a second geometry are all unreachable to a model |
| One `kind Field` per Set, one L3 per Set | Refused at build, with "several would need naming" as the reason |
| A mask cannot say which source it applies to | The `source` attribute does not exist |
| A source's salt is derived from `--set` order rather than assigned | Reordering changes which geometry gets which randomness |

The first three are the sharp ones, because they are surfaces that *already exist* and stop
working the moment a Set holds what M3 taught it to hold. The rest are features that were
capped at one rather than designed for several.

**What it is, concretely.** Names for nodes, written where they are used rather than in the
`.kir` — on the terms HTML gives an `id`, since a procedure used twice is two nodes — and
edges spelled rather than inferred. That is one notation, and every row above is a
consequence of having it. The execution side needs nothing new: the render graph a compiler
would target already exists in miniature, because a node owns its buffers, pipelines and
uniforms and names its inputs.

What it is *not* is fusion. That half is deferred with a trigger — see "Deferred by
decision".

#### ~~Narrowing the element slot~~ — built

**39% off every element buffer across the geometries this repository ships**, and the same
39% off the largest allocation in the system, because an amplifying stage multiplies exactly
that number: one `amplify 6` on a Set at 262144 elements was 120 MiB of derived buffer.
`drift_shell` went 80 bytes to 48, `lattice_shell` 64 to 48.

Every field used to be padded to a `vec4` so the stride was `(2 + emit.len()) * 16` with no
per-attribute case analysis. Now each field is its own width at the offset WGSL's own
placement rules give it, which does better than the estimate above — the estimate counted
only the scalars *before* the attributes and missed that a `vec3` leaves four addressable
bytes behind it, so `position, size` is one 16-byte block and `velocity, age` is another.

It was done on the principle in "What a machine's size is allowed to decide" rather than on a
budget: those bytes bought nothing at any capacity on any machine.

**What it cost is that something now has to know WGSL's layout rules**, where nothing did
before. The host writes bytes at a published offset and the shader reads them through the
struct, so a disagreement is not a compile error anywhere — it is an element reading the
middle of the element before it. The align/size table is in one place, and the naga tests
validate the emitted module rather than trusting the arithmetic.

One test had to change its question rather than its number, and it is the interesting part:
`a_derivations_slot_exists_only_where_something_consumes_it` measured the *stride* to prove a
slot existed, which worked while every slot was sixteen bytes. `birth_t` is a `f32` that now
lands in the four bytes `seed` and `birth_frac` leave — it is free — so the stride assertion
would have read a slot that exists as a slot that does not.

#### Procedures a model can read

**A slice of this was scheduled long before the rest — procedures a model can read — and is
now deferred on evidence rather than on cost.**

It was scheduled because the *first* real MCP session opened by looking for an example —
*"is there a worked example of lines in another slot?"* — found none, because the deck had
one slot, and fell back on four failed compiles to learn what the language allows. A handful
of readable procedures would have answered in one call.

**The second session did not reproduce that**, and the difference is worth reading before
this is picked up again. Asked for line art with `topology lines` in the language, a model
wrote three procedures that landed and held the frame budget, and worked out two hazards the
specification does not state — that scaling a segment by anything but 1.0 pulls consecutive
segments apart, and that a per-segment taper scallops a joined strand. It did that from the
spec resource and the generated vocabulary, with no example to copy.

What changed in between was not the library. It was the *reference material*: the vocabulary
page is generated from the checker's own tables and now carries the topologies and the stage
outputs as well as the builtins, and `docs/ir-spec.md` gained a worked lines example. The
gap the first session hit was a documentation gap that read as a library gap.

So this waits for the failure to recur. **What would revive it**: a session where a model
reaches for something the spec and the vocabulary describe correctly and still gets it wrong,
or asks for an example twice. That is a specific observation to watch for rather than a
feeling, and it is the point of writing it down instead of quietly dropping the item.

The two steps, if it does recur, in order of cost:

- **The shipped examples as MCP resources.** They are files in the repository and they
  already work; exposing them is a `resources/list` entry each. Nothing infrastructural.
- **Saved Sets and their artifacts as resources.** `--save-set` already writes one and the
  store already content-addresses what it points at, so this is a lookup rather than a
  feature.

Both now have a complication they did not have when they were scheduled: **`examples/` is
app presets and a user's Sets are their own**, so a resource list has to say which is which.
See the file layout in `docs/manual.md`.

Embeddings, thumbnails and genealogy stay here in M4, because they answer *"which of two
thousand"* and the above answers *"how is this written"*.

**One design point worth fixing before either is built: the resource list is a curriculum,
not an index.** A model handed two thousand procedures learns nothing it could not have
guessed; a model handed four good ones, chosen to span what the language can do, writes
better code immediately. So resources stay a curated few and *search* over a large library is
a tool call — which is also the only way round the fact that `resources/list` is a list a
client reads in full.

**One gap the three-place layout opened and did not close.** A user preset can
now be loaded into the scratch, edited by a hand or a model, and every version
that compiles is kept — but **nothing saves the result from a running session**.
`--save-set` writes what the flags say and exits, so the loop ends at "find the
version you liked in `<store>/history/` and start a run from it". What it wants
is one control that writes the current material as a Set, reachable from a key,
from MCP, and from whatever surface M5 builds — the same three-way reach every
other control in this system has. Small, and it is the difference between a
library you can put things into and one you can only put things into before you
start playing.

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

- Node editor. Waits on the authoring notation at the head of M4 — M3 introduced node
  *ownership* and deliberately not a graph model, so there is nothing to edit until nodes
  have names
- Parameter surfaces with MIDI learn and signal binding UI. **What they show is decided in
  M3** — a Set declares which of its controls it publishes, so this surface renders an
  interface rather than inventing one. Without that it would be twenty-five knobs per slot
  and a filter nobody can save
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

- The Set lifecycle must already be schedulable by absolute time, not just triggered.
  **Undischarged by two closed milestones, and deliberately**: a transition schedules on beat
  count and nothing else, which is what makes a fade reproduce on a machine at a different
  frame rate and follow a tempo change mid-fade. Absolute time is a second axis rather than a
  correction, and it arrives with whatever needs it
- Signal confidence must already drive transition policy, since these sources are
  unofficial reverse-engineered integrations that will break

~6–8 weeks.

---

## Continuous concerns

Not milestones. These degrade silently if not defended at every step.

### Live safety

- No allocation or compilation on the render thread, ever. **Defended structurally and
  asserted by proxy**, where the audio thread has a counting global allocator behind it: the
  render path's claim rests on where allocation is *possible*, not on a harness that would
  catch it
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
Metadata records the former. Any change touching the frame path comes with a measurement.

**The machine this is developed on is not the baseline, and saying so is a rule rather than
a caveat.** It has been wrong twice in the same direction. Its GPU timestamps advertise a
feature it does not deliver, which is why the probe calibrates instead of trusting a flag.
And it has enough memory that no VRAM figure this project has produced has ever been
uncomfortable — which was then used, once, as evidence that a memory question could wait.
A number measured here is a number from a rich machine: it bounds nothing.

**Development is a desktop and a performance is probably a laptop.** Those are different
machines with different thermal limits and different memory, and the second one is the one
that matters — rehearsal and the night itself run on the same box, and which box that is
belongs to whoever is playing.

#### What a machine's size is allowed to decide

**No hard limit, and no ceiling chosen here.** A rich machine should be allowed to spend
what it has: more capacity, more slots resident, longer chains. Capping that to make a small
machine's experience the only experience would be this project choosing against its own
operator.

**What is not allowed is waste that a bigger machine merely hides.** The two are easy to
confuse and the test between them is simple: *does spending it buy anything?* Capacity buys
elements. A resident Set buys an instant transition. Sixteen bytes holding a four-byte
`seed` buys nothing at any size, so it is not a budget question and does not wait for one —
it is tightened because it is slack, and the argument is theoretical rather than measured.

**4 GiB of dedicated VRAM is the reference for "does this fit"**, and it is a reference and
not a cap: it is the number a design is checked against, the machine a default has to be
comfortable on, and the size at which a refusal must carry a sentence rather than a driver
error. Above it, the operator's machine decides.

**"GPU timestamp" is what that used to say, and it is honoured in labelled substitute.** The
probe asks for timestamps, proves them with a calibration workload, and reports which
instrument it actually used — `GpuTimestamp` or `HostWallClock` — because this crate's own
development machine advertises the feature and lies. So the rule is a measurement that names
its instrument, and the headline frame-path numbers in this repository came from a host
clock and say so.

---

## Deferred by decision

Designed, understood, deliberately not scheduled.

**Fusion — the graph compiler's other half.** Folding a chain of stateless stages into one
shader at codegen. The language already makes it legal rather than merely plausible: an L2's
statelessness is *structural*, since every generated `deform` overwrites its output from its
input before the block runs, so there is no previous value left to accumulate onto. Nothing
has to change in the language for this. It is an optimisation over a materialising
implementation that is already correct.

**It is deferred on a measurement and against an argument, not on cost.**

The argument is that fusion is a trade and not a win: it costs no memory and **pays the
stage's cost once per reader**, so a heavy deformation read by three renderers costs three
times. Materialising pays once however many nodes read it — which is the shape M3 just spent
a milestone building. Fusion makes the multi-renderer case *worse*.

The measurement is that nothing is near a budget. A stage is one compute pass and one
read-write of the element buffer; at 262144 elements and a 96-byte stride that is about
50 MB a frame, or 3 GB/s at sixty — a few percent of a modern part's bandwidth — and the
longest chain anything ships is two stages. There is nothing here to reclaim yet.

**What would revive it**, written down so this is an observation and not a mood: a probe
measurement putting a Set over budget where the cost is in a chain of stateless stages with
**one reader**. `probe.rs` already measures at real capacity with real parameters and the
governor already acts on the number, so the instrument to notice this exists — it does not
need building first, and this item does not need scheduling until it fires.

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
  are salted **per source** from the seed stream — so re-seeding a Set changes its randomness
  without touching anything structural, and two geometries built from the same procedure do
  not draw the same dust. This said *per layer* until a Set could hold two geometries;
  changing it was the redesign this section warns that changing one of these is.

**Time**

- `t` is simulation time, never wall clock. It advances by `steps * dt` where `steps` comes
  from a `tick` record: emitted from real time when live, read back verbatim on replay.
  Nothing in the engine measures anything, and the record is written and read back: a
  recorded session closes each frame with a `tick`, and `--replay` takes the step count from
  it rather than deriving it again — which is why a replay is frame-exact on a machine that
  runs at a different speed.
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
  record, which is what keeps every external coupling declarative and adjustable. **`beats`
  is the one exception and is deliberate**: it reaches IR as an ambient with no `param` and
  no `bind`, because what follows a tempo is not a parameter value but the passage of time,
  and a binding writes one number where a clock has to move everything the procedure does.
- Every name the language gives meaning to is reserved against params and locals:
  attributes, ambients, `id`, `other`, and — within the layer that writes it — stage
  outputs. Generated WGSL additionally mangles all IR-derived identifiers, so a procedure cannot capture a generated name whatever it is
  called.
- One `t` value means one record shape, across every file. A decoder dispatches on `t`
  alone, and every ndjson decoder does.
- **A machine's size decides what an operator may spend, never what the engine wastes.** No
  ceiling is chosen here: more memory buys more capacity, more resident Sets and longer
  chains, and capping that would make the smallest machine's experience the only one. Slack
  that buys nothing at any size is a different thing and is tightened on sight. 4 GiB of
  dedicated VRAM is the reference a design is checked against — the size a default has to be
  comfortable on, not a limit anything is held to. The development machine is not that
  reference and is not evidence about it; a performance is likelier a laptop than the desktop
  this is written on.
- `capacity` is a **per-source** dial with a range declared by the artifact, passed as a
  uniform. It was Set-level while a Set held one geometry; each source now runs at the default
  its own procedure declares, and `--capacity` overrides every source at once.
  It is not part of a procedure's identity — otherwise the library multiplies by every size
  anyone wanted.
- Spawn timing is engine-side. No ambient exposes the birth fraction: an exposed one is
  forgotten by half the generators that need it and applied twice by the other half.

**Editing and revision**

- A Set **value** is immutable and content-addressed; a **compiled instance** is not. Every
  structural change produces a new value and a record, which is what makes replay work.
  Whether the engine rebuilds the instance or updates it in place is an implementation
  choice, and it may update in place whenever the change needs no reallocation and no
  recompilation. In practice only parameter values take that path today: a `bind` change
  needs neither and still rebuilds, because a rebuild restates the whole slot. Editing in the background is therefore fast without punching a hole in the
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
| `docs/manual.md` | How to play it: every flag, every key, and what each does |
| `docs/plugins.md` | The out-of-process boundary, and why those two things are outside it |
| This file | Milestones, their demands on earlier work, and the settled decisions above |

None of the others should restate this one, and this one should not restate them. The two
that overlap most are the README and the manual, and the line between them is audience: the
README says what exists, the manual says how to use it.

---

## Reading order for implementation

1. `README.md` — invariants, then V1 scope
2. `docs/ir-spec.md` — the whole thing before writing any parser code
3. This document — the **Demands on earlier work** sections only, during M1
4. `docs/manual.md` — when there is something to run
