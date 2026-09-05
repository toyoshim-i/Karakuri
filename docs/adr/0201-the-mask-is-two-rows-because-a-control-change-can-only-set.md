---
id: 0201
title: The mask is two rows, because a control change can only set
status: accepted
date: 2026-08-27
supersedes: []
superseded_by: []
principles: [0094]
tags: [vocabulary, midi, mixing, console]
---

# The mask is two rows, because a control change can only set

## Context

The mask is the last thing in the mixer strip that no operation can reach. Five records have said so
from five directions — the console's mask mini is *"a readout of state no operation can set"*
([ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md),
[ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
[ADR-0195](0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md)), and
`karakuri-cli`'s `wipe` builds a `Record::Mask` by hand with a comment saying it is the owed part of
the gesture ([ADR-0198](0198-a-gesture-converts-in-the-parts-that-are-decided.md)). Every one of
them ends at the same place: **the page has no row**, and
`docs/manual/operations.html` is the specification for which operations exist.

Writing the row is the decision. The obvious shape is one:

```rust
SetMask { deck: u8, setting: MaskSetting }   // Shape | Position | Softness
```

which is what `Operation::SetTransition` already does — one row, a sum of three things three
different keys press.

## Decision

**Two rows, and `softness` in neither.**

```rust
SetMaskShape    { deck: u8, kind: WipeKind, angle: f32 }
SetMaskPosition { deck: u8, position: f32 }
```

**The vocabulary's own standing rule decides it.** `karakuri-operation`'s crate documentation says:
*a continuous control is set, not nudged — a control change carries a position and can only set, so
a rule that says every operation is addressable by a map says every continuous operation takes a
value.* A row carrying a sum cannot satisfy that, and the reason is in `karakuri-midi`'s grammar
rather than in anyone's preference. `parse_line` refuses the cross product in both directions:

> a note is a press, and this control takes a position; map a `cc`

> a control change is a position, and this control takes a press; map a `note`

A row that is a sum is a **press** — it names which of several things it is setting, the way
`residency 0 live` names one of three — so a `cc` may not be mapped to it at all, and the mask's
position would be the one continuous control in the instrument no fader could ever reach. That is
[ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)'s
rejected alternative arriving one layer down: **a route that undoes another route as a side effect
of its own payload**, and here it does not even get that far — the route is refused at parse time.

**`Record::Mask` does not change.** It stays whole — a shape, an angle, a position and a softness —
because a replay reconstructs a picture from the record and a stream that moved a front without
saying what shape it was moving would describe a wipe nobody can rebuild. Each of the two rows
writes it whole, filling the half it did not ask for from a reading of the mask that is running,
which is exactly what `karakuri_operation_record::Current` is for and what the look pair already
does.

**`softness` stays out of the vocabulary.** It is in the record, it has one constant behind it —
`karakuri-cli`'s `MASK_SOFTNESS`, whose own documentation says *"Not zero, and not a key"* — and no
control on any surface. That is `white_point`'s reason at `Operation::SetExposure`, unchanged: an
operation exists because a surface can ask for it, and nothing asks for this.

### What lands with it

- **`Deck::set_mask` splits.** `set_mask_position` cancels a running transition, on `set_gain`'s and
  `set_opacity`'s terms; `set_mask_shape` does not, because it writes no position and there is
  nothing under its hand. The old whole-mask setter deliberately did not cancel, and its reason was
  written for the one caller there was — `wipe`, which writes position 0 before scheduling. That
  exemption stops being safe the moment a surface can write a position, and **the split is what
  turns an exemption into a rule**:
  [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md), which
  also records where it stops holding.
- **`wipe` writes six records where it wrote five.** The mask was one hand-built record; it is now
  two operations, each writing `Record::Mask` whole. That is what routing it honestly costs, and it
  is recorded rather than contrived away: a gesture that wrote one record by picking the fields out
  of two operations would be the second derivation `mix.rs` exists to have ended. `mix::mask_record`
  is deleted, the way `gain_record` and its three siblings were.
- **The page recounts to 48 operations and 208 ways in, of which 50 exist.** The two new rows add
  eight badges and no route: `z` is a *console setting* deciding what the next wipe means and not
  this; the map names no mask; MCP publishes six tools and none of them is a mix control. The panel
  has a home for the shape — the strip's mask mini, whose mock tooltip already says *"Click to
  choose one"* — and none for the position, so that row is the third `gap` in the panel column.

## Alternatives rejected

- **One row carrying a sum, on `SetTransition`'s precedent.** The precedent does not transfer, and
  the difference is what makes it safe there: `SetTransition` **writes no record at all** — it is a
  surface's own setting deciding what the *next* move means — and all three of its arms are presses
  (`z`, `n`, `j`). Nothing continuous is hidden inside it. It is also a row the manual already had,
  and the vocabulary may not change the page. The mask's row is being written by this decision,
  which is the one moment its shape is free.
- **One row plus a second row for the position only.** A `SetMask` sum that included the position
  *and* a `SetMaskPosition` beside it would let a map reach the front, at the cost of two operations
  that can both set the same field — and the map would then have two spellings for one change, one
  of which silently rewrites the shape. Two rows that partition the record are the same reach with
  no overlap.
- **Give the shape row a `softness` field.** It would make the conversion need no reading for that
  one field, and it would put a number on the page that no surface can produce and no operator can
  see. `white_point` was left out of `SetLook`'s successors for this reason four records ago; a
  second answer to the same question is the drift the catalogue exists to prevent.
- **Leave the position out and let the wipe own it.** The mask would then be a shape control and the
  front would be reachable only by scheduling a move — which is the instrument saying *you may start
  a wipe and you may not stop one half way*. It also leaves `Control::MaskPosition` as the only
  transition control with no manual writer, so P-0094 would have a hole exactly where the rule is
  hardest.
