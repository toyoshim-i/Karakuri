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
   through the identical interface. **Authority is set per node of a Set**, not a global mode
   — [ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md). This
   sentence read *a per-layer setting* until then, which was one of three answers in play and
   was the oldest of them; it loses because no operation in this system addresses a layer of a
   live Set, and per *deck slot* loses to rule 06 of the manual, which rules out any switch
   that hands a whole instrument over. The senses each word carries are in [What the three
   words mean, and where](#what-the-three-words-mean-and-where).
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

V1 implements L1 and L4 inside a single Set. The full model — **six positions in the
architecture, which is not the IR's list of five `kind`s**: `L1` through `L4` are in both,
`Field` is a kind with no row here, and `L0` and `L5` have a row here and no `kind` file at
all, because [ir-spec.md](ir-spec.md) says outright there is no `kind L5` — compositing has
no code to lower. *Layer* in this column is the model position and nothing else; see
[What the three words mean, and where](#what-the-three-words-mean-and-where).

| Layer (model position) | Role | Milestone |
|---|---|---|
| L0 | Signal bus. Synthesized values from M1; audio, tempo and MIDI landed in M2 | M1 / M2 |
| L1 | Geometry generation. Vertices, particles, SDF builtins used inline. Built, and `kind Field` made the SDF builtins reachable: `examples/melt_blob.kir` is a shape and nothing else, `examples/field_march.kir` marches one | M1 / M3 |
| L2 | Deformation and motion. Time-axis modulation, physics | M3 |
| L3 | Camera and space. Viewpoint, motion grammars | M3 |
| L4 | Render and material. Raster, raymarch, splatting | M1 for sprites, M3 for segments and for raymarch — a fullscreen L4 with `eye` and `ray`. Splatting is unbuilt |
| L5 | Composite. Set mixing, transitions, post, output routing | M2 |

Orthogonal to the layers:

- **Control plane** — agents, director, mix agent, generation worker (M6). **What an agent
  is one *of* is still undecided**: this roster said *node agents* and M6 below asks for one
  per deck slot, and naming it either way here would settle it by wording. It used to be the
  same disagreement as *What authority is set on*, and is not any more — that one is taken,
  per node
  ([ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)), and it
  does not decide this one: an agent scoped to a deck slot can perfectly well respect an
  authority set per node. **MCP
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

**Work is in M5, and the console opens.** A window, every region of the mock in its place with
its bay head, both rooms, dividers that drag under a pointer, panes that fold, `solo`, and **a
real engine frame in the Program bay**.

Underneath it, in three crates that do not know about each other's problems. The workspace is on
`wgpu` 30 and `naga` 30, which is what `egui` needs and nothing else here wanted
([ADR-0155](adr/0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md)).
[`karakuri-layout`](../crates/karakuri-layout/) is the arrangement — views and splits with a
size, a minimum and a maximum each, solved to rectangles — with no toolkit, no device and no
window in it, which is the property
[ADR-0156](adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md) chose to own it
for. [`karakuri-console`](../crates/karakuri-console/) holds the console's own arrangement, every
number read off `docs/manual/style.css`; the panel's model, which is what a pointer and a keyboard
act on; and the `egui` view, whose palette is the mock's and whose seven bay heads are one
component. Its tests still run without a device.

**Two of the seven bays draw nothing but a head — master and sequencer — and a third, staging,
draws its empty state, which is the only state this program can reach.** That is on purpose: a bay
that looks finished does not get replaced. **What each is waiting on is three different things and
was described as one**, which the survey under *Where this goes next* now separates: the master's
value exists and always has a reading (`Deck::out`, since
[ADR-0224](adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)); the
staging lane's producer exists in this workspace and `crates/karakuri` declines to use it; and only
the sequencer is waiting on a value nothing anywhere holds. The other four have bodies: the Program
bay's two regions, the Mixer bay's strips, the Library bay's listing, and the Inspector's node
groups and parameter rows.

The picture is a live engine frame, and under it the **row of four
deck previews** is built: four cells, with **deck A auditioning in the first** and B, C and D
reading `off`. The program's engine is a deck of two slots and only deck A is making texels: deck B
is **parked**, which is neither stepping nor drawn, and C and D have nothing behind them at all. A
cell is painted whether or not a deck is behind it, which is the one place this
crate draws something with nothing handed in, and
[ADR-0170](adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md) is why:
*off* is a state the operator chooses and *not built* is not a state at all, so drawing nothing
would render the two the same. That record also carries the cell's shape — 16:9 and centred in its
track rather than filling it — and names the disagreement forcing it, which is that the mock's row
grows taller with the window and the arrangement's is pinned at 72.

**Each preview is an audition and costs a pass**, which is now literally what the code does: one
`Present`, one canvas, presented twice — into the picture's region and again into deck A's cell.
What a panel frame costs is measured and printed by the program rather than estimated. The still
reading — **0 frames and 0 allocations**
([P-0072](principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
first clause) — **is no longer reachable in this program at all**, and folding does not get back
to it: measured on 2026-08-26, the picture folded away leaves deck A auditioning at 47.0 fps, the
preview row folded as well leaves the parked tally's declared 30 Hz at 28.7–29.0 fps, and folding
the whole mixer bay on top of that changes the price per frame and not the rate, because the fold
hides the chip while the governor's parked slot stays parked. Zero needs a console with nothing
pending, which `crates/karakuri-console/tests/parked.rs` asserts headlessly. With the picture
live, deck A auditioning in the preview row under it, and the mixer bay, the transport, the
outputs row and deck B's parked tally rolling beside them, it is
**58.3–59.3 fps and 18.1–22.2% of a second**, over nine runs on 2026-08-26 on an M4 Pro at
1440x900 with nothing touching the window — a whole-frame median of 3.09 to 3.74 ms across the
nine, and 524 to 538 allocations a frame. The nine were taken over about an hour and drift upward
through it, which is the machine rather than the panel: the allocation count barely moves while
the milliseconds climb by a fifth. That is the price the rest of P-0072 is for.

**This line read 10.9% for two commits after it stopped being true**, which is the failure worth
recording beside the number: 10.9% was the picture and the preview row alone, and what moved it
was the mixer bay arriving rather than the parked deck. Nobody re-took it, because a figure in
prose goes stale in silence. The program now holds its own quoted figure against the run it has
just taken and says so when the two part company — `WRITTEN_ALLOCS` in
`crates/karakuri/src/main.rs`.

**Four figures have stood on this line, and only the last of the steps between them is the
work.** It read 10.7%, then 8.2%, then 10.9% across a preview row being added and the console
adopting `frame::compose`; on the run that gave 8.2% the three passes were *each* about two thirds
of what they are here, which is a clock moving under all of them rather than work leaving one. The
machine is worth a factor of two on its own, and the clearest evidence of it is that the figures
go the wrong way: a reading taken at the commit *before* the parked deck landed put the panel at
**37.1%**, against 18.1–22.2% over nine runs taken within the hour after it, on a panel doing
strictly more, with the whole-frame median at roughly twice what those nine read. The readout says
it in its own last paragraph — this machine reports about a sixth of these numbers with its other
cores loaded, proportions unchanged — so **compare ratios, and take a figure here as the order of
magnitude a decision gets made on rather than as a quantity two of them can be subtracted from.**
The number is restated whenever the code under it changes shape, because a figure taken on a shape
that no longer exists reads as current forever.

**P-0072's inputs are built and its policy is not: there is still no scheduler.** Two regions
declare, through `View::declares`, and each declares **both** numbers. The mixer bay's
staleness is the roll's 30 Hz while a request the engine has not granted is outstanding or a fade
has not run
([ADR-0190](adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
[ADR-0206](adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)), and nothing at
all while the bay that draws it is folded away
([ADR-0193](adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)).
The transport row's is the beat's `view::BEAT_STALENESS`, about forty a second, declared for as
long as the grid is drawn and **without asking whether anything is pending**, which is P-0077's
forced clause
([ADR-0212](adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)).
The cost is `budget::PANEL_PASS`, **1.26 ms** measured over five runs on 2026-08-28 and written down
rather than sampled, one constant every region declares because immediate mode cannot redraw one
region of a panel
([ADR-0210](adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)).
**Both schedulability conditions are asserted** in
`crates/karakuri-console/tests/schedulable.rs`, which is where the two figures live rather than
here: with two terms `Σ (cost / staleness)` is 0.0889 against 1.0, and `max(cost)` is still one
whole pass because both regions declare one. What is left is the arbitration, and what is owed with
it is a **panel's share of the frame budget** — nothing writes one down, so the conditions are
asserted against the whole frame and hold in their most permissive form.

**Half of *making the picture a sink in fact* is built, and it is the half that had to come
first.** `frame::compose` no longer gates the frame on a sink: it takes a slice of them, asks each
one, advances the deck whatever they answer, and draws into and presents the ones that took it
([ADR-0171](adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md)). Every
output may be off and the instrument keeps running, which is what the outputs row needs and what
the manual already promises.

**And the console can reach it now.** The frame loop was in `karakuri-cli`, which has no library
target, so the application could not call the one loop and `karakuri-console`'s example hand-rolled
a second — the two-loops-that-drift failure `frame.rs` was written to end, one crate over. `Sink`,
`Skip`, `Committed`, `Outcome`, `compose`, `Look` and `WindowSink` are `karakuri-engine`'s now;
`PngSink`, the clock and the recorder stayed with the program that owns them. No crate gained a
dependency, and the workspace count did not move, which is what a move should do.

**And the console uses it.** `crates/karakuri`'s `main.rs` has no frame loop of its own: the picture and deck
A's cell are two `Sink`s in a slice, `compose` advances the deck and draws into the ones that took
the frame, and the panel's whole `egui` pass goes in `finally` — the caller's own work in the
frame's encoder, which is what ADR-0166's one submission requires and what
[ADR-0173](adr/0173-a-frames-submission-is-not-only-its-sinks.md) argues is not a sink. Folding a
region away makes its sink refuse, so *hidden and still costing a pass* is answered by the type
rather than by an `if`.

**The sizing hole is closed, and it was two derivations agreeing by luck.** `resumed` computed the
first texture sizes one way and `window_event` computed them again another, with nothing tying them
together and neither reachable from a test. There is one `aims(layout)` now. The injection this
line used to record — size deck A's texture from the picture's rectangle — was caught by **nothing**
before and fails **three** tests now.

**The picture is the canvas's shape now**
([ADR-0181](adr/0181-the-picture-is-the-canvass-shape-and-the-leftover-is-the-consoles.md)), which
is [ADR-0170](adr/0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)'s rule
applied where it was first refused: a deck preview cell had been forbidden to fill its box and
letterbox inside it, and the picture was one level up doing exactly that. It filled its region and
the bars were **inside the texture** — at a 1920-wide window **66.6% of the picture's texels were
black**, cleared, drawn into, uploaded and sampled sixty times a second; 84% at 3440. They are the
console's ground now, and a widening window remakes no texture where every frame of a horizontal
drag used to rebuild and re-register one on the render thread.

Building it found that **the Program bay's height was a rounded 16:9 from the day it was derived**:
`.program-view` at 466 wide is 262.125 tall and the arrangement transcribed 262, so two sentences
claiming *"exactly 466 x 262, which is 16:9"* were never true — and the test below one of them
already knew, checking that ratio 200 times looser than every other assertion in its file.

**And the Program bay rearranges itself.** With a wide, short bay the four deck previews leave the
row under the picture and go into two columns beside it — A and B down the left, C and D down the
right — because that is the placement that gives the larger picture
([ADR-0182](adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md) decides,
[ADR-0184](adr/0184-the-program-bay-rearranges-itself-and-a-still-frame-does-not.md) draws). At a
1920 window the picture is **1.61x** what it was. The crossover is a **curve in width and height**
rather than a width: a bay dragged to its own minimum goes beside at every width, and one dragged
tall comes back. One flip along each axis, never a flicker, asserted by sweep rather than by
threshold.

It cost `karakuri-layout` one bit
([ADR-0183](adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md)): a node
is out of the layout for **two** reasons — the operator folded it, or whoever is drawing has put it
somewhere else — and sharing one flag would have made an unfold put an empty strip back and saved a
fold nobody made. **A still frame still does nothing**: the write is idempotent, the decision is a
dozen divisions, and the repaint answer is `Never` unless the placement moved. The frame it moves
costs two solves and that is written down rather than hidden.

**The Outputs row is built, and it is the console's first clickable control.** One sink —
`program view` — and nothing else, because the projector window, the plugin sinks and `+ add
output` do not exist and a row of dead names is scaffolding that looks finished. The dot is lit
when `layout.visible("program-view")` is, stored nowhere (ADR-0161's test run the other way: this
*can* be derived, so it is).

**It cost the operation vocabulary its pointer**, which is what M5 said it would open with.
`Op::Fold` meant *fold the region under the pointer*; the operations that act on a node now carry
the node ([ADR-0175](adr/0175-an-operation-carries-what-it-acts-on.md)), and *under the pointer* is
one way of naming which region rather than part of what folding is. `Op::Unfold` is new and means
**make this region visible** — the node, its collapsed ancestors, and a solo that is hiding it —
because a dot that expanded the picture inside a still-folded bay would light nothing and say
nothing. **A press on a lit dot darkens it and a press on a dark dot lights it, always**, and that
is a test rather than a sentence.

**And `input`'s standing warning came due.** Its documentation has said since it was written that
the claim rule holds *only while the gaps stay empty*, and that the first control drawn near a
bay's edge would sit under a boundary's grab. It clears by 1.75 pixels, measured rather than
assumed, and that is now a test that fails if the control moves, the row shortens or `GRAB` widens
([ADR-0176](adr/0176-a-control-the-console-draws-is-the-panels.md)).

**The transport row is drawn**, and it shows the four things the console can actually know: the
BPM, the beat grid, the bar number, and the frame readout against the display's refresh interval
([ADR-0177](adr/0177-the-transport-row-shows-what-the-console-can-know.md)). `audio-in`, `tap`,
`learn`, `map`, `landed` and `● rec` are named in the source with what is missing behind each and
drawn nowhere. The grid is **as many dots as a bar has beats** rather than four, because
`karakuri-signal` marks its own `BEATS_PER_BAR` provisional until the format carries a time
signature, and a transcribed 4 would go on saying four the day that changes. **What it draws is a
light that travels those dots** rather than one of them lighting at a time, and the mock's own
picture is a frame of that travel
([ADR-0212](adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)).

**And the row is no longer four readouts and nothing to press**: the **arrangement pill** is drawn
in it — `save`, `load` as the list of names already filed, and `start a new one`, which is the
reset — and it is the first control here whose gesture outlives the press that opened it
([ADR-0225](adr/0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)). It is
also the first surface that takes letters: `crates/karakuri-console` reads no key events, so the
program fills the buffer a character at a time, and the wall on what may be typed is where the file
is written rather than where it is typed. What it cost is a fourth clause in the pointer rule —
while the menu is down every point of the console is the panel's, ahead of a boundary's first
refusal, because the card is drawn out of a 48-tall row and across the bays under it.

**The Mixer bay is drawn**, from the deck's own state — a strip per slot with its name, its
residency, its gain trim, its opacity fader, its meter and its blend and mask marks
([ADR-0178](adr/0178-the-mixer-draws-four-tracks-and-as-many-strips-as-the-deck-has.md)). **Four
tracks and as many strips as the deck has**, settled from the arrangement rather than from taste:
`right-pane`'s minimum is written as *four mixer strips still side by side*, so tracks that
followed the strip count would re-derive that minimum on every install. A track nothing fills draws
nothing — an empty strip is a reading, and inventing one is what the preview cells and the
transport row both refuse.

**Its two faders are played with a pointer**
([ADR-0185](adr/0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md)), and they
are **`karakuri-operation`'s first customer** — nothing had aimed at the vocabulary until now. A
fader turns a drag into an `Operation` and applies nothing, because `src/` has no engine; the
harness turns it into a `Record` and the record moves the deck, which is P-0028 and is what makes a
console fader the same thing as a key press and a MIDI knob. **The strip draws what the deck says**,
asserted mid-gesture, so nothing keeps a second copy of the mix.

The vocabulary survived contact and four things about it are now known rather than assumed:
`Operation` is not `Copy`, so its cheapest variants pay for its heaviest; `deck: u8` costs a cast
per gesture; **there was no `SetMask`**, so the mask mini was a readout of state no operation could
set — the mask has two rows now, a shape and a position
([ADR-0201](adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)), and the mini
is the control that presses the first of them
([ADR-0203](adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)); and the trim reaches
only the bottom half of what `SetGain` expresses, because it is drawn over `[0, 1]` while the mix is
HDR.

**The blend mini is the third control and it cycles**
([ADR-0187](adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)). A
press moves the deck to the next of three and emits `SetBlendMode` naming the **destination** it
arrived at, down the same path a fader's operation takes. There was never a conflict to resolve
here: the line above used to say the mock and P-0074 disagree — *click to cycle* against a
vocabulary with no cycles — and **the premise was false**. P-0074 forbids a cycle *operation* and
names *"a mini that cycles the blend"* as one control emitting three. What the affordance does force
is a decision about MIDI, and ADR-0187 is that decision: the component owns three operations, the
pointer is shown a cycle, and a map is offered the three values rather than a *next*. `Strip::blend`
became a `BlendMode` to carry it, so a fourth engine mode with no operation variant fails at the
harness rather than drawing a word no control can reach.

**The tally reads two values, and says so.** `Deck::residency` is what the slot is doing and
`Deck::requested_residency` is what it was asked to do, and the chip draws the effective one and
**rolls it part of the way toward the request
and back, once a second, while the two disagree**
([ADR-0190](adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)).
**And it can be seen by running the window, which it could not when that record landed**: the
example had one slot and no governor, so a `Deck` there granted every residency it was asked for and
the two halves of the pair could never disagree. It builds a second slot now, asks for it to be
primed against a compute budget computed from what the probe measured, and `Deck::govern` parks it
([ADR-0191](adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md))
— the parked chip on that panel is the governor's verdict read back off the deck rather than a state
the harness wrote.
**And it is the fourth control** — a press on it emits `SetResidency`, one operation naming one of
three since [ADR-0186](adr/0186-one-operation-names-one-of-three-residencies.md), taken from the
residency that was **requested** rather than from the one the chip shows
([ADR-0195](adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md)).
That is what makes a press on a *parked* chip the withdrawal of its own prime request with no case
in the code for it — the affordance P-0076 permits a surface, where cycling from the readout would
put the deck on air.

**And the mask mini is the fifth, which finishes the strip: nothing in it is a readout that a
pointer might have expected to answer.** A press cycles the shape — none, linear, radial, the
engine's own order, *because a cycle should start where a slot starts* — and emits `SetMaskShape`
naming where it arrived
([ADR-0203](adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)). **It is the first
control that carries a value it does not draw.** The row is a shape *and an angle*, and the chip
names only the shape, so the press hands back the angle the slot is already wearing —
`Strip::mask_angle`, read to build the operation and painted nowhere. A default there would make
choosing a shape straighten a diagonal wipe, invisibly, since the mark is the same mark at any
angle. The position and the soft edge are not on this surface at all: `Record::Mask` is written
whole, and the half the operation does not ask for is read off the running mask where the record is
written. **What the press does cost is the move**: a record is a state rather than an intention, so
a shape chosen mid-wipe stops the scheduled move and leaves the front where it had got to — the
honest limit
[P-0078](principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md) already
states about the record stream, now met by the surface it was waiting for.

**And the two faders say where a scheduled move is taking them**, which is the same rule as the
tally's roll on a control whose idiom is *position* rather than a word
([ADR-0206](adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)). A transition is
armed on the beat grid, so with the default quantum a fade is due up to a bar after the key that
asked for it — invisible on the panel until now, and named only in `karakuri-cli`'s status line,
which is the log P-0075 says a destination may not be delivered from. The knob and the fill go on
saying where the control is, a hairline mark says where it is going, and the fill keeps setting off
toward the mark and falling back, never covering the gap. `view::Strip` carries `gain_to` and
`opacity_to` — `Deck::transitions_on` read for a surface instead of a status line — and derives
whether anything is pending every frame, as the tally does. **Neither mark is a control**, which is
the tally's order kept: a readout of a pending state comes before any press that could arm one.
**And the third control a transition can move has nowhere to say it**: an armed wipe moves
`Control::MaskPosition`, the strip has no control and no readout for that number, so it is drawn
nowhere and the gap is named in the record rather than left to be found.

**Four things move on this panel and two of them are live regions that declare**, which is
worth separating because this line used to run the four together. The picture is expensive and moves
every frame — and it is the engine's output, *"already accounted for by the governor"*, so P-0072
keeps it off this budget and off any schedule. **The beat grid is the second region and it declares
now**: it is a light that travels the grid once a bar rather than one dot lighting at a time, it
asks for its own frames at 24.67 ms — about forty a second — and it does so **whether or not
anything is pending**, which is P-0077's forced clause held by something rather than by accident
([ADR-0212](adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)). The mixer's
six readouts change when a hand changes them, and *what the operator does costs what it costs*. What is
left, and the whole of what declares, is *whatever is pending*: expensive, 30 Hz, and running **only
while a request is outstanding and the bay that draws it is laid out**, which can be the length of a
set. It is one live region with **three presentations**: the tally's roll, and a reach on each of a
strip's two faders while a transition is armed on it
([ADR-0206](adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)). They share a
period, a curve and a staleness, so a parked slot and eight armed fades are one deadline rather than
nine — which is P-0075's *everything pending moves together* paying for itself. It was the first region to declare and it is now one of two, it declares a **cost** as well as a
staleness
([ADR-0210](adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)),
and it declares nothing while it is folded away — a region that is not laid out declares nothing
rather than declaring and being dropped by a scheduler that does not exist
([ADR-0193](adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)).
`View::declares` is the declaration and `repaint::Change::Animating` turns its staleness into a
deadline. **There is still no scheduler**, and until a second region declares there is nothing for
one to choose between.

**Owed, and found by building this: `Layout::soloed()` can lie.** The solve never reads it and
`check_structure` only checks that it addresses a node, so a solo's exclusivity lives entirely in
collapsed flags any later `expand` may contradict — and the next `unsolo` restores its snapshot and
throws the expand away with nothing said. Nothing in the console reaches that state today. What
`expand` under a solo should do is a decision nobody has taken.

### Where this goes next, and the decisions it is waiting on

**This is the handover. Read it before anything else in this file if you are picking the work
up.** It says, in the order somebody arriving needs them: *what state the milestone is in*, *what
the next piece of work is*, and *what every remaining piece is waiting on*. Everything after the
third heading is the detail behind the third question; nothing in it needs to be read before
starting.

#### 1. What state the milestone is in

**The panel is a program.** `cargo run -p karakuri` opens the console over a real deck — the
picture, the deck previews, the transport, the mixer, the Library bay and the Inspector — and it
is [`crates/karakuri`](../crates/karakuri), a binary of its own since
[ADR-0214](adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md). What
it needs that is not a surface is
[`karakuri-environment`](../crates/karakuri-environment)'s
([ADR-0215](adr/0215-the-package-is-karakuri-environment-and-a-module-belongs-if-what-it-deals-with-is-outside-this-process.md)),
and `crates/karakuri-cli/src/` is `main.rs` alone. **The console being an example, and the CLI
holding a program nothing could depend on, are both history** — the two sentences that blocked
four bays for most of this milestone.

**Six of the console's nine regions draw a body**: the transport row, the Program bay's picture
and preview cells, the Mixer bay's strips, the Library bay's listing, the Outputs row, and the
Inspector. Of the other three, **Staging** and **Master** draw a body now and only the
**Sequencer** draws nothing but its head.

**Eight controls answer a pointer** — the Outputs dot, the mixer's two faders, its blend chip, its
tally chip, its mask mini, and the transport row's arrangement pill, tone-map capsule and exposure
track — and **six operations are
reached by a key at the panel**: `f` and `g` fold, `z` unfolds, `s` and `u` solo, `r` resets the
arrangement, `esc` quits. The mixer strip is finished as a set of controls; nothing in it is a
readout a pointer might have expected to answer.

**The seventh is the first control whose gesture outlives the press that opened it**, and it cost
the pointer rule a fourth clause
([ADR-0225](adr/0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)): while
its menu is down every point of the console is the panel's, ahead of a boundary's first refusal,
because the card is drawn out of a 48-tall row and across the bays under it and no clearance
arithmetic can be done for that — the card's first row spans 40.25 to 62.75 and the boundary under
that row grabs 42 to 54, so twelve of its twenty-two and a half pixels would be dead. **The same
record settles that the menu is flat and the list of names is the load**, a submenu having lost to
the console page's own sentence. It is also **the first surface here that takes letters** — `crates/karakuri-console` reads no key events, so the program fills the
buffer a character at a time and the console draws it. The wall on what may be typed is where the
file is written and not where it is typed, which is
[P-0076](principles/0076-a-surface-owns-the-affordance-never-the-authority.md) on a path.

**How far the milestone has got is not written down anywhere in prose, on purpose.** The meter is
the panel column of [every operation](manual/operations.html)
([ADR-0213](adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md))
and the key column is the instrument's keyboard rather than the command line's
([ADR-0220](adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md));
both are read with the first of the three commands under *The meter is one column*, below. Run it
before believing any sentence in this file about how much is left.

#### 2. What the next piece of work is

**The deck head's *Set a deck's sync mode* and *Scrub a deck a quarter beat*.** Three reasons, and
the first of them is an instrument this file did not have this morning:

- **Something already performs both.** The fourth command below asks, of every operation, whether
  anything in this workspace constructs it outside the vocabulary and the record crate. `SetSync`
  and `ScrubDeck` are constructed in three places each. That matters more than it sounds, because
  **a `plan` badge does not distinguish a drawing that is owed from a machine that is missing** —
  and fourteen of the rows still wearing one are the second kind.
- **The home is already drawn.** The deck head is a row of the Inspector's pane and has been since
  the Inspector landed — `size::DECK_HEAD_H`, the clock and the anchor. What it has no control for
  is the two things the anchor is *about*.
- **The design is taken.** [ADR-0218](adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)
  settles that re-anchoring is `SetSync` naming the mode the deck is in, and why a cycle
  structurally cannot ask for it. `ScrubDeck`'s reading is settled in
  `karakuri-operation-record`, and the beats-and-signed spelling is the deck head's own.

**The third row of that group is not this.** *Composite a deck's renderers* has `CLI has` and looks
like the same work, and the command below puts `SetCompositing` in the list of operations nothing
constructs. It is the load row's category, not the sync row's.

**What the board says and what it means are two questions now.** The second command groups what is
left by where its control lives; the fourth says which of those are waiting on a machine. Run both
before picking anything up — the *Load material into a deck* row was taken up as drawing work on
2026-08-30 and is not.

**Do not start on Master or Sequencer.** Both are blocked on machinery this workspace does not hold.
**Master's engine half is no longer part of that** — the level and its multiply landed with
ADR-0224; what the bay waits on is the chain and a route to the value.

#### 3. What each remaining piece is waiting on

Everything from here down answers that, and none of it is ordered as work:

- **The meter and the board** — the three commands that replace every figure this section used to
  transcribe, immediately below.
- **The application's own boundary** — *What no list here has ever carried is the application*,
  which is now a record of a move that happened rather than a thing to do.
- **The four items** under *The order that makes each next thing cheaper than it would be alone*,
  of which the first and third are closed and the second and fourth are live.
- **The bays, one by one** — *The remaining bays, surveyed*.
- **The decisions nobody has taken**, each with what it blocks.

**One thing is owed by the mixer and is nobody's next task**: `Control::MaskPosition` is the third
control a transition can move, the strip has no control and no readout for that number, so **an
armed wipe is the one scheduled move the console cannot draw** — an under-draw named in
[ADR-0206](adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md) rather than
discovered later, and closed by whatever surface gives the mask's front a position.

**And the estimate.** The list of seven *Adds* at the end of *M5 — Interface* is the second half
of this milestone and reading it as the whole of it is what makes the progress invisible: the
floor under it —
[what this milestone has delivered](#what-this-milestone-has-delivered-and-why-the-list-below-stopped-counting-it)
— is on no list, because the lists were written before anybody knew what that floor was made of.
**The standing estimate is ~10–14 weeks and it does not carry the application**, which is
[said twice below](#what-no-list-here-has-ever-carried-is-the-application) and is the reason no
new range is written here.

#### The meter is one column, and the board is derived from the page

**The milestone's progress meter is the panel column of [every operation](manual/operations.html)**,
where a `has` badge means an operator running the instrument reaches that operation
([ADR-0213](adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)). **No figure is written here either**, for the reason the section
below gives: the sentence that carried this one went stale three times in three days, twice by
somebody transcribing it and once by a commit that moved it. The first command below is the meter,
and it has been off zero since 2026-08-29.

**No count is written down in this section**, for the reason the vocabulary's is not written down
either. Three commands are the whole of the instrumentation:

```sh
# the meter — how far each surface has got
for col in panel key MIDI MCP CLI; do
  echo -n "$col: "
  grep -o "class=\"rt [a-z]*\">$col " docs/manual/operations.html |
    sed 's/.*rt //;s/">.*//' | sort | uniq -c | tr '\n' ' '; echo
done

# the board — what is left on the panel, grouped by where the control lives
grep -o 'rt plan">panel <b>[^<]*' docs/manual/operations.html | sed 's/.*<b>//' |
  sort | uniq -c | sort -rn

# the vocabulary — how many operations there are
grep -c '<h3' docs/manual/operations.html

# drawing or machine — the operations nothing in this workspace constructs
sed -n '/^operations! {/,/^}$/p' crates/karakuri-operation/src/lib.rs |
  grep -oE '^    [A-Z][A-Za-z]+( \{[^}]*\})? => "[^"]+"' |
  sed -E 's/^ *//; s/ *(\{[^}]*\})? *=> "/|/; s/"$//' |
  while IFS='|' read -r v t; do
    n=$(grep -rn --include='*.rs' "Operation::$v" crates/ |
        grep -vE 'crates/karakuri-operation(-record)?/' |
        sed -E 's#^[^:]*:[0-9]+:##; s#//.*##' |
        grep -c "Operation::$v")
    [ "$n" -eq 0 ] && echo "$t"
  done
```

**The fourth is the newest and it answers a question the badges cannot.** A `plan` badge says a
surface is meant to reach an operation and does not yet; it says nothing about *why*, and the two
whys are not the same work. An operation nothing constructs is one nobody can route to at all —
`Operation::LoadSet` writes no record, so whichever surface names it must perform it, and
`Deck::install` is documented as deliberately unreachable from a key or a surface. That row was
picked up as drawing work and is not, which is what this command exists to stop.

**Two cuts make it honest and both are the ones `panel_column.rs` already makes**: the record crate
is excluded because it classifies every operation and would answer *constructed* for all of them,
and each line is truncated at its first `//` because a doc comment naming an operation is not a call
site — without that cut `LoadSet` reads as constructed, by the very comment that says it is not.

**Nine of what it returns are expected and are not machinery**: the console's own shape reaches its
rows through `karakuri_console::panel::Op` rather than through `Operation`, and
`crates/karakuri-console/tests/vocabulary.rs` is what checks that section, both ways, on a running
panel.

**The second of the three is the board, and it is the page's rather than this file's.** Every `plan`
badge names where its control lives, so what is left arrives already grouped by region and already
ordered by weight, and there is no list here for anyone to keep in step. What each group is standing
on is written region by region under *The remaining bays, surveyed* below and on
[the console page](manual/console.html); this section does not repeat any of it.

**Running it found a gap in the prose survey on the first day, which is the argument for it.** The
`deck head` row reads **three**, and the *Sync mode, anchor and offset* item below named two — the
third is *Composite a deck's renderers*, which routes to the same home the manual does not have. A
`grep` caught what a survey written region by region against the manual had read past twice.

**The panel's own measuring instrument ships, and that is decided rather than inherited**
([ADR-0217](adr/0217-the-counting-allocator-ships-because-a-written-number-nothing-checks-goes-stale.md)). `crates/karakuri` installs a counting `#[global_allocator]` over the
whole process so the reading can be held against `WRITTEN_ALLOCS`, and the argument is that file's
own history rather than a preference: the previous number went 184 → 456 → 525 as the mixer bay and
the parked deck landed and **stayed wrong for two commits, "because nothing was checking it"**. A
cargo feature lost on [P-0012](principles/0012-a-measurement-carries-how-it-was-taken.md) — a number
taken in a configuration nobody ships is about a different program — and a test target lost because
the declared figures are medians of a running instrument with a deck under load, not of a fixture.
What it costs a player is a thread-local increment per allocation, and **that cost is stated rather
than estimated**: the claim that it is negligible is not made.

**The manual has caught up, and writing it found more than it closed** ([console.html](manual/console.html),
2026-08-29). Three homes were reported missing and **one of the three was already drawn**: the
crossfader is `.xfade`'s first row, which `karakuri-console`'s own arithmetic already counts as 61
of the mixer's 316 — it had no name and no tooltip rather than no markup, so it gained prose and a
tooltip and no second control. The **deck head** and the **latency offset** were genuinely absent and
now exist: the deck head under each `.half-head` in the **inspector** rather than on a preview cell,
because a `free | tempo | beat` triplet is about 106px against a preview cell's 112 and rule 05
already says everything about what a deck *is* lives there; the offset beside the `audio-in` pill
rather than beside the tempo, because the operation refuses without an audio input and belongs
against the thing that says whether there is one.

**The fourth home was missing and is written now**: *Halve or double the grid* routed to
`panel ½ ×2` and the transport row drew no such control — verified three ways before a word was
added, including against `view.rs`'s own enumeration of the row, because the crossfader had just
proved that a word absent from the page does not mean a control absent from it. **Both of its
refusals are predictable rather than reportable** — `BeatLock::octave` refuses outside
`BPM_RANGE`'s 60..=200 and everything refuses without an audio input — so the page draws `×2` inert
with its reason at 128 BPM rather than drawing a control that fails when pressed.

**The wrap alarm raised with it was false, and the correction is a lesson about method.** The row
needed 922.6 of 966 px before the offset pill, not the ~898 that was reported, and had 43.4 px
spare; the pill is 89.5 and not ~104. The second measurement got there by downloading the two
webfonts the page links and reading `hmtx` for real advance widths rather than estimating from the
stylesheet — which is the same move ADR-0190 made for the tally, one document over.

**And it found a real defect that predates all of it: the row's fit rests entirely on the webfont.**
With `M PLUS 1 Code` not loaded the fallback needs about 1046 px and the row wraps at every width the
console can be — and it already did before any of this work, at about 1022. `font-display: swap`
makes that the **first-paint** state, so the row a reader sees first is the wrapped one. Nothing has
been decided about it.

**Two more the drawing found.** `½` needs 120 BPM or more and `×2` needs 100 or less, and
`BPM_RANGE` is 1.74 octaves wide, so **between 100 and 120 neither direction is available** — stated
on the page rather than hidden. And the tracker's own doubt has no home: `Estimate::half_tempo_hint`
prints ` x2?` on the CLI's status line and is the thing that tells a performer to press `.` at all,
while the panel draws the control and not the prompt — and above 100 BPM that hint arrives together
with a refusal of the only move that would answer it. **And the sweep's method was wrong in a
way worth writing down** — it tested for the home's *words* on the page, and `transition row`,
`renderer chips`, `sensitivity row`, `tally chip`, `strip fader`, `health readout` and `pane edge`
all appear zero times there and are all drawn. A word test is not a region test; the reliable one is
reading the mock.

**Two numbers the manual moved, and only one of them is settled.** `karakuri-console`'s
inspector minimum **is 151.5 now**, taken there by the deck head the manual specified — and the
guard that recomputes the tree's implied minimum caught the console's claimed smallest window
having been 9.5 px short from the moment that row was specified. The transport row's `fixed(48.0)` is **at risk and was not settled**: with the
offset pill added the mock's row is about 898px of a 966px inner width at the console's 1010px
minimum, so it wraps there and the 48 stops being the height of its contents — but the built console
draws fewer of that row's items than the mock holds, so it has slack the mock does not. **How many
fewer is not written here and is not written in the source either**: `view.rs` deleted its own *six
of the mock's ten* on the third time a figure about that page went stale, and what the mock holds is
`docs/manual/console.html`'s `.transport` block, which is a count somebody takes rather than
remembers. The arrangement pill has since been added to both sides — the mock's `.transport` gained
it and the console draws it — so both halves of that comparison moved on 2026-08-30, which is the
fourth time a number nobody was recounting went out from under this sentence. This wants ADR-0190's
treatment, which is to measure through `egui`'s own text layout rather than to argue from the
stylesheet.

**Five things the drawing found that look designed wrong were adjudicated on 2026-08-29, and two of
the five were not faults.** The tempo-sync refusal is arithmetically right rather than
over-cautious: `beats` is the grid evaluated at the material's own `t`, and under tempo-sync `t`
itself advances at the tempo ratio, so the pair runs beats-driven motion at the *square* of it —
`deck.rs` states exactly that, and
[P-0027](principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md) is what makes a refusal
the right answer rather than a quiet picture. And the crossfader's mark is right as a readout: it is
one number derived from two recorded ones, and a draggable mark would have to invert a projection
that is not invertible, inventing the split between two `SetOpacity` records by a law no record
names ([P-0028](principles/0028-every-control-ends-in-the-same-record.md)). That answers a tension
[ADR-0180](adr/0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md) recorded and left
open — *"one of those needs a different name or a different control"* — and it got a different
control.

**Three were faults and are taken.** Re-anchoring is `SetSync` naming the mode the deck is already
in, and the fault is in the cycling affordances rather than in the vocabulary
([ADR-0218](adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)) — a `ReAnchor` variant lost to
[P-0074](principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md), which
predicted this shape by name. The crossfader spans the selection and the one after it, so the panel
and the `x` key are one gesture ([ADR-0219](adr/0219-the-crossfader-spans-the-selection-and-the-one-after-it.md)); fixed `A…B` and assignable ends both lost. And
**the beat anchor's `±offset` is renamed the scrub** — it already had that name and was not using
it, since its control is *Scrub*, its operation is `ScrubDeck`, and `Transport`'s own field
documentation calls it *"the operator's scrub"*; the latency offset keeps the word because it is the
flag-facing one and, once the scrub is the scrub, the only offset left.

**One was neither**: `SetCompositing`'s row prose describes the flag rather than the operation,
which is what its `launch` badge means, and the sentence is only misreadable. It owes a clause
saying the operation names both directions and that a stream record for per-slot layering does not
yet exist.

**Each one was a control whose sentence could not be written cleanly, which is the manual's stated
reason for existing.** **Re-anchoring has no route
on any surface**: `Transport::engage` says re-engaging the mode a slot is already in is how an
operator re-anchors, and `cycle_sync` always moves to the next *allowed* mode, so neither the `y` key
nor a cycling chip can ask for the mode it is in. **The crossfader's ends were unspecified** — the
operation names both decks and the mock hardcodes `A … B`, with nothing saying which two a
four-deck mixer's fader spans; the page now resolves it to the selection and the one after it, so
the panel and `x` are one gesture. **The crossfader's knob has no record for a half-done throw**, so
it is written as a readout of the two channel faders plus a throw rather than as a drag. **`offset`
names two different things** — the latency offset and a beat anchor's `±offset` — and both now carry
their unit, but read strictly [P-0031](principles/0031-a-name-means-one-thing-across-the-system.md)
wants one of them renamed, which is `operations.html`'s row to change. And **`SetCompositing` is a
`launch` row whose prose says the flag can only turn it on**, while the vocabulary carries a `bool`
and the page now draws a chip that can be clicked off.

#### What no list here has ever carried is the application

**This milestone opened *Goal: the application*, and nothing in it named building one.** Not the
seven *Adds*, not the four items below, not the decisions after them. **That is closed, and what
follows is the record of closing it rather than work outstanding** — the moves collapsed to what a
reader needs in order not to re-ask them, what is still owed kept in full. The four surface items
below are in the order that makes each next thing cheaper than it would be alone.

##### The program moved out of the CLI, and the argument is in the records

**Taken and landed, 2026-08-28 to 2026-08-29.** `karakuri-cli` was **27,517 lines with no library
target** and the panel was hosted by a **6,276-line example**, so neither could reach the other, and
that boundary had already been paid for five times before it was drawn. Both measurements and all
five payments are in
[ADR-0214](adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md), which
decides that **the program moves out of the CLI and two thin binaries sit over a shared package** —
[README.md](../README.md)'s scaffolding and the destination — with `karakuri-console/src/` keeping
the device-free seam [contributing.md](contributing.md) §3 names. The figures are of a shape that no
longer exists and are left as the measurement they were.

**The package is [`karakuri-environment`](../crates/karakuri-environment), and a module belongs in it
if what it deals with lives outside this process — a disk, a device, a port, a socket, another
process — or is the record of what happened**
([ADR-0215](adr/0215-the-package-is-karakuri-environment-and-a-module-belongs-if-what-it-deals-with-is-outside-this-process.md)).
**All thirteen modules pass that test and all thirteen have landed.** That crate's `lib.rs` is the
charter — the test, the order the slices moved in, the five items that are no module's (two of them
refusal sentences
[P-0061](principles/0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md) puts
where both surfaces can reach them), and the rule two binaries impose: **no module here may know
which surface it is under**. **`crates/karakuri-cli/src/` is `main.rs` and nothing else**, and **no
line count is written here**, because it moves with every ordinary edit.

##### Measuring the seam took three commands, and two intra-doc links had been broken all along

**Each measurement found what the one before it could not see**, which is why the recipe is kept:
code lines gave sixteen items, **intra-doc links** four more that compile after a move and rot in
silence, **brace-grouped imports** a third class both had missed.

```sh
sed 's|//.*||' <file> | grep -o 'crate::[A-Za-z_][A-Za-z0-9_]*' | sort -u   # code
grep -o 'crate::[A-Za-z_][A-Za-z0-9_]*' <file> | sort -u                    # and doc links
grep -o 'crate::{[^}]*}' <file> | sort -u                                   # and brace groups
```

**Two intra-doc links in the moved code had been broken all along**: a binary crate gets no rustdoc
run, so a trait method named as an inherent one failed nothing until the code was in a library.
**That is a second thing the package boundary buys, beside the five ADR-0214 counted.**

##### Three things the move left visible rather than fixed, and one the workspace run caught

**Each is written down because a consequence recorded nowhere is one the next person meets rather
than reads.** **`layer_named` exists twice in one library now** — `setfile.rs`'s and `mcp.rs`'s
case-folding private one — and merging them is a redesign rather than a boundary move
([P-0031](principles/0031-a-name-means-one-thing-across-the-system.md) is the question to answer).
**A test in `main.rs` now checks two `setfile` functions**: it still guards what it guarded, and its
home is one crate over. And **the `262144` the example transcribed is gone.**

**What the whole-workspace run caught that four rounds of `--skip gpu::` could not**: the convention
guard `gpu_binaries_still_take_a_device` asserts a floor on how many source files
`crates/karakuri-cli/src` holds, and that directory now holds one. **It is not a GPU test**, so it
was filtered out of nothing and simply absent from every gate an agent ran. This is what
[contributing.md](contributing.md) §5's boundary run is for.

##### The estimate does not carry this, and no new range is written

**The ~10–14 week estimate does not carry this, which is the second time.** *What the range assumes*
named four undrawn bays, three decisions and the manual's missing deck head, and not a program to
draw any of them in — exactly as the original ~8–10 was taken against a list that assumed a panel, a
name and a frame that did not exist. **No new range is written here.** The board makes the panel half
countable for the first time and ADR-0214 settles the shape of the other half without settling its
boundary, so a range taken now would be a third estimate of a list still missing a piece. It is
re-taken once that boundary is drawn.

##### The mixer strip is finished, and what is left of it is another surface's

**Done, and off the list rather than owed.** The blend mini (ADR-0187), the tally
([ADR-0195](adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md)) and
the mask mini ([ADR-0203](adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)) are
drawn, each on a claim rule and an `Operation` and no new machinery. **What the mask cost was a
field**: the row carries a shape **and an angle** and the chip names only the shape, so the strip
carries the angle it is wearing to hand back with the press. **The one thing it leaves owed is the
wipe**: a record is a state rather than an intention, so a shape chosen while a move is running stops
the move — the limit
[P-0078](principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md) named before
there was a surface to meet it, now met. **The MIDI map's two targets are now one** — `cc ->
mask-position N` is a line
([ADR-0202](adr/0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md)) — and the
shape is not waiting on work: an **angle has no spelling in that grammar at all**, so the row keeps
its empty MIDI badge with the reason on the page, until **a control that *shows* an angle** reopens it
([ADR-0209](adr/0209-the-masks-shape-keeps-its-empty-midi-badge-until-a-control-shows-an-angle.md)).

##### The scheduler's inputs are built, and what is left is the policy

**Both of P-0072's numbers are declared and both schedulability conditions are asserted**
([ADR-0210](adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)):
`crates/karakuri-console/tests/schedulable.rs` reads `Σ (cost / staleness)` at **0.0889 against 1.0**
and `max(cost)` at 1.26 ms against 4.17 ms, with nothing over budget. **The beat is the second
declaring region and it landed on 2026-08-28**
([ADR-0212](adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)), declaring
`view::BEAT_STALENESS` **for as long as the grid is drawn and without asking whether anything is
pending**, which is what makes it P-0077's motion where the roll is not.

**What is left is the policy rather than the inputs.** P-0072 states the arbitration in full and
[P-0077](principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md) adds the one
region a budget under pressure may not stop. **Still nothing arbitrates, and the reason has changed**:
it is no longer that there is nothing to choose between, it is that both fit.

**Three things are owed.** The **scheduler** itself. A **panel's share of the frame budget**: the
console writes down only what a *frame* has to fit in, so the first condition is asserted at the whole
frame and cannot catch a panel that fits it while leaving the engine nothing. And a **per-region
cost**: `egui` is immediate mode, so every region honestly declares the same whole pass — a deliberate
over-declaration a scheduler can refine downwards, which stops being needed the day a bay can be drawn
once into a texture and composited after. The argument is not the bytes: two rates **cannot both be
special cases**, and each one written by hand is the first half of the scheduler written badly.

**One hole this records rather than closes**: fold the transport row away and the panel has no
continuous motion at all, which is the operator's doing rather than a scheduler's economy, and is not
what P-0077 forbids. **And one region is not the scheduler's to stop, and it is the beat** — taking
the panel's continuous motion sells the signal that says the frame is in trouble at the moment the
trouble starts (P-0077,
[ADR-0189](adr/0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md); a
preference, since taken by ADR-0212).

##### The other surfaces onto `karakuri-operation`

**All four surfaces have an answer now, and only one of the four was the wholesale move this item
described.** What separates them is what
[`karakuri-operation-record`](../crates/karakuri-operation-record)'s `written` answers: a surface
whose operations write records routes through `Live::operate`, and one whose operations are `Silent`
performs them where the state is, because `operate` would print *no record* and do nothing. The four
routes into [`karakuri-operation`](../crates/karakuri-operation):

- **MIDI moved outright** ([ADR-0196](adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)):
  a map line names a state and an old line is refused with the line to write instead. **What it cost
  is the pad count**, thirty notes where there were eighteen, written into `examples/surface.map`.
  **The one arm left in `run_surface` is `TapBeat`**, `Owed::NotSettled` for want of the beat tracker.
- **The console's `panel::Op` stays, and what blocked it was the page rather than the code**
  ([ADR-0197](adr/0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md),
  [P-0074](principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md)); its
  inventory is a test now so no gap closes by accident. **Two of its three page gaps are taken, and
  both cost something.** The root column and the body row **stay unnamed**
  ([ADR-0204](adr/0204-the-root-and-the-body-row-stay-unnamed-and-a-folded-root-is-not-hit-testable.md),
  which also made a node that is not laid out un-hit-testable, root included —
  [P-0073](principles/0073-a-node-claims-only-what-its-visible-content-can-use.md) and
  [ADR-0193](adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)
  on the pointer axis), so **the console's arrangement operations are the one surface that does not route into the
  vocabulary — permanent rather than pending**; and **`Report` gets no row**
  ([ADR-0205](adr/0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md)), so **the
  console goes on performing an operation the page does not name**. **`Reset` is the one left**: a
  change rather than a question, waiting on a row of its own. **`FoldEnclosing` was the fourth and is
  not a gap** — naming a target by relation is the caller's
  ([ADR-0175](adr/0175-an-operation-carries-what-it-acts-on.md)).
- **The CLI's key handler is surveyed too, and most of it had already moved**
  ([ADR-0198](adr/0198-a-gesture-converts-in-the-parts-that-are-decided.md)): of its thirty-nine keys,
  fifteen already reached `Live::operate`, **nine** name operations that are `Owed::NotSettled` and
  keep their own path with the reason at the function, **twelve** name `Silent` operations and
  **cannot route through `operate` at all**, and **three** (`s`, `h`, `?`) name nothing in the
  vocabulary. What moved is the parts of two gestures, deleting the last three records this program
  derived twice. A test asserts the seven owed operations are still owed, so the day one of those
  conversions lands the failure names the key that is due to move.
- **MCP names its operations and performs them itself**
  ([ADR-0199](adr/0199-mcp-names-its-operations-and-performs-them-itself.md)), because `written`
  answers `Silent` for all six tools and there is no `Live` on a connection thread; what routes is the
  **naming**. **The page needed no edit**, which makes this the first of the four route columns to
  become checkable at all.

##### The remaining bays, surveyed — and the survey is the order

Three of the five draw something now — the Library as a listing, the Staging lane as its empty state,
the Inspector in full — and the other two draw nothing but their heads. **They are not one kind of
item and were counted as one**: the master's level is read by `Deck::out` in every run, so its first
pass is a drawing and nothing else, while the sequencer waits on a pattern no type in any crate holds.
**What each is standing on is [console.html](manual/console.html)'s *What each region is standing
on***, where the argument behind each answer below is written; what decides which comes next is
whether the values behind it exist anywhere in this workspace.

- **Library — built, as a listing**, a row per Set and a foot reading `n of m` off `Store::list_sets`.
  **Six of the mock's controls stay undrawn and each waits on something different**: the *favourite*
  is a fact nothing here writes, so **an operation that sets one is owed**; a folder *scope* waits on
  **an operation that can ask a directory for a listing**, `ListSets { holds, layer }` having nowhere
  to put one; the *filters* have that same operation and no index to answer it; and the foot's pill —
  `load → A` on the page and in `view` alike — is *how a Set gets from the library to a deck*. **That
  last one is blocked on machinery rather than on a control, which a `plan` badge cannot say**: the
  cursor and the deck selection it reads are a selection this crate does not keep, the key is the
  page's to choose, and `Operation::LoadSet` is `Silent(NoRecord)` — so a surface naming it has to
  perform it, and **nothing in this workspace loads a Set into a *running* deck**. `Deck::install` is
  *"deliberately not reachable from a key or a surface"*, and `--set` and `--load-set` are launch
  flags, which is why the row is marked `launch`. **None of that blocks the listing.** The seventh
  omission is [ADR-0200](adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)'s:
  the **time** beside each name had a value and no spelling until `setfile` moved to
  [`karakuri-environment`](../crates/karakuri-environment) on 2026-08-29, and **what it still waits on
  is the program that hosts the panel** — `karakuri-console/src/` may not depend on that crate, so the
  date is formatted by the host and handed in. The program lists `.karakuri` once at startup, which is
  why a Set saved while the window is up does not appear until the next run.
- **Staging — drawn on 2026-08-29, and what it draws is its empty state, which is the only state it
  has.** Two things this entry used to say were wrong: **it needs no operation**, *regeneration of a
  slot* being `WriteProcedure` with a different producer that `mcp.rs` already performs end to end
  ([P-0019](principles/0019-prefer-the-mechanism-that-already-exists.md) a third time); and
  **no engine work is owed at all**, the judging window being public already. What survives is that
  the head's `2 waiting` is not a queue depth, and that nothing reads any of it. **And the reason this program's set was empty has gone**:
  `crates/karakuri` built both slots with `HotSwap::fixed`, whose sender is dropped at
  construction, so no `swap::Event` of any variant was ever emitted. It builds them watched now, and
  a load from the library reaches the same worker a save does — so the lane draws a row for either.

  **The most valuable row is one the mock does not draw**: after `Event::RolledBack` the watchdog puts
  the previous *Set* back on screen and **does not put the previous file back**, while the watcher
  re-reads every file on every rebuild — so the picture is the old version, the disk is the
  over-budget one, and the next unrelated save swaps it in again. **Nothing in the instrument says so,
  and that is what this lane is for.** **Two operations were added** — **`Keep a candidate`** and
  **`Put a node's previous version back`**, *accept* and *reject* being refused because `swap::Event`
  has spent them on the budget's verdict over the same object
  ([P-0031](principles/0031-a-name-means-one-thing-across-the-system.md)) — and they address a node
  rather than a version, so choosing among older ones is `WalkHistory`, still `Undecided` for want of
  exactly that address: **this lane is the surface that row has been waiting for.**

  **Two things the bay still cannot say, and both are records rather than drawings.** `origin` — the
  prompt, the model, the seed — is specified in [ir-spec.md](ir-spec.md) and **produced by nothing**,
  so `you, 14:41` against `agent` cannot be told apart, and the coloured dot goes with it. And **the
  bay's height is pinned to its fullest state while empty is its ordinary state** — `fixed(125.0)` is
  three candidate rows held open over nothing in every run that starts, and it is not expressible:
  `arrangement()` takes no arguments and `karakuri-layout` has no setter for a view's size. *(The
  survey this replaced, whose accounting the work was measured against, is in this file's history.)*
- **Inspector — drawn, 2026-08-29, and the drawing found more than it drew.** All seven of the reads
  it needed answer off a running Set. **The biggest finding is that the manual groups parameters by
  node and the interface that exists names no nodes**: the default interface is entirely wildcards and
  `crates/karakuri` has no `--publish`, so **every control the panel will ever draw is a wildcard** —
  one over exactly one node is drawn as that node's, one over two or more is dropped with a count on
  stdout. **Four page-versus-code disagreements were answered on the page (2026-08-29)**, one of them
  by the engine giving up its alphabetical sort for declaration order, and answering them found **a
  wildcard write crossing authority**: one write moves every node that declares the key, which is
  rule 06's refused switch at the width of a key and was a hole in
  [ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)'s model rather than
  in the page. **And the guard caught a nine-and-a-half pixel lie**, the console's claimed smallest
  window having been short by exactly 9.5 px from the moment the deck head was specified.

  **That hole is closed, and what the bay is now waiting on is not it**
  ([ADR-0223](adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md)). **What
  replaced it is two narrower things, and neither is the Inspector's drawing.** Nothing calls
  `set_authority` outside tests, so every node of every Set is `Manual` and **the row can read while
  nothing can make it change: what is missing is a writer**, a live-session one
  ([ADR-0216](adr/0216-a-node-nobody-has-spoken-for-is-manual-and-a-request-states-only-what-was-said.md)).
  And **the asker is undecided**: whether an *addressed* write by an agent onto a node the operator
  kept is refused cannot be decided until something writes a param on an agent's behalf, and nothing
  does.

  **What is still omitted, each with what it waits on.** Three controls this pass adds none of —
  `showing`'s selection, `keep`'s write into the store, and the scrub's two arrows, which are the one
  control here with no readout at all. Three whose *face* is a reading, so the face is drawn and the
  press is not. **`.param.bound`'s source has no value in this workspace at all**: nothing in
  `crates/karakuri` binds anything, so a bound row is a state the program cannot enter, and the `.sens`
  row waits on the row above it. And a node group past the pane's bottom edge waits on a scroll
  position, which this crate still keeps none of — the panes draw a group **whole or not at all**.
  *(The survey this replaced is in this file's history; of its three blockers, the authority chip's is
  now a writer's, the panes' missing scroll position is above, and the third was `showing`, `keep` and
  `2 up` — the last of them the arena gap `view::outputs` already names as drawn five times.)*
- **Master — still blocked on machinery, and the blocker moved on 2026-08-30.** This entry read *"the
  bay is two things neither of which exists"*. **The first of the two exists now**
  ([ADR-0224](adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md), argued
  in the decision below), and **what the row waits on now is a route, not a value**: no operation, no
  `Record`, no key, no MIDI target and no flag reaches it. **The second of the two is unchanged** — a
  master effect is `console.html`'s own *"giving L5 a writable form"*, which is not built, so the two
  levels are arithmetically indistinguishable in every frame this program can draw, a deliberate cost
  that a test measures. The bay's footnote, *runs in linear HDR, before the one tonemap*, is still
  true and still a sentence rather than a readout.
- **Sequencer — surveyed on 2026-08-29, and what it can draw today is nothing beyond its head**, since
  **nothing in this workspace holds a pattern** — the one region where ADR-0200's rule yields nothing,
  which is the finding rather than a delay. **What the survey overturned is what this entry used to say
  it was waiting for** ([ADR-0222](adr/0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md)):
  a lane is neither a binding nor one more name on `karakuri-signal`'s bus but **a fifth route into the
  vocabulary**, because a binding does not cancel and would be held against the hand sixty times a
  second (P-0078). **That settles two things cheaply** — a step index is `floor(beats ×
  steps_per_beat) mod length` off `Oscillator::beats`, and the lane mute is `TakeParamBack` under
  another name. **What is left is a producer: a step grid and a pattern record, neither of which
  anything in this workspace holds.**

##### Decisions nobody has taken, each blocking something named above

These are questions rather than work, and every one of them was found by building the thing next to
it.

- **Which cut of the previous frame a feedback effect reads, and what holding it costs.** The one
  effect [the console page](manual/console.html) names as an exception when it gives L5 a writable
  form — *"reading the previous frame is a cycle, and wants a decision of its own."* **What that
  page does not say is that the previous frame is not one thing.** It could be a Set's output, the
  raw frame the mix wrote before anything downstream touched it, or the frame as it stands after
  some effect in the chain — so a feedback procedure has to *name which cut it reads*, and the
  vocabulary has no way to say that today.
  **And a cut that is read has to be held, which is a frame-sized target and a copy per frame.**
  Holding all of them against the chance that something reads one is the cost nobody would pay, so
  the shape this points at is that **the selection recomposes the pipeline** — a cut nothing reads
  is not retained, and a `.kir` that names one adds the retention to the graph the way an edge adds
  a pass. Nothing in `karakuri-engine` does that today: the render graph is built from a Set's nodes
  and the fold is fixed.
  **Not decided, and deliberately not yet** — the material above is the framing a decision starts
  from rather than an argument for an answer, and it was written down because it existed nowhere.
  **What it blocks is stated rather than implied**: with
  [ADR-0226](adr/0226-m5-closes-when-the-manual-is-implemented-and-the-meter-is-progress-rather-than-completion.md)
  making the manual the exit condition, and the mock drawing `feedback` beside `bloom` and
  `rgb shift`, **M5 cannot close without either taking this or changing the manual.** That is the
  first place the new exit condition bites, and it is working rather than failing: the alternative
  was a milestone that closed with a control drawn in the specification and absent from the panel.

- **~~Where `Operation` becomes `Record`~~ — taken.** It is
  [`karakuri-operation-record`](../crates/karakuri-operation-record), a crate depending on the two
  leaves that may not depend on each other, and the reading it needs is a **value handed in** rather
  than a trait
  ([ADR-0194](adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
  **Twelve conversions are built. What the survey found is still open.**
  - **Seven operations owe a record nobody can write yet**, and they divide three ways: scheduling
    one (`FadeDeck`, `Crossfade`, `Wipe`, `SelectRenderer`) needs the grid quantised onto a musical
    instant **and the transition settings, which are `SetTransition`'s and which no record carries**
    — that is the largest single question left, and four conversions are blocked on it; moving the
    grid (`TapBeat`, `ScaleGrid`) needs the beat tracker rather than a value, and may be refused by
    it; and `SetSync` needs the engine's anchor clamp, so whether the record carries the anchor that
    was asked for or the one that was clamped is a decision about the bytes on disk.
  - **Five operations name a deck and their record has no slot.** `WriteParam`, `AttachSignal`,
    `WireInput` and `SetProperty` map onto `Record::Param`, `Record::Bind`, `Record::Edge`,
    `Record::Capacity`, `Record::Seed` and `Record::Camera` — every one a **Set file's**, with
    nowhere to put the deck — and `SetCompositing` is the same shape against `Record::Merge`. That
    is a gap in the session vocabulary rather than a conversion waiting to be written.
  - **One control is not one record**: `Crossfade` is four, `Wipe` is **six**, and the mask cost the
    gesture a record by becoming two operations, each writing `Record::Mask` whole.
  - **ADR-0180's "one `From` impl per list in `karakuri-cli`" cannot be written** — the orphan rule
    refuses it, since neither `Blend` nor `BlendMode` is that package's. They are plain functions.
  - **`--bpm` has a record now**, the one `Record::Tempo`'s own documentation describes, which closes
    the gap P-0028 names. **Nothing routes through it yet.**
- **What a MIDI map learns from a control whose affordance is a cycle — answered**: **the component
  owns three operations and a map is offered the three values, never a *next***
  ([ADR-0187](adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md), which
  is what P-0074 permits by name), and the MIDI half is implemented (ADR-0196). **What is still open
  is the *learn* half — how a map is reached from the chip's tooltip while running.**
- **What the tally shows while the two disagree, and what a press on it asks for — decided.** One
  operation names one of three residencies
  ([ADR-0186](adr/0186-one-operation-names-one-of-three-residencies.md)), the console draws the
  *effective* residency while `SetResidency` sets the *requested* one and calls the disagreement
  **parked**, what a pending transition has to say is a GUI rule rather than this chip's
  ([ADR-0188](adr/0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md),
  [P-0075](principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)),
  and the presentation is the roll, two lamps being **impossible on a 53-pixel row rather than
  rejected** — **a parked deck costing about 30 panel frames a second for as long as it is parked**,
  `Repaint::Never` the moment nothing is pending
  ([ADR-0190](adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)).
  The chip is a control now, its press **counted from the residency that was requested** rather than
  from the one it shows
  ([ADR-0195](adr/0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md),
  the first user of [P-0076](principles/0076-a-surface-owns-the-affordance-never-the-authority.md)),
  and **refuses nothing, so there is still nothing to decide about locking the panel.**
- **~~`SetMask` does not exist~~ — taken, and it is two rows rather than one**: *Set a deck's mask
  shape* and *Set a deck's mask position*
  ([ADR-0201](adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)); `softness`
  stays out for `white_point`'s reason, one constant and no control; and the panel and map halves are
  above. **The rule the split makes expressible is P-0078, which also says where it stops holding**: a
  `Record::Mask` is a state and not an ask, so the stream cannot carry which half was asked for.
- **~~What a status line is, on a console with no terminal~~ — taken**: neither the status line nor
  the bindings text gets a row on [the operations page](manual/operations.html), and `karakuri-cli`'s
  `s`, `h` and `?` stay outside the vocabulary
  ([ADR-0205](adr/0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md), answering
  [ADR-0198](adr/0198-a-gesture-converts-in-the-parts-that-are-decided.md) §4).
  **The consequence is noted rather than fixed and it is larger than the key**: the console as drawn
  gives an operator **no way to learn a key binding**, which is rule 01's keyboard surface with no
  teaching path — hooked to the deferred *a keyboard is mapped and learned the same way a control
  surface is* below, beside the tooltip mechanism that entry already names as what both learn paths
  wait on. **What that owed answer turned out to be is the deliverable, and it is five console gaps
  the status line's existence has been concealing.**

  1. **A slot's simulation clock** (`t0.0s`, per slot, from `Set::time`). Nothing in `view::Strip`
     carries a per-deck time, and neither does the mock's strip.
  2. **An armed transition's destination** (`g>` / `o>` / `w>`), and **this one is not a gap but a
     rule already broken**:
     [P-0075](principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
     wants the destination *"identifiable from the surface itself. Not from a tooltip alone, and not
     from a log"*, and the status line is that log. **In progress**: `view::Strip` carries `gain_to`
     and `opacity_to` now and their documentation cites that clause. Two are not covered —
     `MaskPosition`, which is the `w>` of the three, and the armed renderer selection `r>`, which
     `Deck::selections_on` answers and no `view` field carries.
  3. **Non-finite texels** (`x{n}`). `view::Level` carries `mean` and `peak` only, so the console
     draws two numbers without the field whose own comment says it *"explains the two numbers beside
     it"* — a mean and a peak over the texels that were not counted.
  4. **The tempo source** — its name, its peers, the anchors it rejected, and its liveness, which is
     three states and not two (`?`, `stop`, running) beside `GONE`. `view::Transport` has no field
     for any of it. **The `audio-in` pill is not this**: a tempo source is a separate program, of
     which the in-process beat tracker is one kind, and the pill names an input and says nothing
     about peers or liveness.
  5. **Sync mode, anchor and offset** (`T{anchor}`, `B{anchor}±offset`). **It is three operations
     rather than two**, which the board found and this survey had not: *Composite a deck's
     renderers* routes to the same home, and is the one of the three with no key either. **This one
     is closed on the manual's side**: the deck head was written on 2026-08-29, under each
     `.half-head` in the inspector, and the Inspector draws it. What is left of it is three presses.
     The other four of the five name no home at all and are where they were. *(The `±offset` here is
     the **scrub**, and it is spelled that way on the wire since 2026-08-29: `Record::Transport`'s
     field went from `offset_beats` to `scrub_beats`, and a stream carrying the old name fails its
     line rather than reading a zero scrub. The latency offset keeps the word, being the
     flag-facing one.)*

  **A sixth was found beside them and is a routing question rather than a gap**: the page routes
  *Tone map* and *Exposure* to `panel transport`, and the mock's transport row draws neither.
- ~~**The library's two questions are `console.html`'s own**~~ — **all three are answered on the page,
  2026-08-29, and none of them needed an operation invented**: a Set reaches a deck by the deck
  selection, a folder scope reads Sets, and a favourite is kept beside the Sets, in the library, with
  **the cost stated: copy the library and the stars come with it; hand somebody one Set and it arrives
  unstarred.** The arguments are [console.html](manual/console.html)'s, and MIDI is empty because no
  `Target` in `karakuri-midi` carries a string. **What each of the three still waits on is in the
  Library entry above**, and **what is owed there is more than the key, which this bullet read wrong
  until 2026-08-30**: **a `plan` badge does not distinguish a drawing that is owed from a machine that
  is missing**, and that row wears the first one's badge while being the second.
- **The four panel badges left open on 2026-08-29 are settled, and only one of them was `has`.**
  *Move a boundary* is. **The other three are not, and the reason is the same for all of them — the
  control is painted and never hit-tested**, so **the only route to `Fold`, `Solo` and *bring back
  what is folded* is still a key**, and the claim that a pointer press reached `Op::Fold` was wrong.
  **The Outputs dot is the exception**: it **is** hit-tested, and its badge stays `plan` on purpose,
  because *Choose where the frame goes* names a switchable list of sinks and one entry of it is not
  the list. **And the `key` column of that whole section read `gap`** while `cargo run -p karakuri`
  binds those operations; **that is decided now — the key column is the instrument's keyboard, and
  `karakuri-cli`'s keys are its own**
  ([ADR-0220](adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)),
  **at a cost of twenty rows** that stopped reading `built`. **Left named rather than met: the MIDI
  and MCP columns measure a different program than panel and key now do, and nothing checks the MIDI
  column at all.** And **the `.path` row cannot carry a tooltip at all**: it sets `overflow: hidden`
  and a tip is an absolutely-positioned child, so it is clipped — rule 03's *anything compact says
  three things on hover* broken by a declaration rather than by an omission.
- ~~**What authority is set on: a layer, a node, or a slot.**~~ **Taken: per node of a Set** —
  [ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md), guarded by
  [What the three words mean, and where](#what-the-three-words-mean-and-where), below, which landed
  first and is what the record is written in the terms of. **What it unblocks is the Inspector's
  authority row**, above, and **the engine half has landed**
  ([ADR-0216](adr/0216-a-node-nobody-has-spoken-for-is-manual-and-a-request-states-only-what-was-said.md)),
  where **the symptom this bullet described turned out to be the wrong way round**: a node nobody has
  spoken for is `Manual`, so dropping the restatement destroys a **grant** rather than taking one
  back. **What is still owed is a writer**, and a live-session one: `Record::Authority` is
  deliberately excluded from Set-file state, *"a Set file obeying one would hand the node over on
  every load"*. The chip can read; nothing can yet make it change, and the manual's row says so.
  **What an agent is one *of* is a separate question and is still open** — see the control plane
  above.
- ~~**Is the Master bay's `out` the same thing as `exposure`?**~~ **Taken on 2026-08-30, and it is c**
  — [ADR-0224](adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md): two
  values that multiply in **different places**, with the L5 master effects between them
  ([P-0051](principles/0051-an-existing-field-is-not-a-fact-about-the-design.md) cut both ways here).
  **The cost was taken deliberately and is measured**: with the chain empty the two produce
  **byte-identical** pictures, and **that test is deleted the day a master effect lands between
  them**. **What is left is a route**, a flag being refused by
  [ADR-0046](adr/0046-a-flag-writes-into-the-record-it-does-not-invent-one.md) until a record exists
  to write into; `docs/manual/operations.html` gains a *Master out* row when there is an operation to
  name, and the Exposure row does **not** move.
- ~~**A wildcard write crosses authority, and nothing says what an authority may refuse.**~~ **Taken
  on 2026-08-30, and it is a** —
  [ADR-0223](adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md): a bare-name
  write is refused unless every node it lands on is under one authority. **Its cost is real and is the
  argument for the alternative**: a partial grant makes a published control unusable until the operator
  grants the rest or narrows the interface. **It needed no asker, which is why it could land at all** —
  **no agent write of a parameter exists** — and **what is left is the asker**: whether an *addressed*
  write by an agent onto a node the operator kept is refused is undecided until something writes a
  param on an agent's behalf. **Both this and the master's two levels were found by drawing a bay
  rather than by reading for faults**, which is this milestone's pattern and the argument for drawing
  the remaining regions before deciding anything else about them.

- **What `expand` under a solo should do.** `Layout::soloed()` can lie: the solve never reads it and
  `check_structure` only checks that it addresses a node, so a solo's exclusivity lives in flags any
  later `expand` may contradict. Nothing reaches that state today.
- **Five operations are named and left `Undecided`**, each with its question written at the variant:
  moving a boundary, choosing where the frame goes (**no output has an identity anywhere in this
  workspace**), walking the edit history, *edit the file instead* (which names an event rather than
  an operation), and the camera.
- **The console's window has no `a`.** `karakuri-cli`'s snaps the window to the canvas so a capture
  is one texel to one texel; ADR-0181 made the picture the canvas's shape and created that gap.
- ~~**Whether any of M5's gated items belongs in another milestone.**~~ **Taken, on 2026-08-29, and
  the answer is that nothing moves: all seven of the *Adds* are this milestone's**, the maintainer
  closing it against splitting rather than by naming the two the list called candidates. **The reason
  is worth keeping, because it is a rule rather than a ruling**: none of the seven is blocked by
  anything outside M5, and where nothing is in the way, deferring is not caution but the expensive
  choice, because two half-milestones have to be re-understood before either can be finished. **What
  that turns the seven into is an order rather than a filter**, and *gated* now means only *not yet*.

#### What the three words mean, and where

Three words each carry more than one sense, and that — rather than the granularity itself —
is what made *What authority is set on* unrecordable:
[P-0031](principles/0031-a-name-means-one-thing-across-the-system.md) asks that names be
*"disjoint by name"* and not merely *"disjoint in practice"*, so a decision written in words
that mean two things is a defect and not a wording preference. This is the table the next
change is written from. **It takes none of the decisions.**

**`layer` carries three senses.**

- **A kind** — what a procedure lowers to: `L1`, `L2`, `L3`, `L4`, `Field`. This is the
  `layer` field on a `slot`, `capacity`, `param`, `bind` and `procedure` record, the
  `karakuri_store::record::Layer` type and `karakuri_operation::Layer`. There are **five**,
  and [ir-spec.md](ir-spec.md) says outright there is no `kind L5`, because compositing has
  no code to lower. A bare *layer* in `docs/ir-spec.md` means this.
- **A position in the architecture model** — the six rows `L0` through `L5` in *Architecture
  beyond V1*, above. Overlaps the kinds on four: `L0` and `L5` have a row and no `kind`
  file, and `Field` is a kind with no row.
- **What one deck slot contributes to the mix**, stacked in deck slot order. **Never written
  bare**: it is *a deck slot's layer* or *a deck's layer*, with its owner attached.
  `karakuri-store` already wrote it that way; `karakuri-engine/src/deck.rs`,
  `docs/ir-spec.md` and this file do now. [Concepts](manual/concepts.html) disowns the loose
  reading outright — *"A deck is not a layer in an image editor"* — because a deck is running
  material with its own time, which is why it has a residency rather than a visibility.

**Three sentences in the manual still write the third sense bare and are not fixed here**:
*"what a layer covers"* and *"a layer that has to disappear"* on
[the operations page](manual/operations.html), and *"touches what a layer covers"* on
[the console page](manual/console.html). The manual is the specification the implementation
is checked against and is being edited elsewhere, so these are reported rather than touched.
(*"an effect layer's controls"*, also on the console page, is the **kind** sense — `L2` — and
is right as written.)

Ordinary-English uses of the word are not this vocabulary and are left alone: *the
persistence layer* in `docs/ir-spec.md`, and *the declaration-plus-adapter layer* and *the
command layer* in this file.

**`authority` carries two senses, and both are in force.**

- **Where a rule lives** —
  [P-0076](principles/0076-a-surface-owns-the-affordance-never-the-authority.md): *"A surface
  owns the affordance, never the authority."* A constraint on what the instrument will accept
  lives where the record is applied, so every one of the four ways in meets the same wall.
  This sense is about code, and it is settled.
- **Who may move a control** — `man / sug / auto`, which is rule 06 of
  [the seven rules](manual/index.html). This sense is about an operator and an agent, and
  *what it is set on* is the open question above.

They are not the same word twice by accident. P-0076 is the reason the second one cannot be
built as a lock the console holds, whichever address it lands on — a rule that binds one
surface binds none.

**`slot` carries three senses**, and the first two are
[ADR-0049](adr/0049-slot-means-two-things-and-the-clash-is-recorded.md), which recorded the
clash rather than resolving it: *"Neither is renamed. The discrepancy is written into the
vocabulary."*

- **A node of a Set.** `Record::Slot`, and the `slot` lines in a Set file — a procedure at a
  `(layer, index)` address. Say *a node of a Set*.
- **A member of the deck.** The `slot: u8` field on every session record, and the deck's own
  indexing. Say *a deck slot*; `karakuri-store`'s prose now does so everywhere, which it did
  not before.
- **A declared input on a node.** `Record::Edge`'s `slot: String` — what `uses far :
  Geometry` names. Say *an input slot*. This third one is not in ADR-0049 and had no guard
  at all before.

**What this settled, and what it did not.** It took none of the decisions — that was the
point of it. What it made possible is the one that followed:
[ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md) picked *per
node of a Set*, and could be written down as that rather than as *per layer* only because this
table says which of three senses `layer` would have been carrying. The four properties at the
top, the control-plane roster, M5's mode control and M6's agents are updated to it. **What is
still open is what an agent is one *of***, which this table also does not settle.

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
| One frame loop | Works. There were two — the window's and the offscreen PNG's — and the seam between them was meant to be a single line (a live run measures its step count from a clock, a replay reads it from a `tick`). It had become five, and **every replay defect this project has found was one of the four extras**: the look applied once instead of per frame, the governor running on one path only, the present pass skipped on unkept frames. Now `frame::compose` is the loop and a `Sink` is where it goes — a window, a PNG writer, the console's picture and deck A's preview cell, and a test double. It is `karakuri-engine`'s ([ADR-0172](adr/0172-the-frame-loop-is-the-engines-because-the-cli-is-scaffolding.md)), because it was in the CLI, the CLI has no library target, and the application could not reach the one loop and had written a second. The **ordering is structural**: `compose` takes the committing work as a closure and calls it in one place, with the deck's render adjacent and nothing between them, so a `tick` cannot claim steps the deck never took. That was two statements in the right order before, and getting it wrong was invisible. It used to be structural the other way round — the closure was withheld until a sink had a target — and that could not survive a second sink ([ADR-0171](adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md)): **the deck advances on every frame `compose` composes, and each sink takes that frame or misses it on its own.** `compose` takes a slice of them, zero included. The test sink is what finally reaches `Live::frame`'s claims, which no test in this program had ever done |
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
| Blend modes | Works: `add`, `over`, `max`. The set is what survives an **unbounded linear HDR** mix rather than what a VJ mixer usually lists — `screen` and `multiply` assume `[0, 1]` and nothing has tone mapped this far up the pipeline, so `screen` of two 2.0s is 0.0. `over` is the only mode in which one deck slot's layer hides another, and what it hides with is coverage the L4 pass accumulates into alpha; thin material barely covers, which is correct rather than a defect. This is also what finally separates **gain from opacity** — gain is the level the material arrives at, opacity is the fader across the blend and the only control that silences a slot under every mode. What is not built: a stack that is anything other than deck slot order |
| Priming and the governor | Works. Priming steps a slot and does not draw it — L1 owns every piece of per-element state and L4 is stateless, so warming is the compute passes and nothing else. The governor computes each slot's effective residency from the operator's request and a budget of per-Set costs measured by the probe at build time. It **never demotes a Live slot**: an over-budget deck reports and suspends priming. An unmeasured Live slot means the committed cost is unknown, and unknown is not headroom, so priming is suspended until every slot has been measured |
| Audio | Works. Analysis runs in the driver's callback, not on the frame path, and the frame reads one small value through a `try_lock` on both sides so neither can block. Level is RMS mapped −60 to −6 dBFS: a mastered track's loud windows sit near the top of that, where a top at 0 dBFS would leave real music between 0.80 and 0.90 and a bound parameter barely moving |
| Beat tracking | Works. The grid is **predicted, not chased**: once locked it free-runs, takes a slow trim, and moves not at all for a single disagreeing estimate — re-acquiring takes eight consecutive consistent revisions. **The octave is folded, not judged**: every candidate period is halved or doubled into a one-octave window centred on the grid, which starts at `--bpm`. So a window centred an octave off tracks an octave off, `,` and `.` are the fix, and there is deliberately no automatic one — the alternative is a heuristic that can be confidently wrong, which is the failure that shows on stage. A 3:2 error is a different problem and is not touched. The correction leads by the analysis lag plus the output lag, so what is shown lands on the beat rather than behind it |
| Where the lead comes from | The measurable part is computed and the rest is a **signed operator offset**, on `o`/`p`. That is not a gap waiting for a better sensor: sound and picture leave by different paths, neither ends at this machine, and what has to line up is what a person in the room sees and hears. So the measured half aims at being *stable* rather than complete — a constant unknown costs one adjustment, a drifting one costs the whole night — and the offset goes negative, because a delayed PA makes the sound the late one |
| Per-slot metering | Works. Mean and peak linear Rec.709 luminance per drawn slot — Live, or being auditioned — reduced on the GPU and read back without ever waiting, so it lags a few frames and says by how many. A slot nobody is drawing reads nothing rather than reading what it last drew. Texels whose luminance is not a finite number are counted and left out of both figures: one NaN admitted to a sum makes the mean NaN, and dividing by a value that reaches zero is ordinary enough in a shader that a routine artifact would otherwise cost a slot the number its fader is set by |
| `kind Field` | Works. A signed distance function in a file of its own: handed a `point`, it assigns a `distance`, and it draws nothing and holds no elements. **It has a file and no node** — the L5 argument run backwards, since a `kind` says what a procedure lowers to and this has only code to lower — so it is spliced as one WGSL function per *slot* into whichever procedures declare one, and into no others. **A caller says what it takes and the Set says which** — `uses shape : Field` in the header, `shape(p)` in the body, `--edge field_lens.shape=melt_blob` on the command line — and a slot nothing binds is refused rather than filled in from whatever field is lying around. That replaced `field(p)`, a reserved word, which is a breaking change to the language and is exactly what capped a procedure at one field. Its cost is **per evaluation** and the caller pays it once per call, so a marcher and a field that each fit alone can still be refused together, with both figures in the message. Several per Set, each a node with a name and an address of its own |
| Masks | Works: a straight front at an angle, and an iris, per slot. A mask multiplies that deck slot's layer opacity per texel — everything the fader does, done to part of the frame — so it needed no new place in the composite. Both ends of the front are exact, 0 revealing nothing anywhere and 1 revealing everything everywhere, which is what lets a deck slot masked to nothing be **skipped**: a third escape from material that has gone NaN, beside residency and the fader. **A wipe is a mask and one scheduled move** (`c`), and neither half knows about the other. Out: a mask read from a texture — an arbitrary shape, or another deck slot's luminance. `kind Field` now exists and is what such a mask would be written as; what is missing is the deck's mask reading one, since `shaders/composite.wgsl` is a hand-written engine shader with a fixed set of mask kinds rather than a generated one |
| Transitions | Works. One scheduled move — a control, a destination, a musical duration, a curve — and a crossfade is two of them sharing a start and a length. `f`/`g` fade the focused slot out and in, `x` crossfades to the next slot, `n` and `j` choose where a fade starts and how long it lasts. **The first thing in the engine that schedules on the beat clock** — the transport already *follows* it — and a fade is a function of the session's beat count and of nothing else, so the same records reproduce it on a machine at a different frame rate and a tempo change mid-fade moves the fade with it. One record schedules the whole move and the values it produces are not recorded, which is `tick`'s shape from the other end. A hand on a control cancels whatever was moving it. A wipe is one of these carrying a mask's front, which is why there is no `wipe` record and no `crossfade` record — the first-class things are the shape and the move |
| Slot preview | Works. `v` cycles what the output shows: the mix, then each slot. An audition **adds a draw and never a step**, so an off-air slot is drawn while it is being looked at and nothing moves that would not have moved anyway — an allocated slot shows the still it stopped at, a priming one shows what it is warming into. Not a second pass: the mix runs as always with that slot's terms at unity and the others skipped, so what lands is its own texels through the same tone mapper. Shown ignoring its faders, and metered, because the number wanted before putting it on air is the level the material arrives at. What is not built is a default renderer per topology — there is no slot holding geometry with no L4 to draw it, so there is nothing yet for one to do |
| MIDI in | Works. `--midi-in` opens a port, `--midi-map` says what each knob and pad does, and a mapped message **is** a `karakuri_operation::Operation` — the same name a key press carries, ending in **the same record a key press writes**, so a surface can do nothing a key cannot and a session recorded from one replays with neither attached. A line names a state rather than a step (`residency 0 live`, `blend 0 over`), and a file written against the older spellings is refused line by line with the line to write instead ([ADR-0196](adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)). Eight targets, the newest of which is `mask-position` — a wipe's own front, `[0, 1]` with both ends exact, and a hand on it stops the move that was carrying it; the mask's *shape* has no line, because the grammar has no bare number to write its angle with ([ADR-0202](adr/0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md)). The map is a file and is deliberately **not** in the stream: which knob is which belongs to the hardware in the room. With no map, every message prints the line that would map it, which is how a surface is discovered until M5 has a UI to assign one in. 7-bit; the 14-bit MSB/LSB convention is not implemented, which is about 0.8% of a fader's range per step. **A continuous control says one thing per frame**: the router keeps the last value a fader sent within a frame and drops the ones before it, so a sweep of several hundred messages builds one record rather than several hundred — which is what takes a record's `String` off the frame path on `exposure` and `mask-position`, where P-0001 has no clause for a small allocation. Pads are untouched, because two presses in one frame are two things that happened. What it costs the stream is fewer records for a swept fader and nothing that was ever reconstructible, since a replay applies every record between two `tick`s before drawing the frame they close ([ADR-0207](adr/0207-a-continuous-control-says-one-thing-per-frame.md)). **MIDI out is not built**, so a surface's LEDs and motorised faders do not follow the deck — which starts to matter the moment two things can move one fader, and a transition is now one of them — on the mask's front they are the same number, and a motorised fader is the only thing that could show which of the two last wrote it |
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
than estimated: **35 live operations reachable from `karakuri-cli`'s keys** — a figure that says nothing about whose keyboard, which is the ambiguity [ADR-0220](adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md) closed — and only from the *focused* slot,
since nothing but `0`–`3` addresses another; **8 from MIDI** — `gain`, `opacity`, `exposure`,
`mask-position`, `residency`, `blend`, `preview` and `tap`, which is what a map file's grammar
accepts and no
longer a vocabulary of the crate's own; it cannot express a node address, a parameter name or an
id, and it was read as 8 while `on-air` and `prime` were two targets over the three states
`residency` now names
([ADR-0196](adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)) before falling to 7
and coming back a different way. **What it still cannot express is a bare number**, which is
exactly the line between the mask's two rows: the front is a fader and the shape carries an angle,
so `mask-position` landed and the shape has no line to write
([ADR-0202](adr/0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md)); **6 MCP
tools**, which touch nothing in the mix, the clock or the deck's residency; and **38 command
line flags**, many of which are the *only* route to what they set.

The sharpest consequence: **a model can rewrite a whole procedure and cannot write a single
parameter.** `--param` exists and is reachable from one surface at one moment. Reachable from
nothing at all while running: parameter writes, binding changes, edge rewiring, the camera,
seeds, capacity, canvas size, un-merging a slot, restoring every renderer to the fold, a gain
fade, and starting or stopping a session recording.

**The vocabulary exists now**
([ADR-0180](adr/0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)):
`crates/karakuri-operation/` is a leaf crate with **no dependencies at all**, holding every one of
them, and a test reads [every operation](manual/operations.html) and asserts that the page and
the type enumerate the same ones both ways round. **How many there are is not written down here**,
for the reason the paragraph below this list gives: it held 46 when it landed and has been 45, 46,
48, 49, 50 and more since, and every one of those numbers was written into this file and then had
to be chased. `grep -c '<h3' docs/manual/operations.html` is the count. What has moved it: `SetOnAir` and `SetPriming` were two booleans for three states with both `false`
destinations unnamed, and they are one `SetResidency { deck, residency }` now
([ADR-0186](adr/0186-one-operation-names-one-of-three-residencies.md)); `SetLook` demanded a tone
map beside every exposure, so `cc -> exposure` could not become an operation at all, and it is
`SetTonemap` and `SetExposure` now
([ADR-0192](adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)) —
which is what unblocked the MIDI map's migration; and the mask was a record no operation named, which
is `SetMaskShape` and `SetMaskPosition` now
([ADR-0201](adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)). The last two
are rows the page **gained** rather than split: resetting the arrangement is the default case of a
family rather than a thing standing alone
([ADR-0208](adr/0208-resetting-is-the-default-case-of-restoring-an-arrangement.md)), and authority
is set per node of a Set
([ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)). **The MIDI map is migrated**
([ADR-0196](adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)) and `Action` is
deleted; the other three surfaces are surveyed and answered rather than pending — the console's
`panel::Op` stays where it is
([ADR-0197](adr/0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md),
[ADR-0204](adr/0204-the-root-and-the-body-row-stay-unnamed-and-a-folded-root-is-not-hit-testable.md)),
fifteen of the CLI's thirty-nine keys already reach `Live::operate` and the rest divides into keys
that cannot route at all and keys whose record is owed
([ADR-0198](adr/0198-a-gesture-converts-in-the-parts-that-are-decided.md)), and MCP names its
operations and performs them itself
([ADR-0199](adr/0199-mcp-names-its-operations-and-performs-them-itself.md)). Each of those was a
change that could be reasoned about because the target had stopped moving. The order of work and
what each is still waiting on is under *Where this goes next*, above.

Building it found the thing worth having built it for: **`Action` and `panel::Op` each claim to be
the vocabulary idea and contradict each other head-on.** `Action` is engine-neutral by declaration
and therefore full of toggles and cycles; `panel::Op` argues at length that a vocabulary must have
none. The two rules are not jointly satisfiable — *a crate that refuses to name a value cannot say
`set blend to over`, so the only gesture left to it is `step it`* — which is
[P-0074](principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md), and the
vocabulary owns the value lists because of it.

Five things are named and left `Undecided`, each with the decision written at the variant rather
than guessed: **moving a boundary** (most splits are unnamed and `set_divider` takes a viewport
pixel a key press cannot mean), **choosing where the frame goes** (*no output has an identity
anywhere in this workspace* — `Sink` has no name and no id, and the console reaches its one sink by
folding a layout region), **walking the edit history**, **"edit the file instead"** (which names an
event rather than an operation), and **the camera**.

**So M5 opens with naming rather than drawing**, which is the shape M4 opened with — nothing
could be edited there until nodes had names, and nothing can be routed here until operations
do. [Every operation](manual/operations.html) is where that enumeration is written down, and a
surface missing from an operation's row is a line of this milestone's work. **How many of them
exist is not written here, and that is the point of this paragraph.** The console's own shape is
where several of them came from — moving a boundary, folding a bay or a pane, bringing one back,
solo, and resetting the arrangement, and since
[ADR-0221](adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md) saving
an arrangement and putting a saved one back. Every one of the first six has a route now, which it
did not on 2026-08-28, and **the two ADR-0221 added have one since 2026-08-30**: the transport
row's arrangement pill reaches `save`, `load` and `start a new one`, the last of which is the reset
([ADR-0225](adr/0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)). The
three rows moved `plan` to `has` with it, which is what the first command under *The meter is one
column* now reads.

**The figure this sentence used to carry went stale four times, the same way every time.** It read
50 of 208 while the page said 51; then 49 of 212 while the page said 50 of 216; then 51 while five
panel badges made it 56; then 56 while
[ADR-0220](adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)
moved it to 42 by settling whose keyboard the key column measures. **Three of the four were caused by
work in this repository that had no reason to look here**, and the fourth was caused by the commit
that had just finished writing *do not transcribe it again* two lines below. Saying it did not stop
it, so the number is gone rather than corrected: the two commands at the end of this paragraph are
the answer, and there is nothing left to keep in step. The first three read as 50 of 208 while
the page said 51 of 208 — the mask's front gained a MIDI route in
[ADR-0202](adr/0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md) and this
sentence was not recounted with it — and it then read 49 of 212 while the reset row
([ADR-0208](adr/0208-resetting-is-the-default-case-of-restoring-an-arrangement.md)) and the
authority row
([ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)) took the page to
50 of 216. Both figures are transcribed off a page nothing in the workspace counts, so they go
stale in silence: `grep -c '<h3' docs/manual/operations.html` is the first, and the `class="rt has"`
badges in it against every `class="rt ` badge are the second.

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
- ~~**A step sequencer is a signal source, not an effect.**~~ **Overturned on 2026-08-29, and both
  halves of it were false** ([ADR-0222](adr/0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md)). It said a sequencer is *one more name on
  `karakuri-signal`'s bus* and that *a lane is a binding, so a lane drives a Set parameter as
  readily as a deck fader*. **A lane is a fifth route into the vocabulary**, which is what
  [the console page's own rules](manual/console.html) had been saying all along and what
  [P-0078](principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md) already
  treats a lane and a binding as. What it still produces is no pixels, which was the part that was
  right.
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

  *(One sentence here was wrong and was removed, and has since come true: it said a composited
  frame "already goes to *n* sinks", when `docs/plugins.md` designed that, the `Sink` trait
  existed, and `compose` took exactly one. The fan-out is built now — what is owed is a second
  sink to put in the slice.)*

  **And the sink no longer gates the frame**
  ([ADR-0171](adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md)).
  `compose` used to return `Skipped` before calling the closure that commits, so a frame with no
  target read no clock, wrote no record and did not advance the deck — right for a lost swapchain,
  wrong as a steady state, and meaningless with two sinks. Advancing a frame and publishing it are
  two things now: every sink is asked first, the deck advances whatever they answer, and each sink
  that took the frame is drawn into and presented. **Every output may be off and the instrument
  keeps running**, which is what the manual promises about setting up before doors.

  It cost three early returns in `Live::frame`, and all three were bugs waiting: a transient
  refusal — every frame of a resize, every frame of a minimised window — used to skip the deck's
  event drain, the `procedure` records, the governor and the status line, and a latched fault
  skipped them for good, so a build landing while the window was wedged was never announced and
  never recorded.
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

**It left the operations page short, and the naming has since happened.** Arranging the panel is
operations, and the page carried no row for folding a pane, for moving a divider, or for `solo`.
[Every operation](manual/operations.html) now heads an *Arranging the console* section —
moving a boundary, folding a bay, folding a pane, bringing one back, solo, resetting the
arrangement, and since 2026-08-29 saving one and putting one back — which is how the vocabulary
reached its present width. **No count of that section is written here**: it was *six* until the
family landed, and the sentence beneath it went stale in the same commit for the same reason. Run
the second command in *The meter is one column* instead. **What the naming did not
close is the rule that made it worth doing**: every operation is reachable from the keyboard alone,
and most of that section's routes are still empty. The pointer reaches exactly one — the drag — and the instrument's keyboard has five, which is what [ADR-0220](adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md) settled.
*Move a boundary* is the sharpest of them and is `Undecided` for exactly this reason —
`set_divider` takes a viewport pixel a key press cannot mean. So the count is right now and the
first rule is not yet kept here, which is a different debt from the one this paragraph opened
with.

#### What building the console's arrangement found

`karakuri-console` holds the arrangement as one `Spec`, with every number read off
`docs/manual/style.css` and every derivation written beside it. Building it against the manual
turned up four things, and the first is architectural.

**A region cannot be added or removed while the panel is running.** The mock's `2 up` control says
a wide window fits three or four inspector panes and that the operator chooses how many *live*; a
`Spec` builds a `Layout` once and the arena has no insert and no remove. Rebuilding to change the
count throws away every drag, every fold and the solo state in the whole panel. What is missing is a
pair of arena operations that keep the rest of the tree and its ids, with `solo`'s saved flags
resized alongside.

**Two claims in that paragraph were checked and are wrong.** The mock has **six** such controls, not
five — the Sequencer head's `+` for a third sequence was not in the list — and **only one of the six
(`2 up`) names a region of the arrangement**; the other five add something a bay draws inside its own
body, which the arena has never held. So it is not one gap drawn five times: it is one arena gap and
five body gaps. And *"it is not a change to the solver"* is true of **insert** — `measure` and the
placement walk `children` and index the solved buffers, so an append plus a `children` insert plus
resizing those buffers is the whole of it — and hides the harder half of **remove**: `NodeId` is a
bare index into `nodes`, so a removal shifts every id anything is holding, which reaches
`check_structure`, the wire format, and whatever the panel has in hand.

**A node has one `[min, max]` pair, along its parent's axis.** The mixer wants a width floor (four
strips side by side) *and* a height floor (a strip is never cut off), and both could be stated only
because they happen to sit at different levels of the tree. Two constraints on one node cannot be.

**A split's minimum is not derived from its children's**, and it is half answered. The body row
declares 530, which is the right pane's three bays and its dividers added up by hand three levels
away, and a test recomputes it so the two cannot drift. The solver now measures the same quantity
for itself — [ADR-0174](adr/0174-a-node-claims-only-what-its-visible-content-can-use.md) walks the
arena bottom-up before placing anything — but it uses it as a **ceiling** rather than as the
minimum: a declared minimum becomes `min(declared, what the visible content can use)`. Deriving the
minimum outright is a separate decision and not obviously right, because a declared minimum may be
*larger* than its children's sum and often should be — 530 is one. The hand-added copies stay for
now, and so does the test that recomputes them.

**And a fixed split no longer holds height its content cannot use** — the same record. Folding the
picture used to leave the Program bay at its whole 378 with the preview row swollen to fill it,
which made the manual's *"turn it off and that picture goes, giving its height to the inspector"* a
sentence about nothing. A test asserted the swelling as correct, with a comment reasoning that the
sentence was the sink's doing rather than the fold's — and the manual says the picture is on screen
exactly when the sink is on, so there is one state and it cannot have two geometries.

**And three places where the manual does not agree with itself**, all found by trying to build from
it rather than to read it. `.body-grid`'s centre track says `minmax(340px, 1fr)`, at which each
inspector pane is 165 wide — but `.param`'s fixed tracks come to 207 before the fader has any width
at all, and `.console`'s `min-width: 1010px` is what actually holds the panel, at which they are
237. The panel has two minimum widths and the smaller one is unreachable. The lede says *every
divider drags* and the mock draws a grip on four bays of eight. And *"height given back is rows:
roughly 48 in the library, and near 40 per inspector pane"* does not divide: a `.lib-row` and a
`.param` are the same box, so the same height is the same number of rows.

Two rules the panel is built to whatever draws it. **It must be testable without a GPU or a
window** — the model and the command layer answer headless and the view stays thin, which is what
keeps this milestone's tests off the `mod gpu` side of `docs/contributing.md`'s split. And **a
still panel costs nothing, while what must be live declares a cost and a staleness** —
[P-0072](principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md).

*(That second rule replaces one that said the panel "must not allocate on the render frame path",
which was P-0001 carried across and is false of any immediate-mode toolkit: `egui` allocates 184
times and 226.2 kB per frame with every bay empty. P-0001 forbids deferred work landing an
unbounded stall on whichever frame is first, and that hazard is unchanged on the engine's path.
The panel's is a budget, and its two schedulability conditions are arithmetic a test can assert —
[ADR-0164](adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md).)*

**The sequencing starts from the bottom, and the first piece is the layout.** It had been left
until the manual's operations page existed, since that is what the vocabulary is read off; the
page is written, and what it says is that the panel is a set of regions an operator arranges. So
`karakuri-layout` is built and tested before anything is drawn into it — it needs no toolkit, no
device and no window, which means the `wgpu` 30 migration and the layout model were done at the
same time without waiting on each other. Beyond that first piece the order is open, and nothing
below is blocked on anything else.

#### What this milestone has delivered, and why the list below stopped counting it

**The list below is the second half of the work, and reading it as the milestone is what makes
the progress invisible.** Not one of its seven items was finished when that was written, and one is
now — *an arrangement is saved and restored*, whose control landed on 2026-08-30. Meanwhile
`karakuri-layout` and `karakuri-console` were written from nothing — the first commit in either
is 2026-08-24 — the operation vocabulary and its record crate landed beside them, and the records
and principles behind them landed with them, from
[ADR-0156](adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md), which opened the
console, onward. **No count is written here**, because this one has already had to be chased twice:

```sh
# records written since the console opened, and principles since P-0071
ls docs/adr/*.md        | awk -F/ '{print $NF}' | awk -F- '$1+0>=156' | wc -l
ls docs/principles/*.md | awk -F/ '{print $NF}' | awk -F- '$1+0>=71'  | wc -l
```

None of that is on the list, because the list was written before anybody knew what the floor under
those seven items was made of. This is what it turned out to be.

- **An arrangement this repository owns, and it round-trips.** `karakuri-layout` hands out
  rectangles with no toolkit in it, solves without mutating what the operator arranged
  ([P-0071](principles/0071-solving-a-layout-never-mutates-it.md)), honours a maximum as trailing
  space rather than as stretch ([ADR-0157](adr/0157-a-maximum-is-honoured-and-the-leftover-is-trailing-space.md)),
  lets a node claim only what its visible content can use
  ([ADR-0174](adr/0174-a-node-claims-only-what-its-visible-content-can-use.md),
  [P-0073](principles/0073-a-node-claims-only-what-its-visible-content-can-use.md)), and
  **refuses a saved arrangement that disagrees with itself rather than repairing it**
  ([ADR-0158](adr/0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md)).
  A boundary is a rectangle rather than a coordinate
  ([ADR-0160](adr/0160-a-boundary-is-a-rectangle-not-a-coordinate.md)) and gets first refusal on a
  pointer ([ADR-0163](adr/0163-a-boundary-gets-first-refusal-on-a-pointer.md)); solo remembers
  which region because it cannot be derived
  ([ADR-0161](adr/0161-solo-remembers-which-region-because-it-cannot-be-derived.md)); and a region
  that is not laid out declares nothing rather than being dropped later
  ([ADR-0193](adr/0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md),
  [ADR-0183](adr/0183-a-node-is-out-of-the-layout-for-two-reasons-and-they-are-two-bits.md),
  [ADR-0204](adr/0204-the-root-and-the-body-row-stay-unnamed-and-a-folded-root-is-not-hit-testable.md)).
  **This is the half of *an arrangement saved and restored* that nobody costed**, and it is done.
- **One vocabulary, and the surfaces beginning to route into it.**
  `crates/karakuri-operation/` is a leaf crate with no dependencies at all
  ([ADR-0180](adr/0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)) whose
  specification is [every operation](manual/operations.html), checked both ways round by a test.
  An operation says what it wants and never which way to move
  ([P-0074](principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md)) and
  carries what it acts on ([ADR-0175](adr/0175-an-operation-carries-what-it-acts-on.md)); it asks
  for what a surface can say while the record stays whole
  ([ADR-0192](adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md));
  three residencies are one operation
  ([ADR-0186](adr/0186-one-operation-names-one-of-three-residencies.md)) and the mask is two rows
  because a control change can only set
  ([ADR-0201](adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)).
  **Authority is set per node**, which was the fiftieth row and is the first concept the page gained
  rather than split
  ([ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)); it writes
  `Record::Authority`, the second session record to name a node, and **the engine field a per-node
  flag needs to survive a rebuild is written** — `swap::Request::authorities`, restated on every
  rebuild, with a node nobody has spoken for `Manual` so that dropping the restatement destroys a
  grant rather than taking one back
  ([ADR-0216](adr/0216-a-node-nobody-has-spoken-for-is-manual-and-a-request-states-only-what-was-said.md)).
  **What is owed is a writer**: nothing calls `set_authority` outside tests, so every node of every
  Set is manual and the chip can read while nothing can make it change.
  A wildcard write over nodes that disagree is refused rather than landed partially
  ([ADR-0223](adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md)), which
  is why `Set::write_param` and `Set::set_published` both carry a `Result` now.
  `karakuri-operation-record/` is where an operation becomes a record, a crate that depends on
  both ([ADR-0194](adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
  **The MIDI map is migrated and `Action` is deleted**
  ([ADR-0196](adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md),
  [ADR-0202](adr/0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md)); MCP names
  its operations and performs them itself
  ([ADR-0199](adr/0199-mcp-names-its-operations-and-performs-them-itself.md)); the console's `Op`
  stays for now and what blocks it is the page rather than the code
  ([ADR-0197](adr/0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md)).
- **The console draws six of its nine regions, each argued against the manual.** The transport
  row shows what the console can know
  ([ADR-0177](adr/0177-the-transport-row-shows-what-the-console-can-know.md)); the Program bay
  arranges itself for the larger picture and rearranges when it must
  ([ADR-0181](adr/0181-the-picture-is-the-canvass-shape-and-the-leftover-is-the-consoles.md),
  [ADR-0182](adr/0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md),
  [ADR-0184](adr/0184-the-program-bay-rearranges-itself-and-a-still-frame-does-not.md)); the mixer
  draws four tracks and as many strips as the deck has
  ([ADR-0178](adr/0178-the-mixer-draws-four-tracks-and-as-many-strips-as-the-deck-has.md)), where a
  fader translates a drag into an operation and applies nothing
  ([ADR-0185](adr/0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md)) and
  marks where it is going while it reaches for it
  ([ADR-0206](adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md),
  [P-0075](principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md),
  [ADR-0188](adr/0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md)). A
  control the console draws is the panel's
  ([ADR-0176](adr/0176-a-control-the-console-draws-is-the-panels.md)), and a surface owns the
  affordance and never the authority
  ([P-0076](principles/0076-a-surface-owns-the-affordance-never-the-authority.md)). The fifth is
  the Library, whose first pass draws the values that exist and omits the rest
  ([ADR-0200](adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)) — a
  row per Set and a foot reading `n of m`, with six of the mock's controls waiting on something
  different each. **The sixth is the Inspector**, drawn on 2026-08-29 off a running Set — the pane
  head, the deck head, a group per node with its address and its `man / sug / auto`, the renderers
  folded into one, and the parameter rows — and its arrival is what caught the console's claimed
  smallest window having been 9.5 px short since the day that deck head was specified. **What is
  left is `staging`, which draws its empty state and has no other, and `master` and `sequencer`,
  which draw nothing but their heads.**
- **The frame the panel is drawn on.** `egui` draws it and it cost a `wgpu` 30 bump
  ([ADR-0155](adr/0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md)); the engine's frame and
  the panel's are one submission
  ([ADR-0166](adr/0166-the-engines-frame-and-the-panels-are-one-submission.md),
  [ADR-0173](adr/0173-a-frames-submission-is-not-only-its-sinks.md)); the frame loop is the
  engine's because the CLI is scaffolding
  ([ADR-0172](adr/0172-the-frame-loop-is-the-engines-because-the-cli-is-scaffolding.md)); **the
  deck advances and each sink either gets the frame or misses it**
  ([ADR-0171](adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md)),
  which is what makes a second sink a thing to add rather than a thing to design. A still panel
  costs nothing and what moves declares its price
  ([P-0072](principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md),
  [ADR-0164](adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md),
  [ADR-0210](adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)),
  the repaint decision is one closed list
  ([ADR-0165](adr/0165-the-repaint-decision-is-one-closed-list.md)), and motion may carry the
  meaning, so a stopped animation is a fault to report
  ([P-0077](principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md),
  [ADR-0189](adr/0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md)).
- **And the measuring apparatus was corrected on the way.** The timestamp verdict is the
  backend's rather than the machine's
  ([ADR-0169](adr/0169-the-timestamp-verdict-is-the-backends-not-the-machines.md)), which took
  three machines to find out; a transcribed number cites the rule it was copied from and the
  citation is checked
  ([ADR-0179](adr/0179-a-transcribed-number-cites-the-rule-it-was-copied-from.md)); and a question
  whose reply the vocabulary cannot say gets no row, which is what caught the status line
  concealing five console gaps
  ([ADR-0205](adr/0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md)).

**None of that is a changelog entry.** Every one of the seven items below assumed a panel to be
drawn into, a name to be routed by, and a frame to be drawn on, and none of the three existed when
the list was written.

**Adds**

**Each bullet says whether it is ready, gated or partly done, and on what.** They read alike
today, which is most of why the count stopped meaning anything: a bullet waiting on a decision
nobody has taken and a bullet waiting on an afternoon's drawing are not the same item. Nothing has
been moved between milestones here — that is the maintainer's, and two of these are candidates for
it.

- Node editor. ~~Waits on the authoring notation at the head of M4 — M3 introduced node
  *ownership* and deliberately not a graph model, so there is nothing to edit until nodes
  have names~~

  **That blocker is stale, and it is the one worth the maintainer's attention.** The authoring
  notation is built and M4 is closed on it: a procedure declares `uses far : Geometry` and the Set
  binds it with `--edge morph.far=sphere_shell`, an unbound slot is refused
  ([ADR-0152](adr/0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md),
  [P-0068](principles/0068-a-kir-never-names-a-node-of-a-set.md)); `Set::node_names` answers what
  every node is called, whether or not a caller wrote one; and M4's own table marks *"which
  geometry a node's second input takes is `--set` order"* **Closed**. Nodes have names. There is
  something to edit.

  **Partly done, and what it is really waiting on is two things this bullet never mentioned.**
  Naming a node's *source* is done — `ReadProcedure` and `WriteProcedure` are operations, and so is
  `WireInput`, the one operation addressed by name at both ends. What has no name anywhere is
  **adding or removing a node**: there is no such operation in `karakuri-operation`, no row on
  [every operation](manual/operations.html), and the same gap has an arena half —
  `karakuri-layout`'s arena has no insert and no remove either, and `NodeId` is a bare index, so a
  removal shifts every id anything is holding. **The second was a surface and is not one any more**:
  the inspector is drawn, so a node editor has a bay to be argued against. **Ready to be worked,
  gated on nothing outside M5.**
- Parameter surfaces with MIDI learn and signal binding UI. **What they show is decided in
  M3** — a Set declares which of its controls it publishes, so this surface renders an
  interface rather than inventing one. Without that it would be twenty-five knobs per slot
  and a filter nobody can save. **How learning works is settled above**: a control is bound to
  a deck and a position in that published interface, the assignment lives in the control's own
  tooltip, and the map is a file saved per controller

  **Both stated blockers hold, and a third one is not stated at all — this is gated inside M5.**
  `Set::published` is there and `WriteParam`, `AttachSignal` and `TakeParamBack` are named
  operations, so the *what* half is real. But **the console draws no tooltip anywhere** and has not
  half-drawn one either: a tooltip needs `egui` to own a widget, and this console paints, with no
  widget anywhere in it — the four controls of the mixer that carry a `data-tip` in the mock get
  none, and neither does the Outputs row. So the learn mechanism starts from something that does
  not exist. **Gated on a decision, not on work**: who owns the pointer — whether the panel gains
  `egui` widgets, or paints its own hover layer — which
  [`karakuri-console`'s `input`](../crates/karakuri-console/src/input.rs) names as its own decision
  and deliberately does not take. Nothing outside M5 is in the way; the decision is.
- Set browser with live previews of priming Sets. **It carries M4's unsettled thumbnail question with
  it**: a stored still is not the live preview beside it, and what a thumbnail is *of* is
  undecided because a metadata card is per artifact and one procedure cannot be rendered
  alone — see what M4 still owes, where the machinery is costed and the question stated

  **Checked, and it holds exactly as stated.** M4 is closed and says so honestly — *"What closed is
  the machinery, tested at the scale that exists"* — and it owes six things, of which thumbnails is
  one, filed as *"a decision rather than machinery"* and handed here in its own words: *"M5's Set
  browser is where somebody will next be in a position to judge a framing."* **Partly done and
  gated on that judgement.** The library bay is drawn and lists what the store holds; the previews
  and the thumbnail beside them are what waits. Note the judgement is M5's to make and the
  machinery it would revive is M4's — **do not read this as a reason to move the item**; M4 is
  closed and its debt is recorded where it belongs.
- Staging lane — where candidates appear before they go live. Its first producer is the
  operator's own regeneration of a slot, which needs no agents and makes the lane useful
  and testable as soon as it exists; M6's agents write to the same place rather than
  inventing one. Because a rejected candidate costs nothing — the previous artifact is
  still in the library and the slot record still points at it — this is an A/B between two
  versions, not a merge tool

  **The stated producer does not exist, and it is not an agent's absence that stops it** — and
  *"gated on naming an operation for regeneration"*, which this bullet said until 2026-08-29, was
  the wrong diagnosis. **Regeneration needs no operation**: it is `WriteProcedure` with a different
  producer, and `mcp.rs`'s `write_procedure` already checks, hands back the diagnostics, writes,
  snapshots and swaps at a frame boundary. What the lane needed instead were the two operations
  nothing could say, and both landed — **`KeepCandidate`** and **`RestoreProcedure`**, addressed by
  node rather than by version.

  **The bay draws rows now, and the producer it was gated on is wired.** It built both deck slots
  with `HotSwap::fixed` — whose sender is dropped at construction — so no `swap::Event` was ever
  emitted; it builds them watched, and a library load reaches the same worker a file edit does. What
  the lane still omits is the node address, `origin`, the timestamp and the head's count, each named
  at the code. See *The remaining bays, surveyed* under *Where this goes next*, which carries the
  omission-by-omission accounting.
- The `man / sug / auto` control: Manual / Suggest / Auto — ~~**on an address this document
  has not settled**, see *What authority is set on*. This bullet said *per-layer*, which is
  one of the three answers in play and not a decision~~

  **The address is settled — per node — and the vocabulary half landed while this was being
  written.** `karakuri-operation` holds `SetAuthority { deck, node, authority }` over an `Authority`
  of `Manual` / `Suggesting` / `Automatic`, three destinations and no toggle, on
  [P-0074](principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md)'s terms; the
  operator wins and an automatic writer yields to a hand
  ([P-0078](principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md)). Its
  record is the session's and the engine holds no copy of the value list yet, which is what that
  work still owes:
  [ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md), which names the
  engine field as the thing between this and a value the chip can draw.

  **Partly done, and the region it needed is drawn.** `man / sug / auto` is a node head, the node
  head is the inspector's, and **the inspector draws its node groups with `man / sug / auto` on
  each of them** since 2026-08-29; the engine half of a per-node authority landed with it
  ([ADR-0216](adr/0216-a-node-nobody-has-spoken-for-is-manual-and-a-request-states-only-what-was-said.md)),
  restated on `swap::Request` so a rebuild cannot silently take a grant back. **What is left is a
  writer**: `Record::Authority` is deliberately excluded from Set-file state, so it can only be a
  live-session writer, and the chip can read while nothing can make it change. **Gated inside M5**,
  and no longer gated on a decision.
- **Per-slot tempo-sync and beat-sync toggles, with beat-sync greyed out for accumulating
  material.** The two are different requests and only one of them is always available:
  tempo-sync changes the *rate*, which any procedure can follow because it only ever means
  stepping more or fewer times — **except one, and this sentence was wrong about it until
  2026-08-29**: `Transport::allows` refuses `Sync::Tempo` on material that reads `beats`, because
  it is already on the room's grid by its own hand and scaling its clock as well would make it
  follow twice (`Refusal::AlreadyOnTheGrid`). **Two of the three modes are conditional, not one**,
  which is what the sync chip has to be able to say; beat-sync locks the *position*, which needs the ability to
  jump and reverse. Continuous beat-sync is therefore closed-form only — an accumulating
  slot re-running its whole history on every correction is not a feature. A *one-off* scrub
  is available to anything that fits the budget, so what the surface shows there is a price
  ("8 bars back — 340 ms") rather than a disabled control.

  Worth noting what makes the greying-out possible: `closed_form` is decided statically in
  the check pass, travels with the artifact, and arrives at the interface as a fact. Had it
  been left for the engine to infer at runtime, the surface could not have said anything
  until the Set was built and run.

  **The vocabulary is there, and the region it would live on has gone from undescribed to drawn.**
  `SetSync { deck, sync }` over `Free` / `Tempo` / `Beat` and `ScrubDeck` are both operations, and
  both already have a key. Both rows on [every operation](manual/operations.html) route their panel
  way in to a **deck head**, and **a third routes there too and is not this bullet's** — *Composite
  a deck's renderers*, which has no key at all, so that region carries three operations rather than
  the two costed here.

  **The manual had no deck head at all, which is what this bullet was gated on, and it has one
  now** (2026-08-29): under each `.half-head` in the **inspector** rather than on a preview cell,
  because a `free | tempo | beat` triplet is about 106 px against a preview cell's 112 and rule 05
  already says everything about what a deck *is* lives there. **The Inspector draws it**, and the
  arithmetic it moved is the inspector minimum above.

  **So this is a control waiting to be drawn into a region that exists**, and what is left is the
  press: three `plan` badges on the board under `deck head`, none of which is waiting on a page
  question any more. **Gated inside M5, on nothing but the work.**
- **An arrangement is saved and restored, and resetting is the special case of restoring the
  default.** ~~The panel's own state — every fold, every boundary, what is soloed — lives only in a
  running process today, and the one operation that changes it wholesale is `Reset`.~~ That is the
  wrong way round: an operator who has spent a set arranging the console wants that arrangement
  back tomorrow, and *the default* is one arrangement among the ones they could name. So `Reset`
  belongs to a family rather than standing alone, and the family is what decides its prose — the
  page promises on *Move a boundary* that a window dragged too small **forgets nothing**, and an
  operation that discards every fold and drag has to say so against that promise.

  **The default member has landed as a row**
  ([ADR-0208](adr/0208-resetting-is-the-default-case-of-restoring-an-arrangement.md)): *Reset the
  arrangement* is on [every operation](manual/operations.html) with all four routes empty, and its
  prose is written from this entry's framing — it says what it discards against *Move a boundary*'s
  promise that a window dragged too small forgets nothing. *(Its panel badge named no home when it
  landed, because the home was the family's rather than the reset's, and its key badge was empty;
  both are filled now — the home is the transport row, and `r` resets.)*

  **The rest of the family has landed**
  ([ADR-0221](adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)).
  What this entry named as missing — *a name for an arrangement, a record carrying one, and two
  operations beside `Reset`* — is answered, and one of the three is answered by not existing:

  - **A name.** An arrangement is named by a name the operator types, with a stamped id where
    nobody can type one. A content address lost to
    [P-0053](principles/0053-a-value-that-must-be-stable-is-recorded-not-derived.md)'s own example — move a divider
    and every save is a new file — and a slot number lost to the same principle's position clause.
  - **A place, and it is a fourth one.** `<store>/arrangements/<name>.arrangement.json`, because
    each of the three that exist refuses it for its own reason: an artifact is immutable and
    content-addressed, `write_set` refuses a line that is not Set state, and ADR-0208 already put
    arrangement operations in `Silent::Surface`. **The default is not a file and there is no
    reserved name** — [P-0048](principles/0048-what-ships-what-you-saved-and-what-you-are-editing-are-three-places.md)'s
    three places map onto it exactly — so `read_arrangement` must never fall back to the built-in.
  - **A record, which is the one that is answered by absence.** Both operations answer
    `Silent(Surface)`. Not `OnLanding`, where `SaveSet` sits and looks identical: that arm answers
    only when a record is written, and no record in the session vocabulary is an arrangement. Not
    `NoRecord`, which is a gap somebody owes a row for: here the absence is the decision, because a
    replay reconstructs nothing from an arrangement.
  - **Two operations beside `Reset`.** `SaveArrangement { name }` and `RestoreArrangement { name }`,
    both on the page with the transport row as their home, both with their refusals already
    written. A listing operation is not owed — the panel reads `list_arrangements` the way the
    Library bay reads `list_sets`.

  **And the control has landed, on 2026-08-30**
  ([ADR-0225](adr/0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)): the
  transport row's **arrangement pill**, with `save`, `load` as the list of names already filed, and
  `start a new one`, which is the reset — the map pill's shape, which is the argument for the row
  rather than a resemblance. **It is the first control here that takes letters**, so the wall on
  what may be typed had to be written where the file is written rather than where it is typed
  (`checked_name`, P-0076), a path `Store::write_arrangement` says outright it does not check and
  that until now no caller could reach. **And it is the first whose gesture outlives the press**,
  which cost the pointer rule a fourth clause and a stated price: a boundary cannot be dragged with
  a menu open, because the card is drawn out of a 48-tall row and across the bays under it.
  The store side is nine tests through a real file, and the panel side is `Panel::restore` rather
  than a ninth `Op`, because `Op` is `Copy + Eq` and names a `NodeId` or nothing while a whole
  `Layout` is a payload only a third party holding a store can produce. **No key is bound and the
  reason is decided rather than pending**: a bare press cannot type a name, *the most recent* is a
  handle derived from where a file sits, and a save key alone would keep arrangements nothing in
  this program can put back.

  What it did not need is a serialiser — **`karakuri-layout`'s `Layout`
  round-trips today**, an unbounded maximum is written as `null` deliberately, and a `NodeId` goes
  on the wire as the bare number it is. *(That sentence said `Arrangement` derives
  `Serialize`/`Deserialize`, and it does not: `Arrangement` is a private struct deriving `Debug,
  Clone`, and what round-trips is `Layout`, through a hand-written `Serialize` over a borrowed
  `WireOut` and a `TryFrom<Wire>` that refuses an arrangement disagreeing with itself rather than
  repairing it —
  [ADR-0158](adr/0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md).
  The conclusion survives the correction and is stronger than it was written: the load half is
  built and hardened, not merely derivable. What it did **not** say is that what round-tripped was
  a `String`: both tests went through `serde_json` in memory and neither had ever seen a file,
  which is why ADR-0221's set is nine rather than two — a defect that filed the arrangement in the
  wrong directory entirely passed both the round-trip and the solo test, because they write and
  read through the same wrong path.)* **Done as far as this milestone's own list reaches**: the
  record, the place, the two operations and the control all exist, and what is left of the family
  is a key, which is decided as *not one* rather than pending.

  The mock has the shape one level over: the Library's scope row already lists **presets** beside
  favourites and a folder, and says the scope list is itself extensible.

**~10–14 weeks from here, and the original ~8–10 was not a bad estimate of the wrong thing.** It
was written against the seven items above, and those seven turned out to be the *second half* of
this milestone: a panel to draw into, a name to route by and a frame to draw on were all assumed
and none of them existed. The section above is what the first half cost. **The pace is not what was
wrong** — `karakuri-layout` and `karakuri-console` went from nothing to five drawn regions, a
solved and round-tripping arrangement and a migrated MIDI map in five days, with the whole of
[what this milestone has delivered](#what-this-milestone-has-delivered-and-why-the-list-below-stopped-counting-it)
behind it. The list was.

**What the range assumes, and three of its assumptions have since been met.** That the four undrawn
regions — `staging`, `inspector`, `master`, `sequencer` — are the bulk of what is left, and that
they cost roughly what the five drawn ones did. **The inspector is drawn and the staging lane draws
its empty state**, so what is left of that assumption is `master` and `sequencer` — and **only the
sequencer is blocked on machinery**. The master's level exists and is read in every run; its first
pass is a drawing the size of the exposure track. The range is not simply reduced by two bays
because one of the two is still a bay nothing can draw, not because both are. That three decisions get taken rather than deferred: **who owns the pointer** (which the
parameter surfaces and every tooltip in the console wait on), **what a thumbnail is of** (M4's, and
judged here), and **the arena's insert and remove** (which the node editor and the inspector's `2
up` both want, and whose remove half moves every `NodeId`) — **all three are still open.** That the
manual gains a deck head, before the panel does: **it has one**, written on 2026-08-29 under each
`.half-head` in the inspector, and the Inspector draws it, so that assumption is met and the three
operations routed there are waiting on nothing but the press. And that the vocabulary keeps converging — `panel::Op` and
the CLI's match arms onto `karakuri-operation`, one at a time.

**What is not in the range.** The five operations still named `Undecided`, if any of them turns out
to need designing rather than naming; a second `Sink`, which the fan-out is built for and which
nothing has been written for; and **the sequencer's producer** — the region is costed above, and
what fills it is a step grid and a pattern record, neither of which exists.
[ADR-0222](adr/0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md) is what that is now
known to be: *a step sequencer is one more name on `karakuri-signal`'s bus*, which this sentence
carried until 2026-08-29, was false in both halves, and a lane is a fifth route into the vocabulary
rather than a binding. **Widen the top of the range rather than the bottom if those land here.**

---

### M6 — Autonomy

**Goal:** the system prepares material without being asked and is safe to let run.

**Adds**

- Agents, and **what each one is attached to is still undecided.** This bullet asked for one
  per *deck slot* rather than per node of a Set; the architecture roster above said *node
  agents*. **What is no longer part of that question is the authority**: it is set per node
  ([ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)), and an
  agent scoped to a deck slot can respect a per-node authority perfectly well — so this
  granularity is a question about agents and not about rule 06 any more. Whatever it is
  attached to, each owns a prompt,
  watches designated inputs, selects from its prepared variants, and queues generation for
  material it expects to need. Autonomous selection means choosing among what already exists,
  never generating on a path the frame waits for
- Director. Coherence across the **kinds** — whether the camera behaviour suits the
  geometry, whether L4 is killing the shape L1 produced
- Mix agent. Set-level arc, energy trajectory, monitoring for staleness
- Authority model. Budget constraints ("camera changes at most once per four bars"),
  and manual intervention instantly demoting to `Suggest` whichever agent was moving the
  control — *which* agent that is depends on the undecided granularity above. **What the
  demotion writes is decided**: `Operation::SetAuthority` on the node, and
  [P-0078](principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md) is
  the rule it is the standing case of
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

**A keyboard is mapped and learned the same way a control surface is.** The keys are a
`match` in `karakuri-cli` today, and there is no reason they should be: the vocabulary is what
made a MIDI map possible, and a key is another way to name the same operation. The grammar is
already written — `examples/surface.map` says `<target> <deck> <value>`, and a key is a third
`Key` beside `Cc` and `Note` — so the map file's parser is nearly all of the work, and whether
that lives in `karakuri-midi` or moves out of it is the question that goes with it, since a
crate named for a wire would then be parsing something that never touches one.

**Learning it belongs on the control, which is the maintainer's own model**: a GUI component is
a translator between one Karakuri operation and N ways in, so *learn this control* is one
gesture from one place, and MIDI is only the first of the N. **Both learn paths wait on the same
missing mechanism**, and it is worth knowing which: **this console draws no tooltips at all**.
`view.rs` says so where the Outputs row and the mixer would have had them — a tooltip needs
`egui` to own a widget and this console paints — so the mock's `⊕ MIDI: note 41` has nowhere to
be shown yet, and neither would a key.

**And the mock has no key to show even where a tooltip could show one.** It draws no key hints
anywhere, and the tooltip contract it does carry is the control's *MIDI assignment* — so **the
console as drawn gives an operator no way to learn a key binding at all**, which is rule 01's
keyboard surface with no teaching path. Found while separating `h`/`?` from `s`
([ADR-0205](adr/0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md)), where the
CLI's answer to the same need — reprinting a static bindings text into a terminal — is what the
console has no equivalent of. It is one hole with two halves: this entry's *learn* gesture needs a
tooltip to hang on, and a key hint needs somewhere to be drawn in the first place.

**What a keyboard map has to name that a MIDI map never did**: twelve of the CLI's thirty-nine
keys write no record at all — the deck selection, the transition settings, the window, quitting
— because they are the surface's own state (ADR-0194's group (c), ADR-0198). Every operation a
map line can produce today writes a record, so a keyboard map is the first map file that would
name operations the record layer never sees, and what a map file may say is then a slightly
larger question than *which keys*.

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
