---
id: 0164
title: The panel is budgeted rather than forbidden to allocate
status: accepted
date: 2026-08-24
supersedes: []
superseded_by: []
principles: [0091]
tags: [ui, performance]
---

# The panel is budgeted rather than forbidden to allocate

## Context

`docs/roadmap.md` said the panel "must not allocate on the render frame path", carrying
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)
across to the interface. Then `egui` was chosen and measured: **184 allocations and 226.2 kB per
frame with every bay empty**, 0.268 ms median and 1.571 ms worst on this machine. Immediate mode
rebuilds the whole frame's shapes every time it draws, because there is no retained tree — that is
the trade it makes, and it is not negotiable within the toolkit.

So the sentence is false as written, and the question is which half was wrong.

**P-0001's own text answers it.** What it forbids is *deferred* work: a pipeline compiled on first
use, a buffer sized at the moment a record asks for it, an upload left on the queue for whoever
submits next. Its examples are all about an unbounded stall landing on whichever frame happens to
be first. A few hundred small host allocations from a warm allocator, before the encoder exists, is
a different hazard, and the rule that catches both by forbidding the word *allocate* catches the
wrong thing.

## Decision

**The engine's frame path keeps P-0001 unchanged. The panel gets a budget instead of a
prohibition**, stated as [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md).

Three things make it a rule rather than a hope.

**A still panel costs nothing.** The 226 kB is paid on frames where nothing changed, which is most
of them, and that is a fixed cost named nowhere — exactly what P-0001 exists to prevent, arrived at
from the other direction.

**What must be live declares a cost and a staleness in milliseconds**, and everything else is
scheduled into the remainder by how stale it is against what it can afford. A declaration rather
than a measurement, because there is nothing here to measure with: GPU timestamps do not work on
this machine at all, and a host clock cannot resolve a region whose update is microseconds. It is
also the better answer even with a working clock — a measured schedule reorders itself with the
machine's noise, so the same state behaves differently frame to frame for reasons the operator
cannot see, and when a declaration is wrong it is wrong somewhere a person can read.

**Schedulability is arithmetic and a test asserts it.** `Σ(cost/staleness) ≤ budget/interval` and
`max(cost) ≤ a small part of the budget`. The second matters as much as the first because an update
is not divisible: a region costing most of the budget blocks everything else on every frame it
runs. This is [P-0089](../principles/0089-a-check-you-have-not-watched-fail-is-guessing.md)
applied to a frame budget — a bay that breaks it fails a test rather than a performance.

**And when it still does not fit, the rate steps down deliberately.** `PresentMode::Fifo` already
quantises a missed 16.6 ms into 30 fps, so the steps exist; what has to be added is choosing one.
Aiming at 60 and missing gives an oscillation between 60 and 30, which is far more visible than a
steady 30 — so the drop is fast, the recovery is slow and needs real margin rather than a borderline
frame, and **the panel's absolute budget does not grow when the frame lengthens**. The extra time
belongs to the engine, which is what is actually struggling; a panel that absorbs the slack delays
the recovery it was given for.

## Alternatives

**Keep the sentence and hold the panel to P-0001 literally.** Rejected: it rules out every
immediate-mode toolkit, which is the decision
[ADR-0155](0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md) already took, and it forbids
allocation where the hazard was always the unbounded stall.

**Repaint the panel at a lower rate — 30 Hz against the engine's 60.** Attractive and wrong: the
cost does not go away, it clumps, and the frame it lands on is the one that misses. A periodic
hitch reads worse than a constant load, which is the same instinct this repository already has
about a flaky measurement being worse than a broken one.

**Measure each region and budget in milliseconds.** No clock to do it with, and it would make the
schedule a function of noise. See above.

**Plain round-robin over the regions.** Bounds the wait and says nothing about urgency, so a
tempo readout and a library listing take turns as equals.

**Priority numbers with an aging boost.** The proposal this replaces, and it is close: aging does
bound the wait. Two things beat it. A priority number has no meaning across authors, drifts within
a year and cannot be checked — where *"at most 40 ms stale"* means one thing to everyone and is
testable. And aging solves contention but not insolvency: when the live set consumes the budget
every frame, a region that does not fit in the remainder is starved no matter how urgent it
becomes, and only the arithmetic above sees that coming.

**Declare staleness in frames.** Simpler, and it breaks under the one condition it exists for: when
the rate halves, every tolerance doubles in wall time, so the panel becomes most permissive exactly
when the machine is most loaded.
