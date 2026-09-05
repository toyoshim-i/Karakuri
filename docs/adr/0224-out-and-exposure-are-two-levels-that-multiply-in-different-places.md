---
id: 0224
title: out and exposure are two levels that multiply in different places
status: accepted
date: 2026-08-30
supersedes: []
superseded_by: []
principles: [0064, 0087, 0093]
tags: [engine, colour, mixing, console, ui]
---

# `out` and `exposure` are two levels that multiply in different places

## Context

[The console page](../manual/console.html)'s Master bay draws one row — `out 1.00` — over a chain of
three effects, under a footnote reading *"runs in linear HDR, before the one tonemap"*.
`karakuri-engine` already carries a level answering to that description: `Look { op, exposure,
white_point }`, whose `exposure` is *the level going into that transfer*, applied in the present pass
on values in the linear HDR target. [Every operation](../manual/operations.html) routes **Exposure**
to `panel transport`. So two pages promised two homes for what might be one number, and the bay's
first row could not be drawn without knowing which.

**The word `master` appears in `crates/karakuri-engine/` zero times**, counted rather than
remembered, and that is the whole of what the engine says about a master chain today. What it does
have is a mix: `Deck::begin_frame(..).render(present.hdr_view(), ..)` folds one to four slot targets
in `shaders/composite.wgsl` and writes the result straight into the one `Rgba16Float` target the
present pass reads. **There is nothing between the two passes.** That is not a gap in the survey; it
is the reason this question is hard to answer by looking at a frame.

Three levels were already distinguished in the source and none of them is this one: a procedure's
`param exposure` (how bright that material is), `Deck::set_gain`'s per-slot L5 gain (how one Set
balances against the others), and the tone mapper's `exposure`. `deck.rs` says so at its `Slot::gain`
field and `present.rs` said so at its tonemap buffer, where the third was still described as *L5's
future per-Set gain* although `Deck::set_gain` had landed. **A fourth level with no home is exactly
how a name comes to mean two things**, which is
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md).

## Decision

**Two values, multiplying in different places.** `out` is applied where the mix **writes** the
composited frame — the entry to the master chain — and `exposure` is applied where the present pass
**reads** that frame, at the tone mapper's input. The L5 master effects the bay draws beneath the row
(feedback, bloom, rgb shift) go between them.

Concretely, and it is small: `shaders/composite.wgsl`'s fragment ends
`acc.rgb * mix_in.master_out`, the scalar rides in the ninth column of the mix uniform that pass
already writes every frame, and `Deck::set_out` is what writes it. Nothing else moved. Colour only —
the fourth channel is coverage and a level does not change what a frame covers, which is the same
asymmetry `gain` has.

### Why one value lost

**`out` *is* `exposure`, its panel route moves to the master out, and the bay's first row draws today
with nothing added to the engine.** It costs nothing now and it is wrong the day the chain is not
empty. A master effect reads what it is handed, so which side of it a level sits on **is** the
picture: a feedback trail fed at half level decays from half, and one fed at full level and dimmed
afterwards decays from full. Folding the two into the tone mapper's multiply would therefore have to
be pulled back out of it the day L5 lands — and pulling a level back out is not the same size of
change as putting one in, because by then a record, a MIDI target and a fader will all be pointing at
the folded one.

### Why two values in the same place lost

**A second scalar multiplying where the first already multiplies cannot justify itself.**
[P-0087](../principles/0087-name-the-property-never-the-shape.md) cuts both ways here
and it is worth saying which way each: `exposure` existing does not prove `out` is not a second
thing — that is the leak-from-the-current-shape half — but a field is still owed a reason to exist,
and *two numbers you can multiply together beforehand* is not one. Under this decision the second
scalar's justification is not its name and not its surface: it is **the position**. Take the position
away and the field goes with it.

## The cost, which is the argument for the alternative

**Until the master chain exists there is nothing between the two multiplications, so the second
scalar is indistinguishable from the first in every frame this program can draw.** A master out of
0.5 and an exposure of 0.5 produce the same picture, byte for byte — measured, not assumed:
`with_nothing_in_the_master_chain_the_two_levels_are_the_same_picture` asserts it, and on this
machine (Metal, 2026-08-30) the two `Rgba8UnormSrgb` readbacks differ by 0. The test allows one byte
because the two paths round differently in principle — the master out is stored to `f16` before the
exposure reads it, so a texel already in the subnormal range can round on the way through — and that
allowance is unused.

The maintainer weighed that against the other cost and took this one. **The choice is between a cost
that is paid now in a scalar nobody can see and a cost that is paid later in an unpicking**, and it
was made on which of the two is cheaper to be wrong about.

## What this leaves undone

- **The chain itself.** `feedback`, `bloom` and `rgb shift` are drawn on the page and exist nowhere.
  Until one of them lands, `out` is a level with no argument you can see for it, and the test above
  says so in its own words: **it is deleted the day the chain has an effect in it**, not weakened.
  That is the tripwire — the undone work fails a test rather than going quiet.
- **No route reaches `Deck::set_out`.** There is no operation, no `Record`, no key, no MIDI target
  and no CLI flag; `karakuri-operation`, `karakuri-store` and the panel are all untouched by this
  record. The engine holds the value and only a test writes it. A CLI key was considered and refused
  on [ADR-0046](0046-a-flag-writes-into-the-record-it-does-not-invent-one.md) — a flag writes into a
  record it does not invent, and there is no record for this yet.
- **No transition, and it is not an oversight.** `Control` is per slot, so scheduling a move on the
  master out needs a control that names the deck instead — which is a decision, and `set_out`
  therefore does not call `cancel` because nothing can be moving it.
- **The Master bay is still not drawn.** This record is the engine and the manual; the surface is
  not, and nothing here starts it.

## Consequences

- **The Exposure row in [operations.html](../manual/operations.html) does not move.** Under the
  losing option it would have: `panel transport` would have become the master bay's row. Exposure
  belongs to the tone mapper, the tone mapper is in the transport, and that is where it stays. What
  is owed there instead is a **new** operation for the master out, once one exists to name — with
  every route a gap.
- **[The console page](../manual/console.html) is corrected with this record rather than after it.**
  The footnote under the chain described `exposure`'s place while sitting under `out`'s row; it now
  says which end of the chain each level is. The `out` fader gained the tooltip every other control
  on that panel has, and the region notes gained *Master, and the two levels that are not one level*.
- **A deck of one is still a bare Set bit for bit.** The master out comes up at 1.0 and a multiply by
  1.0 is exact for every value including an infinity, so `a_deck_of_one_is_a_bare_set_bit_for_bit`
  and every single-Set expectation resting on it are untouched. A nested `Merge` passes 1.0 for the
  same reason and for a better one: it is an L5 inside a Set, and the master chain begins one L5
  further out.
- **[P-0064](../principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)
  is unmoved.** The new multiply is in the linear HDR pass that already existed, on values above 1.0
  as well as below, and nothing about where sRGB is encoded changed. The master out is deliberately
  unbounded above 1.0 and floored at zero — `clamp_gain`, the same function and the same reasoning as
  the per-slot gain.
- **`present.rs` now names four levels where it named three**, and one of the three had gone stale:
  the per-Set L5 gain it called *future* is `Deck::set_gain` and has been for some time.
- **What the separation can be asserted by today is one property, and it is enough.** The composited
  frame moves when `out` moves and does not move when `exposure` moves, while the picture moves for
  both — `the_master_out_is_at_the_chains_entry_and_exposure_is_at_the_tonemaps_input`. A build that
  folded the two into one multiplication fails it whichever end it folded them at.
