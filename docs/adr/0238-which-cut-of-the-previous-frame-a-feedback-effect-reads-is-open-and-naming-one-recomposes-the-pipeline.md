---
id: 0238
title: Which cut of the previous frame a feedback effect reads is open, and naming one recomposes the pipeline
status: accepted
date: 2026-09-01
supersedes: []
superseded_by: []
principles: [0036, 0051, 0067, 0068, 0085]
tags: [ir, engine, render, console, docs]
---

# Which cut of the previous frame a feedback effect reads is open, and naming one recomposes the pipeline

**This record settles nothing.** No cut has been chosen, and none is proposed here. What it does is
state the question, check each step of the argument against the code and the specification, and
name what answering it would cost. It is `accepted` because this registry has two statuses,
`accepted` and `superseded`, and neither means *open*. Read the status as *this record stands*, not
as *this decision was taken*.

## Context

`docs/manual/operations.html` was rewritten on 2026-09-01 from prose into a matrix, and each
operation's explanation became a tooltip of roughly 270 characters. One row did not fit.
*Feedback*'s prose ran 1,941 characters and carried an argument recorded nowhere else; the tooltip
now holds 365 of it, and `docs/manual/console.html` says only that *"Feedback is the exception:
reading the previous frame is a cycle, and wants a decision of its own"*. `grep` over `docs/adr/`
finds no record of the rest.

The argument survived in two places outside the manual: `docs/roadmap.md`'s no-home section, under
*Which cut of the previous frame a feedback effect reads, and what holding it costs*, and the doc
comment on `Operation::SetFeedback` in `crates/karakuri-operation/src/lib.rs`. Both are present
tense and both rot. This record is where the reasoning now lives.

**What is not recovered is the wording.** The 1,941-character original is in git history and is not
reproduced here. What is reproduced is its three steps, each checked against something that can be
read today rather than restated.

*Feedback* is the sharpest of the three master-chain rows because it is the one no surface reaches
and no surface can be given. On the operations page it carries no `has` badge at all: `panel master
chain` and `MCP` are `plan`, `key` and `MIDI` are `gap`, and there is no CLI badge. The dashes are
not an oversight in the drawing.

## The question, in three steps

### 1. The previous frame is not one thing, so a procedure has to name which cut it reads

There are at least three candidates and they are different pictures:

- **A Set's output.** A Set is *"a named subgraph with exactly one `Texture` output"* — `ir-spec.md`,
  *What a Set is, exactly*. Every deck slot produces one.
- **The raw composited frame the mix wrote**, at the entry to the master chain.
  `crates/karakuri-engine/src/mix.rs` applies the master out *"where that frame is written"*.
- **The frame as it stands after some effect in the chain.** The chain is feedback, bloom, rgb
  shift, in that order, between the master out and the exposure the present pass applies —
  [ADR-0224](0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md).

They are not one image at different gains. `console.html`'s *Master, and the two levels that are not
one level* makes the difference concrete on this exact effect: *"a feedback trail fed at half the
level decays from half, and one fed at full level and dimmed afterwards decays from full"*. A frame
effect reads what it is handed, so which cut is handed to it is the picture.

**And nothing in the vocabulary can say which.** A procedure declares the inputs it takes by name
and type, and the Set binds them — that is `uses` plus `edge`, and a `.kir` may not name a node of a
Set ([P-0068](../principles/0068-a-kir-never-names-a-node-of-a-set.md)). The types a slot may have
are `ast::SlotTy`: `Geometry`, `Field`, `Camera`, `Source`. **None of them is a texture.** There is
no declaration a procedure could write that means *the frame*, let alone *this cut of it*, and the
`edge` record on the other side has nothing to bind.

### 2. A cut that is read has to be held, and holding it costs memory for as long as it is held

A cut read next frame is a cut retained across the frame boundary. That is a frame-sized
`Rgba16Float` target — `Present::HDR_FORMAT`, 8 bytes a texel — plus the write that fills it each
frame, because the target a pass writes is the target the next pass reads, and reading the previous
frame means not overwriting it.

The bill is already written down for the shape beside it.
`crates/karakuri-engine/src/deck.rs` states it rather than leaving it to be rediscovered: *"four
targets at 1280x720 `Rgba16Float` is 8 bytes a texel, 7.03 MB each, 28.1 MB for the deck"*. A
retained cut is one more of those per cut retained, and it scales with resolution rather than with a
Set's capacity.

**Nothing today retains a frame.** Double buffering retains one previous generation of *element*
buffers, which is a different thing — `ir-spec.md` corrects exactly this confusion where it withdraws
the claim that a derived `velocity` needs a third buffer: *"double buffering retains only one
previous frame — true — but the obstacle it points at is worse than that"*. No frame-sized target
survives a frame anywhere in `karakuri-engine`.

So holding every cut against the chance that something reads one is a cost nobody would pay, and
which cuts are held has to follow from which are named. That is the step that turns the question from
a parameter into a graph change.

**It also lands in the budget that is not enforced.** `crates/karakuri-engine/src/governor.rs` says
the governor *"does not budget VRAM"* — the deck's size is two budgets
([ADR-0026](0026-the-deck-is-l5s-surface-and-its-size-is-two-budgets.md)) and only the compute one is
buildable today. A held cut is VRAM, so nothing would refuse it for being too expensive.

### 3. So naming one recomposes the pipeline rather than adding a parameter to it

A cut nothing reads is not retained. A `.kir` that names one adds the retention to the graph the way
an `edge` adds a pass. Nothing in `karakuri-engine` does that: `Set::build_many` takes the renderers
in draw order, the fold is `crate::node::Merge` and the compositing is fixed
(`shaders/composite.wgsl`). The render graph is built from a Set's nodes, and no procedure can cause
a target to be allocated by asking for one.

The mechanism that already exists is `uses` plus `edge`
([P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)), and feedback needs it
extended to a type it has not got. The extension changes what the graph builder allocates, not what a
uniform holds. **That is the whole of why this is not a knob**: the answer decides passes and
allocations.

**Which is where feedback parts from the rest of the L5 story.** `ir-spec.md`'s *L5 — built* argues
that admitting frame effects is *giving L5 a writable form* rather than extending the algebra: a
master effect is the mix node's own signature, `[Texture] -> Texture`, with one input. Feedback does
not follow from that, and the specification says so at the same place: *"reading the previous frame
is a cycle"*. A cycle is not a fold, and the fold is what already exists.

## The two other things *Feedback* waits on, which are separate from this

Both are checked, and neither is the question above. Blurring them together is what makes this row
look like the other two.

**1. `ast::Kind` has no L5.** `crates/karakuri-ir/src/ast.rs` declares `L1 | L2 | L3 | L4 | Field`,
and `Kind::ALL` is those five. `parse_kind` in `crates/karakuri-ir/src/parse.rs` refuses anything
else with `unknown kind`, naming the five this compiler builds. `ir-spec.md` states the reason as a
condition rather than an omission: an L5 has no code to lower, so a `kind L5` file would have nothing
to contain. **This blocks all three rows of M5.8 equally** — it is not feedback's.

**2. `Bloom` and `RGB shift` carry `params: Undecided` because their knobs have never been named.**
`Operation::SetBloom` and `Operation::SetRgbShift` in `crates/karakuri-operation/src/lib.rs` both
carry `params: Undecided`, and the doc comment gives the reason: the chain does not exist, so no
parameter of it is named anywhere — the same reason
[ADR-0227](0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md) declines to
name a chain's contents, *"a record kept for a thing that does not exist would be inventing its
contents"*. This is a naming job, and it is finished the day the chain is built.

**One correction to how that second point is usually stated.** `Operation::SetFeedback` carries
`params: Undecided` too — `gate.rs` constructs all three that way. Its payload is `Undecided` for the
bloom-and-rgb-shift reason **and** for the question this record states, which the doc comment calls
*"`Undecided` twice over"*. So *Feedback* is not distinguished by carrying `Undecided` where the
others do not. It is distinguished by having a second reason that naming knobs does not remove.

Together: an L5 kind and a named parameter set would let *Bloom* and *RGB shift* be built, and would
leave *Feedback* exactly where it is
([P-0051](../principles/0051-an-existing-field-is-not-a-fact-about-the-design.md) — the shape of the
payload is not the shape of the question).

## What a decision here would touch

- **The pipeline's composition.** Which cuts exist as retained targets, and where in the chain they
  are taken. Today the chain has no members and the fold is fixed.
- **What a procedure may read.** A fifth `SlotTy`, or some other notation for a frame input, and the
  `edge` form that binds it. Both are language changes with a check pass behind them
  ([P-0067](../principles/0067-the-language-is-bounded-so-a-procedure-can-be-priced-before-it-runs.md)):
  whatever is added has to be priceable before it runs, and a retained target is a cost the estimate
  does not currently carry.
- **The memory a held cut costs.** One frame-sized `Rgba16Float` per retained cut, in the VRAM budget
  the governor does not enforce.
- **M5.8, and M5 itself.** `docs/roadmap.md` names *Feedback* as one of M5.8's three rows and says
  what it blocks: the operations page carries a `plan` row for it, so M5 cannot close without either
  taking this decision or changing the manual.

## Consequences

- The argument has an address. A future proposal for a feedback effect starts from the three steps
  rather than rediscovering them, and the tooltip's 365 characters are a pointer rather than the
  whole record.
- The three blockers on M5.8 stay named apart. Two of them close by building; this one does not.
- Nothing changes in the code, the specification or the manual. The rows stay `plan`, the payloads
  stay `Undecided`, and the roadmap entry stays where it is
  ([P-0036](../principles/0036-an-invariant-that-is-not-yet-true-says-so.md) — the state of the
  question is stated rather than implied).
- Whoever takes the decision supersedes this record with one that has a `Decision` section.

## What this leaves undone

The decision. Which cut a feedback effect reads is unanswered, and so is the prior question of
whether one cut is chosen for the whole chain or a procedure names its own.
