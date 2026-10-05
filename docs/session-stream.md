# Session Stream Format

What the engine actually consumes is a **timeline**, and it lives in its own file. Same
ndjson, same records, plus `tick` — a session stream is a **head** saying what the deck held,
followed by the record of what happened. The head is specified under
[The head](#the-head--what-the-deck-held-at-frame-0) below; at its simplest, on a deck of one,
it is a Set file:

```ndjson
{"t":"set","id":"drift_01","v":1}
{"t":"slot","layer":"L1","proc":"sha256:a3f2c1…"}
{"t":"slot","layer":"L4","proc":"sha256:9c1b04…"}
{"t":"tick","steps":1}
{"t":"tick","steps":1}
{"t":"param","layer":"L1","key":"radius","value":2.6}
{"t":"tick","steps":1}
{"t":"bind","layer":"L1","key":"turbulence","signal":"energy","curve":"pow2","range":[0.1,2.4]}
{"t":"tick","steps":1}
```

Keeping the two files apart pays in both directions. A Set file stays a few dozen lines a
human can read, instead of accumulating 216,000 `tick` records an hour. And a session gains
something a Set file cannot express: every edit lands at an exact frame position, because
it sits between two known ticks. That is what makes replaying a live performance exact
rather than approximate.

A Set file is one slot's session stream with the ticks dropped and the state folded down.
Saving a Set is that projection; loading one is a session whose head describes a deck of one
and whose tail has not been written yet.

### Measurement in the stream — `audio` and `tempo`

Live audio is not reproducible, and the same record stream is required to reproduce the
same output bit for bit. Those can only both be true if **the measurement joins the
stream**, which is exactly what `tick` already does with elapsed time: derived from the
world when live, read back verbatim on replay, and the engine cannot tell which happened.
Audio adds two records on those terms, and no third path.

```ndjson
{"t":"audio","energy":0.42,"onset":0.75,"bands":[0.9,0.4,0.2,0.11,0.05,0.02,0.01,0.0],"confidence":1.0}
{"t":"tempo","bpm":128.03,"shift":-0.0041,"confidence":0.86}
{"t":"tick","steps":1}
```

**`audio` is one frame's worth of measured signals**, at most one per frame, before the
tick it belongs to. Named fields for the named signals and a positional array for the
bands, because `band0`…`bandN` *are* positions — the array is the naming scheme rather
than a second one, and a map of names would both repeat those names 200,000 times an hour
and let a stream invent names the bus has rules about. The array's length is the band
count, so a stream with more bands than a reader knows about still decodes.

One `confidence` for the whole line, not one per signal: these values came out of one block
of samples at one instant, so their staleness is one number. It is **full while a device is
open and delivering, falling as the last block goes stale, and 0.0 when nothing has
arrived**. A silent room is `energy` 0.0 at confidence 1.0 — a measurement, which drives a
bound parameter to the bottom of its range — and an interface pulled out mid-set is the
same zeroes at a confidence sliding to 0.0, which hands every bound parameter back to
whoever set it. The two are different lines, and the difference is the whole reason
confidence is a number rather than a flag.

**A frame with no `audio` record is not a frame of silence.** It is a frame with no
provider, and every name answers exactly what it answered before audio existed — `energy`
and the bands invented at confidence 0.1, `onset` at 0.0 and confidence 0.0. Nothing
branches on which of the two it is; the bus is complete either way.

**`tempo` is what the local oscillator is corrected to**, and it carries the *correction*
rather than the estimate behind it: a new `bpm`, and `shift`, a phase shift in beats,
positive meaning the next beat arrives sooner. Recording the decision rather than the
observation is what keeps a session replayable after the analyser has been improved — and
it is why replay needs no audio at all. The first `tempo` in a stream is also what sets the
session tempo, which **v0.2 had no record for**.

Neither is state, so neither appears in a Set file: both are what a frame *saw* or
*decided*, and the tempo belongs to the session rather than to any one Set.

### The mix in the stream — `gain`, `opacity`, `blend`, `mask`, `transition`, `select`, `residency`, `look`, `master_out`, `master_chain`, `canvas`, `procedure`, `authority`, `ride`, `source` and `transport`

A session that carried the material and not the performance would replay the same Sets, on
the same beat, all at whatever gain they happened to start at, with nothing ever going on
or off air. Sixteen records carry what an operator moves — fourteen states and two events:

```ndjson
{"t":"gain","slot":0,"value":0.75}
{"t":"opacity","slot":0,"value":0.5}
{"t":"blend","slot":1,"mode":"over"}
{"t":"mask","slot":1,"kind":"linear","angle":0.0,"position":0.0,"softness":0.02}
{"t":"transition","slot":1,"control":"mask","to":1.0,"start":64.0,"beats":8.0,"curve":"smooth"}
{"t":"select","slot":0,"renderer":1,"start":64.0}
{"t":"residency","slot":1,"level":"priming"}
{"t":"look","op":"aces","exposure":1.2,"white_point":4.0}
{"t":"canvas","width":1920,"height":1080}
{"t":"procedure","slot":0,"layer":"L4","proc":"sha256:486779…"}
{"t":"authority","slot":0,"layer":"L1","authority":"manual"}
```

**`gain` is a deck slot's level into the mix**, and **`opacity` is its fader.** The slot is
a position on the deck, not anything about the Set in it: moving a Set to another slot moves
it under another fader, which is what a fader is.

The two are separate records because they are separate controls, and what makes them
separate is `blend`. Every mode composites its **colour** as
`acc <- mix(acc, f(acc, gain * src), opacity)`: gain is the level the material arrives at
and touches colour alone, opacity is how much of the blend lands and is the only one of the
two that scales what a deck slot's layer *covers*. Under `add` they collapse into one
multiply and a stream carrying either would replay the same; under `over` one dims that
layer and the other stops it hiding what is beneath.

Coverage is the exception to that formula and composes as `over` under every mode, because
"there is material at this texel" is an `over` question even when the colour is being added.
`opacity` is a proportion of a blend and is clamped to `[0, 1]`; `gain` is a level into an
HDR mix and deliberately is not clamped above 1.0.

**`blend` is how a deck slot's layer meets the ones under it** — `add`, `over` or `max`. Slot
order is stacking order, so this is the one mix control whose meaning depends on where the
slot sits.

The set of modes is chosen by what survives an **unbounded linear HDR** mix rather than by
what a VJ mixer usually lists. `screen` is `d + s - d*s` and `multiply` is `d * s`; both
assume display-referred inputs in `[0, 1]`, and nothing has tone mapped this far up the
pipeline — `screen` of two 2.0s is 0.0. They belong after the transfer curve or not at all.

`over` needs to know what a deck slot's layer covers, which is why **alpha in a slot target is
coverage**: the L4 pass accumulates `1 - prod(1 - a_i)` there while colour adds, so the
colour that reaches L5 is premultiplied and emissive material still sums past what its
coverage would allow. Sparse material barely covers, so `over` on a thin point cloud reads
close to `add` — which is correct rather than a defect, since a handful of sprites does not
occlude anything.

**`transition` is a mix control moving over musical time** — a fade, a cut, or half of a
crossfade. One record for the whole move, and **the values it produces are not recorded**: a
value per frame would be 216,000 lines an hour describing something the grid already
determines, which is the argument `tick` makes from the other end.

`start` is an absolute position on the session's beat count rather than "in two bars",
because a relative instant is a different instant depending on when it is read. Quantising
to the next bar happens where the operator asked, once. `beats` of 0 is a cut. **`from` is
deliberately absent**: it is read where the move is *scheduled*, and a reader replaying the
stream reads it the same way. Capturing it at the start instead would mean capturing it on
the first frame at or after a musical instant, and a machine running at a different rate
would capture it at a different beat — which is the one property this record exists to
have. Nothing can move the control in between: a hand cancels the move and another move
replaces it.

`control` is `gain`, `opacity` or `mask`. The third is the `position` a `mask` record
carries, and `mask` is its spelling on the wire — a transition names the record whose
number it moves rather than the field inside it — which is the whole of what makes the
wipe described below a scheduled move rather than a mode of its own. `curve` is a `bind`'s
vocabulary — `lin`, `pow2`, `sqrt`, `smooth` — because a fade's shape and a signal's shape
are the same question. There is no `crossfade` record and there should not be: a crossfade
is two of these sharing a start and a length, a fade-in is one, and a cut is one with a
duration of zero. The first-class thing is the move.

**`select` is which renderer of a slot's Set is the live one**, from a musical instant on.
The other scheduled event, and a record of its own rather than a `control` on `transition`,
for the reason that vocabulary already states: the things a transition moves are
**positions**, and this is a **choice**. "Half way to renderer 2" does not name a picture, so
there is no `beats` and no `curve` — a selection is a cut, and a cut is a fade of zero beats
with nothing left to interpolate. `start` is absolute, on `transition`'s terms and for its
reason.

`renderer` is an index in draw order, the numbering `--param L4:1:key=value` and a `slot`
record's `index` use. It says something only where the slot was built to composite: an
overdrawing slot's renderers share one attachment and have no edges into an L5, so a `select`
on one is carried, replayed and without effect — which used to be the ordinary case, because a
Set file could not record its layering and one loaded back overdrew. It records it now, in the
`merge` record, so a composited Set saved and loaded back still composites. A `renderer` the
slot does not draw with is refused where the record is applied rather than where it is
decoded, since how many renderers a slot has is a property of the Set in it and can change
under a hot swap between the schedule and the beat.

**What a `select` still cannot be folded into is that record**, and the reason is now on the
session's side rather than the Set's. A selection is addressed to a **deck slot**, and no
session record says which slots composite, nor which deck slot the Set at the head of a stream
was played in — a session head is one slot's material and the stream never says whose. So a
projection folding a session down cannot tell whether a selection it meets is about the Set it
is writing, and folding it into a `merge` would be guessing that it is. A selection stays a
property of the run, like a gain, until a session record says otherwise.

**What it selects between is one geometry drawn several ways.** The renderers are L4s over
one simulation, so this is the half of a variant pool that a Set can hold — see
[ADR-0148](adr/0148-a-variant-pool-is-a-set-and-the-deck-stays-a-mixer.md) for the other
half, alternatives that differ at L1, and for why the two halves' costs are different
costs. And it is the *fold* that skips an unselected renderer, never the draw: every
one of them still fills a frame-sized target of its own, which is what makes the choice a
uniform write and what it costs to have it be one.

**`mask` is what shape of the frame a deck slot's layer reaches.** It multiplies that
layer's opacity per texel, which is what makes it a mask rather than a second fader:
everything opacity does — how much of the blend lands, and under `over` how much that layer
covers — is
what a mask wants done to part of the frame. `kind` is `none`, `linear` or `radial`;
`angle` is the linear front's, in radians; `position` is how far it has travelled, `[0, 1]`,
**exact at both ends** — 0 reveals nothing anywhere and 1 reveals everything everywhere, for
any `softness`.

**The two lines above are a wipe, and there is no `wipe` record.** A mask at position 0 on a
deck slot's layer that blends `over`, and a `transition` carrying `mask` to 1: the front hides what is
beneath it exactly where it has passed. Neither half knows about the other — the transition
moves a number and the mask reads one — which is the same shape as a crossfade being two
`transition`s. Both ends being exact is what makes it *finish*: a `position` of 1 that left
a corner half-lit would be a wipe that stopped short.

What is deliberately not here is a mask read from a **texture** — an arbitrary shape, or
another deck slot's luminance. That needs somewhere for the shape to come from, and the answer
is a [`Field`](#the-field-block) rather than a third `kind`.

**There was a `preview` record and it has been removed.** It carried which slot the output
was showing — `{"t":"preview","slot":2}`, `null` for the mix — and it was in the stream for
one reason: the preview *was* the output, so a replay that ignored it would have shown the
mix where the operator was looking at one slot. That reason expired, and this section said
it would. Switching a preview turned out to be a **bay-internal** move rather than an engine
one: the Program bay's picture is the master mix and its four deck preview cells monitor
every live deck continuously, so nothing about a performance changes when one is looked at
and there is nothing to record
([ADR-0240](adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md)).
A stream written before this holds `preview` lines; they are an unknown `t` now and are
passed over by the rule above, which is what that rule is for. **The name is not reused**
— `thumbnail` is the library asset, and that stays (see the metadata file format below).

### The head — what the deck held at frame 0

**The head is every record before the first `tick` that is not a frame's own.** A frame
writes its edits, then what it heard, then the tick that closes it, so an `audio` or `tempo`
line in front of the very first tick is that frame's measurement and belongs to it; every
other record before that tick describes the deck a replay is about to build.

It has two halves, and they are two vocabularies rather than two positions:

```ndjson
{"t":"set","id":"drift_01","v":1}
{"t":"slot","layer":"L1","proc":"sha256:a3f2c1…"}
{"t":"slot","layer":"L4","proc":"sha256:9c1b04…"}
{"t":"param","layer":"L1","key":"radius","value":2.6}
{"t":"seed","stream":"L1","value":19274}
{"t":"canvas","width":1280,"height":720}
{"t":"gain","slot":0,"value":1.0}
{"t":"opacity","slot":0,"value":1.0}
{"t":"blend","slot":0,"mode":"add"}
{"t":"residency","slot":0,"level":"live"}
{"t":"mask","slot":0,"kind":"none","angle":0.0,"position":1.0,"softness":0.0}
{"t":"transport","slot":0,"sync":"free","anchor_bpm":120.0,"scrub_beats":0.0}
{"t":"procedure","slot":1,"layer":"L1","proc":"sha256:7d40ae…"}
{"t":"procedure","slot":1,"layer":"L4","proc":"sha256:9c1b04…"}
{"t":"gain","slot":1,"value":0.4}
{"t":"opacity","slot":1,"value":1.0}
{"t":"blend","slot":1,"mode":"over"}
{"t":"residency","slot":1,"level":"live"}
{"t":"mask","slot":1,"kind":"none","angle":0.0,"position":1.0,"softness":0.0}
{"t":"transport","slot":1,"sync":"free","anchor_bpm":120.0,"scrub_beats":0.0}
{"t":"look","op":"aces","exposure":1.0,"white_point":4.0}
{"t":"master_out","value":1.0}
{"t":"master_chain","slots":[]}
{"t":"tick","steps":1}
```

**One Set file, for slot 0.** A Set file describes one Set, and it is the only thing that
carries a Set's params, bindings, seeds, edges, capacity, camera and layering. It is slot 0's
and never whichever slot was selected, because a replay numbers its slots the way the deck
did and *which slot is described in full* must not depend on where a hand was.

**One `procedure` record per node of every other slot.** The same record a hot swap writes
mid-stream and a replay already obeys: a deck slot, a node of it, and the store address its
source is at. The bytes are in the store before the head names them — a record naming bytes
nobody kept is the same silence as no record at all. Those slots are built against the head's
Set file: its parameter table and its bindings, because a run has one of each and writing a
second copy per slot would be the same numbers said twice. What a `procedure` record cannot
carry, it does not — a slot's own capacity and salt are the procedure's declared defaults,
and a value ridden onto another slot before the recording began is not in the head.

**Then the deck, always and for every slot.** `canvas` first, because a replay reads it before
it allocates anything. Then, slot by slot in deck order, that slot's `procedure` records —
where it has any — followed by its `gain`, `opacity`, `blend`, `residency`, `mask` and
`transport`; then `look`, `master_out` and `master_chain`, which belong to the fold rather
than to anything folded. **Every one of them is written whether or not it differs from what a
fresh deck holds.** A head that omitted the defaults would need a reader that knew them, and
a reader that knows the engine's defaults is a second place they are written down.

A replay builds a deck as wide as the head names — one slot more than the highest any
`procedure` record names — installs each named slot, applies those records in order, and then
reads frames. A head that names no other slot is a deck of one, which is every session
written before this.

What a head is **not** is a resume. A Set file carries no running state, so a recording begun
mid-performance replays the same material from the top rather than continuing the picture that
was on screen; for a closed-form renderer the two are the same and for an accumulating one
they are not.

**`residency` is what a slot is asked to do** — `live`, `priming` or `allocated` — and it
is always the *request*, never the effective level. The governor recomputes the second
every pass from the budget of the machine that is running, so a session recorded on a fast
machine and replayed on a slow one must re-derive it; recording what was decided would
replay one machine's budget onto another's. That is the opposite of the choice `tempo`
makes, and for the opposite reason: there, what was decided is the reproducible thing.

**`look` is the output look, all of it in one line.** Tone map operator, exposure and white
point together, because it is one value written to one uniform, and a stream that could set
the exposure without saying which operator it applies to would describe a look nobody can
reconstruct. Session-wide rather than per slot, since tone mapping happens once, after the
mix.

**`master_out` is the level the composited frame enters the master chain at**, and it is
deliberately not a fourth field on `look`:

```ndjson
{"t":"master_out","value":0.75}
```

The two are levels and they multiply in **different places** — this one where the mix writes
the composited frame, `look`'s `exposure` where the present pass reads it, with the master
chain's effects between them
([ADR-0224](adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)).
Folding them into one line would be recording their product, which is exactly what a replay
could not take apart the day an effect lands between them. Session-wide and never per slot,
for `look`'s reason: it is applied to the fold rather than to anything folded. Floored at
zero and **open above 1.0**, because the pipeline is linear HDR and this level is applied to
values no tone mapper has seen yet.

**`master_chain` is what the master chain is**, whole: the ordered list of its slots.

```ndjson
{"t":"master_chain","slots":[
  {"proc":"sha256:a3f2c1…","cut":"exit","params":{"amount":0.5}},
  {"proc":"sha256:77b9e0…","params":{"amount":0.8}}]}
```

The chain runs between `master_out`'s level at its entry and `look`'s exposure at the tone
mapper's input. **The order is on the line because it stopped being a constant**: a slot holds one
`kind L5` procedure named by a content address, and a chain is whichever of them an operator put in
it ([ADR-0340](adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)).
`params` is keyed by the name the procedure declared, a param nobody moved is absent and the
declaration's own default is what runs, and the range each is brought into is the one the
declaration gave it. `cut` is present exactly where the procedure declares `retains`: `mix`, the
frame as the mixer wrote it before this chain touched it — one echo and not a trail — or `exit`,
this chain's own output, which compounds. An **empty** `slots` is a real value and the default one:
an empty chain draws nothing and the frame is the mix, bit for bit.

**Written whole, which is `look`'s argument at one more row.** The same 0.5 is a one-frame echo
under `mix` and a compounding trail under `exit`, so an amount without its cut is not a picture
anybody can reconstruct, and a stream that moved one slot without saying where the others stood
would describe a chain a replay could not put back. A record per slot was refused on exactly that;
the place that turns a press into this one already knows the chain that is running and fills the
rest in, the way a `look` record is completed with the operator a control change cannot say.

Session-wide and never per slot, for `master_out`'s reason: the chain reads what the fold produced,
after every deck's edge has been applied. **And a record of its own rather than a fifth field on
`master_out`**, which is that record's own argument against being folded into `look`: the out is a
**level**, ridden continuously by a fader, and this is a set of **settings** moved by a press —
folded, a fader ride would rewrite four settings sixty times a second and a settings change would
rewrite the level a hand was holding.

Every value is brought into the range its procedure declared where the record is **applied** and at
no surface, and a cut word this build has not got is refused rather than defaulted: a default there would not report a
wrong level, it would silently play one echo where the session had a trail. A replay decodes the
line in `karakuri-cli`'s `mix::change` and puts it back through `mix::apply_chain`, which reaches
the same `Present::set_chain` a live press does, so no path sets a chain behind the record's back.
`--replay` starts from an **empty** chain, which draws nothing at all, because no flag names a slot
of this chain and a flag writes into a record rather than inventing one
([ADR-0046](adr/0046-a-flag-writes-into-the-record-it-does-not-invent-one.md)). **What has no writer
yet is the press**: no key on the CLI's surface names a slot, so a stream `--record-session` writes
carries none of these today.

**The four-field shape this replaced is refused rather than read.**
`{"t":"master_chain","feedback":…,"cut":…,"bloom":…,"rgb_shift":…}` is what streams recorded
between 2026-09-09 and M5.16 carry; the decoder says `unknown field 'feedback', expected 'slots'`
with the line number, because a silent default here would play an empty chain where the session had
three passes. See [*L5's chain*](#l5s-chain--built-and-its-surface-is-not).

`level`, `mode`, `op` and `cut` are strings for the same reason `curve` and `noise.kind` are: an
unrecognised value is the engine's to diagnose against what it actually supports, not the
decoder's to reject before anything can say what the alternatives were.

**`transport` is what a slot's clock does with the session's**, and it carries two numbers
because the mode alone does not mean anything without them:

```ndjson
{"t":"transport","slot":0,"sync":"beat","anchor_bpm":126.0,"scrub_beats":-0.25}
```

`anchor_bpm` is the tempo at which this material runs at 1×, and it has to be recorded
because **material has no intrinsic tempo**: a `.kir` declares parameters and a capacity,
not a bar length, so "one beat of music is how many seconds of material" is an operator's
answer rather than the artifact's. `scrub_beats` is where the operator scrubbed the slot to, in
beats off the room's position — signed, unbounded, and the one value in this format that is meant to
go backwards. **It was `offset_beats` until 2026-08-29**, and the rename reached the wire on purpose:
the word *offset* also names the latency offset, which the record format is expected to gain, and a
pre-rename line fails loudly rather than reading as a zero scrub, because the field has no serde
default. Both are carried under every mode,
including `free` where neither does anything, so that a slot moved back onto the grid
returns to where the operator left it rather than to a default.

**`canvas` is what the session renders at**, in texels — the surface every deck slot's
layer draws into, what every deck slot is sized to match, and what an offscreen render
writes.

It is **not the size of any window.** A window is a preview of what leaves by some other
route, so it is fitted to the canvas rather than the other way round: dragging one changes
what an operator can see and nothing about what is drawn. Until this record existed the two
were the same number, and a session played in a small window and replayed at a large one
rendered different pixels with nothing in the stream saying which was the performance.

**Written once, at the head, and a stream carries no second one** — which makes it the only
record here that is session state without being something a hand can reach mid-set. Changing
a canvas reallocates every slot's target and the frame path allocates nothing, so the canvas
is a property of a *run*. A reader that meets a later one should report it rather than obey
it: a replay sizes everything it allocates from the first, before any frame exists.

**`procedure` is the material a deck slot is playing, from that moment on.**

```ndjson
{"t":"procedure","slot":0,"layer":"L1","proc":"sha256:9da973…"}
{"t":"procedure","slot":0,"layer":"L4","proc":"sha256:486779…"}
{"t":"procedure","slot":0,"layer":"L4","index":1,"proc":"sha256:5e7d20…"}
```

Written when a hot swap lands and when one is rolled back — the two moments the material
actually changes. **Without it a session recorded the material once, before the first frame,
and replayed the whole run with whatever it started with**: a set in which a procedure was
rewritten at minute ten replayed as though it never had, silently. That was survivable while
the only way to rewrite one was a human with an editor; it stopped being survivable when a
model could, because rewriting procedures is the whole of what that surface does.

It is the **session's** and not the Set's, which is what `is_set_state` is for: a Set file's
`slot` record says what a Set *is*, and this says what a deck slot *became*, at a point in
time. That is a fact about a performance.

`proc` is a content address and the source lives in the store, on the same terms `slot`
uses — so a rewrite costs one line here and a few kilobytes once, however many times the
same procedure comes back. **Every one of a slot's procedures is written together** even
when only one changed, because a Set is built from all of them and a reader that rebuilt on
the first would compile an L1 against the L4 it is replacing.

`index` says **which node of that layer**, since a slot draws with one L1 and however many
L4s. It is 0 for the L1 and for the first renderer, and **absent when it is 0** — so a
stream written before stacks existed replays byte for byte, and a new one carries the field
only where it says something.

**Who may move one node of the Set a deck slot is playing** is `authority`.

```ndjson
{"t":"authority","slot":0,"layer":"L1","authority":"manual"}
{"t":"authority","slot":2,"layer":"L4","index":1,"authority":"suggesting"}
{"t":"authority","slot":1,"layer":"Field","authority":"automatic"}
```

Three levels — `manual` is yours alone, `suggesting` proposes and waits, `automatic` acts —
and the address is a **node**, which is the manual's sixth rule: *"There is no switch that
hands the whole instrument to an agent, because the useful arrangement is almost always
partial."* A flag beside the `slot` and nothing else would be that switch at deck
granularity.

**It is a session record and not a Set file's**, which is the one thing about it worth
stating twice. Every other record that names a node of a Set — `slot`, `capacity`, `param`,
`bind`, `seed` — goes in a Set file and carries no `slot`, because what a Set *is* does not
depend on which deck slot it is playing in. A Set does not know which agent is watching it
either: an authority is an arrangement made during a performance, and a Set file carrying one
would hand that node over wherever it was next loaded. So it is addressed the way `procedure`
is, and it is the second record in this vocabulary to name a node.

`index` says which node of that layer and is **absent when it is 0**, on `procedure`'s
terms. A `kind Field` node takes one like any other: it draws nothing and its params are
still declared, addressable and an operator's to ride. The L5 that folds a Set's renderers
cannot be addressed at all — `crate::node::Merge` is a node and an L5 has no `kind`, so there
is no `layer` value that names it, and nothing on it can be moved by anybody today either.

The level is carried as a word and not interpreted here, on `residency`'s terms: what a level
is allowed to be is the engine's to say.

**A rebuild carries it and the console writes it.** `swap::Request::authorities` is restated
on every rebuild, so a node an operator granted to an agent — or took back from one — comes
up granted after the next save of any `.kir`; the `man / sug / auto` chips on a node head are
the three destinations a press names. **What it does not yet do is refuse anything on its
own**: nothing in this system writes a parameter on an agent's behalf, so a level is not
consulted at an addressed write. What it does reach is the wildcard refusal — a bare `key`
over nodes that are no longer all under one authority is refused whole from the next press —
and the record is kept so that an agent's write can be refused against it when something
writes one. See
[ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md) and
[ADR-0319](adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md).

**What a live grant does not survive is a re-point.** A rebuild restates what the *watcher*
was handed, and a grant made during a run is not written back into it, so re-pointing a slot
starts it with no grants — which is what loading material is. It is in the stream as an
`authority` record either way.

**`ride` is a parameter an operator moved on a deck slot that is playing**, and it is the
third record here to name a node of the Set in a slot.

```ndjson
{"t":"ride","slot":0,"at":{"layer":"L4","index":1},"key":"glow.x","value":0.4}
{"t":"ride","slot":2,"key":"exposure","value":2.0}
{"t":"ride","slot":1,"key":"glow","value":[0.4,0.7,1.0]}
```

**It is the session's twin of `param`, and the two are two records rather than one grown a
`slot`.** That is `slot` and `procedure`'s arrangement exactly — both say *this node runs
this procedure*, one in a Set file and one in a stream — and the reason is the one stated at
`authority` above: what a Set *is* does not depend on which deck slot it is playing in, and
what an operator *did* is addressed to a deck slot or it is addressed to nothing. A `param`
in a stream can only mean the Set at the head of it, and a deck holds four.

**The address is one field and not two.** `at` absent is *every node declaring `key`* —
`param`'s wildcard, and the useful default: one knob moving every renderer that has an
`exposure`. `at` present names one node, and `index` inside it is absent when it is 0, on
`procedure`'s terms. Written this way because a wildcard names no node and therefore no
layer, so a `layer` beside an `index` would have to carry a placeholder for it — which is
what `param`'s `layer` is, and what that record's *present or absent as a unit* is prose for.
Here nothing can write half an address down.

**`key` is a component key where the parameter is a vector** — `glow.x` and never `glow` —
because a parameter is driven one component at a time. A wide `value` is legal for `param`'s
reason: it is one line a person or a model writes, and the reader that has the Set in hand
expands it into one write per component.

**A bare `key` is refused where the nodes it lands on are not under one authority**, and the
refusal names them — the same refusal a `--param` and a `param` meet, in the same place,
because there is one entry point into a Set's parameters and every route comes through it.

**It goes in a session stream and never in a Set file**, and it is the sharpest of the drops
a projection makes: it names a layer, an index, a key and a value, so a fold *could* key it
and the result would look right. What it also names is a deck slot, and a Set file has no
deck: a stream's head says which slot each Set was played in —
[The head](#the-head--what-the-deck-held-at-frame-0), above — and a projection folds to *one*
Set, which is then loaded into whatever slot an operator picks, so folding a `ride` into it
would carry one slot's value onto another's material. The value an operator ended on reaches
a Set file the other way, through `save`, which reads the live Set. Decided in
[ADR-0280](adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md).

**What no reader does yet is put it back after a rebuild.** A `--watch` rebuild restates the
parameters its request carries, and a value a knob moved is not one of them, so a save of any
`.kir` walks that knob back to where the slot was loaded. That is a live-run defect and not a
replay one — a replay meets `procedure` records and then these, in that order, at the frames
they happened — and ADR-0280 records why closing it is a decision about what a rebuild *is*
rather than about where a value is kept.

**`source` is what is driving one parameter of a deck slot that is playing, or nothing**,
and it is the fourth record here to name a node of the Set in a slot.

```ndjson
{"t":"source","slot":0,"layer":"L1","key":"turbulence","source":{"signal":"energy","curve":"pow2","range":[0.1,2.4]}}
{"t":"source","slot":2,"layer":"L4","index":1,"key":"exposure","source":{"signal":"noise","curve":"lin","range":[0.8,1.6],"noise":{"kind":"perlin","rate":0.5}}}
{"t":"source","slot":0,"layer":"L1","key":"turbulence"}
```

**It is the session's twin of `bind`**, on exactly the terms `ride` is `param`'s: a `bind`
says what a Set *is* and carries no deck slot, and this says what an operator *did*, to one
deck slot, at one instant.

**One record for the attachment and for taking it back**, and `source` present or absent is
which. The third line above is *Take a parameter back*. They are one fact — what is driving
this parameter — so two records would be two things a reader has to keep in step by care;
present or absent as a unit is a thing nothing can write down half detached, which is the
argument `ride`'s `at` field carries one record along.

**Taking a parameter back removes the attachment rather than suspending it.** There is no
suspended state anywhere for a record to carry: *not driving this parameter* is already
written, and it is the absence — the blend under *What a binding does* writes the param's own
value when nothing is attached. What is given up is handing it back in one press, and what
replaces it is this record: the signal, the curve and the range are in the stream on the line
that attached it.

**The address is `bind`'s and not `ride`'s, and the difference is not a drift.** A value
lands wherever the name is declared, so `ride`'s wildcard names no layer; a binding is
resolved through the nodes of **one** layer, so `layer` here is always said and is read, and
`index` absent is *every node of that layer declaring `key`*. There has never been a binding
that meant every layer, so an address that could ask for one would be an address the engine
refuses for a reason the format already knew.

**A hand on the value does not appear here.** Writing a bound parameter moves the value the
binding blends *from* and leaves the attachment where it is — order-independent by
construction, which is what blending on confidence buys — so nothing an operator does to a
knob detaches a signal by accident. Only this record attaches one and only this record
detaches one.

**It goes in a session stream and never in a Set file**, for `ride`'s reason read one field
along: a `source` names a layer, an index and a key, so a fold could key it, and it also
names a deck slot, which nothing in a stream can attribute. The attachments a Set ends up
with reach a Set file the other way, through `save`, which reads the live Set and writes
`bind` records.

**What no reader does yet is put it back after a rebuild**, which is `ride`'s open question
in the same words: a rebuild restates the bindings its request carries and an attachment made
live is not one of them, so a save of any `.kir` walks it back. Decided in
[ADR-0319](adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md).

**One thing it cannot carry.** A rollback restores the outgoing Set at the `t` it was parked
at; a reader meeting these records builds afresh, so `t` restarts there. A swap *in* is
defined to start cold and therefore replays exactly — only a rollback differs, and a
rollback means the candidate was over budget, which is an exceptional frame already.

**These fifteen stay out of a Set file**, thirteen because they are state that is the
session's rather than any Set's and `transition` and `select` because they are not state at
all.  `select` is out for a second reason of its own, and it is no longer that a Set file
cannot say whether a slot composites its renderers — the `merge` record says exactly that,
and a composited Set now loads back composited. It is that no **session** record says it: a
selection names a deck slot, and nothing in a stream says which of the deck's slots
composite. The head says what each slot *held* —
[The head](#the-head--what-the-deck-held-at-frame-0), above — and a projection folds to one Set
out of a deck of them, so it cannot tell whether a selection it meets is about the Set
it is folding, and folding it into a `merge` would be guessing that it is. That is a second
reason for a record to be absent from a Set file and it is not the `audio` one: there is
something to fold here, and this is not the projection it folds into. A Set file that
restored a gain would apply it to whatever slot it was next loaded into, and one whose
`residency` said `live` would put a Set on air by being opened. They belong to a session stream, which
`--record-session` writes and `--replay` reads. What does not exist is a *session*
projection — folding one down to the deck state it ends at — because nothing needs to
resume a deck yet; folding a session down to a **Set file** does exist and is specified
above.

**Signal names.** `energy` and `band<N>` are the ones the synthesized bus already answers,
deliberately: a measured `energy` and an invented one are the same signal from different
sources, and a binding that had to be rewritten when a microphone appeared would defeat the
arrangement. `onset` is new — a **decaying envelope in `[0, 1]`**, 1.0 at a detected
transient and falling from there, so that a `bind` with a `curve` reads it as a hit. It is
not an impulse: analysis blocks and rendered frames are not locked to each other, so an
impulse one block wide would be missed by some frames and counted twice by others. The
synthesized bus does **not** invent an `onset`, because an invented one would be `beat`'s
pulse under a second name.

Levels are `[0, 1]` because `curve` and `range` are defined over that: they are RMS in
dBFS, mapped from −60 dBFS to −6 dBFS, and a band reads the level of the part of the signal
inside it on the same scale. So a full-scale tone pins, a well-mastered track lives in the
top third, and silence is exactly 0.0.

### Records with an effect outside the stream — `save`

Every record above describes the deck, and a reader that obeys it reproduces the
performance. **One record describes something else: a file that was created.**

```ndjson
{"t":"save","slot":0,"id":"20260816-143052-271"}
```

**`save` says a deck slot's material was written out as a Set file**, under that id, at that
point in the timeline. It carries the slot and the id and deliberately nothing else — the
Set file under that id already names every node it holds, and a copy of them here would be
one fact in two places, free to be right on the day it was written and wrong the moment the
two are read apart. What this record is for is saying *that* a save happened and *what it is
called*; what was saved is a question the Set file answers.

That makes the general rule worth stating, because it is the first record to need it:

> **A replay is a sandbox.** Some records have an effect outside the stream. A replay does
> not perform those effects, and it says which ones it skipped. Bringing outside state into
> a replay environment is the operator's responsibility.

For `save` the reasoning is concrete. A replay that wrote Set files would be writing into
ids that already exist, in a store nobody asked that run to touch, holding somebody else's
material — and `--replay` would stop being a function from a stream to some frames. So it
renders the frames and prints the id it passed over. **Said rather than silently dropped**,
on the terms everything else in this system follows: a Set file load prints every note it
could not honour, and the session writer counts the batches it lost rather than losing them
quietly. The id is named because it is the thing an operator would go and load by hand if
they wanted the material that record is about.

**This is a second question about the vocabulary, not a third reason inside `is_set_state`.**
That function asks *whose state is this* — the session's or a Set's — and `save` is refused
by it for the ordinary reason: a save is a fact about a performance, and a Set file carrying
one would claim, every time it was opened, that a save had just happened. *Does this reach
outside the stream* is orthogonal, `save` is so far the only record for which the answer is
yes, and folding the two together would give one function two jobs.

**Written at the frame the save landed, not at the key press.** The store write happens off
the render thread, so it finishes some frames later and can fail; a record written when the
key was pressed would claim a file the disk then refused. That is the same rule `procedure`
follows — a record that describes a change already made — and it is why **a failed save
writes no record at all** and prints instead.

What this record does *not* do is make a replay reconstruct the library. There is no
`--replay` mode that writes the Sets a session saved, and there should not be: reconstructing
them means writing into a store, which is the effect the rule above exists to keep out.

---
