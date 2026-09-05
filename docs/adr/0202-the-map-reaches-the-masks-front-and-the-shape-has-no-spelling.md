---
id: 0202
title: The map reaches the mask's front, and the shape has no spelling to reach it with
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: [0209]
principles: [0090, 0094]
tags: [midi, vocabulary, surfaces, mixing]
---

# The map reaches the mask's front, and the shape has no spelling to reach it with

## Context

[ADR-0201](0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md) gave the mask two
rows — `SetMaskShape { deck, kind, angle }` and `SetMaskPosition { deck, position }` — and split
them **so that a control change could reach the position at all**. It landed with both rows `MIDI —`
on [the operations page](../manual/operations.html), which is the row a decision about reachability
is meant to change.

The position is the cleanest continuous target since `exposure`:
`karakuri_engine::deck::Mask` says of it that 0 shows nothing anywhere and 1 shows everything
everywhere *for any softness* — a claim `Blend::silent_at` reads at the bottom and a wipe that has
to actually finish reads at the top. So the range is `[0, 1]`, it is the control's own rather than a
default chosen for a fader, and both ends have to land exactly.

The shape is not, and the reason is in the grammar rather than in anyone's preference.

## Decision

**`cc N -> mask-position D` is a target. There is no shape target, and the row keeps its gap with
the reason written on the page.**

### The front

`Target::MaskPosition { slot, range }`, `Shape::Linear`, `MASK_POSITION_RANGE = [0.0, 1.0]` beside
`GAIN_RANGE`, `OPACITY_RANGE` and `EXPOSURE_RANGE`. Linear because a front travelling at an even
rate is what a wipe is; a range an operator writes is theirs as a gain's is, and buys less, because
the engine clamps a mask to the unit interval on apply where a gain above unity is a real place to
be.

**`karakuri-cli`'s `deck_of` gains a sixth arm, and it is not cosmetic.** The other five are there
for silence — a slot the deck does not hold is refused once by `mix::change` rather than once per
message. A mask operation does not reach `mix::change`: `Live::operate` cannot read the mask of a
slot that is not there, `karakuri_operation_record::written` answers `Owed::NotRead(Reading::Mask)`,
and `operate` **prints** that. Without the arm, `cc 9 -> mask-position 4` on a deck of four is a
blocking write per message inside `Live::frame` for as long as the fader moves, which is the cost
the router exists to keep off the frame path.

### The word

**`mask-position`, hyphenated, and not `mask`.** The short word is the one the *shape* would want —
`note -> mask 0 radial` is how the roadmap has been writing that line for as long as it has been a
leftover — and the two are halves of one `Record::Mask`. A grammar that spent `mask` on the front
would leave the shape with no natural spelling and the pair reading as unrelated controls. The
hyphen is not new: `on-air` was a target word until ADR-0196 refused it. And the two words are the
manual's own row titles with *Set a deck's* taken off, which is what ADR-0196 means by *the target
words in a map file are the manual's badges verbatim*.

Rejected: **`position N`**, which is the shortest true word and means four other things in this
system — a session's position on the grid, a slot's position along it, a node's position in a
layout — where [P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md) says a name
means one thing. **`front N`**, which is the word the prose uses and which nothing else in the
vocabulary or on the page says, so an operator reading the manual would not find it. **`mask N` with
the shape taking a value word later**, which is the collision above.

### The shape, and why this record does not settle it

**A map line can say three things: a slot number, a word out of a value list, or a trailing
`[lo, hi]`. None of them is a bare number**, so an angle cannot be written. `parse_target` has no
float form and this is not an oversight — `[lo, hi]` is refused on a press, so even the one place a
number appears is unreachable from the kind of line a shape would be.

A pad naming a kind alone would therefore have to **invent** the angle it wrote beside it. It cannot
borrow the one the slot already has: `karakuri-midi` reads nothing back, which is
[ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)'s
finding and ADR-0196's twice-rejected alternative, and is the whole of this crate's test story —
`Map::operation` is a pure function of one message. So a shape target is `cc -> exposure` before
`SetLook` was split, one field along: an operation asking for what the surface *cannot* say, and
writing the rest from a default.

**Three ways out, and this record takes none of them**, because each costs something in a different
document and the evidence does not separate them:

- **A float form in the map grammar.** A line could then say `note 62 -> mask-shape 0 linear 0.785`.
  It is the only option that leaves the operation as ADR-0201 wrote it. It costs a fourth thing a
  line can say, in a format whose whole discipline is that a value is a word out of a list the
  vocabulary owns — a number is the one kind of value that cannot be checked against anything, so
  the refusal an operator gets for a typo stops being *write one of these*. It also raises what
  radians on a pad mean, since the angle is in radians and no surface shows it.
- **An angle-less shape operation.** `SetMaskShape { deck, kind }` would be a pure value-word target
  the day it landed. It changes a row that landed yesterday, and it moves the angle to wherever the
  conversion gets it from — which is a reading, on a crate that has none, or a default, which is the
  fault above wearing a different hat. ADR-0201's own rejected alternatives are the argument against
  it: `softness` is out of the vocabulary because no surface can ask for it, and an angle two
  surfaces *can* ask for is not the same case.
- **The row stays `MIDI gap` with the reason on the page.** Costs nothing today and leaves the first
  rule broken for one row, which the page already says out loud in *What the gaps say*.

**The evidence favours the third for now**, and weakly: it is the only one that does not change a
document to serve a surface that has not asked yet. The console's mask mini is where the shape's
first real writer will be, it is item 1 of the roadmap's *where this goes next*, and a picker that
can show an angle is the thing that would tell us whether a map needs to say one. Taking the
decision before that writer exists is choosing between two grammars on no evidence.

## Alternatives rejected

- **Add both rows and let the shape target default the angle to 0.** One line of code, and it is the
  defect: a pad pressed mid-wipe would square the front to horizontal, and the record it wrote would
  be a picture nobody asked for that replays perfectly. P-0094 is what makes it worse rather than
  better — `Record::Mask` is a state, so nothing downstream can tell a restated front from a hand on
  it, and the wipe stops as well.
- **Add both rows and let the shape target carry the angle in the range slot** — `mask-shape 0
  linear [0.785, 0.785]`. It needs no new grammar. It also needs `[lo, hi]` to mean a scalar on
  exactly one target, and the refusal *"`{name}` is a press and has no range"* to grow an exception;
  a format's error messages are the part an operator reads, and one with an exception in it is a
  format with two rules.
- **Hold `mask-position` back until the shape can land with it.** Two rows arriving together reads
  tidier and is the shape of ADR-0201. But the position is not blocked by anything, it is the one
  control in the mixer strip a fader could never reach before this, and P-0094's *the operator wins*
  has its hole exactly there — a wipe an operator cannot stop half way. Waiting would trade a
  control that works for a symmetry nobody performs with.
- **A new principle.** Nothing here is a rule a future proposal would violate. The rule already
  exists twice — a surface asks for what it can say (ADR-0192), and an operation names a value out
  of a list the vocabulary owns (P-0090) — and this is those two meeting a field neither list can
  hold. What is owed is a decision, and it is named above rather than ruled on.

## Consequences

- **The page recounts to 48 operations and 51 of the 208 ways in**, from 50; MIDI reaches **8** of
  the 48, from 7. Counted from the page itself.
- **`examples/surface.map` gains four lines and no pads**, so the file's thirty-pad bill is
  unchanged. What it does gain is a third continuous control per strip — twelve knobs for four
  slots — and the file says so where it says the rest of its bill.
- **The shape's gap is now written down in three places that agree**: the row's own prose on the
  page, *What the gaps say*, and `karakuri-midi`'s module documentation. It was previously written
  as *"a mask's angle has no spelling there even once a mask target exists"* — which was a
  prediction, and is now the current state of one row rather than of the pair.
