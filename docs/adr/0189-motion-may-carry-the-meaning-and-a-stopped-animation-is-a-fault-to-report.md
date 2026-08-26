---
id: 0189
title: Motion may carry the meaning, and a stopped animation is a fault to report
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: [0075, 0077]
tags: [ui, docs]
---

# Motion may carry the meaning, and a stopped animation is a fault to report

## Context

[ADR-0188](0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md) settled
what a control has to say while its request stands and has not been granted, and wrote it as
[P-0075](../principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md):
where it is, where it is going, and that it has not arrived. The presentation was deliberately left
open — the file states **clauses a presentation is checked against** rather than a picture, which is
[P-0060](../principles/0060-name-the-property-not-the-shape.md) applied.

One of those clauses was **“It reads as unsettled in a single frame.”** Every instant of the
animation had to say *not yet*, justified by three cases in which a surface is caught at one
instant: a screenshot, a test, and a compositor that has stopped servicing the window. It was
written and landed on 2026-08-26, and it did not last the day.

## Decision

**The clause is deleted. P-0075 has three clauses, not four, and nothing replaces it.** A
presentation may carry what it is saying in *change* rather than in state, and it is admissible on
the strength of what it says while it is moving.

### Perception is through motion, and a manual's picture is chosen

This is the maintainer's argument and it is the substance rather than a citation. Human vision has
strong difference detection on real-time imagery: a great deal of what an operator reads off a panel
is read **because it moves**, and a large class of designs mean nothing at all until they do. There
is no reason to demand that any single frame carry the whole message — that is a demand made of a
medium the design is not in.

Where a still genuinely has to stand for the animation, the still is **chosen**. A sample image in a
manual is picked, deliberately, at a frame that is unambiguous; where one frame cannot carry it, the
animation is explained with several. That is the ordinary way to document something that moves, and
the clause forbade the technique rather than protecting anything.

### The three cases, one at a time

- **The screenshot does not survive.** Nobody is handed a random frame. A screenshot is taken by a
  person choosing when to take it and kept by a person choosing to keep it, and a sequence of them
  is how motion is documented. Requiring every frame to be self-sufficient buys nothing that
  choosing the frame does not already buy.
- **The test does not survive, and P-0075's own architecture is why.** The file already requires the
  animation phase to arrive as **“a plain value written per frame by whoever has the clock”**, so
  that `karakuri-console/src/` keeps no clock — and it says the consequence in the same section:
  *“A presentation driven by a value is also a presentation a test can assert at a phase it chose,
  which one reading wall time is not.”* A test **chooses** the phase it asserts at. It never catches
  one. The clause was standing beside its own refutation, two sections down.
- **A window nobody can see does not survive.** Occluded, the window stops being drawn and stops
  being looked at in the same moment, and a repaint follows when it comes back. There is no reader
  of that frozen frame to mislead.

### What the strong clause would have cost

It rules out any presentation whose meaning is carried by change rather than by state — a fade, a
pulse, a travelling highlight, a lamp that simply blinks — which is most of the useful ones. What
would have survived it is the narrow set whose every phase is separately distinguishable from rest,
and that set is **a shape**. P-0060 is the rule against writing a shape into a principle, and
P-0075 opens by citing it: a clause that admits one family of animations and excludes the rest
reinvents the same failure the file was written to avoid, one level down.

### A stopped panel is read as stopped, and that is the signal

**This is the argument that completes the removal, and it is the maintainer's.** The clause was
built on a worry that runs backwards: that a frozen frame would be taken for fact. It would not.
**People do not read information off a display that has stopped.** A screen that has been moving
regularly and then stops *tells the operator something has gone wrong*, and they know from that
moment not to trust anything on it — without being taught, and without a convention.

So the answer to *what does the panel say when it cannot keep up* was never a drawing convention.
**The panel visibly stops, and the stopping is the message.** What is broken being seen to be broken
is worth more than a frozen picture being legible, and it is worth more precisely because it is
read correctly by a person who has not read any of these documents.

That is why the clause is **deleted rather than narrowed**. The failure mode it guarded against does
not exist on a panel that has continuous motion on it — and this one does and will: the transport
row's beat indicator is the thing that has been moving regularly, which is a large part of why it
matters. The condition is not free, though, and it has a price a scheduler will be tempted by, so it
is written down as a rule of its own:
[P-0077](../principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md).

### The intermediate position, and why it was rejected

**It was the author of this record's, it stood for an hour, and it is written down because it is
exactly the kind of half-measure that reads as principled later.** The proposal was to keep the
clause and scope it to the stopped case:

> A stopped animation must not say it arrived.

A condition on what the presentation looks like when phase updates stop, rather than on every frame
of it. A blink frozen at its bright phase is a lamp that reads as *landed* and states the opposite
of the truth; a roll frozen halfway is unmistakably halfway.

Its justification was not any of the three fallen cases but **this project's own mechanism**:
[P-0072](../principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md) has
every live region declare a cost and a staleness and be *scheduled into what is left*, so a
scheduler this repository has not written yet could starve a pending animation **while it is on
screen and being watched** — not an accident to guard against, but a decision the panel takes under
a budget.

**It loses, and it loses on what the budget is for.** A GUI animation stopped long enough for
anybody to read it is a **dead application**. That is not a state to design a graceful appearance
for; it is a failure, and the effort this project has spent has gone into the mechanisms that stop
it happening rather than into how it should look when it does:

- P-0072's two conditions are arithmetic over the named regions — `Σ (cost / staleness) ≤ budget /
  frame interval` and `max(cost) ≤ a small part of the budget` — and the file says why they are
  written that way: they are *“sums over the named regions, so a test asserts them rather than a
  stage discovering them.”* The second condition exists precisely so that no single region can hold
  the frame.
- When a panel cannot meet a staleness it declared, it is over budget, and the answer to over budget
  is that **the instrument says so**.
  [P-0030](../principles/0030-an-instrument-says-what-it-did.md) — *an instrument that goes silent
  is unusable* — and
  [P-0027](../principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md) — a plausible
  wrong picture loses to a loud failure — are the two rules that decide it, and they decide it
  against the appearance.

A convention for reading a frozen animation is a **quiet accommodation of exactly the failure those
two rules exist to make loud**. It would be paid for by every presentation this console ever draws,
forever, to buy a graceful surface in a state the instrument must not be in silently. The mechanism
it cited is real; the conclusion drawn from it was the wrong one.

And the narrow rule buys nothing even in the case it was written for. A pending animation frozen by
a budget is frozen on a panel whose beat indicator has stopped with it — the operator is already
being told the panel is not current, which is a stronger and earlier statement than any single
chip's appearance. **A rule whose whole benefit is a legible frame inside a failure that announces
itself is a rule with no benefit.**

### The two presentations converge

ADR-0188 left two on the table and recorded that they differ on this clause. They no longer do.

- The record said of the roll: *“It reads as unsettled in every frame. A half-rolled word cannot be
  mistaken for a settled one.”* That was true and is no longer an advantage, because nothing asks
  for it.
- It said of the blink that it *“only meets the still-frame clause if its two appearances are both
  kept distinct from the settled one — a condition on the animation that the obvious implementation
  violates.”* That condition was a consequence of the clause and goes with it. How a blink's phases
  are drawn is now legibility and design language, not compliance.

**What still separates them is what ADR-0188 also measured.** The roll names the destination on the
surface in the same 53 pixels rather than deferring it to a hover the console does not draw, which
is the second clause and is a real difference. Against that: space — a mixer strip is 53 wide inside
`.strip`'s `padding: 7px 4px`, and the roll costs a taller clip region and a second galley — and the
design language, which today has **no animation of any kind** (`docs/manual/style.css`, 570 lines,
no `@keyframes`, no `animation`, no `transition`). The costs ADR-0188 put on them stand unchanged:
roughly 370 allocations and 450 kB a second for the blink against about 2800 and 3.4 MB for the
roll, on a panel that repaints whole because `egui` is immediate mode.

### ADR-0188 is annotated, not rewritten

[P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md) is
the test, and this is the case it names outright: *an argument that would have to change is a new
record*. ADR-0188 correctly describes what was decided when it was written, including a clause that
was believed and is not any more, and rewriting it would destroy the only account of how P-0075 came
to be shaped this way.

What it gets is what the front matter is for. Its `superseded_by` names this record — the only
forward pointer the front matter has, and P-0066 names precisely those fields as where *what a
record later became* is written. Its `status` stays `accepted`, because what fell is one clause of
one principle rather than the decision: the vocabulary, the derivation every frame, the refusal to
keep the constraint in the surface, the rejection of a coined word, and the measured costs all
stand. **No landed record in this directory annotates in prose**, checked across all 189 of them,
and this one does not start the habit.

## Consequences

- **P-0075 has three clauses and keeps its number.** The rule the file states is unchanged — a
  pending transition still shows where it is, where it is going, and that it has not arrived — so
  §4's *delete the file and re-record it under a new number* does not apply: that is for a rule that
  has stopped being true, not for a check that stopped being right. The file says so itself, because
  it is current-tense and a reader must not have to know it changed.
- **The tally's open question is unchanged in substance and narrower in grounds.** Which
  presentation the residency chip uses is still undecided, and the deciding grounds are now space,
  legibility and the design language rather than a clause. `docs/roadmap.md` carries it.
- **A second principle,
  [P-0077](../principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md).** One
  decision with two consequences rather than two decisions: if the freeze is what tells the operator
  the panel is not current, then something on the panel has to be moving for the freeze to be
  visible at all, and that motion is not spare budget. It rules out the obvious economy — a
  scheduler buying headroom by stopping the panel's continuous motion under load — which is item 2
  of the roadmap's order of work and therefore a proposal that will be made.
- **What the panel does when it cannot meet a declared staleness is owed to the scheduler, not to
  the presentation.** It is a fault, and the panel's own stopping is the first way it is reported.
  Whether anything else says so — the console draws no tooltips, and `karakuri-cli`'s status line is
  not a panel — is undecided.
- **The mock is still where the choice between the two presentations gets made**, and it still has
  no animation. Nothing in ADR-0188's reasoning about that changed.
- **Nothing in `crates/` or `docs/manual/` changed in this record.** As with ADR-0188, this is a
  decision taken before the work.
