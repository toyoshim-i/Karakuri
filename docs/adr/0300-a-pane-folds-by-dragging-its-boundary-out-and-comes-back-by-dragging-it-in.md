---
id: 0300
title: A pane folds by dragging its boundary out, and comes back by dragging it in
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0073, 0082, 0087, 0090]
tags: [console, ui, operations, panel, layout, m5]
---

# A pane folds by dragging its boundary out, and comes back by dragging it in

## Context

[ADR-0295](0295-the-grip-is-the-fold-and-a-panes-outer-edge-is-the-other-one.md) gave *Fold a pane
away* — a row of *Arranging the console* on [every operation](../manual/operations.html), with **pane
edge** named as its panel home since the page was written — a band on the pane's own outer edge,
`panel::GRAB` deep and the full height of the pane, drawn as nothing at all. `view::pane_edge` was
the derivation and `tests/fold_grip.rs` measured it.

**It was never registered, and its own record says why.** The band lies over whatever the pane's bays
draw at their outer edge, and everything clears it except one thing: `.lib-list`'s padding is
`size::LIB_LIST_PAD` = 3, so the outer three pixels of every row of the Library bay's list are under
it. ADR-0295 answered that by ordering — the band asked last, the row winning in those three pixels —
which is `view::Mixer::select`'s sentence one level out. **The ordering is sound and the situation is
not**: a bay's own promise about its rows is not a thing an arrangement-level control should be
spending, and no row in `input::PROBES` and no arm in the window loop's press handler was written.
`docs/roadmap.md`'s *The console's own shape* and commit `a26e0e7` carry the hold.

So the row had no pointer route, and the operator's only way to fold a pane was `g`.

**The maintainer proposed a different shape**, and it dissolves the conflict rather than paying for
it: *ペインは畳んでもサイズゼロにはならず、そのまま外縁を持ち続けるんじゃだめ？も一度それを内側に引っ
張ろうとすると最小サイズで出現、とか* — a folded pane does not leave the layout; it keeps its outer
edge, and dragging that edge inward brings it back at its minimum size.

Three things make it better than the band, and each of them is a thing the band could not have:

- **While the pane is open there is nothing at the window's edge at all.** Its boundary is where it
  always was, between the pane and the centre. The band only exists once the pane is closed, which is
  exactly when nothing is drawn under it — so the overlap that stopped ADR-0295 cannot happen.
- **A boundary is already claimed, before any control is asked.** `input::claim` asks
  `Layout::hit(p, GRAB)` under rule 3, ahead of every rule-4 derivation. So this needs no `Probe`, no
  `claim` change and no press arm, and the Library bay's promise is untouched: a divider is the
  arrangement's control and not a bay's.
- **There is a way back through the pointer**, which no fold on this console has ever had. `Op`'s own
  sentence — *"a folded region has no rectangle, so the pointer could never be over one"* — has held
  since the operations were written, and `z` is the way back for every one of them.

## Decision

### 1. A fold may leave the node's edge behind, and that is a reading rather than a third bit

`karakuri-layout` gains one declaration and one reading, and no new stored state.

**The declaration** is `Spec::keeps_its_edge()`, a `bool` on the node beside `min`, `max` and
`collapsed`. **The reading** is `Layout::is_closed(id)`: the node is collapsed, it declares that it
keeps its edge, and **no solo is in force**. A closed node takes zero extent like any other fold and
stays one of the children its parent tiles — so the divider beside it is still solved, still drawn,
still answers `Hit::Divider`, and is still what `set_divider` counts.

**Two questions come apart and each gets its own name.** `Arrangement::out_of_layout` — *takes no
extent, is not drawn, nothing under it is visible* — is unchanged and is `collapsed || aside`. The
new one is `Arrangement::placed` — *is one of the children its parent tiles* — and it is
`!aside && (!collapsed || is_closed)`. `Layout::visible_children` was the public form of the first
and answered the second; it is renamed **`placed_children`**, `Layout::is_placed` is the same
question about one node, and `visible_count`/`visible_child` become `placed_count`/`placed_child`.

**The name had to move.** *Visible* was already doing two jobs — a child inside a folded split has
always been listed by `visible_children` while `Layout::visible` answered no for it — and a closed
node makes the divergence one a reader meets rather than one they can rationalise. Six call sites and
a handful of test names is the whole of the churn.

### 2. It is a third state, and it is derived from two things already stored

**Not a third bit.** ADR-0183's two bits are two *independent reasons* a region leaves the layout,
arriving from two callers and cleared by two, which is why both have to be readable. This is not a
third reason: it is the operator's own fold, read against what the arrangement says folding this node
does. So there is nothing to save, nothing to restore, and nothing that can come apart from the fold
it describes. `Layout::collapse` is unchanged and there is **one fold and not two** — which of the
two it turns out to be is read off the node, so a key, a pointer, a MIDI map, a restored file and MCP
cannot disagree about it.

**Declared on the node rather than chosen by the caller**, and that is the same argument. The
alternative was `Layout::close(id)` beside `collapse(id)`, with the console picking — and the console
already had that list, `view::FOLDING_PANES`. Two callers choosing apart is two answers to *what does
folding this do*, and `Panel::op` would have needed the list anyway, which puts it in the layout in
the end.

**A solo suppresses it, and that term is load-bearing rather than tidy.** `Layout::solo` collapses
every node off the solo's path, so without the term a solo on the program would leave two closed
panes and two 10px gaps, and the picture would hold the window less twenty pixels. `console.html`
promises the opposite outright — *"the panel folds away and only the picture is left, which is also
how you capture this window"* — and ADR-0157 is the same promise from the maximum's side. Reading the
solo costs one conjunct and no state; `unsolo` restores the folds it replaced and every edge comes
back with the fold it belonged to.

**What it costs is the divider.** A parent with a closed child spends one gap on it. Folding the left
pane used to give the centre 340 + 10; it now gives it 340, and the 10 is the control.

### 3. `set_divider` refuses a boundary beside a closed node

A closed node's extent is zero and is not the boundary's to give away. Without the refusal the clamp
would write a size the next solve caps straight back to zero — and the size it overwrote is the one
`expand` exists to restore, so a drag against a closed pane would quietly forget how wide that pane
used to be. The boundary is returned where it stands. **What a drag there means instead is the
caller's**, because it is a gesture with a threshold in it and this crate has no hand.

### 4. The threshold is `GRAB`, and it is the panel's

`panel::PULLED_THROUGH = GRAB` = 6, in `karakuri-console` where every other number about a hand
already is. `STOPPED` is where *held* begins — a boundary against its stop is the ordinary end of a
drag, met every time an operator makes a pane as narrow as it goes — and closing the pane at the
first hundredth of a pixel past that would make the fold an accident of where the drag ended.

**`GRAB` rather than a number chosen here.** It is this console's one statement of how far from a
boundary a hand is still on it, and past it the pointer is further from the boundary it is holding
than the distance at which it could have taken hold of it at all: it is no longer pressing on the
boundary, it is pulling the region out from behind it. A second number would be a second answer to a
question the panel has already answered, which is ADR-0295's own argument for `GRAB` one control
along — and it reads the same both ways round, `GRAB` past the stop to close and `GRAB` in from the
closed edge to open.

**The threshold is not in `karakuri-layout`.** That crate is *pure geometry and constraint solving*
and carries no pointer number at all; a rule about how hard somebody is pulling is a hand, and a hand
in there would be the first.

### 5. The overshoot is read after the drag, and its sign says which side

`Panel::moved_boundary` calls `set_divider` first and reads `by = landed - asked`. Its **sign** says
which side of the boundary the pointer is pressing into, its **size** says how far past the stop the
hand has gone on, and whether that side is already `is_closed` says whether the drag is closing it or
opening it. Opening is tested first, because a boundary with a closed pane beside it is not one
anybody can be pushing against.

**After the drag rather than before it**, and that is a defect this record caught rather than a
preference: measured against the boundary's position *before* the move, a drag that runs from a
pane's full width to well past its minimum in one pointer event finds the pane not yet at its stop
and declines the fold it plainly asked for.

**A pane is closed only when it is at its own minimum**, not merely when the boundary is held.
`set_divider` clamps against the pair's *combined* bounds, so a boundary can be stopped by the far
side's maximum with this side nowhere near its floor, and folding it there would be a fold nobody
asked for.

### 6. A close is `Op::Fold`, performed here, and reported as `Dragged::Pane`

**Performed**, because the module's own division says so: *a boundary drag moves the arrangement,
which is this crate's, so `moved` writes it and reports where it landed*, where a fader drag moves
the engine's value and only reports it. Handing an unperformed `Op` back would leave the layout
disagreeing with itself for one event, on the pane the next move is about to ask a question of.

**An `Op` at all**, because a drag that only zeroed the pane's extent would leave nothing
`is_collapsed` could answer yes to, and `z` (`Op::UnfoldAll`) would not bring it back — which is the
way back the page promises for everything that folds. It costs nothing: `karakuri-operation-record`
answers `Silent::Surface` for all four fold operations either way, so nothing routes through the
window's `written` and there is no `Operation` for it to route as (ADR-0204).

`Dragged` gains one variant, `Pane(Op)` — `Op::Fold` where a pane was pulled out, `Op::Unfold` where
one was pulled back. One value and not two: the node is inside the operation already, which is
`Dragged::Fader`'s rule one drag along.

### 7. A reopened pane comes back at its declared minimum, and the layout's own clamp says what that is

The hand that opened it is at the window's edge; a pane springing back to the 340 it was would jump
out from under the pointer. So `Panel::open_at_minimum` asks `set_divider` for the boundary to go all
the way to the pane's own side — `f32::NEG_INFINITY` or `f32::INFINITY` — and the clamp
`set_divider` already applies is what stops it at the pane's minimum. **Working the position out here
would be a second copy of that clamp**, one term of which is the *other* region's bounds. What the
pane was is still stored, and `z` still brings that back.

### 8. One gesture asks for one fold

`Boundary::acted`. Without it the two conditions are each other's inverse the instant either fires —
a pane closed at `GRAB` past its stop puts its own edge under a pointer that is already `GRAB` the
other side of it, which is the open condition, met on the very next move — and the pane would flicker
at sixty asks a second for as long as the button was down. So it is once per gesture and the way back
is another gesture, which is `Fading::said`'s rule about a drag that has already said what it had to
say. The boundary's new position is recorded as what this drag *said*, so the fold is reported once
and the moves after it have nothing to add.

## Consequences

- `view::pane_edge` and `view::FOLDING_PANES` are **deleted**. Which panes fold this way is now
  `karakuri_console::arrangement`'s `.keeps_its_edge()` on `left-pane` and `right-pane`, which is
  where the layout can read it, and `tests/fold_grip.rs` walks the tree for the list rather than
  holding one.
- **`crates/karakuri/src/main.rs` owed two mechanical repairs and they are made**: `pane_edge` is
  gone from its `use`, and its `moved` arm covers `Dragged::Pane`. Neither is the registration; that
  is still owed for the grip.
- **`Fold a pane away` is reachable from the panel today** — no `PROBES` row, no press arm — and its
  badge on `docs/manual/operations.html` is still `plan`. The badge and `tests/vocabulary.rs`'s
  demonstration move together and neither is taken here.
- **`docs/manual/console.html` and `operations.html` do not describe this**, and a control an
  operator cannot be told about is half a control. The pages are the maintainer's.
- `tests/vocabulary.rs`'s grid sweep still asserts that nothing it does folds anything, and **its
  sentence has moved**: it held because no press could fold, and it now holds because the grid drags
  24 pixels one way and every boundary it can take hold of has that much room to give. The assertion
  is narrower than it was and its message says so, with the three readings a failure now has.
- `tests/panel.rs`'s two drag-out-and-back tests skip the boundaries beside a pane that keeps its
  edge, because a drag far past every stop is now the gesture that closes one. `tests/fold_grip.rs`
  is where that is the subject rather than the accident.
- The wire format gains `edge`, defaulted: a file written before this says nothing about it and what
  it means is *false* — every node in it folds the way every node used to.

## What survives of ADR-0295, and what does not

**The grip stays, whole.** `view::bay_grip`, `view::FoldGrip`, `view::BAY_GRIPS`, the reservation the
head makes for the mark, the 0.75 of a pixel it lends the boundary above it, and the reading that *a
reading which is a control does not get a second shape invented for it* — none of that is touched.
The bay head never had the conflict, and *Fold a bay away* is still waiting on the registration
alone.

**Sections 4, 5 and 7's second half do not survive.** The band on the pane's outer edge is gone, the
ordering that let a library row win in three pixels is gone with it, and *there is no unfold on either
control* is now true of the grip and false of the pane: a closed pane's own boundary is the unfold.
Section 6 — *a fold writes no session record, and neither of these routes through `written`* — holds
unchanged and is restated above.

**Its open questions**: *whether an unmarked band at the window's edge is the right affordance* is
answered by being withdrawn — there is no band, and what is at the window's edge is a divider, which
is the one thing on this console an operator already knows how to grab. The other two are untouched.
