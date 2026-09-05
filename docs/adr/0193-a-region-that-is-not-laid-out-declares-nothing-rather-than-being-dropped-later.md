---
id: 0193
title: A region that is not laid out declares nothing, rather than being dropped later
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: [0073, 0091]
tags: [ui, perf]
---

# A region that is not laid out declares nothing, rather than being dropped later

## Context

[ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md) has
two halves. The first is *a panel with nothing changing on it is paid for once and not again*; the
second is *what **must be live** is named, and each named thing declares what its update costs and
how stale it may get*. `View::animating` is the whole of the declaring half today, with one client —
the mixer strip's residency chip, which rolls at
[`ROLL_STALENESS`](../../crates/karakuri-console/src/view.rs) while a slot's request has not
landed ([ADR-0190](0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md))
— and `repaint::Change::Animating` turns the number into the deadline the window waits on
([ADR-0165](0165-the-repaint-decision-is-one-closed-list.md)).

**It answered off the deck.** `View::mixer` is a `Vec<Strip>` the harness rewrites every frame from
`Deck`, so *does any strip carry a request the engine has not granted* is a fact about the engine
and about nothing on screen. The console draws the chip inside the Mixer bay, and the bay can be
folded away — by `f` over it, by `g` over the pane around it, by a solo elsewhere — with the slot
behind it still parked, because parking is the governor's and no window event reaches it.

**Measured on 2026-08-26 on `examples/panel.rs` at 1440x900 logical, an Apple M4 Pro, host clock,
debug profile with dependencies at opt-level 3, the folds applied at startup through the same
`Op::Fold` the `f` key sends** — a temporary knob, because a synthesised keystroke needs an
Accessibility grant this process has not got, and it was removed before this landed. With the
picture and the preview row folded — nothing in the Program bay making texels — the window drew **84
and 85 frames in 3.0 s, 28.0 and 28.3 a second, at 427 and 425 allocations a frame** over two runs.
**Folding the Mixer bay on top of that moved the price of a frame and not the rate**: 87 and 86
frames, 29.0 and 28.7 a second, at 259 and 261 allocations — two runs taken with this record's
change backed out, which reproduces what P-0072 already had written down. The chip was off screen
for the whole three seconds of each, and the window went on being woken thirty times a second to
redraw a panel on which nothing an operator could see was moving.

With the change, the same three folds draw **0 frames, 0 allocations and 0 bytes over 3.0 s**, in
both of two runs.

That is P-0072 read as *what is pending declares* rather than as *what must be live declares*. It is
also the rule this repository already holds on the other axis:
[P-0073](../principles/0073-a-node-claims-only-what-its-visible-content-can-use.md) — **a node
claims only what its visible content can use** — where what is claimed is a share of the viewport.
This is that claim in time.

## Decision

**`View::animating` declares a staleness only where the region that draws the pending thing is laid
out.** It takes the layout, and the roll's term is `layout.visible(layout.find("mixer"))` and the
strips together.

### The question is asked of the layout, which already answers it

[`Layout::visible`](../../crates/karakuri-layout/src/layout.rs) is ADR-0183's disjunction — the
operator's `collapsed` and the drawing's `set_aside`, read as one private predicate — walked up the
ancestors. So no second derivation of *is this laid out* is written in the view, a folded pane takes
the bay with it, and a solo elsewhere does too. Asking `is_collapsed` on the bay alone is the
attractive wrong answer, and it is attractive because it is right for exactly one of the two ways in;
it is one of the injected defects the test is run against.

**It is not [`view::mixer`](../../crates/karakuri-console/src/view.rs)'s `None`**, which is a
different and stricter question. That one answers *can the strips be laid out in this rectangle, this
pass*, and says no for a deck with no slots, for a window merely too small to hold a strip row, and
for the frame before `egui` has fonts. None of those is a reason for the panel to stop being honest
about what it must redraw — a strip row squeezed out of a small window is a defect to see, not a
reason to sleep — and only being out of the layout is. `Layout::visible` also answers without a
solve, which matters at the call site: the fold is applied and the declaration is asked before the
next solve.

### The alternative that lost: declare it anyway, and let the scheduler drop it

**This is a real position and it nearly won.** P-0072's second half *is* a scheduler that arbitrates
between declarations by how stale each is against what it can afford. A folded region could go on
declaring — *I would need 33.33 ms if I were drawn* — and whatever schedules would decline to spend
anything on a region it is not drawing. The declaration would then be a property of the region alone,
which is the tidier factoring, and the visibility rule would live in one place instead of once per
declaring region.

It lost on four counts.

1. **A staleness is not a wish; it is a claim about being wrong.** *This may be 33.33 ms out of date
   and no more* says that past that point what is on screen misrepresents what the deck is doing.
   A region nobody can see is not showing anything, so it cannot be showing anything wrong. The
   sentence a folded region would be declaring is not merely unaffordable — it is **false**, and the
   scheduler would be arbitrating between one true deadline and one untrue one. The place to be
   honest is the declaration.
2. **Discarding it is not arbitration.** The arithmetic in P-0072 — `Σ (cost / staleness) ≤ budget /
   frame interval`, and `max(cost)` small — is about *capacity*: which of several true deadlines fit
   in a frame. Dropping a term because the thing is invisible is not that computation and does not
   belong in it. Worse, the rule for dropping it is a fact only the console holds — which node the
   region is and whether that node is laid out — so pushing it downstream hands the layout to a
   component whose whole subject is time.
3. **It costs now and pays at an unknown date.** The scheduler does not exist; ADR-0165 and P-0072
   both say so, and it arrives with the second declaring region. Deferring the honesty to it means
   the window is woken thirty times a second for an invisible chip for however long that takes,
   which is the measurement above and is the defect this record closes.
4. **P-0073 settled the same question on the other axis, the same way.** A folded child does not
   claim its stored size and then have the space taken back by a later pass; the **claim itself** is
   capped by what its visible content can use, and
   [ADR-0174](0174-a-node-claims-only-what-its-visible-content-can-use.md) says outright that the
   failure the other way is silent and reads as a design choice. This is that rule with the frame
   budget in place of the viewport.

**What the losing position was right about** is kept: a scheduler will eventually need to know that
a region is not drawn — for the beat, for the picture, for whatever declares next. It does; and this
is where that fact is, in the view, which holds the layout. What crosses into `repaint` is already a
number rather than a region, so nothing downstream loses information it had.

### Only out of the layout, and nothing else

**A region is silent because the console is not laying it out, and for no other reason.** Not a
window minimised or occluded, not a panel behind another application, not a bay scrolled out of
view, not focus. Those are the operating system's or the compositor's, and a repaint decision taken
from them is a decision this crate cannot test and cannot see go wrong
([ADR-0165](0165-the-repaint-decision-is-one-closed-list.md) is one closed list for the same
reason). The fold is a fact the console holds, an operator caused, and a test can set.

## Consequences

- **`View::animating` takes `&Layout`.** The two call sites in `examples/panel.rs` pass
  `self.readout.panel.layout()` beside the view they already hold; nothing in `src/` gained a
  dependency it did not have.
- **The declaration is re-derived every frame from the arrangement**, so the fold is not a latch and
  unfolding declares again while the slot is still parked. That is asserted rather than assumed —
  the latched version is one of the injected defects.
- **P-0072's *Where it holds* changed in the direction nobody expected.** It said the first clause
  was *no longer reachable on the example that measures it*. It is reachable again: with the
  picture, the preview row and the Mixer bay folded, `examples/panel.rs` now draws **0 frames in
  3.0 s** and prints *"so P-0072's first clause holds here"*. The paragraph was re-measured rather
  than deleted.
- **The example's own reading said the opposite in prose**, and it moved with the behaviour: *"Nothing
  about folding it away stops that: the slot is parked whether or not the chip is on screen"* was a
  sentence about this defect, printed as though it were the design.
- **A second declaring region carries its own node the same way**, and the answer is the soonest of
  them. The beat is the candidate ([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)),
  and it is in the transport row, which folds.
- **Nothing here schedules.** Neither schedulability condition is checked anywhere yet, and no region
  declares a **cost**. This changes which regions declare a staleness and not what is done with one.
