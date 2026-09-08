---
id: 0295
title: The grip is the fold, and a pane's outer edge is the other one
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0087, 0090, 0094]
tags: [console, ui, operations, panel, m5]
---

# The grip is the fold, and a pane's outer edge is the other one

## Context

*Fold a bay away* and *Fold a pane away* are two rows of *Arranging the console* on
[every operation](../manual/operations.html). Both have carried a `plan` badge in the panel column
with a home named — **bay head** for the first, **pane edge** for the second — since the page was
written, and the console has drawn the furniture of the first the whole time and hit-tested none of
it. So the only route into either row was `f` and `g`, and the page was promising a control an
operator could see and not press.

Three things had to be decided before either could be drawn.

**Which part of a bay head takes the press.** The page's cell says *bay head*, and a head is 27 tall
by the whole width of the bay — a very large target for an operation that takes a bay off the
screen, and the natural home of a press meaning *focus this bay*, which
[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
has the keyboard reaching by Tab and nothing reaching by pointer yet. The alternative is the mark
already in the head: [console.html](../manual/console.html)'s `.grip`, six dots at the right of four
of the seven heads the console draws.

**What the grip currently means, and whether a control may take it.** The page's lede is explicit —
*"The grip in a bay's head says whose size you are setting: a bay with one is yours to size, and a
bay without one is the height of what is in it. Both sides of a divider move either way; the grip
says which side the number belongs to."* It is a **readout**.

**What a pane edge is.** The panes' outer edge is the window's own: `.console`'s 10px of padding is
not drawn, so the left pane's left edge is x = 0 and the right pane's right edge is the viewport's.
Nothing is drawn there at all, in the mock or on the console.

## Decision

### 1. The grip is the control, and no second shape is drawn

A press on the grip in a bay head asks `panel::Op::Fold` of that bay.

**A reading which is a control does not get a second shape invented for it** is
[ADR-0291](0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)'s rule, taken
over the transport row's tempo figure a fortnight ago; this is the same move one bay over. The mark
says *this bay's height is a number you set*, and a fold is the far end of setting it — so the
reading a press takes is the reading the mark already carries, and `view::bay_head` paints exactly
the six dots it painted before `view::bay_grip` existed.

**The head around it is not claimed.** A control claims what it acts on and no more
(`karakuri_console::input` rule 4), and the rest of a bay head is the one rectangle on this console
with a plausible second meaning already queued behind it — *focus this bay*. Taking 27 pixels by the
width of a bay for a fold would spend that.

### 2. The target is the strip the head reserves for the mark

The mark is 5.5 by 9, which is not a target a hand finds — the sentence `panel::GRAB` is written
under. So the rectangle a press must land in is **`GRIP_W + PILL_GAP` wide by `PILL_H` tall**,
hard against `HEAD_PAD_X` and centred on the head's mid-line: 10.5 x 16.5.

That is not a number chosen here. `view::head_pills` steps `GRIP_W + PILL_GAP` back from the head's
right-hand padding before it places the first capsule, so that strip is **what the head has already
set aside for the mark** and the one place in a head where no other control can ever be drawn. The
target is that reservation, grown down the head to the line height every other control in a head is.

**Padding it like a pill was the alternative and it fails**: `.pill` is its content inside
`padding: 0 8px`, and 5.5 + 16 = 21.5 reaches six pixels into the capsule beside it. As reserved, the
two abut on one edge and never overlap — two rectangles sharing a line, which two rows of the Library
bay's list already are — and the capsule is asked first in both the claim and the caller.

**What it costs against a boundary is `program_head`'s 0.75 of a pixel**, and it is the same
arithmetic because it is the same capsule box in the same head: 27 less 16.5, halved, is 5.25 of head
above the target against a `GRAB` of 6. Rule 3 gives the boundary first refusal, so the top 0.75 of
the target drags the divider and the other 15.75 folds the bay. `tests/fold_grip.rs` measures it by
asking `Layout::hit` at the target's own corners and stepping down its top edge, and finds 0.75 in
all four bays.

### 3. Four of the seven headed bays fold from the panel, and that is the page's number

The mock draws a grip in the Library, the Program bay, the Inspector and the Master, and in none of
the Mixer, the Staging lane or the Sequencer. **So those three keep `f` and have no pointer route**,
and `Fold a bay away` is reached from the panel in four bays out of seven.

**Drawing a grip in every head would buy the other three one and is refused**: the mark would stop
saying which side of a divider the number belongs to, which is the whole of what the lede says it is
for, and a mark present everywhere reads as a state wherever it is absent
([P-0087](../principles/0087-name-the-property-never-the-shape.md)'s neighbour argument, made at
`view::Outputs`). Changing it is an edit to `console.html` and is the maintainer's; the badge does not
need it, because ADR-0213's `has` means *an operator running the instrument reaches the operation*
and four bays reach it.

### 4. A pane folds from its own outer edge, in a band `GRAB` deep

`view::pane_edge` answers a band on the pane's outer edge, `panel::GRAB` deep and the whole height of
the pane, asking `Op::Fold` of the pane.

**`GRAB` rather than a number chosen here**: it is this console's one statement of how far from an
edge a hand is still on it, and a second number would be a second answer to a question the panel has
already answered for every boundary.

**Which edge is derived rather than listed** — the pane is the first or last visible child of the row
it is in, and the band goes on the leading edge of the first and the trailing edge of the last, read
off the split's own axis. **Which panes fold is listed**, in `view::FOLDING_PANES`: fold the left pane
and the centre inherits its outer edge, so a derivation reading the edge alone would hand an operator
the fold the page refuses — *"The middle one is not a third pane on purpose: folding it is not a thing
anybody wants, and solo is."*

**Nothing is drawn there.** This crate's rule is that a shape which looks like a control and is not
does not get drawn; the converse is not a rule, and `view::Mixer::select` is the standing case of a
control that is a rectangle rather than a capsule. An unmarked band is what the page's *pane edge*
already specifies, and telling an operator about it is `console.html`'s job rather than this
derivation's.

### 5. The band is asked last, because a library row is three pixels from the edge

The band lies over whatever the pane's bays draw at their outer edge. Everything clears it — the scope
chips and filter fields at 9, a mixer strip at `STRIPS_PAD`'s 6 exactly, the transition row and the
Master bay's out at 10 — **except a library row**, because `.lib-list`'s padding is 3.

So the pane edge is **asked after every control inside the pane**, and the row wins in those three
pixels. That is `Mixer::select`'s sentence one level out — *a press anywhere on this strip that no
knob under the pointer claimed* — applied to a pane rather than to a column. It costs `claim` nothing
(it answers a `bool` either way) and it is a requirement on the **order of the arms in the window
loop's press handler**, which is where the two rectangles are told apart.

### 6. A fold writes no session record, and neither of these routes through `written`

`karakuri-operation-record` answers `Silent::Surface` for all four fold operations — a surface's own
state. Both controls hand back a `panel::Op`, `Panel` performs it, and an `Outcome` comes back; there
is no `Operation`, no `Record` and no `Acted::Emitted`. This is the arrangement `Outputs::op` and
`ProgramHead::op` already have, and ADR-0204 is why it cannot be an `Operation`: two splits in this
arrangement have no name.

### 7. There is no unfold on either control

`Outputs::op` and `ProgramHead::op` each choose between two operations because the thing they act on
is still on screen when it is off. A folded node has no rectangle, so **neither of these controls is
drawn once its press has landed** — the grip goes with the head and the band goes with the pane. That
is `Op`'s own sentence about the pointer, met by a control instead of by a key, and the way back is
`z` for the same reason it is for `f`.

## Consequences

- `view::FoldGrip`, `view::bay_grip` and `view::pane_edge` are the derivations; `view::BAY_GRIPS`
  counts the heads that carry one, off `REGIONS`, so a grip drawn in a fifth head raises the count
  rather than waiting to be noticed. `view::head_grip` is the one statement of which heads those are,
  and `head_of` reads it too.
- **Neither control is registered yet.** A row in `karakuri_console::input::PROBES` and an arm in
  `crates/karakuri/src/main.rs`'s press handler are what make a press reach them, and until both land
  a press on either goes to `egui` and reaches nothing. **The two panel badges are earned when the
  registration lands and not before**, which is the same division `tests/keep_pill.rs` records for the
  Inspector's `keep` capsule.
- Neither derivation asks `egui` for anything: a mark and a band are not as wide as a word, so these
  are the cheapest probes on the console and the only bay-head control that exists before the first
  frame.
- `tests/fold_grip.rs` is what stands under all of it, and it measures every clearance by asking
  `Layout::hit` rather than by repeating the arithmetic.

## Open, and named rather than left

- **Whether a bay without a grip should gain one**, which is the only way the Mixer, the Staging lane
  and the Sequencer fold from the panel. It is a change to `console.html`'s lede and to what the mark
  means, so it is the maintainer's.
- **Whether an unmarked band at the window's edge is the right affordance.** It is what the page
  specifies and it is the one control on this console with nothing drawn for it; it is also on the
  strip of window some platforms use for their own resize handle, which is
  *Size the window*'s neighbourhood.
- **Whether *Fold a bay away* and *Fold a pane away* are one row.** They are one variant, one type and
  one derivation shape here, and the page keeps them apart because the consequences differ.
  `tests/vocabulary.rs`'s `rows_of` already gives `Op::Fold` both, and merging them is the page's
  decision rather than this record's.
