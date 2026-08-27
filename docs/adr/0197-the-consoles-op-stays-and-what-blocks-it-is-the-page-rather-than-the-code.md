---
id: 0197
title: The console's `Op` stays, and what blocks its migration is the page rather than the code
status: accepted
date: 2026-08-27
supersedes: []
superseded_by: []
principles: [0074, 0036]
tags: [console, vocabulary, surfaces, layout]
---

# The console's `Op` stays, and what blocks its migration is the page rather than the code

## Context

`docs/roadmap.md`'s item 3 is *every surface routes into the named operation*. Two of the four
have moved. The console's deck controls did it a control at a time — a drag becomes
`Operation::SetGain` and the panel applies nothing
([ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md)), the blend
mini and the tally chip the same way
([ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
[ADR-0195](0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md)) — and
`karakuri-midi` moved a whole surface at once, deleting `Action`
([ADR-0196](0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)).

The one named as next is `karakuri_console::panel::Op`: the console's own arrangement operations —
`Fold`, `Unfold`, `FoldEnclosing`, `UnfoldAll`, `Solo`, `Unsolo`, `Reset`, `Report`. This record is
the survey that was done before writing any of it, and the survey is the decision.

### It is not the migration MIDI did, because there is no record and no second surface

`karakuri-operation-record`'s `written` answers `Written::Silent(Silent::Surface)` for `FoldBay`,
`FoldPane`, `Unfold` and `Solo` — *it is a surface's own state and writes no record*. So
`Live::operate`, `Record` and the compiler's exhaustiveness over forty-six variants, which are what
the MIDI path gained, are none of them on this path. Nothing downstream of the panel changes at all.

What routing means here is the other direction. On MIDI the surface **emits** an `Operation` and
something else performs it; the panel **performs** its own, so routing into the vocabulary means the
panel **accepting** one — a caller says `FoldBay { bay: "mixer" }` and the panel resolves the name.
That is an inbound adapter, and today nothing is inbound: `docs/manual/operations.html` marks every
route on all five *Arranging the console* rows `—` except the pointer, and *Bring back what is
folded* has no route at all. `karakuri_layout::NodeId` has a private field and only a `Layout` hands
one out, which is exactly why a caller with no `Layout` needs the name — and there is no such caller
yet.

### The two-way inventory, which is what the survey found

Eight variants against five rows.

| `Op` | Row in *Arranging the console* |
|---|---|
| `Fold(NodeId)` | *Fold a bay away* **and** *Fold a pane away* — one variant, two rows |
| `FoldEnclosing(NodeId)` | *Fold a pane away*, when the enclosing split is a pane |
| `Unfold(NodeId)` | *Bring back what is folded* |
| `UnfoldAll` | *Bring back what is folded* |
| `Solo(NodeId)` | *Solo a region* |
| `Unsolo` | *Solo a region* |
| `Reset` | **none** |
| `Report` | **none** |
| — | *Move a boundary*: **no `Op`**, and there was never going to be one |

Four gaps, and each is a question about the page rather than work in the crate.

1. **`Reset` and `Report` are operations nobody specified.** The page is the specification for
   which operations exist — that is what `karakuri-operation`'s own manual test is built on — so a
   variant with no row is not a variant to rename. Giving each a row is an edit to the
   specification.
2. **`Move a boundary` is a gesture, not an operation.** It is `Panel::press`, `moved` and
   `released` over `Layout::hit`, and `Operation::MoveBoundary` carries `Undecided` for the two
   reasons stated at that variant. Nothing here changes it.
3. **`FoldEnclosing` names its target by relation**, where every row names one outright. It is an
   addressing form over `Fold` — *the split around whatever the pointer is over* — and the
   vocabulary has no way to say it, nor should it.
4. **Two splits in the console's arrangement have no name at all**, and this is the one that costs
   something. `Layout::name` answers `None` for a split the arrangement left unnamed, and the
   console has exactly two: the root column and the body row holding the three panes. Both are
   handed to a caller as `Hit::Divider { split, .. }`, `examples/panel.rs`'s `g` turns that into
   `Op::Fold(split)`, and `Outcome::Folded { root: true }` exists to report one of the two. A
   `String` cannot say either.

## Decision

**`panel::Op` keeps its `NodeId` and does not become `Operation`. The inventory above becomes a
test instead** — `crates/karakuri-console/tests/vocabulary.rs`, which reads the checked-in page and
checks it against the type in both directions, exactly as `karakuri-operation`'s
`the_manual_and_the_vocabulary_agree.rs` does one level in.

Five assertions, and four of them are one of the gaps above made executable: a row an `Op` names
that the page does not have, a row the page has that no `Op` reaches, the variants with no row being
exactly the two written down with their reasons, and **the unnamed splits being exactly two and
being those two**. The last one is the cost of the migration, counted rather than described: a third
would be a third region only a mouse could fold, arriving without anyone deciding it should.

**A region is named by its layout name where it has one**, and that half was already settled —
`Layout::new` refuses an arrangement that uses a name twice, so *"the answer here is the only node
that could be meant"*, and `crates/karakuri-console/src/lib.rs` states region by region why each
name is the manual's word. `Operation::FoldBay` says the same at its own definition. **What is not
settled is the two splits that have no name**, and naming them is a decision about the page: the
manual does not say the root folds, and *"the centre is not a third pane and does not fold"*
([ADR-0159](0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md)) is the nearest thing to a ruling on the body
row.

## Alternatives rejected

**Replace `Op` with `Operation` now.** It is the roadmap's own wording and it is the obvious move.
It fails on all four gaps at once: `Reset` and `Report` would have to leave the type or arrive on
the page unspecified, `FoldEnclosing` would have to be resolved to a name by the caller and would
resolve to `None` over the transport or the outputs row, and the two unnamed splits would stop being
foldable — a behaviour change with no row asking for it and nothing in the manual to appeal to. It
also puts a `String` allocation in the model on the pointer path, which `panel.rs` is written to
keep free of everything the view carries (ADR-0156).

**Add `Panel::operate(&Operation)` beside `Panel::op` as an inbound adapter.** Cheap, additive,
loses nothing — and it has **no call site**. The keyboard resolves a pointer to a `NodeId` and would
round-trip it through a name it may not have; the Outputs dot already holds the `NodeId` it drew
from; MIDI and MCP do not reach the console. *Before adding an abstraction, confirm it has at least
two call sites* — this has none, and an adapter with no caller is a second answer to *how a region
is named* waiting to disagree with the first.

**Name the root and the body row so the vocabulary can reach them.** This is the decision worth
taking and it is not this record's to take: it asserts that folding the whole panel away, and
folding the row of three panes, are operations an operator wants and a MIDI map should be able to
ask for. Neither has a row on the page. Recorded here as the thing that unblocks the migration
rather than decided.

**Say nothing and leave the inventory in a session transcript.** What the roadmap said before this
was *the console's `panel::Op` has not moved*, which reads as work not yet done. It is four
questions about the specification, and the difference is what somebody picking it up next needs.

## Consequences

- The console's suite now fails when a row in *Arranging the console* is renamed, added or deleted,
  which is a page this crate did not previously read.
- The four gaps have one place that names them, and closing any one of them is a page edit that the
  test will demand a matching change for.
- `docs/roadmap.md`'s item 3 keeps `panel::Op` on the list of what has not moved, and now says what
  it is waiting on. The CLI's key handler is untouched by this and is still its own change.
- `Op`, `Outcome` and `repaint.rs`'s closed list of changes are unchanged, so
  [ADR-0165](0165-the-repaint-decision-is-one-closed-list.md) is untouched.
