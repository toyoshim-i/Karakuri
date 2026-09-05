---
id: 0254
title: A written figure is re-measured, rather than predicted from the change somebody remembered
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0091]
tags: [performance, ui, docs]
---

# A written figure is re-measured, rather than predicted from the change somebody remembered

## Context

[ADR-0217](0217-the-counting-allocator-ships-because-a-written-number-nothing-checks-goes-stale.md)
decided that the counting allocator ships, because a written number nothing checks goes stale. It
recorded the failure that argued for it — ADR-0164's 184 allocations quoted as current after the
mixer bay had taken the panel to 456 and a parked deck to 525, for two commits, *because nothing was
checking it*.

**The guard then fired, and what it caught was not what anybody expected.** Written now because that
reading lived only in P-0072's *Where it holds* — *A still panel costs nothing, and what moves
declares its price* — and in a doc comment, and P-0072 is retiring into
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md).

`WRITTEN_ALLOCS` stood at **525**, taken on 2026-08-26. The note beside it did not merely leave the
number alone — it **predicted** where the number had gone, reasoning from
[ADR-0191](0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)'s
**69 allocations and 137.4 kB a strip** that the two mixer strips added since would put the panel
near **663 and 969 kB**. That is inside `DRIFT`, so a run would have read *still one this window
produces* and the sentence would have stayed.

**Re-taken on 2026-08-31 over nine runs it read 1518 allocations and 1781.6 kB a frame** — every one
of the nine agreeing to the allocation and to the tenth of a kilobyte. That is **2.89× the 525**
against a band of two, and against a prediction of 1.26×.

It was not two strips that landed. Three things the previous reading's own *what the panel had in
it* paragraph does not mention were on the panel: the Inspector's two panes drawn off the running
Set, an armed `audio-in` pill over an input measured every frame, and the arrangement pill.

## Decision

**A figure predicted from the one change somebody remembered is precisely the figure that goes stale
in silence, so the figure is re-measured — several runs at a time — and re-dated, and the prediction
is kept beside it as the argument for the counter rather than deleted as an embarrassment.**

`WRITTEN_ALLOCS` is 1518, `WRITTEN_KB` is 1781.6 and `WRITTEN_ON` is `2026-08-31`. `DRIFT` stays at
**two**: it is what caught 184 going to 456 (2.5×) and what caught 525 going to 1518, and a wider
band would have caught neither.

**The spread being nothing at all is a reading and not a guarantee.** The nine runs of 2026-08-26
disagreed by 14 allocations and these nine agreed exactly, because an untouched panel tessellates
the same work every frame and nothing in the run varies it. That is not a promise about a tenth run
and it is not a licence to take one: what makes a number here trustworthy is that several runs were
asked, and **a single run's median is what produced the last wrong one**.

## Alternatives rejected

**Widen `DRIFT` so the reading stops complaining.** The band exists to be uncomfortable. Two is the
smallest factor that still catches what has actually happened twice; ten would have caught neither
of them, and a guard that has never fired is indistinguishable from no guard.

**Keep 525 and annotate it with what has landed since.** This is the failure this record is about,
one iteration later. The prediction *was* the annotation, it was carefully reasoned, and it was
wrong by a factor of two in the direction that would have passed.

**Predict again from the three things now known to be on the panel.** Cheaper than a window and
three still seconds, and it fails for the same reason: the list of what is on the panel is itself
what nobody keeps current. The reading prints what the panel had in it while the numbers were taken
for exactly this reason.

**Assert the count in `cargo test` instead.** Not reachable — the number is a per-frame median of a
running instrument with a deck under load and a parked slot, which needs a window, a device and
three seconds of nobody touching it. ADR-0217 rejected the same move for the same reason.

## Consequences

- **The guard is now the second thing this repository has watched a written number fail against**,
  and both failures were upward and silent. `crates/karakuri`'s `WRITTEN_ALLOCS` doc carries the
  episode where the next person to quote a figure will be standing.
- **`PANEL_PASS` is a separate verdict and did not move with it.** An allocation count and a
  declared cost are different claims; the second is what a schedulability condition is asserted
  against
  ([ADR-0210](0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)).
- **The bytes are not held**, and that is a distinction rather than an omission: they move with the
  allocation count, so a verdict on them would be the same verdict twice.
