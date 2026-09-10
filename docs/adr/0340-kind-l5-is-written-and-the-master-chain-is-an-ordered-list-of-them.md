---
id: 0340
title: kind L5 is written, and the master chain is an ordered list of them
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0064, 0085, 0086, 0091, 0092]
tags: [ir, codegen, engine, console, format, store, vocabulary, m5]
---

# `kind L5` is written, and the master chain is an ordered list of them

## Context

**Two records deferred this and each named exactly what would end the deferral.**

[ADR-0317](0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md) made the
master chain **three hand-written passes in the engine** — feedback, bloom, rgb shift, between the
mix's write and the tone map — and refused a writable `L5` as *deferred rather than refused*, with
the revival condition written into the rejection: it *"is the better answer to a different
question: it buys operator-written frame effects, closes `+ add`, and makes the deck count a
property of a procedure rather than of one built-in shader"*, not taken because *"it is a language
change bought to draw three rows the console already draws"*. **What would revive it is what
`ir-spec.md` already names: somebody writing the compositing down.**

[ir-spec.md](../ir-spec.md)'s [kind](../ir-spec.md#kind) states `L5`'s absence as **a condition
rather than a principle** — a `kind` says what a procedure *lowers to*, the compositing is fixed,
so the one L5 that exists has no code to lower and a `kind L5` file would have nothing to
contain — and says the same thing from the language's side: *"admitting frame effects is giving L5
a writable form, and a written form is code to lower."*

[architecture.md](../architecture.md)'s *The Layer Model* had already worked out that the algebra
does not have to move for it: `node/merge.rs` writes the signature `[Texture] -> Texture`, fan-in
is solved by `uses` plus `edge`, and the deck count stops being a system constant. **Feedback was
the one exception** — reading the previous frame is a cycle — and ADR-0317 took that half already,
retaining one cut and letting the parameter choose.

### What changed on 2026-09-10, and it is a library rather than a language

[ADR-0338](0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md)
makes **a procedure a row of the Library**, with a badge saying which layer it implements, a filter
row over the kinds, and a load that writes one `.kir` over one layer of what a deck is playing. So
the shape *drag a part out of the library and drop it on something* exists, for every kind but one.
The question this record answers is whether the master chain is the something.

The maintainer's, on 2026-09-10, asked exactly that:

> 進める（M5.15 の後）

**Proceed — after M5.15.**

## Decision

**`kind L5` joins the language, the master chain becomes an ordered list of L5 slots, the three
fixed passes ship as procedures under `examples/`, and the Library drops one onto the chain.**
After M5.15 lands, which is the only thing it waits on.

**Everything in this record is specification.** Nothing here is built, `Kind::ALL` is five,
`parse.rs` is untouched and `kind L5` is still refused where it is written; the build is
[roadmap.md](../roadmap.md)'s M5.16. [ir-spec.md](../ir-spec.md)'s *L5, written — not built* is the
specification and this is the argument for it; where the two could disagree, the specification is
the one that moves first (`docs/contributing.md` §6).

### 1. The kind

**`L5 : [Texture] -> Texture`**, one required block called `frame`, which is a fullscreen fragment
body over the incoming texture. It is [ADR-0098](0098-l5-is-one-node-kind-with-two-roles.md)'s one
kind with two roles intact: a frame effect is that signature with one input, the mixer is the same
with several, and what differs between the two roles is still only whether a surface is attached.
`crate::node::Merge` keeps its place beside the written kind exactly where the built-in orbit
camera keeps its place beside `kind L3` — a node with no procedure has no `kind`, gets no `slot`
record, and is described by a record of its own.

**It reads** the incoming texture (`src`, implicit — what supplies it is the chain's position or
the Set's `edge`s, and a procedure that named its supplier would be coupled to one arrangement),
`point_coord`, `t`, `beats`, `dt`, its own `param`s, and `held` where it declares `retains`. **It
does not read** `seed`, `copy`, `source`, `point`, `capacity`, `camera`, `eye` or `ray` — the first
three because a frame pass has no element, which is the fullscreen L4's rule rather than a second
one, and the last three because the frame in front of an L5 may hold several decks' material seen
from several cameras, so there is no viewpoint for them to name.

**It declares no geometry and no `vertex` block**, and the only slot type it may take is
`uses … : Texture`, any number, bound by an `edge`. **A chain slot's L5 declares none of those**:
the chain's only fan-in is its order, and there is no Set for an `edge` to be written in.

### 2. `retains` is bare, and the cut is the chain's answer

A procedure declares `retains` — *this reads a retained frame* — and that is all it says. **Which
cut is the slot's answer**, `mix` or `exit`, ADR-0317's two words unchanged so that no second
spelling enters for a value the record already carries.

That split is [P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md) at the width
of one declaration, and it is the same division `uses` and `edge` already draw. It is also what
makes ADR-0317's cost rule expressible: **the engine allocates a retention only where a slot's
answer names one**, at most two ever — one per cut, however many slots read them — so *a cut that
is read has to be held* stays a price somebody asked for.

**The retained frame is sanitised at the copy rather than at the read**, which moves
`master.wgsl`'s `keepable` one step earlier so that a `.kir` neither needs the guard nor can leave
it out.

### 3. Three builtins, and no other language change

| | |
|---|---|
| `texel(Texture) -> vec4` | this fragment's own texel, unfiltered, **no coordinate argument** |
| `tap(Texture, vec2) -> vec4` | a filtered sample at a frame coordinate |
| `frame_step(float) -> vec2` | a distance as a **fraction of the frame's height**, in the coordinates `tap` takes |

`texel` takes no coordinate on purpose: a coordinate is an invitation to resample, and a centre tap
that resamples makes a pass at an amount just above zero differ from one that did not run by what
a filter did rather than by what the effect is. `frame_step` is the load-bearing one — a
displacement across a frame is the one thing a frame effect cannot express without the render size,
and no ambient carries it (P-0086, and one frame is rendered at the largest enabled output's size
and scaled into the rest, [ADR-0247](0247-one-frame-is-rendered-and-scaled-into-each-output.md)).
So the conversion is a builtin that performs it **without handing the number over**, and a `.kir`
has no way to write a radius in texels.

Loops are untouched: literal bounds, no `while`, no recursion, nesting multiplying into the
estimate
([ADR-0252](0252-the-language-is-bounded-so-a-price-can-be-computed-before-anything-is-built.md)),
which is what prices a tap count before anything is built.

### 4. The chain is an ordered list of slots, and zero is not a skip

A slot holds one L5 procedure, its params, and its `cut` where the procedure declares `retains`.
`src` of the first slot is what the mix wrote; `src` of every other is what the slot before it
wrote; what the last writes is what the tone map reads. **One chain, at the master**, which is
[ADR-0227](0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md)'s reading
unchanged.

**The default chain is empty**, so the default look is bit-identical for free rather than by a
zero-amount rule.

**ADR-0317's zero-skip is not carried forward.** *An amount of zero records no pass at all* existed
because the three passes were **always loaded** and there was no other way to stop paying for one
nobody was using. A list has a better spelling for *no pass*: no slot. So a slot in the chain runs,
at zero as at one, and a chain costs the slots it holds rather than a function of the values in
them — which is the shape [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) prefers
on its own terms, a constant cost rather than one that appears and disappears under a hand riding a
fader through zero.

### 5. Cost: one axis, the fullscreen ceiling, and the output's size

An L5 is priced on **`ops_per_fragment`** and is zero on the other two axes, having no element and
no spawn — the [`Field`](../ir-spec.md#the-field-block)'s shape rather than a new one, and
[ADR-0013](0013-cost-has-three-axes-that-must-not-be-added.md)'s three axes still never summed.

**The ceiling is `MAX_OPS_PER_FULLSCREEN_FRAGMENT`, per slot**, for the fullscreen renderer's
reason verbatim: 512 stands in for `capacity` sprites times their area times whatever they overlap,
and an L5 covers the frame exactly once with nothing overdrawing, so the stand-in has nothing to
stand in for.

**A chain's cost is the sum over its slots, and that is not the addition ADR-0013 forbids** — those
are three different quantities, these are rates against one, the frame's texels covered once by
every slot. It is said out loud because it looks like the forbidden thing.

**In milliseconds it is that figure against the frame's area at the largest enabled output's
size** (ADR-0247, [ADR-0325](0325-the-frame-follows-the-largest-enabled-output-and-a-resize-costs-0-145-ms.md)),
which is what the governor spends where the estimate answers
([ADR-0296](0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)),
charged against the frame beside the decks rather than against any one of them, and any measurement
of it names which resolution it is about
([ADR-0303](0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md)).
**Where each is checked is the validation pipeline's own division extended and not amended**: stage
4 rejects one artifact against a per-fragment ceiling and knows nothing about a chain; a chain has
a total where `capacity` and the frame budget do, at stages 6–8.

### 6. The record grows a list where it had four scalars

```ndjson
{"t":"master_chain","slots":[
  {"proc":"sha256:a3f2c1…","cut":"exit","params":{"amount":0.5}},
  {"proc":"sha256:77b9e0…","params":{"amount":0.8}}]}
```

**Whole, and ADR-0317's argument for that is unchanged**: a stream that moved one slot without
saying where the others stood describes a chain a replay cannot put back, and the place that turns
an operation into the record already knows the chain that is running and fills the rest in
([ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
`proc` is a content address, so the **procedures** become part of the state a replay reproduces:
a stream naming one the store does not hold is refused with the address in the message.

**A Set file gains nothing for the chain** — it is one level out from every Set, so a Set carrying
it would reconfigure the master the moment it was loaded into any deck (ADR-0227) — **and one line
for a nested L5**, which now has a procedure to reference and so takes a `slot` record like any
other node, with the built-in mix keeping the `merge` record. That is *Several cameras*'
arrangement exactly.

**The library tier is the same list under a name**, on
[ADR-0221](0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)'s shape.
ADR-0227 left the file to *the record that has something to serialise*, and this is the first time
there is something: the `slots` array and nothing else. The directory's name stays with the
*Mx — TODO* entry that already holds that half.

### 7. The surface: one row for a parameter, two for the list

**The three named rows retire and one replaces them.** *Feedback*, *Bloom* and *RGB shift* are rows
naming passes that are always there; with a list, a surface asking for *feedback* is asking for a
pass that may not be in the chain, and **what a surface can say is this slot's this parameter**
(ADR-0192). So [every operation](../manual/operations.html) gains *Set a chain effect's parameter*,
addressed by position in the chain and parameter key, and loses the three.

**Two rows for the list itself** — *Add an effect to the master chain* and *Remove an effect from
the master chain*. Reordering is drawn by no control in the mock and is named here as what the page
will owe rather than decided: a chain the operator can only append to and empty is a real limit and
is recorded as one.

**The way in is M5.15's, and one half of it transfers literally while the other does not.** A row
with an `L5` badge, the kind filter's seventh toggle, and the carry — the drop mark says *where* and
never *whether*, and the chain's rows become a third set of rectangles a release can land on
([ADR-0265](0265-a-carried-set-names-its-deck-at-the-release-and-the-panel-refuses-no-drop.md),
[ADR-0273](0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)). **The re-aim is
reused for a nested L5 and not for the chain**: ADR-0338's load writes one file over one layer of a
deck's material and sends an aim, which is exactly right for `L5:0` of a Set and wrong for the
master, because the chain is not any deck's material and no aim carries it. A drop on the chain
appends a slot and writes `Record::MasterChain`.

**The Master bay's `+ add` becomes real**, and it leaves the arena's gap for the Sequencer's own
reason: `+ lane` left that list on 2026-09-09 because a lane is a row from `Pattern::lanes` and not
a region, and a chain slot is a row from the chain for the same reason. The arena still has no
insert and no remove, and this does not need one.

### What is forced and what is chosen

**Forced.** That an L5 runs in linear HDR, unclamped, **before** the one tone map and the one sRGB
encode ([P-0064](../principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
That every length is a fraction of the frame's height and never a count of texels, which is why
`frame_step` exists and why no ambient carries the render size (P-0086, ADR-0247). That a procedure
knows only what it declares — `src` implicit, the cut answered by the slot, no node named in a file
(P-0086). That the retained frame is part of what a replay reproduces
([P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)). That targets are
allocated at build and at resize and never when a parameter moves (P-0091). That the cost is
computable before anything is built (ADR-0252) and the three axes are not summed (ADR-0013). One
kind, two roles (ADR-0098). One record, written whole (ADR-0317).

**Chosen, and a wrong one is a one-clause revision.** The block's name `frame` and the two reserved
names in it, `src` and `held`. The three builtins' names and shapes. `retains` bare with the cut
answered by the slot rather than `retains mix` in the file. No zero-skip. A chain slot's L5
declaring no `uses`. `uses … : Field` refused on an L5. One chain at the master rather than one per
deck. The shipped `bloom.kir` as a single 81-tap pass. And that the four-field `master_chain`
record is refused rather than read.

## Alternatives rejected

**Keep the three passes fixed.** What is running today, and it is not wrong — ADR-0317's argument
for it stands on its own date, and nothing here says that record should have decided otherwise. It
loses now for the reason that record itself gave for deferring rather than refusing: the language
change is no longer *bought to draw three rows the console already draws*. ADR-0338 has since made
a procedure a row of the library with a drag onto material, so the L5 gets that surface for free,
and what the chain gains is a thing the fixed form cannot do at any price — an effect nobody in
this repository wrote, in an order the operator chose, with `+ add` a control instead of a note.
**The cost of keeping it fixed is now visible where it was not**: every new frame effect is a
fourth entry point in `master.wgsl`, a fourth field on a record, a fourth row on the operations
page and a fourth control in the bay, and that is a bill paid per effect forever.

**A writable L5 as a `Field` over the frame.** The tempting cheap answer, because `Field` is
already the kind with only code to lower and no node: write the frame effect as `vec2 -> vec4` and
splice it. It is refused on what a `Field` *is*. A field has no target, no pass and no position in
the chain — that is exactly why it needs no buffer and why its cost is *per evaluation*, an axis of
its own, with the number that has to fit under a ceiling being **the caller's** with the field's
multiplied in. A frame effect has all three: it owns a pass, it has a position, and it is the
caller. Filing it under `Field` would either make a field mean two things — the name-means-one-thing
rule `docs/contributing.md` §4 is written against — or give the cost estimate a `Field` whose
ceiling is nobody's. And the fan-in would be lost: a `Field` cannot take a `Texture`, so the nested
role would need a second mechanism, which is the *one kind, two roles* that ADR-0098 settled being
undone to save a `kind` line.

**A chain per Set instead of per deck.** Refused by ADR-0227's own argument at full strength: the
chain is one level out from every Set on the panel, and a Set that carried the chain's settings
would reconfigure the master the moment it was loaded into any deck — the same failure
`record.rs` names for a Set carrying its gain, *"a Set file reaching outside the Set"*. **Per deck
is a different proposal and is not refused here**: architecture.md's *a per-deck effect and a
master effect become one node at different points* is a thing this kind makes possible, and nothing
in M5.16 builds it. What is decided is that there is **one** chain and it is the master's; a
per-deck chain is a second list at a second point, and it will want its own record, its own rows
and its own budget line.

**`retains mix` — the cut as an operand in the file.** Shorter, and it puts the two words where a
reader of the `.kir` can see them. It is refused by P-0086 and by ADR-0317 together: which cut is
read is *a picture the operator picks between* — that is the whole reason ADR-0317 made it a
parameter rather than a design decision — and a file that fixed it would be two procedures where
there is one, `feedback_mix.kir` and `feedback_exit.kir`, with the operator's choice spelled as a
library swap. It would also make the retention's cost a property of which files are in the chain
rather than of what the slots asked for, which is the same number arrived at less legibly.

**A zero-skip declaration — `identity when <param> == 0`.** The literal way to carry ADR-0317's rule
into a written chain. It is refused on what P-0091 asks of a classification made ahead: *trusted at
runtime, so it is conservative*, and nothing static can check this one. A wrong declaration is not
a diagnostic, it is a pass silently not running — the failure mode P-0091 names for claiming closed
form loosely, *"permissive-wrong … while telling the governor it needed no warming"* — and the
thing it buys is available for free by removing the slot.

**Reading both record shapes.** The old four-field `master_chain` beside the new list, so sessions
recorded between 2026-09-09 and M5.16 replay. Refused on
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md): before v1 a
compatibility cost is a bill rather than an argument, and this bill is a handful of sessions
against a decoder carrying two shapes of one record for the length of the project. What is owed for
it is a **refusal that says which shape it found**, on
`a_record_this_build_cannot_obey_says_so_rather_than_vanishing`'s terms — not a silent default,
which here would play an empty chain where the session had three passes.

**Bloom as two chain slots.** The obvious way to keep the 19-fetch separable blur: `bloom_bright`
then `bloom_blend`, two slots in a row. It does not work and the reason is structural rather than
awkward — the second half needs **both** the bright buffer *and* the frame the bright pass was
taken from, and a chain slot's output replaces the frame, so by the time the second slot runs the
frame it needs is gone. Letting it name the earlier value is a graph, and the chain is a list. So
the shipped `bloom.kir` is one 9×9 pass — 81 fetches per texel against the pair's 19, at the same
radius and the same picture — and **M5.16 owes a fresh measurement**, ADR-0317's 0.80 ms having
been taken on the two-pass form. What would close it is a procedure that can name a second output;
nothing owes that.

## Consequences

**Dated 2026-09-10 and written as what M5.16 builds**, to be rewritten by that work as a
description of the tree. **The language half and the chain half are both built as of
2026-09-10**; what is unmarked below is M5.16's second pass, which is the surface.

### The language half — built, 2026-09-10

- **`karakuri-ir` has a sixth `Kind` and a fifth block.** `Kind::L5`, `BlockKind::Frame`, the
  `frame` block, the bare `retains` header declaration, `SlotTy::Texture` on `uses`, and
  `Builtin::Texel` / `Tap` / `FrameStep` in `Builtin::ALL` — which is also how they reach a
  model, the MCP curriculum rendering the checker's own table rather than a second copy
  (`docs/contributing.md` §4). `check.rs` carries the refusals above, each with a negative
  control beside it (`docs/contributing.md` §3); the eight refused ambients each name the fix
  rather than the rule (P-0083).
- **A texture is not a value, and that is structural rather than a rule.** There is no `Ty` for
  one: the first argument of `texel` and `tap` is a `Shape::Texture` position that the check pass
  resolves as a *name*, and `TExprKind::Sample` carries which binding rather than an expression.
  So `let x = src;` is refused at the read with both builtins named, and it can never become a
  value by accident. **This is the one shape not written down above**, and it is the answer to a
  question this record did not ask.
- **`cost.rs` prices an L5 on `ops_per_fragment` alone**, zero on the other two axes, and holds
  it to `MAX_OPS_PER_FULLSCREEN_FRAGMENT`. `fragment_ceiling` switches on the kind as well as on
  `topology == Fullscreen`.
- **`karakuri-codegen` has `l5.rs`**, one fullscreen pipeline over `master.wgsl`'s own bind group
  layout — uniform, `src`, `held`, sampler, then the Texture slots, with binding 2 left empty
  rather than renumbered where nothing is retained — and `naga_test.rs` validates the three
  shipped procedures and asserts `feedback`'s and `rgb_shift`'s expressions against the
  hand-written bodies term for term (`docs/contributing.md` §7).
- **`examples/feedback.kir`, `examples/bloom.kir` and `examples/rgb_shift.kir` exist**, compile
  through every stage, and cost **17**, **3889** and **41** ops per fragment against a ceiling of
  4096.
- **Bloom is at 95% of the ceiling, and the measurement this record owes is now overdue rather
  than merely owed.** The ceiling was calibrated against a raymarcher — thirty-odd iterations of
  a distance function — and not against a blur, and the three weights behind the figure (`texel`
  2, `tap` 4, `frame_step` 2) are the least examined numbers in `cost.rs`. A tap weighted at a
  transcendental's 8 refuses a 9×9 kernel outright, which would be a ceiling saying no to a shape
  every post-processing stack ships. **What the estimate is worth here is a question a GPU
  answers**, and nothing has asked one.
- **A `kind L5` compiles and has nowhere to run**, which is stated as a decision rather than left
  as a gap: `compile::sort_compiled` refuses one where a slot is loaded, with the reason, and
  MCP's `LAYERS` stays five because advertising a layer a slot cannot hold would offer a client
  an address that cannot resolve. `Set`'s node addressing answers `L5` with an empty range on the
  terms `L3` did before the built-in camera became a node.
- **The vocabulary's sixth word landed with the kind**: `karakuri_store::record::Layer::L5`,
  `karakuri_operation::Layer::L5`, `LibraryKinds`' seventh boolean, `history::LAYERS`,
  `setfile::kind_name` / `layer_named` / `layer_ordinal`, and `declared_kind` reading `L5` back.
  **The two word tables with a wildcard needed naming rather than compiling**, which is what
  `layer_from_ordinal`'s own comment predicted one kind earlier: a match on a `&str` has no
  compiler behind it, so `karakuri/src/main.rs`'s `kind_of` and `setfile::layer_named` would each
  have read a `kind L5` file as declaring nothing.
- **`docs/ir-spec.md`'s specification moved rather than being restated.** The kind, its three
  builtins, its cost and its lowering are in the present tense above the *Beyond* boundary; what
  is left below it is the chain — the list, the cuts, the record, the surface — under *L5's chain
  — specified, not built*.

### The chain half — built, 2026-09-10

- **`karakuri-engine`'s `master.rs` is a list.** `Chain` is `Vec<Slot>`, a `Slot` is a compiled L5
  with its cut and its params, `Present::set_chain` installs one and `Present::set_chain_params`
  moves a running one's values without allocating. The three hand-written fragment entry points
  are gone and `tests/master.rs` holds each shipped procedure to the pass it replaced on a GPU:
  **feedback and rgb shift are bit-identical**, and bloom is within 0.1% of the frame's peak,
  which is a bilinear tap's difference and not a different kernel.
- **What `master.wgsl` kept is the retention**, and it kept it because of a clause of this record:
  the retained frame is sanitised **where it is written**, so `keepable` had to become a pass
  rather than a `copy_texture_to_texture`. That is the one hand-written thing left in this chain,
  and it is exact for every value that is a number.
- **The targets are the entry, up to two to ping-pong between, and one per retained cut.** An
  empty chain takes **none** — where four were always allocated — and three slots with one cut
  take four. **The entry is held apart from the pair on purpose**: it is what makes the `mix`
  cut's copy position-independent, which a list needs and ADR-0317's between-two-passes copy
  could not give. Both cuts are copied at the chain's end and are the same two pictures.
- **Measured at 1280x720** (Metal, Apple M4 Pro, 2026-09-10, `examples/master_cost.rs`, medians
  over 100 submissions with 20 discarded cold, each over that run's own empty-submission floor):
  **feedback 0.16 ms** at the mix cut and **0.13 ms** at the exit cut, **bloom 0.92 ms**, **rgb
  shift 0.15 ms**, **all three 0.98 ms** — 5.9% of a 60 Hz frame, each over that run's own
  empty-submission floor of 0.019 ms and stable to about 3% over five runs. Beside ADR-0317's
  0.38 / 0.41 / 0.80 / 0.34 / 1.14 on the same machine: two of the three got **cheaper** — a
  retention is now a fullscreen pass rather than a 7.03 MB texture copy, and the chain holds
  fewer targets — and bloom is the one that got dearer.
- **Bloom's measurement is the one this record owed, and it is 0.92 ms against the pair's
  0.80.** 1.2x for 4.3x the fetches, because eighty-one taps into one frame hit cache where a
  second pass hits bandwidth. **So nothing is owed a second output.** The alternative *Bloom as
  two chain slots* stays refused and the reason it named — a procedure that can name a second
  output — stays unowed, now on a number rather than on a fear.
- **What the same run says about the estimate is the sharper finding.** `cost.rs` prices the pass
  at 3889 of 4096 and the three-slot chain at 3947, and the GPU says that chain is 5.9% of a
  frame.
  The three weights behind it (`texel` 2, `tap` 4, `frame_step` 2) over-predict a cached tap by
  about four to one here. That is the question this record said a GPU would answer; re-weighting
  is a change to every artifact's price and is named rather than taken.
- **`Record::MasterChain` is `{"slots":[…]}`**, `mix::change`'s arm reads it back with each cut
  word refused rather than defaulted, `karakuri-cli`'s replay resolves each slot's address before
  the frame that needs it, and `crates/karakuri`'s `apply` writes the list onto `Engine::chain`
  which the frame loop puts on the `Present` where the two differ. **The four-field form is
  refused with the key it found named** — `#[serde(deny_unknown_fields)]` on this variant and on
  no other, which is what this record asked for in place of a silent default.
- **The shipped three are compiled into `karakuri-environment`** as `mix::shipped`, addressed by
  the hash of their bytes, and a chain of presets therefore resolves with no store at all. Putting
  them *in* a store stays a separate act, on `Placed::put`'s own division: a run that records
  nothing creates nothing.
- **`SetFeedback`, `SetBloom` and `SetRgbShift` still work**, which is this record's *for now*:
  each resolves to the slot whose procedure is the matching shipped address, **appending one where
  the chain has not got it**, and every other slot comes from the chain that is running.
  `karakuri_operation_record::Chain` carries the list and the three addresses beside the three
  amounts the Master bay's rows still draw.

### Still M5.16's, and it is the surface

- **[Every operation](../manual/operations.html) loses three rows and gains three**, and the page
  moves first (`docs/contributing.md` §5). **This reopens a closed sub-milestone's exit if the two
  halves land apart**: M5.8 closed on *no `plan` badge in the panel column of this bay's rows*, and
  three rows retiring in favour of three that start at `plan` is that grep going red. M5.16 owns
  both halves, which is why they are one sub-milestone and not a note on M5.8.
- **`karakuri-operation` loses `SetFeedback`, `SetBloom` and `SetRgbShift`** and gains
  `SetChainParam`, `AddChainEffect` and `RemoveChainEffect`, each classed in `gate.rs` with no
  wildcard arm, and `karakuri-operation-record` says what each writes. ADR-0338's `LibraryKinds` is
  seven named booleans rather than six.
- **`Record::MasterChain` changes shape**, `mix::change`'s arm with it, and `karakuri-cli`'s replay
  driver resolves each slot's address against the store before the first frame.
- **`karakuri-store` gained the record's new shape and nothing else**; a `chains/` directory is
  still owed when the library half is taken, which is *Mx — TODO*'s.
- **Nothing spends the chain's cost.** `Present::chain_ops_per_fragment` is the sum over the slots
  and is read back off the running chain; no governor is handed it, because nothing sets an
  estimate on this side of the frame at all (ADR-0325 recorded the same gap for a slot's).
- **Where a slot is compiled is the second pass's to move.** `mix::build_chain` compiles and
  builds pipelines on the thread that applies the record — which is before the frame's encoder
  exists on every path that calls it, and is a press rather than a fader ride — where a Set's
  build runs on `HotSwap`'s worker (ADR-0033). The Library's drop is what makes that worth
  moving.
- **`docs/architecture.md`'s two edits are made**, both having been sentences about a count
  rather than about the layer: *The Layer Model*'s L5 paragraph now says the kind is built and the
  chain is not, and *Words that carry more than one sense* says there are six, with
  `karakuri_store::record::Layer` and `karakuri_operation::Layer` grown to match.
  `docs/ir-spec.md`'s *A note on the word* says it from the other side, and what it had to give up
  is the clause *"only `L1` through `L4` are kinds"* — `L0` is the only model position with no
  kind behind it now, and the two senses stay distinct anyway.
- **ADR-0317 is annotated by this record rather than edited.** Its deferral is taken up and its
  revival condition met; its *Alternatives rejected* already says what would do it, and an ADR is a
  description of history ([ADR-0151](0151-an-adr-is-a-description-of-history.md)). Two of its
  clauses are superseded and both are named above — the zero-skip, and the four-field record — and
  the rest of it, the two cuts and the retention rule above all, is carried forward intact.
- **No new principle.** This applies P-0064, P-0085, P-0086, P-0091 and P-0092 and takes nothing
  back from any of them.
- **Blocked on nothing.** M5.15 closed on 2026-09-10, which is what this record waited on; the
  Library row, the kind badge, the filter row and the drag are ADR-0338's and are there to be
  reused.
