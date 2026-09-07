---
id: 0273
title: The carry lands on two sets of rectangles and wears a face
status: accepted
date: 2026-09-07
supersedes: []
superseded_by: []
principles: [0090]
tags: [console, library, mixer, program, operations]
---

# The carry lands on two sets of rectangles and wears a face

## Context

[ADR-0265](0265-a-carried-set-names-its-deck-at-the-release-and-the-panel-refuses-no-drop.md) built
the carry: a press on a Library row takes a Set in hand, a move asks for nothing, and the release
names the deck by the mixer strip it landed on. It shipped with one destination and no drawing at
all, and its own Consequences say so twice — *"What the drag does not draw is the carry itself.
There is no drop-target state, no ghost under the pointer and no mark on the row in hand beyond
`.lib-row.cursor`, because the mock draws none of those"*, and *"`InHand` gained a third answer and
the cursor gained no third shape."* Both clauses were conditional on the page, and the page has
moved.

**`4ec14db` specified all three of them**, and specified a second destination with them. What
`docs/manual/console.html`'s *How a Set reaches a deck* now says:

> **While the Set is in hand, the rectangle under the pointer is ringed, and it is the only thing on
> the panel that is.** … **It says where the release lands and never whether it will be allowed**:
> every strip drawn is a target, nothing about the deck under the pointer is read, and a drop on a
> deck that is live asks for the load exactly as the key does.

> **Over anything that is not a target, nothing is lit.** … Over an alley, over a bay head, over the
> transition row, over another bay entirely: no ring anywhere, and letting go there loads nothing and
> says so. **The pointer itself is a grab for as long as the Set is in hand**, which is this
> console's third cursor after the arrow and the resize, and it is the one thing that says a gesture
> is still running while the hand is over nothing at all.

> **The four deck preview cells take a drop as well, and each names the deck its letter names.**

`docs/manual/style.css` carries the mark as `.strip.drop, .cell.drop { outline: 2px solid
var(--c-text); outline-offset: 0; }` with the argument for the ink and for the side of the edge
written above it, and the mock draws deck A's strip wearing the drop mark and the selection at once.

**So the question this record answers is not whether to draw it.** It is what a cell drop *means*
where a strip drop already has an answer, and what the panel does with the one rectangle the page
draws for a deck that does not exist. The mixer draws a strip per slot and a deck holds one to four
of them (`MAX_SLOTS`, asserted in `Deck::new`); the preview row is `DECKS` cells whatever the deck
holds, because *"a cell that vanished would move the other three and the letter is the only thing
naming a deck"*
([ADR-0170](0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md)). On a
three-slot deck those two numbers differ by one, and that cell is a rectangle whose letter names no
deck.

**The page said something about that case which was not true.** Its own note read *"a drop on the
fourth cell can name a deck the keyboard's own route to this operation cannot reach today"*, filed
as unsettled. It is not: `pointed` (`crates/karakuri/src/main.rs`) refuses `SelectDeck` on
`View::mixer.len()` — the **slot count** — and `0`–`3` emit `SelectDeck` unconditionally, so a deck
with no slot is refused from the keyboard already, and `l` loads wherever the selection is. The two
routes differ in nothing there.

## Decision

**Four things, and the last is what makes the first three one decision rather than three.**

1. **The drop mark.** `View::draw` resolves *where a carried Set would land* once for the frame,
   off the derivations the release itself asks — `Mixer::dropped` for a strip and
   `ProgramBay::dropped` for a cell — and `view::drop_ring` paints
   `size::DROP_RING` of `pal.text` on the outside of that one rectangle's edge
   (`StrokeKind::Outside`, at the target's own corner radius). The selection's ring is inset and
   this is not, so a strip wears both.
2. **The cells are the second set of rectangles a release can land on.** `ProgramBay::dropped`
   answers the deck a cell's letter names, and `crates/karakuri/src/main.rs`'s release arm asks it
   after `Mixer::dropped`. The order cannot matter: the strips are in the right pane and the cells
   in the centre column, so a point inside one set is outside the other.
3. **The cursor is `CursorIcon::Grabbing` for as long as the Set is in hand**, wherever the pointer
   is, and `view::cursor`'s argument against it is rewritten rather than deleted.
4. **A cell whose letter names no deck is not a target.** `ProgramBay::dropped` takes the deck's
   slot count beside the point and answers `None` past it, which is `View::select`'s rule met by a
   second surface: *"a deck the mixer has no strip for is refused … a selection past the deck's
   slots would be a ring nowhere and a letter naming a deck the press would be turned down on."*

**How the console knows the slot count, and that it is not a fifth reading.** `View::mixer` is
*"one per slot the deck has, in slot order"* — the strips the host writes per frame — so its length
is the deck's own count arriving the way every other reading does. It is what `View::select`
refuses on and what `pointed` prints, and `ProgramBay` is handed it rather than reading it because
that type is the bay's *geometry*, derived from a solved layout and nothing else.

**That is not the refusal ADR-0265 forbids.** What is never read is the deck's **residency** and the
cell's **material**: a drop on a live deck asks for the load, and a cell drawing nothing because no
engine has handed it a picture is a target like any other — `View::previews`'s `None` is two states,
*a deck of fewer slots than there are cells* **or** *a console with no engine behind it*, and only
the first is a letter naming no deck. What is read is whether there is an operand at all. The mark
still says *where* and never *whether*: a ring on a cell that loads nothing would be the mark
promising a landing that is not made, which is the same failure as a mark snapped to the nearest
strip.

**The page's own unsettled clause is corrected rather than carried.** `docs/manual/console.html`
now says the fourth cell is refused, that the keyboard is refused on the same count, and that
*no slot* under a cell is two states written as one word. `docs/manual/operations.html`'s row and
the mock's deck D tooltip carry the same clause.

## Alternatives rejected

**Keep the mixer as the only destination, which is what shipped.** The smallest answer and the one
already working: `Mixer::dropped` needs no argument, `ProgramBay` stays pure geometry, and there is
no fourth-cell case to decide at all. It loses to where the two bays *are*. The Library bay is in
the left pane and the mixer in the right, so a carry between them crosses the whole window, and the
cells sit in the centre column beside the list the Set comes out of — the page calls them *"the near
destination for the same command rather than a second command"*. It also loses to what a cell is:
nothing routes one, `A` is deck A whatever is loaded (ADR-0240), so a cell is an operand and never
a choice, and refusing the drop there would be refusing the shortest route to a deck the letter
already names. The cost is one `or_else` in the release arm and one entry in
`press_handler::TABLE`.

**A ghost of the row following the pointer.** The obvious drag affordance, and the one every file
manager has. It was declined in the design pass and the page states why for both it and the mark
below: *"What is drawn is the carry and not the Set: there is no ghost under the pointer, and the
row in hand keeps the cursor mark it already had and gets nothing else."* The argument is that it
tells the operator something they already know — a hand that is dragging knows it is dragging, and
what it does not know is where a release lands. A ghost also has to be drawn *over* the panel, which
on this console means over the program picture and over four running previews, in the one region the
page protects from anything the console wrote (`.preview`'s own rule, and the caption under the
image rather than on it).

**A mark on the row in hand.** The cheaper half of the same idea: light `.lib-row` while its Set is
being carried. Declined for the ghost's reason, and it costs something the ghost does not — the row
already wears `.lib-row.cursor`, which is *which Set `l` would load*, and a second mark on the same
rectangle in the same moment is two things to tell apart where the page has just spent a paragraph
telling the selection and keyboard focus apart. The cursor is on the row in hand already, because
the press moved it there (ADR-0265).

**No cursor change**, which is the argument `view::cursor` carried until this record:
*"`CursorIcon::Grabbing` would be a third mark in a vocabulary of two, said by one control on the
panel."* It is overruled by the page, and the argument was wrong on its own terms twice. A carry is
**not** said by a control — the Set leaves the Library bay's list and is over no control at all for
most of the gesture, which is the one state on this panel that nothing drawn can report. And a carry
is the only drag here that can be **cancelled**: a boundary let go off its track still lands
somewhere legal and a fader dragged past its end is still at its end, so a cursor for either would
be decoration, where this one is the difference between *a gesture is still running* and *the panel
missed the press*. The comment is rewritten to say that rather than deleted, because a reader who
proposes removing the grab should meet the reason it is there.

**Lavender for the mark, or mint.** Ruled out by the page twice over and re-stated here because it
is the cheapest thing to get wrong. The selection is already a lavender ring round a strip meaning
*this is the deck the keys are addressed to*, and the deck being carried to is usually the deck
already selected — so a lavender drop mark would be the same mark twice, on the same strip, in the
moment it is read fastest. Mint means *armed, on, or held by a signal*. `--c-text` is the one ink
that names no state, which is exactly what a mark saying *where* and not *whether* has to say.

**Mark the nearest strip when the pointer is between two.** The forgiving reading, and the mirror of
the release ADR-0265 already refused. `.mixer-strips` has a `gap: 4px` and a `padding: 6px`, and
those are real places to let go — so a ring that snapped would name a deck nobody pointed at and the
release after it would load one. The mark and the release answer off the same derivation for that
reason: two spellings of *which rectangle is the pointer in* would be a ring round one deck and a
load into another.

**Mark the fourth cell anyway, and let the release be the thing that refuses.** The reading the
page had, and it is a coherent one: a cell is welded to its letter, so ring it and let the operator
find out. It loses because the ring is a promise about the next event and nothing else — the page's
own *"it says where the release lands"* — so a ring on a rectangle that loads nothing is worse than
no ring, and it is worse in exactly the moment the operator is going fastest. It also loses to the
keyboard: `3` on a three-slot deck is refused with a sentence, and a pointer that accepted where a
key refuses would be the second rule in the second place that P-0090 and ADR-0265 both exist to
prevent.

**Read `View::previews[deck]` instead of the slot count.** The reading that is right there — the
cell already knows whether it has a picture, and the caption under it already says `no slot`. It is
wrong twice. It reads the *material*, which the page forbids: every test in `karakuri-console` and
every console with the preview row folded has all four entries `None` while the deck has four slots,
so the drop would be refused on every deck for want of a texture. And `no slot` is two states
written as one word, only one of which is a letter naming no deck.

## Consequences

- **`ProgramBay::dropped(p, slots)` is the console's second drop derivation**, and it is the only
  offer in either bay that takes a reading beside the point. `ProgramBay::cell` and
  `ProgramBay::owns` are unchanged, and `tests/preview_cells.rs` still holds the boundary
  arithmetic.
- **`press_handler::TABLE` gained `("ProgramBay", "dropped", "program_bay", "cells")`**, the second
  entry in that table asked on a button **up**. `EXEMPT`'s `("ProgramBay", "cell", …)` gained a
  composer — it was `None`, meaning *a real offer this window never asks*, and `dropped` now
  composes it. A press on a cell still asks for nothing, and ADR-0240 is still why: nothing is in
  hand at a press, so there is no second operand for it to name.
- **`mixer_into` takes a sixth argument and `caption_into` a sixth**, both pointers rather than
  readings: which strip is marked, and whether this cell is. The cell's is `.cell.drop .caption b`'s
  `color: var(--c-text)` — the letter comes up out of its dim with the ring, because the ring says
  where the release lands and the letter says which deck that is.
- **`room::size::DROP_RING` is the one constant added**, citing `.strip.drop`'s `outline: 2px solid
  var(--c-text)`. The radii are the target's own, `STRIP_RADIUS` and `PREVIEW_RADIUS`; the mock's
  `.cell.drop` sets a `border-radius: 8px` of its own for the `.cell` box, which is a box with no
  background that this console does not draw — a cell here is its image and the caption under it.
- **The console rings the image and not the image plus its caption**, which is the one place the
  paint is narrower than the mock's selector. The reason is the rule the mark lives by: `cells[deck]`
  is exactly what `ProgramBay::cell` hit-tests, so ringing more would be marking ground a release on
  which loads nothing. The caption is not left out of it — its letter lifts.
- **`view::cursor` sets `CursorIcon::Grabbing` and returns**, so a carry never reaches the boundary
  hit test. Still one writer and one icon per frame: `egui_winit` writes the window's cursor out of
  `PlatformOutput`, and a second setter is a flicker that depends on event order.
- **ADR-0265's *"`InHand` gained a third answer and the cursor gained no third shape"* is history
  and stays as written.** This record is where it stopped being true, per `docs/contributing.md`
  §4 — *an ADR is a description of history*, annotated with what it became rather than revised.
  The same goes for its *"What the drag does not draw is the carry itself"*, whose own last clause
  named the condition that has now been met: *"anything richer is an edit to `console.html` first."*
- **The page's unsettled clause is gone and is not replaced by another.** The two routes refuse the
  same decks, on the same count, so there is nothing left open about the fourth cell.
  `docs/manual/console.html` carries the settled rule in *How a Set reaches a deck* and in *What a
  deck preview cell shows, and when*; `docs/manual/operations.html`'s **Load material into a deck**
  row and the mock's deck D tooltip carry the clause; nothing about the drawn mark changed, because
  the mock already drew it.
- **A release over nothing now names two ways to land**, in the window's own line: *"a drop names
  its deck by landing on that deck's strip, or on its preview cell in the Program bay."*
- **Five tests hold this in `crates/karakuri-console/tests/carry.rs`** — the cell drop, the cell
  with no slot, the one-rectangle mark, the nothing-is-marked cases, and the grab — and one in
  `crates/karakuri/src/main.rs`,
  `a_drop_on_a_preview_cell_loads_the_deck_its_letter_names`, which is the release arm asking the
  second bay at all. **No source scan was needed for any of it**: the mark and the cursor are
  `View::draw`'s and are read out of the frame's own shapes and `PlatformOutput`, and the routing is
  `Readout::pointer`'s, which exists as a method precisely so a gesture can be driven without a
  window.
