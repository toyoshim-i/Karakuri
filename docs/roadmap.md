# Karakuri — Vision and Roadmap

Companion to [docs/principles/](principles/) (the rules in force, one file each) and
`docs/ir-spec.md` (the IR). What has already been built is *What exists today* below; the
milestones that closed are kept whole in [history/](history/).

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
- **Library** — search, genealogy, embeddings, thumbnails (M4), on top of the content
  addressing that exists from M1. The separate metadata file it also wants is now written
  for every stored artifact **and read back**: the `read_set` MCP tool answers what a saved
  Set holds off those cards, so a model choosing between kept material compiles nothing.
  The reading at scale is there too: `list_sets` and `--list-sets` enumerate what a store
  holds, filtered by what a node is called or by which layer a Set uses. What is not built
  is grouping and genealogy, and a search over more than a name — the card fields those
  would need have no producer. Thumbnails are not built either, and what is missing there is
  a decision rather than machinery: a card is per artifact and one procedure cannot be
  rendered alone, so *what a thumbnail is of* has to be settled, which M5's Set browser is
  the right place to settle it beside. See what M4 still owes, below

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
element in, `N` out — and `uses <name> : Geometry` breaks it on the *arity* axis, taking a
second geometry and returning one. Both stay in the same slot position, because what a `deform` writes is decided
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

  **What is not built is the identity.** Nothing exposes `source` to a procedure, so nothing
  downstream can mask on one. **What is missing is a read rather than a thing to carry**,
  which is a change from what this paragraph said: it read "there is no `source` attribute —
  nothing carries one", and the spec has since retracted the attribute. A chain instance
  knows statically which source it runs over, so `source` is a per-source *uniform*, and the
  value it would hold is the salt. The **salt** is built: `--save-set` writes a `seed`
  record per geometry carrying what that source was running at and `--load-set` gives
  each one back to the source its index names, which is the *assigned and recorded* the
  design asks for and the reason it asks for it — every derivation fails on a case this
  system has, since graph edits move a position, a `.kir` edit moves a content hash under
  `--watch`, and a declared procedure name collides for the same lattice used twice. A Set
  nothing has recorded still derives from the ordinal, which the spec licenses in the same
  breath: where a value came from stops mattering once it is recorded.
  So the value a mask would compare is assigned, recorded, and already resident in every
  generated module as `seed_salt`. What is left is the read, and a mask that names a source
  rather than spelling a number. See "Naming what a Set holds".
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

## What exists today

**What is built, part by part, and how much of it is proved** — the answer to *does it
actually do that yet*, as against the milestones below, which say where it is going. Kept
here rather than in a document of its own because a status page nobody opens is a status
page nobody updates, and this is the file that gets opened.


There is exactly one assumption to prove:

> Can an LLM generate constrained IR, can we validate it and compile it to WGSL, and can
> we hot-swap it without dropping a frame?

One Set, slots hardcoded, no UI. Local oscillator and synthesized signals only. L1 for
geometry and L4 for rendering, nothing else.

**That assumption is proved and V1 is closed**, and two milestones have closed on top of it.
M2 made it playable — a deck, an L5 mix, tone mapping, audio and a beat grid, MIDI,
transitions, Set files and session replay. M3 made it deep: **every layer in the model is now
an IR `kind` with a node behind it**, so a Set is a chain rather than a pair — several
geometries, deformations that stack and mask and amplify and pair, a camera written as a
procedure, several renderers, and a signed distance field any of them can call.

M4 then closed too — the library at scale — and it opened with the one thing M3
left: **the nodes in a Set had no names.** They have them now, and the first *edge* between
them is spelled — `uses far : Geometry` in a procedure and `--edge morph.far=sphere_shell`
in the Set, so `--set` order no longer decides which geometry a morph reads. A Set file, a
hot-swap rebuild and the MCP surface all reach past a slot's first geometry. The same
notation now carries a **field** — `uses shape : Field` and `shape(p)` replaced the reserved
word `field(p)` — and a **camera**: `uses view : Camera` and `view.clip`, so a Set holds as
many viewpoints as its files declare and each renderer says which one it draws from. All
three inputs that were capped at one are named, bound and refused the same way.

A fourth slot type takes none of those. `uses only : Source` binds a geometry's *identity*
rather than its elements — a `u32` in a uniform against a bind-group entry — so a `mask`
can finally say which of a Set's geometries it applies to, with `source == only`, and a
node may declare several where it may declare one `far`.

**Work is in M5, the interface, and two pieces of it exist.** The workspace is on `wgpu` 30
and `naga` 30, which is what `egui` needs and which nothing else here wanted
([ADR-0155](adr/0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md)). And
[`karakuri-layout`](../crates/karakuri-layout/) holds the console's regions: an arrangement of
views and splits with a size, a minimum and a maximum each, solved to rectangles, with dividers
that drag, panes that fold away and a `solo` that leaves one region holding the window. It has
no toolkit, no device and no window in it, so its 35 tests run in a fifth of a second on a
machine with no adapter — which is the property
[ADR-0156](adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md) chose to own the
arrangement for. **Nothing draws any of it yet.**

The table below is what exists, part by part. What is still absent, and why, is in the
milestones further down rather than listed here: agents, the library, the node editor.

#### Status

**All three clauses hold.** `.kir` text goes through every stage and 262144 elements come
out on a GPU with no hand-written shader in the path.

An LLM can write this language from the specification alone. Three models were each given
`ir-spec.md`, a one-line aesthetic prompt, and nothing else — no example files, no
source, no tests. **Two of the three compiled on the first attempt with no diagnostics at
all**; the third failed on one type error and passed on the second attempt after reading
it. That is a result rather than an assumption, at n=3.

What none of them got was the *look* they were asked for. All three rendered a saturated
blob, and the same procedures rendered correctly once exposure and scale were turned down —
the code was right and the viewing conditions were guesses. The specification did not say
where the camera is, what world scale to work in, or that usable exposure falls as element
count rises. It does now, but the underlying problem is that a generator has no way to know
a value that depends on a Set-level dial. That is the tone mapper's job, not the prompt's.

The third clause closed last. A `.kir` file that changes is reparsed, rechecked,
regenerated, and rebuilt into a whole new `Set` — shader modules, pipelines, buffers, bind
groups — on a worker thread, and the render loop collects it with a `try_recv` at the top
of a frame. There is no `recv` and no `device.poll(Wait)` anywhere on the frame path.
`--watch` is both the demo and the development loop. What a swap does and does not do is
worth stating exactly:

- **It lands between two frames, never inside one — by convention, not by construction.**
  The live Set is only ever replaced at the top of `begin_frame`, which returns it as a
  `&mut` borrow the caller holds for the whole frame body, so no single reference changes
  identity underneath a frame. That is weaker than it sounds and the difference is worth
  stating: the command encoder is the caller's and borrows nothing, so a caller that calls
  `begin_frame` twice inside one encoder gets two Sets in one frame, and it compiles. Both
  callers in this repository call it once per frame; nothing in the types says they have
  to. Making it structural means handing the encoder out from `begin_frame` too, behind a
  guard that submits on drop, and `crates/karakuri-engine/src/deck.rs` now does exactly
  that: `Deck::begin_frame` returns a guard owning both the `&mut Deck` and the encoder, so
  a second frame while one is open does not compile and two generations of Sets cannot
  reach one encoder. That is the shape a caller with more than one Set has to use. A
  `HotSwap` driven on its own — which is still what the CLI does — keeps the weaker
  property above.
- **It transfers no state.** A new procedure means new buffers, so the incoming Set starts
  cold: `t` at zero, nothing primed. Warming a Set out of sight before it is shown is
  priming, which the deck does and this does not.
- **It is reversible for a window.** The outgoing Set is kept alive — and unstepped, so its
  `t` stands still — through eight warmup frames and thirty measured ones. If the median
  frame interval over those thirty exceeds the budget, the candidate is dropped and the
  outgoing Set is live again at exactly the `t` it was parked at. Only after it passes is
  the old one released, and it is released on the worker thread: dropping a Set frees GPU
  resources, and a free on the render thread is the same invariant as an allocation on it.
- **Saves that land during a window are collapsed to the newest.** A judging window is
  about forty frames, which is long enough for two more saves to compile behind it. The
  channel is first-in-first-out, so the verdict frame drains it to the last finished build
  rather than taking the front of the queue; a superseded Set is never shown, and a
  superseded build that *failed* still prints its diagnostics.
- **A build that fails changes nothing.** A `.kir` that will not compile prints its
  diagnostics on the worker thread and never becomes a request at all; a pair `Set::build`
  refuses is reported and put down. Either way the running Set keeps its `t`, its element
  buffers, and its live count.

Measured across a swap at capacity 262144, 1280×720, **on a host clock** — the same caveat
as every other number in this file. Two more that are specific to these: it is a frame
*interval* rather than a GPU cost, and it was taken in the **headless** test harness, which
has no surface and therefore no vsync and no compositor. Its `device.poll(Wait)` per frame
stands in for the pacing a real window would get, so these are the cost of producing a
frame, not the cost of showing one:

| | median | worst |
|---|---|---|
| steady, before the request | 5.0 ms | 13.0 ms |
| the frame the swap landed on | 4.6 ms | — |
| steady, after the swap | 5.0 ms | 10.3 ms |

Three to five frames were rendered between the request going out and the swap landing,
which is what "does not block" means operationally — a blocking receive would make that
number zero. The swap frame has not, across runs, cost more than the worst ordinary frame
in the same run; but it is one sample of a noisy quantity, so the honest claim is
"indistinguishable from jitter", not "free".

The watchdog measures a median rather than a worst case, for the reason `Probe` does: on a
host clock a single sample carries whatever the OS scheduler was doing, and one hitch is
not a reason to throw away generated material. It also discards the first eight frames after
a swap, because a cold Set's first frames pay for pipeline first-use and for its own
whole-capacity buffer upload — a budget check that fired on frame one would roll back every
candidate that ever existed.

The budget defaults to 20 ms: one 60 Hz frame plus slack, 60 Hz being the rate `dt` sets.
**That default does not generalise to a faster display, and this one is faster.** On the
120 Hz panel it was developed on, steady state is 8.3 ms and a frame rate cut in half reads
as 16.7 ms, which the default does not catch. `--budget-ms` is the operator's answer; a
budget derived from the display rather than from a constant belongs with the budget
governor and is not built, alongside the decision about whether GPU timestamps can be trusted at all. That decision is a **ratio against a second measurement of the same submission**, not a constant: a fixed floor was what let an adapter's meaningless 0.095 ms reading through, and it is checked on every measurement rather than once, because the adapter that produced it passed calibration and lied afterwards.

| | |
|---|---|
| IR: parse, type and contract check, cost estimation | Works. Diagnostics carry a span, a hint, and every error at once. A `consumes` not covered by `emit` is rejected outright, except `age` and `velocity` which the engine synthesises — so a streak renderer pairs with any geometry emitting `position`, and everything else is still a refusal by name at build rather than a shader that comes up short at runtime |
| WGSL generation | Works, validated through naga. Generated names cannot be captured by anything a `.kir` can spell |
| Set, buffers, pipelines, compute and render | Works. Double buffering, Set-level `capacity`, parameters as uniform writes. Nothing on the frame path allocates: both per-frame uniform writes go through storage sized once at build time |
| Linear HDR end to end, sRGB once at output | Works |
| Store | Works, and the engine obeys it. `--save-set` writes a Set file and puts every node's `.kir` source in the store as a content-addressed artifact; `--load-set` reads it back and builds from it, resolving each procedure by hash or from inlined `src` when the file is bundled. `--bundle ID` **writes** that bundled form — the Set file plus one `src` run per artifact it names, to standard output, because a bundle is a thing you send somebody rather than something the library keeps — and refuses whole, naming the node, where the store cannot supply one of the sources. `--unbundle FILE` takes one back in: every inlined source must hash to the address its `slot` names before anything is stored, each becomes an artifact with a metadata card, and a Set id this store already holds is refused rather than overwritten, because an id you type is an instruction and an id that arrived inside somebody else's file is not. The `k` key — and the `save_set` MCP tool, which is the same control and the same code — writes the same file from a *running* session, for the focused slot or for one a call names, reading the capacities, params, bindings, camera, layering, selected renderer, seeds and sources off the Set on screen rather than off the flags — bar the two the Set does not hold, which come from the run: the edges, which nothing can rewire mid-run, and each node's spelled name. **Every record type has a writer.** `audio` and `tempo` go through a record every frame, `gain`, `residency`, `look` and `transport` on every key that moves them, `set`/`slot`/`capacity`/`param`/`bind`/`edge`/`camera`/`merge`/`seed` on every save and load, `src` on every `--bundle` and read back by `--unbundle` and by any `--load-set` of a bundled file, `save` on every live save that reached the disk — a save still being written when the window closes is waited for, up to five seconds, so quitting straight after pressing `k` still records it, and past that bound the run says how many it left behind rather than letting a hung disk hold the quit — and `tick` once a frame into a session stream. `--record-session` writes the timeline as it happens and `--replay` renders it back, so a performance is replayed rather than only rebuilt |
| What a Set file cannot carry | Named rather than dropped, and printed on load. A `param` may be a vector and the engine's map holds `f32`; and a `camera` record carries two of `Orbit`'s six fields, so the other four come back as defaults. Each is a real disagreement between the format and the engine rather than an omission in the loader, and a Set file that half-applied in silence is the failure this repository keeps refusing. Two have left this list. `seed`, `capacity` and `param` keyed by layer against an engine holding one of each per Set: params are per node now, the salt is per source, and each source runs at its own capacity. And a `slot`'s **name**, which used to be recorded and reported because no surface pointed at a node by one — an `edge` does, so a name is carried back to the node it belongs to |
| One frame loop | Works. There were two — the window's and the offscreen PNG's — and the seam between them was meant to be a single line (a live run measures its step count from a clock, a replay reads it from a `tick`). It had become five, and **every replay defect this project has found was one of the four extras**: the look applied once instead of per frame, the governor running on one path only, the present pass skipped on unkept frames. Now `frame::compose` is the loop and a `Sink` is where it goes — a window, a PNG writer, and a test double. The **ordering is structural**: `compose` takes the committing work as a closure and calls it only after the sink has a target, so a frame with nowhere to draw cannot record a `tick` claiming steps the deck never took. That was two statements in the right order before, and getting it wrong was invisible. The test sink is what finally reaches `Live::frame`'s claims, which no test in this program had ever done |
| Replay fidelity | Works, and it is now checked end to end rather than argued: `crates/karakuri-cli/tests/replay.rs` runs the binary, replays two sessions differing in one record, and reads the pixels. Every assertion there is paired with a **control** — the same session replayed twice, byte for byte — because this material moves on `t`, so "the frames differ" is true whatever the code does. Building it closed a real hole: a `look` record moved nothing on replay. The offscreen renderer took a look and applied it once before the loop while the replay driver wrote every decoded one into a variable nothing read again, so a session replayed under the look it *started* with however many times the operator changed it — and with no automatic gain by design, the exposure is a control an operator is expected to ride. It also closed a second one of the same family: a `tick` is a **terminator, not a header** — `split` files each record into the frame of the next tick — and a frame used to push its `tick` before the `audio` and `tempo` it had just measured, so a replay showed frame N what frame N−1 heard, in the two signals every binding is driven by. Both bugs were in the record ordering of the one function that has no test reaching it, `Live::frame`, which needs a window; the splitter half is tested and the writer half is checked by reading, and that is said where the test is rather than left to assume |
| Session recording | Works, and it carries its own material: `--record-session` writes the Set at the head of the stream — from `--load-set` when given and saved under `ID-material` when not — so `--replay` needs nothing else and neither does this flag. It is now **refused** with `--render`, `--seq` and `--replay` rather than silently dropped: the recorder is only built on the path that opens a window, so those runs exited 0 having written no session and said nothing, which is the failure the recorder's own construction site carries a comment warning against. It used to write the `--load-set` file *or nothing*, and "or nothing" was a timeline of ticks with no material under it — a session that refused to replay long after the set was over, with nothing said at the time. What is still short: a session stream has no way to say what a **deck** held, so only slot 0's material is at the head and a multi-slot session replays the rest of the performance against a deck of one |
| Set file and session stream | The two projections are kept apart by `Record::is_set_state` and `project`. A Set file drops the mix as well as the ticks, and for a different reason: a gain is state, but the *session's*, and a Set file that restored one would pull down whatever fader it was next loaded under. A session folds *down* to a Set file — `Store::save_session_as_set`, ticks dropped and state folded — which is the projection the format specifies. What does not exist is the other one: folding a session to the deck state it ends at, so a run could resume where the last one stopped. Nothing needs it yet |
| Transport | Works, per slot. `free`, `tempo` (rate, scaled by the room's tempo against a per-slot anchor) and `beat` (position lock, jumps and reverses). **Two mechanisms, because there are two kinds of material**: closed-form state at `t` is a pure function of `t`, so a seek costs one element pass; accumulating material integrates, and reversing a sum is impossible rather than slow, so all it can be given is a rate. The anchor is a per-slot dial because **material has no intrinsic tempo** — a `.kir` declares parameters and a capacity, not a bar length — and it defaults to the tempo at the moment sync is engaged so that engaging it moves nothing. What is not built: a position source that can itself reverse. Audio only ever moves forward; a deck link (M7) does not, and arrives at the same entry point |
| `beats` | Works. The session's tempo grid, readable from IR as an ambient beside `t` — per substep, continuous across a tempo correction, and the same instant `t` names. **Without it the picture ran at wall time whatever the music did**: a tempo change moved the grid every binding was sampled on and moved nothing that was drawn, and a `bind` cannot close that, because what follows a tempo is not a parameter value but the passage of time. `beat` and `bar` stay bus names and stay different — phases in `[0, 1]` for driving a parameter, where `beats` is unbounded and monotone for driving a position |
| Signal binding | Works. A binding samples the bus, curves it, maps it onto a range, and blends into the parameter **by confidence** — so a binding to `beat` or `bar` takes full effect today, and one to `energy` moves fully when a microphone is open and a tenth as far when none is. The binding did not change when audio arrived; the same name started answering with a different confidence |
| Oscillator, synthesized bus, noise | On the frame path now. One oscillator per session, owned by the deck, advanced by the same `steps` the slots are |
| Element lifecycle | Works. `spawn` and `kill()` run, order-preserving compaction is wired into the L1 dispatch, and `element` and the draw are both indirect off one counts buffer. A procedure that can neither spawn nor kill skips the scan entirely and dispatches in place |
| Tone mapping | Works. Four operators — clamp, Reinhard, ACES, AgX — chosen by a uniform, so switching one mid-set is a buffer write. ACES by default, picked by rendering all four across five exposures and looking; `cargo run -p karakuri-engine --example tonemap_compare` regenerates that. Applied once, immediately before the single sRGB encode |
| The deck and the mix | Works. Up to four slots, each with its own `HotSwap` and its own HDR target, folded together in slot order with a per-slot **gain, opacity and blend mode**. Allocated holds its state, so a slot brought back resumes rather than restarts |
| Diagnostics | Works, and it turned out to have a second audience. Seventy-three diagnostics carry a `hint`, written to be kind to whoever typed the mistake. The first real MCP session showed they are also **the specification an LLM reads**: denied an example to copy, a model probed the checker instead — safely, because `write_procedure` checks before it writes, so a failed compile changes nothing. `hint: v0.2 defines 'points' only` closed off line primitives in one try — a hint that has since had to be rewritten, because the language grew the primitive it was refusing. And the hint that carried a *reason* — `loop bounds cannot reference a param… cost estimation needs them fixed at parse time` — changed the model's whole approach rather than its syntax: it concluded that integrating a streamline per element was incompatible with the cost model and switched to an analytic curve. **A hint that only corrects syntax produces a procedure that compiles and cannot be afforded.** Every hint should say why |
| Examples | Twenty-two procedures. **`drift_shell + swirl_warp + soft_points` is the first chain**: an L2 twisting the geometry between the simulation and the draw, which no `.kir` could reach before because a Set was a pair. `drift_shell + soft_points` is the default and is particles; **`drift_shell + drift_streaks` is the same L1 drawn as segments**; **`drift_shell + glass_shell` is the same L1 again, drawn as a solid** — the same cloud that is a lamp under `additive` is an opaque ball under `weighted`; **`drift_shell + field_march` draws no elements at all** and marches a distance field instead; **`drift_shell + melt_blob + field_lens` splits that in two** — `melt_blob` is a shape and nothing else, `field_lens` is a marcher containing no shape, `--edge field_lens.shape=melt_blob` says which shape it marches, and either can be swapped without touching the other. `drift_shell + kaleidoscope + soft_points` turns one point cloud into six; **`drift_shell + beat_jump + soft_points` is the camera written as a procedure** — `beat_jump` cuts to a new angle on the beat and keeps facing the centre, and does it by hashing the beat number, so it holds no state and stays seekable under a scrub; **adding `second_eye` draws the same cloud from a second viewpoint in the same frame** — it declares `uses view : Camera` and `--edge second_eye.view=orbit` points it at the built-in orbit while `soft_points` keeps `beat_jump`; **`drift_shell + swirl_warp + late_bloom + soft_points` stacks two deformations** and `late_bloom` is where a `mask` block earns the layer its claim — three lines of `deform` that are a different effect masked on `age`, on `seed` or on distance, with a `weight` beside it as the fader; **`lattice_shell + sphere_shell + morph + soft_points` is a cube becoming a sphere on one fader** — two geometries in one Set, paired element by element, with `--edge morph.far=sphere_shell` saying which of them the morph reaches for. `beat_strands + beat_strokes + beat_bloom` are three a model wrote over MCP in one session, and what they do with `lines` is not what I would have written — `velocity` redefined as the offset to the next sample, so consecutive segments share an endpoint. **`strand_shell + strand_strokes` is line art made *without* a line primitive**, kept as written now that `lines` exists because it is the measurement of what one topology cost: roughly twice the elements. `beat_shell` reads `beats`, `spark_fountain` spawns and kills |
| **Rendering primitives** | **Three topologies and two blends.** **`lines` cost the engine nothing** — the same quad, six vertices per instance and a `TriangleList` pipeline, laid along the segment from `clip` to `clip_b` instead of around a point. Both ends of a segment belong to **one element**: no element links to another, deliberately, since an index into the element buffer is what compaction invalidates. **`fullscreen` cost rather more, and it is what the SDF builtins were for** — `sd_sphere`, `sd_box`, `sd_torus`, `sd_plane` and the four CSG operators passed the checker from M1 with nothing able to draw them. An L4 declares it by having **no `vertex` block**, gets `eye` and `ray`, and **consumes nothing** — which is a checked rule rather than a consequence, and is what makes skipping the paired L1's whole simulation provable instead of merely plausible. Its fragment budget is eight times a sprite renderer's, because a fullscreen pass covers the canvas exactly once where sprites overdraw an unknown number of times. **`blend weighted` is the second way fragments combine, and the first way material in this project has ever occluded anything** — weighted blended OIT: an accumulation target, an `R16Float` revealage whose blend state multiplies rather than adds, and a resolve pass. Order independent, so it does not fight compaction the way a depth sort would. It reads `color`'s alpha as *opacity* where `additive` reads it as emission strength, and clamps it, which is the one thing an author has to know. What leaves the resolve is exactly what the additive path leaves — premultiplied colour, coverage in alpha — so **L5 was not touched**: the mix reads a weighted slot without knowing the mode exists |
| Blend modes | Works: `add`, `over`, `max`. The set is what survives an **unbounded linear HDR** mix rather than what a VJ mixer usually lists — `screen` and `multiply` assume `[0, 1]` and nothing has tone mapped this far up the pipeline, so `screen` of two 2.0s is 0.0. `over` is the only mode in which one layer hides another, and what it hides with is coverage the L4 pass accumulates into alpha; thin material barely covers, which is correct rather than a defect. This is also what finally separates **gain from opacity** — gain is the level the material arrives at, opacity is the fader across the blend and the only control that silences a slot under every mode. What is not built: a layer stack that is anything other than slot order |
| Priming and the governor | Works. Priming steps a slot and does not draw it — L1 owns every piece of per-element state and L4 is stateless, so warming is the compute passes and nothing else. The governor computes each slot's effective residency from the operator's request and a budget of per-Set costs measured by the probe at build time. It **never demotes a Live slot**: an over-budget deck reports and suspends priming. An unmeasured Live slot means the committed cost is unknown, and unknown is not headroom, so priming is suspended until every slot has been measured |
| Audio | Works. Analysis runs in the driver's callback, not on the frame path, and the frame reads one small value through a `try_lock` on both sides so neither can block. Level is RMS mapped −60 to −6 dBFS: a mastered track's loud windows sit near the top of that, where a top at 0 dBFS would leave real music between 0.80 and 0.90 and a bound parameter barely moving |
| Beat tracking | Works. The grid is **predicted, not chased**: once locked it free-runs, takes a slow trim, and moves not at all for a single disagreeing estimate — re-acquiring takes eight consecutive consistent revisions. **The octave is folded, not judged**: every candidate period is halved or doubled into a one-octave window centred on the grid, which starts at `--bpm`. So a window centred an octave off tracks an octave off, `,` and `.` are the fix, and there is deliberately no automatic one — the alternative is a heuristic that can be confidently wrong, which is the failure that shows on stage. A 3:2 error is a different problem and is not touched. The correction leads by the analysis lag plus the output lag, so what is shown lands on the beat rather than behind it |
| Where the lead comes from | The measurable part is computed and the rest is a **signed operator offset**, on `o`/`p`. That is not a gap waiting for a better sensor: sound and picture leave by different paths, neither ends at this machine, and what has to line up is what a person in the room sees and hears. So the measured half aims at being *stable* rather than complete — a constant unknown costs one adjustment, a drifting one costs the whole night — and the offset goes negative, because a delayed PA makes the sound the late one |
| Per-slot metering | Works. Mean and peak linear Rec.709 luminance per drawn slot — Live, or being auditioned — reduced on the GPU and read back without ever waiting, so it lags a few frames and says by how many. A slot nobody is drawing reads nothing rather than reading what it last drew. Texels whose luminance is not a finite number are counted and left out of both figures: one NaN admitted to a sum makes the mean NaN, and dividing by a value that reaches zero is ordinary enough in a shader that a routine artifact would otherwise cost a slot the number its fader is set by |
| `kind Field` | Works. A signed distance function in a file of its own: handed a `point`, it assigns a `distance`, and it draws nothing and holds no elements. **It has a file and no node** — the L5 argument run backwards, since a `kind` says what a procedure lowers to and this has only code to lower — so it is spliced as one WGSL function per *slot* into whichever procedures declare one, and into no others. **A caller says what it takes and the Set says which** — `uses shape : Field` in the header, `shape(p)` in the body, `--edge field_lens.shape=melt_blob` on the command line — and a slot nothing binds is refused rather than filled in from whatever field is lying around. That replaced `field(p)`, a reserved word, which is a breaking change to the language and is exactly what capped a procedure at one field. Its cost is **per evaluation** and the caller pays it once per call, so a marcher and a field that each fit alone can still be refused together, with both figures in the message. Several per Set, each a node with a name and an address of its own |
| Masks | Works: a straight front at an angle, and an iris, per slot. A mask multiplies the layer's opacity per texel — everything the fader does, done to part of the frame — so it needed no new place in the composite. Both ends of the front are exact, 0 revealing nothing anywhere and 1 revealing everything everywhere, which is what lets a layer masked to nothing be **skipped**: a third escape from material that has gone NaN, beside residency and the fader. **A wipe is a mask and one scheduled move** (`c`), and neither half knows about the other. Out: a mask read from a texture — an arbitrary shape, or another layer's luminance. `kind Field` now exists and is what such a mask would be written as; what is missing is the deck's mask reading one, since `shaders/composite.wgsl` is a hand-written engine shader with a fixed set of mask kinds rather than a generated one |
| Transitions | Works. One scheduled move — a control, a destination, a musical duration, a curve — and a crossfade is two of them sharing a start and a length. `f`/`g` fade the focused slot out and in, `x` crossfades to the next slot, `n` and `j` choose where a fade starts and how long it lasts. **The first thing in the engine that schedules on the beat clock** — the transport already *follows* it — and a fade is a function of the session's beat count and of nothing else, so the same records reproduce it on a machine at a different frame rate and a tempo change mid-fade moves the fade with it. One record schedules the whole move and the values it produces are not recorded, which is `tick`'s shape from the other end. A hand on a control cancels whatever was moving it. A wipe is one of these carrying a mask's front, which is why there is no `wipe` record and no `crossfade` record — the first-class things are the shape and the move |
| Slot preview | Works. `v` cycles what the output shows: the mix, then each slot. An audition **adds a draw and never a step**, so an off-air slot is drawn while it is being looked at and nothing moves that would not have moved anyway — an allocated slot shows the still it stopped at, a priming one shows what it is warming into. Not a second pass: the mix runs as always with that slot's terms at unity and the others skipped, so what lands is its own texels through the same tone mapper. Shown ignoring its faders, and metered, because the number wanted before putting it on air is the level the material arrives at. What is not built is a default renderer per topology — there is no slot holding geometry with no L4 to draw it, so there is nothing yet for one to do |
| MIDI in | Works. `--midi-in` opens a port, `--midi-map` says what each knob and pad does, and every action ends in **the same record a key press writes** — so a surface can do nothing a key cannot, and a session recorded from one replays with neither attached. The map is a file and is deliberately **not** in the stream: which knob is which belongs to the hardware in the room. With no map, every message prints the line that would map it, which is how a surface is discovered until M5 has a UI to assign one in. 7-bit; the 14-bit MSB/LSB convention is not implemented, which is about 0.8% of a fader's range per step. **MIDI out is not built**, so a surface's LEDs and motorised faders do not follow the deck — which starts to matter the moment two things can move one fader, and a transition is now one of them |
| Window output | Works, and it is a **preview**. `--canvas` is what a run renders at (1920x1080 by default) and `--size` is only how big the window is; the canvas is fitted into the window and the leftover is black, so dragging one changes what an operator can see and nothing about what is drawn. `a` sizes the window to the canvas one texel to one texel, which is what a downstream window capture wants. Three things came out of splitting them: the canvas now goes through a **record**, so a replay is at the size the performance ran at rather than at whatever `--size` says; a window resize no longer reallocates every slot's target on the render thread, which was the last GPU allocation on the frame path; and `present_mode` is now chosen (`Fifo`) rather than taken from whatever order the driver listed, which had made frame pacing a property of the machine with nothing saying so. Not built: fullscreen and display selection, because anything past a preview is the output-routing seam's job |
| MCP | Works. `--mcp PORT` serves the Model Context Protocol on loopback, so a chat client can read a slot's procedure, rewrite it, and be told what the compiler and the frame budget made of it — **vibe live coding**, where "the one that's showing, a bit more vivid" is a small edit to a declarative file. It is the **third control surface** after the keyboard and MIDI and obeys the same rule, so **a set a model rewrote replays with no model attached** — which is not a feature anyone could retrofit but this invariant used for something it was not designed for. It was not true when this landed: a review found that a procedure change went through a *file* and not a record, so an AI-driven set replayed with the procedures it started with. `procedure` records close it, and the material is now the last thing a session was missing. Most of it never touches the frame — reading is a file, writing is a check and a file, and the compile, the frame-boundary swap and the rollback are `--watch`'s, so **a model that writes something too expensive is caught by the machinery that already catches a human who does**. `write_procedure` returns the checker's diagnostics rather than a bare failure, which is what closes the loop. `save_set` is the fourth tool and the first control in this system a hand, a model and a future interface all want: it is the `k` key, reachable from a client, and it is one control rather than three — a call names a slot and may name the file, and everything after that is the key press's own code path, refusals and session records included. It needed a **server-to-loop request channel**, because what a slot is playing lives on the render thread and this server is a thread that cannot reach it; the tool waits for the outcome and says which id it landed under, because a model told "saved" before the disk has answered will tell its user something nobody can support. **The wait happens with the server's state unlocked** — there is a thread per connection and one mutex, so a call that waited while holding it would stop a client that only wanted to read a procedure. Two resources come with it: the IR spec, and a built-in vocabulary **generated from the checker's own table**, because prose drifts from code and a generated list cannot. Loopback only, on purpose; `ssh -L` is the way in |
| Tempo source | Works, and the verification is the interesting part. `--tempo-source` runs a separate program that reports where the beat is, as an **anchor** — a beat, a tempo, and the source's own clock reading at which both were true — so the pipe's delay never becomes phase error. The first anchor aligns the grid and every one after it is trimmed, because the source is another program from another repository; a shared grid carries a beat *number*, which is the one thing a beat tracker structurally cannot find. **The first implementation is Ableton Link, and what it is for changed when it was tested.** rekordbox joins a Link session and its decks can be made to follow it, but it never publishes the deck's BPM — Pioneer's answer is that Link has no master for a tempo fader to be the master of — so with rekordbox the picture does not follow the music by itself. That is a smaller failure than it first reads: a set is not a rehearsed timeline, matching the timing on the night is the craft, and "set by hand" is the answer this milestone gave to gain, the tempo octave and the panic key alike. What Link adds even when set by hand is that the setting is **shared** — `b` taps a grid on this machine, a Link tap puts every peer on it at once. With a peer that *drives* Link it follows on its own |
| **A panic key** | Deliberately not built, and the reasoning is in `roadmap.md`. Everything it would undo is already manually recoverable, and an anomaly is usually one frame and usually harmless — so a control that resets a performance in response to one does more damage than the thing it responds to. What was missing was a way to *see*, not a way to recover, which is the third time this milestone has landed on **show the number, act on nothing** |
| **Bloom** | Does not exist. Values above 1.0 are what would feed it |
| **Automatic gain** | Deliberately not built. The meter shows the number; nothing acts on it. An exposure that moves by itself is the worst thing that can happen on stage, and the honest order is to show the measurement first |
| Hot swap | Works. Built on a worker thread, installed at a frame boundary, watched for a window, rolled back automatically. `--watch` on the CLI. Starts cold — no state transfer |
| **Prompt-driven generation** | No in-app prompt. Generation happens through MCP from an outside client, and what reaches the engine is a file whoever wrote it — see the MCP row. What is not built is a control *here* that asks for material |

One thing known to be fragile rather than merely absent:

- **GPU timestamps.** See the working style in [contributing.md](contributing.md).
  Every performance number in this repository came from a host clock.

Simulation time used to be an f32 running sum (`t += dt * steps`), whose rounding depended
on how a frame's steps were grouped — twenty steps taken one at a time and ten taken two at
a time landed two ULP apart at `dt = 1/60`, which the built-in camera could turn into a
different last bit of a rendered pixel. It is now `steps_taken * dt` from an integer
counter, and `t` advances once per substep rather than once per frame: a frame of two steps
runs its two `element` passes at the two instants two frames of one step would.

The scan, measured because the spec asks for it rather than assuming it is free: **0.083 ms
per frame at capacity 262144** on this machine — a host clock, taken as the slope between
4 and 64 back-to-back scans in one submit so that submit overhead cancels. The same workload
under `TIMESTAMP_QUERY` resolves to 0.000 ms, which is the flakiness
[contributing.md](contributing.md)'s working style describes rather than a fast shader.

The hand-written `Points` pipeline is still present. It was the vertical slice that had to
keep working while everything else was built, and it can go once the generated path covers
what it covers.

---


## Milestones

Sizes are rough and relative — a sense of which milestone is larger than which, not an
estimate of anyone's calendar.

### M1 — Closing the loop — **closed**

Proved the assumption the whole project rests on: an LLM can generate constrained IR, it can
be validated and compiled to WGSL, and it can be hot-swapped without dropping a frame.
[history/m1.md](history/m1.md).

---

### M2 — Playable — **closed**

A deck of Sets, a mix, transitions on a beat grid, MIDI and audio in, a session recorded and
replayed frame for frame. [history/m2.md](history/m2.md).

**Still owed.** MIDI *out*, so a surface's LEDs and motorised faders follow the deck, which
matters the moment two things can move a fader; 14-bit control changes, so a fader is more
than 128 positions. And **a session stream still cannot say what a deck held** — a Set file
describes one Set, so only slot 0's material reaches the head of a recording and a multi-slot
session replays the rest against a deck of one, reporting what it skipped. Closing that is a
format change.

---

### M3 — Expressive depth — **closed**

Every `Ln` in the algebra is a node with a `kind` behind it, and a Set holds a chain of them.
`--set` reaches all of it with no new syntax, parameters and bindings address a *node* rather
than a layer, and a Set can say which of them a console sees.
[history/m3.md](history/m3.md).

**Still owed.** The graph compiler's second half — fusing adjacent nodes into one pass —
which the authoring half in M4 was the precondition for. And **how a mask names a source**,
with the `source` uniform it would read.

---

### M4 — Library at scale — **closed**

A Set can be named, saved from a running session as the material on screen, listed, read
without compiling anything, round-tripped whole with its layering and fold, and sent to
somebody else as one self-contained file whose every inlined source is checked against the
address it claims. [history/m4.md](history/m4.md).

The goal sentence — *finding the right thing among two thousand artifacts is faster than
generating a new one* — cannot be checked, because there are no two thousand artifacts and
nothing generates one. What closed is the machinery, tested at the scale that exists.

**Still owed, and each with what would revive it.**

- **`parent`, recorded from the first generated artifact**, or the genealogy has a hole at
  its root that no later pass can fill. Untouched deliberately: nothing generates procedures,
  so an empty one written now would fill the hole rather than leave it visible. This is the
  sharpest demand in this document and it lands on M6.
- **`perf`, which needs the stage-7 probe.** What a compile pass can answer is
  `ops_per_element`, a different quantity in different units, and publishing one under the
  other's name is a mistake this project has made once already.
- **Thumbnails**, where the missing piece is a decision rather than machinery: a metadata
  card is per artifact, an artifact is one procedure, and one procedure cannot be rendered —
  so *what a thumbnail is of* has to be settled. M5's Set browser is where somebody will next
  be in a position to judge a framing.
- **Dual embeddings**, blocked on both halves: the text side needs `origin` and `tag`, which
  have no producer, and the visual side needs the thumbnail above.
- **Search over more than a node's name**, and grouping — the card fields both would need
  have no producer yet.
- **A deck-slot variant pool**, where alternatives differ at L1 or L2 and so carry state of
  their own. It needs the unselected ones primed off air, which does not exist. The half that
  lives inside one Set is built.

---
### M5 — Interface

**Goal: the application.** Karakuri is a GUI application for playing a VJ set in real time —
you pick a Set per deck and mix them live — and this milestone is where that application
exists. **MCP is a mouth, not the control stick**: it is there to help a person, and the four
properties at the head of this document are all about what a *person* can do in front of an
audience.

**That is a correction, and the sentence it replaced is worth keeping visible**, because it
is the one a reader arrives at first and it sent this milestone in the wrong direction: *"a
surface where a human can see what the system is about to do and disagree with it."* That is
true and it is M6 read backwards — it describes the agent-oversight half of a panel whose
main job is being an instrument. Disagreeing with an agent is one thing the console does,
not what it is for. The restatement is the author's, in this session's own words.

The ordering below it still holds, and for its original reason. **This comes before agents
deliberately.** An autonomous system that cannot be observed and overridden is not usable on
stage, and building the observation surface afterwards means retrofitting it into decisions
already made.

#### The manual is the reference, and it is written before the panel

`docs/manual/` is the console's manual, published from this repository's `docs/` with GitHub
Pages. It was written **ahead of the implementation on purpose**: the panel does not exist, so
the manual is the only place its behaviour can be pinned, and it is a better place than a
specification because a sentence you cannot write about a control is a control designed
wrong. Read it before touching this milestone. Four pages, published at
<https://toyoshim-i.github.io/Karakuri/manual/>: [the seven rules](manual/index.html) the
surface obeys, [what the words mean](manual/concepts.html), [the console](manual/console.html)
region by region with a working mock of the panel in it, and
[every operation](manual/operations.html) with each way in.

**Its seven rules are capped and `docs/principles/` is not**, which is the difference between
the two documents rather than an inconsistency between them. Principles accumulate, because a
codebase learns. The manual's rules must not, because they are the mental model somebody holds
before the panel makes sense, and a set of rules that keeps growing is a set nobody holds:
adding one there means removing or merging one.

#### One vocabulary, and this milestone opens with naming

Every operation must be reachable from the panel, from the keyboard alone, from a mapped MIDI
control, and from MCP. That is structural before it is visual: **each operation is named once,
and every surface routes into that name.** An operation only the mouse can reach breaks it; so
does one the map cannot address. A step sequencer is a fifth route and works for the same
reason.

**It is checkable rather than intended.** With the vocabulary exhaustive, a test asserts that
every command carries a key binding, is addressable by a map, and is what the panel emits —
the shape `crates/karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs` already uses, and the
shape `Record::vocabulary` already uses for records.

**The four surfaces barely overlap today, and here is the count**, read off the code rather
than estimated: **35 live operations reachable from keys** — and only from the *focused* slot,
since nothing but `0`–`3` addresses another; **8 from MIDI**, which is the whole of `Target`
in `karakuri-midi`'s map (`gain`, `opacity`, `exposure`, `on-air`, `prime`, `blend`,
`preview`, `tap`) and cannot express a node address, a parameter name or an id; **6 MCP
tools**, which touch nothing in the mix, the clock or the deck's residency; and **38 command
line flags**, many of which are the *only* route to what they set.

The sharpest consequence: **a model can rewrite a whole procedure and cannot write a single
parameter.** `--param` exists and is reachable from one surface at one moment. Reachable from
nothing at all while running: parameter writes, binding changes, edge rewiring, the camera,
seeds, capacity, canvas size, un-merging a slot, restoring every renderer to the fold, a gain
fade, and starting or stopping a session recording.

**So M5 opens with naming rather than drawing**, which is the shape M4 opened with — nothing
could be edited there until nodes had names, and nothing can be routed here until operations
do. [Every operation](manual/operations.html) is where that enumeration is written down, and a
surface missing from an operation's row is a line of this milestone's work. **It is written**:
46 operations, and 50 of the 199 ways in exist. **Five of those arrived from
the console's own shape** — moving a boundary, folding a bay or a pane, bringing one back, and
solo — and all twenty of their routes are empty except the pointer, which is the first rule broken
by the surface the first rule is about.

#### What was settled about the panel, and where each thing is argued

Recorded here because they are decisions, and the manual states behaviour rather than reasons.

- **The mixer is an L5, and so is a master effect.** `karakuri-engine/src/node/merge.rs`
  already writes the signature: `[Texture] -> Texture`. A master effect is that with one input
  and the mixer is the same with several, so admitting frame effects is **giving L5 a writable
  form** — adding it to `Kind::ALL`, whose one stated reason for excluding it (no code to
  lower) lapses the moment an L5 is authored — and not extending the algebra. Fan-in is
  already solved by `uses` plus `edge`; a per-deck effect and a master effect become one node
  at different points; and **the deck count stops being a system constant**, since how many
  inputs an L5 folds becomes a property of the procedure rather than of one built-in shader.
  **Feedback is the exception** and wants a decision of its own: reading the previous frame is
  a cycle.
- **A step sequencer is a signal source, not an effect.** It produces no pixels. What it does
  is write controls on the beat, and both halves exist: `karakuri-signal`'s bus is a
  name-to-value lookup already answering `bpm`, `beat`, `bar`, `energy`, `band`, `band3`, and a
  `bind` record names a signal as a string with its curve and range. A sequencer is **one more
  name on that bus**, stepping on the oscillator that already drives `beat`. A lane is a
  binding, so a lane drives a Set parameter as readily as a deck fader.
- **A MIDI control is bound to a deck and a position in its published interface**, never to a
  Set's parameter by name. Name-binding makes the mapping a cost paid again on every swap,
  where a learn flow exists to make it a cost paid once; and the deck being in the address
  settles the crossing when one Set is loaded into two decks. The order is what a Set
  *published*, and where it published nothing, the order its parameters were declared —
  `Published` is already an ordered list, and its own doc's rule holds here: publishing decides
  what is *shown*, never what is *reachable*, so this must not become the only way in.
- **Learning happens from the tooltip.** Every compact control already explains itself on
  hover, so the assignment lives there — which makes the first rule structural rather than
  remembered: a new operation gets a tooltip, and a tooltip is a thing a map can reach.
- **A map is a file, saved and recalled per controller**, which is the shape the MIDI section
  above already gives it and for that section's reason. What is missing is reaching it while
  running.
- **There are two focuses and they must not look alike.** The deck selection persists and is
  what a key press is addressed to; keyboard focus is transient. Drawing them the same erases
  which of the two a reader is looking at.
- **A projector window is a second `Sink` and lives in this repository.** Syphon, Spout and NDI
  are the plugin ones. **Fullscreen is what an application on an operating system does**, not an
  output mode of its own, which supersedes the "not built, deliberately" note in M2's output
  routing bullet.

  *(One sentence here was wrong and is removed: it said a composited frame "already goes to *n*
  sinks". `docs/plugins.md` designs that and the `Sink` trait exists, but `compose` takes exactly
  one sink and all eight call sites pass one. What is built is the seam, not the fan-out.)*

  **And the sink gates the whole frame today, which the outputs row will have to undo.**
  `compose` returns `Skipped` before calling the closure that commits, so a frame with no target
  reads no clock, writes no record and does not advance the deck. That is right for a lost
  swapchain, which is momentary. It is wrong as a steady state: an operator who turns every
  output off would stop the instrument rather than stop publishing it. Advancing a frame and
  presenting it are one thing in this code and have to become two — which is the same change *n*
  sinks needs, so it is owed once rather than twice.
- **`gain` is a trim and `opacity` is the fader**, which `karakuri-engine/src/mix.rs` says
  outright and which the two behave as: opacity at zero silences under every blend mode, gain
  at zero does not silence `over`. Drawing them as two identical sliders throws that away.

#### What draws the panel — `egui`, and it cost a `wgpu` bump

**`egui` draws it** ([ADR-0155](adr/0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md)),
and the recommendation this replaces carried one claim that measurement falsified. It said `egui`
"takes the `wgpu` and `winit` versions already here". `winit` yes; **`wgpu` no** — there is no
`egui` release bound to `wgpu` 26. `egui-wgpu` 0.32 wants 25, 0.33 wants 27, 0.34 and 0.35 want
29, and 0.36 wants 30, and **26 was skipped entirely**. A toolkit that renders through `wgpu`
shares the engine's `Device`, so adopting one *was* a `wgpu` migration and the only question was
how far.

That question was measured rather than argued, by copying the workspace and changing the version
strings: **`wgpu` 27 costs 6 errors in 2 kinds, `wgpu` 30 costs 54 in 12.** All of the 54 are
descriptor fields added or removed except one — `Device::pop_error_scope` is gone, and
`push_error_scope` now returns a guard whose `pop()` yields the error. The workspace went to
**`wgpu` 30 and `naga` 30**, taking the larger migration once rather than the smaller one twice;
the argument and the rejected 27 are in the record.

**What `egui` buys is text and input, not widgets.** This panel's controls are faders, meters,
step grids and lanes, and no widget library supplies any of them — they are custom painting either
way. What would otherwise be written here is font loading, shaping, a glyph atlas, and the input
and hit-testing plumbing under it. That, and not a widget set, is the job being bought.

#### The arrangement is not the toolkit's

Every divider in the console drags, every pane has a width it will not pass in either direction,
and any pane folds away to give its space to the rest. `egui`'s own panels can express that —
`Panel` anchors to an edge with a `CentralPanel` taking the rest, panels nest through
`show_inside`, and `resizable`, `size_range` and `show_collapsible` cover the behaviour. **The
arrangement is still this repository's**
([ADR-0156](adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)), in
`karakuri-layout`, a crate with no toolkit in it that hands out rectangles.

The reason is the vocabulary above rather than anything about `egui`: **widening the library and
folding the sequencer away are operations**, and an operation is named once with every surface
routing into that name. State the panel owns is state a key press, a MIDI control and MCP can all
reach; state in `egui::Memory` under an `Id` is state only a pointer reaches comfortably. The
second reason is testing — the arrangement answers as arithmetic on a machine with no adapter,
rather than by running a `Context` and reading rectangles back out of the toolkit's memory.

**Solving never mutates the arrangement**
([P-0071](principles/0071-solving-a-layout-never-mutates-it.md)): a viewport too small for the
minima produces small rectangles and changes nothing stored, so a window dragged narrow and back
comes back to exactly what it left.

**And it leaves the operations page short.** Arranging the panel is operations, and
[every operation](manual/operations.html) carries no row for folding a pane, for moving a divider,
or for `solo` — which the console page already draws as a control with a tooltip. So the 41 is not
the whole count. What makes this more than bookkeeping is the first rule: **every operation is
reachable from the keyboard alone**, and a divider that is only draggable is not. Naming these is
what decides how a pane is resized without a mouse, and it is owed before the panel is drawn.

#### What building the console's arrangement found

`karakuri-console` holds the arrangement as one `Spec`, with every number read off
`docs/manual/style.css` and every derivation written beside it. Building it against the manual
turned up four things, and the first is architectural.

**A region cannot be added or removed while the panel is running.** The mock's `2 up` control says
a wide window fits three or four inspector panes and that the operator chooses how many *live*; a
`Spec` builds a `Layout` once and the arena has no insert and no remove. Rebuilding to change the
count throws away every drag, every fold and the solo state in the whole panel — add a third
inspector pane and the library's width, the program's height and the sequencer's fold all snap
back. What is missing is a pair of arena operations that keep the rest of the tree and its ids,
with `solo`'s saved flags resized alongside; it is not a change to the solver. **The same control
appears four more times** — `+ lane` in the sequencer, `+ add` in the master chain, `+ add output`,
and `+` on the scope list — so it is one gap, drawn five times.

**A node has one `[min, max]` pair, along its parent's axis.** The mixer wants a width floor (four
strips side by side) *and* a height floor (a strip is never cut off), and both could be stated only
because they happen to sit at different levels of the tree. Two constraints on one node cannot be.

**A split's minimum is not derived from its children's.** The body row declares 530, which is the
right pane's three bays and its dividers added up by hand three levels away. A test recomputes it
so the two cannot drift, and that test helper wants to be a method on `Layout` rather than a copy
in every consumer.

**And three places where the manual does not agree with itself**, all found by trying to build from
it rather than to read it. `.body-grid`'s centre track says `minmax(340px, 1fr)`, at which each
inspector pane is 165 wide — but `.param`'s fixed tracks come to 207 before the fader has any width
at all, and `.console`'s `min-width: 1010px` is what actually holds the panel, at which they are
237. The panel has two minimum widths and the smaller one is unreachable. The lede says *every
divider drags* and the mock draws a grip on four bays of eight. And *"height given back is rows:
roughly 48 in the library, and near 40 per inspector pane"* does not divide: a `.lib-row` and a
`.param` are the same box, so the same height is the same number of rows.

Two rules the panel is built to whatever draws it. It must not allocate on the render frame
path, and **it must be testable without a GPU or a window** — the model and the command layer
answer headless and the view stays thin, which is what keeps this milestone's tests off the
`mod gpu` side of `docs/contributing.md`'s split.

**The sequencing starts from the bottom, and the first piece is the layout.** It had been left
until the manual's operations page existed, since that is what the vocabulary is read off; the
page is written, and what it says is that the panel is a set of regions an operator arranges. So
`karakuri-layout` is built and tested before anything is drawn into it — it needs no toolkit, no
device and no window, which means the `wgpu` 30 migration and the layout model were done at the
same time without waiting on each other. Beyond that first piece the order is open, and nothing
below is blocked on anything else.

**Adds**

- Node editor. Waits on the authoring notation at the head of M4 — M3 introduced node
  *ownership* and deliberately not a graph model, so there is nothing to edit until nodes
  have names
- Parameter surfaces with MIDI learn and signal binding UI. **What they show is decided in
  M3** — a Set declares which of its controls it publishes, so this surface renders an
  interface rather than inventing one. Without that it would be twenty-five knobs per slot
  and a filter nobody can save. **How learning works is settled above**: a control is bound to
  a deck and a position in that published interface, the assignment lives in the control's own
  tooltip, and the map is a file saved per controller
- Set browser with live previews of priming Sets. **It carries M4's unsettled thumbnail question with
  it**: a stored still is not the live preview beside it, and what a thumbnail is *of* is
  undecided because a metadata card is per artifact and one procedure cannot be rendered
  alone — see what M4 still owes, where the machinery is costed and the question stated
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

**These moved.** Every rule that was listed here is now one file in
[docs/principles/](principles/) — stated once, so it cannot drift between documents — and the
decision behind each, with the alternative that lost, is one record in [docs/adr/](adr/).
`ls docs/principles/` is the index, because each filename is the rule it states.

This section said *reference, not history*, and that was the right split; what it had no room for was
the history, which is what `docs/adr/` is. It also had to carry supersessions inline — *this said per
layer until a Set could hold two geometries* — which a record's front matter now carries instead.

Nothing was left behind. The one entry that had been held here as unsettled — whether reference
material handed to a generator belongs in `origin` — is settled in
[ADR-0133](adr/0133-what-was-fed-to-a-generator-is-a-field-not-an-object.md): the object stays
rejected, the field does not.

---

## Document map

| | |
|---|---|
| `README.md` | The front door: what this is, a quickstart, and where every other document is |
| [`docs/principles/`](principles/) | The rules in force, one file each — `ls` is the index |
| [`docs/history/`](history/) | Milestones that closed, kept whole. History, never the present tense |
| `docs/ir-spec.md` | The IR: grammar, semantics, lowering, record formats |
| `docs/manual.md` | How to play it: every flag, every key, and what each does |
| `docs/plugins.md` | The out-of-process boundary, and why those two things are outside it |
| This file | Milestones, their demands on earlier work, and the settled decisions above |

None of the others should restate this one, and this one should not restate them. The two
that overlap most are this document's *What exists today* and the manual, and the line between them is
audience: the status document says what exists, the manual says how to use it.

---

## Reading order for implementation

1. `docs/principles/`, then this document's *What exists today* — the rules, then what is built
2. `docs/ir-spec.md` — the whole thing before writing any parser code
3. This document — the **Demands on earlier work** sections only, during M1
4. `docs/manual.md` — when there is something to run
