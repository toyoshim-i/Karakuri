---
id: 0203
title: The mask chip carries the angle it does not control
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0074, 0078]
tags: [ui, console, vocabulary, mixing]
---

# The mask chip carries the angle it does not control

## Context

The mixer strip's mask `.mini` was the last readout in it. Three records had said why — there was no
`SetMask`, so the chip drew state no operation could ask for
([ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md),
[ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
[ADR-0195](0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md)) — and
[ADR-0201](0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md) wrote the row:

```rust
SetMaskShape { deck: u8, kind: WipeKind, angle: f32 }
```

The press itself is the shape ADR-0187 and ADR-0195 settled and is not what this record is about:
the console translates a press into an `Operation` and applies nothing, the harness turns it into a
`Record`, the record moves the deck, and the strip draws what the deck says. The cycle is a `match`
in `view.rs` for `after`'s reason, in `karakuri_engine::deck::MaskKind::ALL`'s order — *"`None`
first, because it is the default and a cycle should start where a slot starts"* — and the claim is
`input.rs`'s rule 3 taking a fifth control without changing.

**What is new is one field.** The row carries a **kind and an angle**, and the chip names only the
kind: `view::Strip`'s mask field says so in its own words — *"`.mini` is a chip that says which
shape, and three numbers about that shape are the inspector's row, not this one."* So a press has
to put a number in the operation that the control it came from does not control, and there is no
third option: the field is not optional and the record is written whole.

## Decision

**The strip carries the angle the slot is already wearing, and the press hands it straight back.**

`view::Strip` gains `mask_angle: f32` — `Deck::mask(slot).angle()`, written by whoever owns the deck
like every other field on a strip, **read to build the operation and painted nowhere**. A press
emits `SetMaskShape { deck, kind: the next shape, angle: strip.mask_angle }`.

**It is `Strip::requested`'s arrangement exactly**, one control along: a value a strip carries
because a press has to be computed from it rather than because the strip draws it. That is the
precedent this rests on, and it is why the field needed no new idea — the tally already reads two
values and draws one.

**Nothing else about the mask crosses the seam.** The record also carries a position and a softness,
and neither is in the operation: `Record::Mask` is written whole and the half a shape operation does
not ask for is filled in from a reading of the running mask, where the record is written
(ADR-0201, `karakuri_operation_record::Current`). A surface carrying values no operation names would
be this crate keeping half a deck.

## The alternative that lost

### Send `0.0` — the chip names a shape, and the angle is not its business

This is the tidy reading, and it is the one somebody re-proposes: the control's whole visible
business is choosing between a circle and a straight edge, `Strip` gains no field, and the operation
is built from what the chip knows.

**It loses because the record is written whole from what the operation says.** A press would set the
shape *and rewrite the angle to zero*, so choosing `radial` on a slot wearing a linear front at 0.9
radians and choosing it back would leave a straightened wipe behind. Nothing on the panel would say
so: the mark a `.mini` draws is the same mark at any angle, and the number lives in an inspector row
that does not exist yet. **A press that silently changes something nobody asked it to** is exactly
what
[ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md) is
about — an operation asking for what a surface can say, with the record staying whole — arriving one
layer further out, and ADR-0201 is the same fault one layer further in.

It is worth naming what makes it *plausible* rather than dismissing it: the angle really is not the
chip's to choose, and a surface inventing a value is a real fault. The answer is that **handing back
what was read is not inventing** — it is the same move `karakuri_operation_record` makes for the
position and the softness, and the same move `Current` exists for. What would be inventing is a
constant.

- **Rejected too: leave `angle` out of the row and give the shape its own operation.** That is
  ADR-0201 reopened from the other end, and it costs the row its only way of saying which way a
  linear front runs. It also does not help: the record still carries an angle, so something would
  have to supply one, and the only candidates are the reading — which is this decision with an extra
  step — or a default, which is the alternative above.
- **Rejected too: put the angle on the chip.** A drag round the mini for the angle and a click for
  the shape is two gestures on a 23-pixel target, and the strip is 53 wide inside its padding.
  ADR-0187 measured what that row can hold; this is smaller than anything it rejected.

## Consequences

- **`input.rs`'s rule 3 takes a fifth control and did not change to hold it** — the Outputs sink, a
  fader knob, the blend chip, the tally chip and now the mask mini, each asked the same way: the
  derivation that draws it, asked whether the point is on it, with nothing stored, and the caller
  that acts on the press asking the same function again rather than copying its answer. **The bay is
  still derived once per event** and asked for all four of the mixer's controls.
- **Its clearance from a boundary's grab was measured rather than inherited, for the fifth time, and
  the number is neither of the two beside it.** The blend chip clears the pane divider down the left
  of the bay by **8.97** and the tally's capsule by **14.23**; the mask mini clears it by **41.03**,
  because it sits at the far end of the same mode row. It is also the first control whose *nearest*
  boundary is not the same one on every strip: on the three strips that are not against the pane the
  nearest is the boundary under the bay, **74.50** below. Inheriting the blend chip's number would
  have named the wrong boundary and the wrong figure at once. `tests/mask.rs` asks `Layout::hit`
  directly as well as `claim`, with the guard that makes a bay with no strips fail rather than pass.
- **`view::Mask` gets no `ALL`, and that is a decision rather than an omission.** `Tally::ALL` exists
  because `mixer` measures the capsule against the widest of the three words, and `BlendMode::ALL`
  because a map file is offered the values a target may end in. Nothing reads a list of mask shapes:
  the mini holds a **mark** rather than a word and is the same width whichever shape it shows, and
  no map target names a shape — which is why `karakuri_operation::WipeKind` has no `ALL` either and
  says at its own `name` that one *"arrives with the first reader"*. This is not that reader. So
  `view::next_shape` is the only statement of the order in the crate, and `tests/mask.rs` carries
  the copy it is checked against — with a `match` in the test, so a fourth shape does not compile
  until it is in both.
- **A shape press stops a running wipe, and that is P-0078's stated limit rather than a new one.**
  [P-0078](../principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md) says
  `Deck::set_mask_shape` does not cancel *"and the exemption is now a split"*, and then says where
  that stops holding: `Record::Mask` is a **state**, so nothing downstream can tell a shape change
  that restated the front from a hand that moved it, and the decode applies the whole state and
  cancels. It closed with *"nothing routes one today … so the first surface to make that press is
  where this is decided again"*. **This is that surface, and the decision is that it routes through
  the record like everything else.** The alternative — the harness calling `Deck::set_mask_shape`
  directly to keep the wipe alive — is a second route to the deck for one control, which is P-0028
  broken to save a case that ADR-0201 already priced; it would also drop the softness, since the
  shape setter does not carry one. The manual's tooltip says what a press costs rather than leaving
  an operator to find out.
- **The harness reads a mask now**, where all four earlier controls handed in `Current::default()` —
  *I read nothing* — because their records are written from the operation alone. `examples/panel.rs`
  gains `reading`, which takes the reading for **the deck the operation names**, and `apply` gains a
  `Record::Mask` arm that decodes through `Deck::set_mask`. It reads the softness back rather than
  substituting a constant the way `karakuri-cli`'s `mix::current_mask` does: that program writes
  wipes and has a softness of its own, and this window has never written one, so reading it back is
  what stops a press rewriting it.
- **The strip is finished.** Nothing left in the mixer bay is a readout a pointer might have expected
  to answer — the meter and the number are the deck talking, and a fader's track is deliberately not
  a target. `docs/roadmap.md`'s item 1 is now the MIDI map's half alone
  ([ADR-0202](0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md)).
- **The page's `panel` badge stays `plan`, and the row was left alone.** `docs/manual/operations.html`
  states its own rule — *"Every panel route is marked designed rather than built, because the panel
  itself is not built"* — so the blend chip's row and the tally chip's are still `plan` with the
  controls built, and this one is no different. The count of ways in does not move either.
