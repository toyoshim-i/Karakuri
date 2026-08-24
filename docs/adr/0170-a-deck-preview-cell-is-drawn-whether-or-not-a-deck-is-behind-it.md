---
id: 0170
title: A deck preview cell is drawn whether or not a deck is behind it
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# A deck preview cell is drawn whether or not a deck is behind it

## Context

`karakuri-console` draws every region of the mock and puts **nothing** in any of them, on a rule
written into `view.rs`'s own documentation: *scaffolding that looks finished does not get
replaced*. A greyed-out control that hints at what will be there is worse than an empty box,
because the next person to open the panel is in doubt about what exists.

The Program bay's picture was the one exception and is not really one — its texels come from a
device rather than from this crate's imagination. The row of deck previews under it was the last
region in that bay with a rectangle and nothing in it, and building it forced two questions the
rule does not answer.

## Decision

### A cell is painted whether or not a texture was handed in

Every cell draws `.preview`'s well, its 7px radius, its inset hairline, and the deck's letter. A
cell with a texture behind it draws the texture and labels itself `A`; a cell with none labels
itself `A · off` in `--c-faint`.

**Rejected: draw nothing in a cell nothing is behind, exactly as every other empty body does.** It
loses because it makes two different things look the same. *Off* is a state the manual gives the
operator a control for — the Program bay's head reads *previews 2 of 4* and its tooltip says
*click to choose how many* — and *not built yet* is not a state at all. Drawing nothing renders
both as blank, and the operator who turned a preview off has no way to tell the console heard
them.

The rule the crate actually holds is narrower than the sentence it was written as, and this is
where the difference shows: **nothing is drawn that claims something exists which does not.** A
cell that is off exists. It is the region's face rather than a placeholder for one, which is why
the two rules are one rule and not an exception to it.

### A cell is 16:9 and centred in its track, rather than filling it

`preview_rects` divides the row into four equal tracks with `.previews`'s 6px gap between them,
then puts a 16:9 cell in each — as large as the track's width and the row's height both allow,
centred. At the width the mock draws, that is exactly the mock: 112 x 63, filling the track edge
to edge.

**Rejected: fill the track and letterbox the texels inside the cell.** It keeps the row looking
like a grid at every width, and pays by stretching the cell away from the shape of what it holds —
a 16:9 audition in a 200 x 63 well, with bars this crate draws itself inside a rectangle
`Present::draw` has already fitted. That is two answers to *how does a 16:9 canvas sit in this
box*, which is the thing `WHOLE_TEXTURE` refuses one level up for the picture. Centring costs
ground either side of a cell at wide windows, and ground either side is what the console shows in
every divider anyway.

## The constraint that forces the choice, and it is a disagreement worth naming

**The mock's preview row grows taller with the window and the arrangement's cannot.** `.preview`
carries `aspect-ratio: 16/9` in a grid whose tracks are `1fr`, so in CSS the cell's height
*follows* its width. `karakuri-console`'s arrangement pins `deck-previews` at fixed 72 and minimum
72, on the grounds that *there is nothing in a row of four cells at a fixed type size that gets
smaller* — true of shrinking, and silent about growing.

So the two agree at exactly one width, `.console`'s `min-width: 1010px`, and part company at every
width above it. The mock is a reference at its own narrowest and nowhere else, and a reader who
takes a growing row out of the CSS is reading the mock correctly and the console wrongly. Once the
height is fixed, a cell cannot both fill its track and stay 16:9, and one of the two has to give.

**What would reopen this:** the row ceasing to be a fixed height — an operator given a boundary to
size the previews with, or a bay that lets the row grow with the window as the CSS does. Then
filling the track is available again and this record is the argument to weigh against.

## Consequences

- `Kind::Previews` joins `Kind::Picture` as a region kind of its own, for the same stated reason:
  `View::draw` has to know *which* pane the cells go in, and the alternative is comparing a name on
  the frame path.
- `View::previews` is `[Option<Picture>; 4]` — the same seam as `View::picture`, four times over.
  Four because `karakuri_engine`'s `deck::MAX_SLOTS` is 4; the number is transcribed with its
  derivation rather than reached for by adding a dependency, which is the seam
  [ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md) left standing.
- **The mock said `C · off` and `D` for two cells in the same state.** Its head reads *previews 2
  of 4*, so both are off and only one said so. `console.html` now says `D · off` too, because the
  manual is the specification and a specification that disagrees with itself in adjacent cells is
  the fourth such disagreement this bay has produced.
- `picture_rect`'s own derivation called `.program-body`'s `gap: 8px` a 9. The arithmetic
  everywhere was right and only the sentence was wrong; it is corrected rather than left, being a
  fact that was never true rather than an argument that changed
  ([P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md)).
