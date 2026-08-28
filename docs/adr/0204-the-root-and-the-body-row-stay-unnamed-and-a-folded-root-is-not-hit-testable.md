---
id: 0204
title: The root and the body row stay unnamed, and a node that is not laid out is not hit-testable
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0073]
tags: [console, layout, vocabulary, ui]
---

# The root and the body row stay unnamed, and a node that is not laid out is not hit-testable

## Context

[ADR-0197](0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md) is the
survey that was done before migrating `karakuri_console::panel::Op` onto
[`karakuri-operation`](../../crates/karakuri-operation). It found four gaps between eight `Op`
variants and five rows of *Arranging the console*, made the inventory a test
(`crates/karakuri-console/tests/vocabulary.rs`), and left exactly one of the four **named rather
than decided**: *"Name the root and the body row so the vocabulary can reach them. This is the
decision worth taking and it is not this record's to take."*

`Layout::name` answers `None` for a split the arrangement left unnamed, and the console has exactly
two — the root column and the body row holding the three panes. Both fold through the pointer today:
`Layout::hit` hands a split out as `Hit::Divider { split, .. }`, `examples/panel.rs`'s `g` turns
that into `Op::Fold(split)`, and `Outcome::Folded { root: true }` exists to report one of the two.
An `Operation` names a region by `String`, so both would be lost by the migration.

This record takes that decision, and it lands with a behaviour change the decision made visible.

## Decision

### The two splits stay unnamed, and `panel::Op` therefore stays `Op`

**Folding the root produces a blank window, and a blank window is not something an operator asks
for.** The manual already reaches the outcome they do want — *only the picture on screen* — by a
different operation, and says so in its own row: *Solo a region* is *"the panel folding away and
only the picture left, which is also how you capture this window."* A blank window and the picture
are two different results; naming a row for the first would put an operation on the specification
whose whole effect is that there is nothing to see, beside a row that already delivers what was
being reached for.

**The body row has no word in the manual at all.** [ADR-0159](0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md)
went through exactly this exercise for the three columns — the manual *"names every bay and has no
word at all for the three columns those bays are stacked in"*, so `left pane`, `centre` and
`right pane` were chosen and went back into it — and it did not name the row holding them. The row
appears in the manual only as `.body-grid`, a class in `style.css` and the mock's markup. A name
invented for it here would be an operation specified from the *arrangement* rather than from
anything an operator does, which is the specification written backwards
([ADR-0198](0198-a-gesture-converts-in-the-parts-that-are-decided.md) §4 refuses the same move for
the status line).

**And a map line for either would need both directions, or it strands the operator.** A folded
region has no rectangle, so the pointer cannot reach it to undo itself — the page says that
outright, and calls this *"the one operation here that a mouse cannot be the only way into"*. A pad
that folded the root would leave a black window whose only way back is a route neither MIDI nor MCP
has: they would each need the unfold as well, which is a second row on a page that has none for the
fold.

### What it costs, and it is a cost rather than a saving

**`panel::Op` cannot become `Operation` without losing two foldable regions.** So the console's
arrangement operations stay the one surface that does not route into the named vocabulary, while
the deck controls (ADR-0185, ADR-0187, ADR-0195, ADR-0203), `karakuri-midi` (ADR-0196) and MCP
(ADR-0199) all have an answer. ADR-0197's rejected alternative — *"Replace `Op` with `Operation`
now … the two unnamed splits would stop being foldable"* — is the chosen one, taken deliberately
and with the reason recorded here rather than met later as a surprise. `docs/manual/operations.html`'s
*"Arranging the console is reachable from a pointer and nothing else"* stays true, and for these two
regions it stays true permanently rather than until somebody gets to it.

The count is pinned: `exactly_two_splits_have_no_name_for_an_operation_to_use` fails if anybody
names either, and its reasoning now says the decision was taken rather than pending. A **third**
unnamed split would still be a defect — a third region only a mouse can fold, arriving without
anybody deciding it should.

### A node that is not laid out is not hit-testable, root included

**With the root folded, the pointer resolved to regions nobody could see**, and that was verified
against the code before it was changed: `a_folded_root_is_not_a_hit_target` failed with
`hit` answering `View(NodeId(4))` — the library bay — at a point on a panel drawing nothing at all.

The mechanism is an asymmetry between two walks. `Layout::hit` starts at the root and tests each
node's **children** against `out_of_layout`, so the root is the one node nothing asks about, and it
has no parent to ask on its behalf. The solve has the same shape and it is why the rectangles are
still there: a fold takes a node's extent out of its *parent*, the root's rect is the viewport
whatever its own bits say, and everything under it solves as usual. Meanwhile `view::plan_into`
skips every node `Layout::visible` says no to, and `visible` walks **up** to the root. So on a blank
panel `f`, `g` and `s` went on folding and soloing invisible regions.

**`Layout::hit` answers `Hit::Nothing` when the node it would report is out of the layout, root
included.** In the code that is one guard on the root, because every other node is already reached
through the filter in the descent — which is the whole of *the node reported is laid out*, and is
why the change cannot widen past the root by accident.

The rule family deciding it is already in force. `karakuri-layout` measures what a node **can use**
so that a fixed bay whose content is folded away claims only what its visible content can use
([P-0073](../principles/0073-a-node-claims-only-what-its-visible-content-can-use.md),
[ADR-0174](0174-a-node-claims-only-what-its-visible-content-can-use.md)), and `View::animating`
declares a staleness only where the region drawing the pending thing is laid out
([ADR-0193](0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)).
Space, then time, and now the pointer: **a region that is not laid out claims nothing, declares
nothing, and is under nobody's pointer.** ADR-0193's first argument transfers word for word — a
staleness for an invisible region is not merely unaffordable but *false*, and a hit on an invisible
region is a claim that the operator is pointing at something that is not on screen.

**No principle is added, deliberately.** The rule is P-0073's and ADR-0193's on a third axis rather
than a new constraint, and the sentence a principle would state — *a region that is not laid out is
not there, whichever question is being asked of it* — would be a fourth copy of a rule already
stated once, which is what [ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md)
exists to prevent. If a fourth axis appears, generalising P-0073 is cheaper than adding a file.

### What that leaves working, checked rather than assumed

`Op::UnfoldAll` and `Op::Reset` **take no target**, so the way back was never the pointer's:
`UnfoldAll` reads the arrangement for what is collapsed, and `Reset` builds a fresh one at the same
viewport. `Op::Unsolo` is the same shape. `Op::Unfold(id)` takes a node and reaches it by id or by
name, neither of which is a rectangle. That is asserted at the panel —
`a_folded_root_answers_no_pointer_and_z_and_r_are_still_the_way_back` — with the round trip compared
rectangle for rectangle against the arrangement before the fold.

`examples/panel.rs` needed no change: `target` and `enclosing` already answer `Hit::Nothing` with
*"nothing under the pointer"*, and the fold that produced the blank window already prints *"that was
the root, so the panel is empty; z brings it back"*.

## Alternatives rejected

**Name the two splits and complete the migration.** The move ADR-0197 named as the one that unblocks
everything. It loses on what naming *asserts*: a name on the page is a claim that an operator asks
for the operation, and *fold the whole panel away* is a request for a blank window when the row next
to it already delivers the picture alone. It would also owe two rows rather than one — the fold and
a bound way back — since a pointer cannot reach a folded region, and the page would be growing an
operation for the sake of a type migration. That is the specification following the code.

**Leave `Layout::hit` alone and have the console filter its result.** The console holds the layout
and could ask `visible` before acting on a `Hit`. It loses for ADR-0193's second reason: the rule is
about the *arrangement*, so the arrangement's own crate is where it belongs, and a filter at one
caller is a rule the next caller has to remember — there are four call sites already
(`Panel::press`, `Panel::under`, `input::claim`, `view`'s cursor) and each would carry the same two
lines. Worse, the filter would be a second derivation of *is this laid out*, which is the exact
duplication `Layout::visible` was extracted to end.

**Treat it as harmless, since only the example can fold the root.** True today and not a reason:
`Outcome::Folded { root: true }` exists because the model already reports the case, `g` over the
transport reaches it in one keystroke, and what the operator gets is a black window that answers
presses. A defect nobody has hit yet is still one nobody can see go wrong, and this one costs a
single guard.

**Make `hit` answer `Nothing` whenever anything in the tree is out of the layout.** Nobody proposes
this out loud, but it is what a careless reading of *not laid out is not hit-testable* implements,
and it would stop the panel answering the pointer the moment one bay was folded.
`folding_a_bay_leaves_everything_else_hit_testable` is the injected-defect test for exactly that,
and it is the shape of the rule the guard is written to hold: the answer is about the node being
**reported**, not about the arrangement having folds in it.

## Consequences

- **`Layout::hit` gains one guard**, and nothing else in `karakuri-layout` changes. `visible`,
  `boundaries`, `boundary` and the solve are untouched, so a folded root still has boundaries a
  caller holding ids can address; what it does not have is a pointer route to them.
- **The console's arrangement operations are outside the vocabulary for good, not for now.**
  `docs/roadmap.md`'s item 3 loses this from what it is waiting on and carries the cost instead.
  Two of ADR-0197's four gaps are still open — `Reset` and `Report` have no row, and *Move a
  boundary* is a gesture — and both are still page questions.
- **ADR-0197's front matter is annotated and its prose is untouched** (P-0066), the way ADR-0185 and
  ADR-0188 were annotated with what they later became.
- **What this does not reach: the controls the view derives from a rectangle.** `view::outputs` and
  `view::mixer` read `Layout::rect` and are `None` only where the rectangle is too small to hold the
  control — which a folded *row* produces and a folded *root* does not, since a fold empties a node
  by taking its extent out of its parent. So with the root folded `input::claim` can still hand a
  press to a control nobody can see. It is the same rule on the same axis one layer out, it is not
  new with this change, and it belongs in `crate::view`, which is being edited elsewhere as this
  lands. Named here so it is met as a record rather than as a surprise.
