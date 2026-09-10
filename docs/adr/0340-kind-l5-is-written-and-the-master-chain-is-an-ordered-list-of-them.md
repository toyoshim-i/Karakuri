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
description of the tree.

- **`karakuri-ir` gains a sixth `Kind` and a fifth block.** `Kind::L5`, `BlockKind::Frame`, the
  `frame` block, the `retains` header declaration, the `Texture` slot type on `uses`, and three
  builtins in `Builtin::ALL` — which is also how they reach a model, `mcp.rs` serving the checker's
  own table rather than a second copy (`docs/contributing.md` §4). `check.rs` gains the refusals
  above, each with a negative control beside it (`docs/contributing.md` §3).
- **`cost.rs` prices an L5 on `ops_per_fragment` alone** and holds it to
  `MAX_OPS_PER_FULLSCREEN_FRAGMENT`. `fragment_ceiling` currently switches on
  `topology == Fullscreen`, which an L5 does not declare; it switches on the kind as well.
- **`karakuri-codegen` gains `l5.rs`**, one fullscreen pipeline per slot over the chain's bind
  group layout, and `naga_test.rs` covers the three shipped procedures — the checklist item that
  is not optional for this crate (`docs/contributing.md` §7).
- **`karakuri-engine`'s `master.rs` stops being three passes and becomes a list**, `master.wgsl`
  goes with the three procedures that replace it, and `present.rs`'s targets become two plus one
  per retained cut instead of four.
- **`examples/` gains `feedback.kir`, `bloom.kir` and `rgb_shift.kir`**, a second kind of shipped
  part beside `surface.map`, and nothing in this program writes there (P-0096).
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
- **`karakuri-store` gains nothing for the session half** and a `chains/` directory when the
  library half is taken, which is *Mx — TODO*'s.
- **`docs/architecture.md` owes two edits this record does not make**, both being sentences about a
  count rather than about the layer: *The Layer Model*'s L5 paragraph is annotated here, and
  *Words that carry more than one sense* says *"There are five"* kinds, with
  `karakuri_store::record::Layer` and `karakuri_operation::Layer` named beside it. Both become six
  the day M5.16 lands, and `docs/ir-spec.md`'s *A note on the word* says the same thing from the
  other side.
- **ADR-0317 is annotated by this record rather than edited.** Its deferral is taken up and its
  revival condition met; its *Alternatives rejected* already says what would do it, and an ADR is a
  description of history ([ADR-0151](0151-an-adr-is-a-description-of-history.md)). Two of its
  clauses are superseded and both are named above — the zero-skip, and the four-field record — and
  the rest of it, the two cuts and the retention rule above all, is carried forward intact.
- **No new principle.** This applies P-0064, P-0085, P-0086, P-0091 and P-0092 and takes nothing
  back from any of them.
- **Blocked on M5.15 and on nothing else.** The Library row, the kind badge, the filter row and the
  drag are ADR-0338's, and none of them is worth building twice.
