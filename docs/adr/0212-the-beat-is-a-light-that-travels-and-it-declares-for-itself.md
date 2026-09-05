---
id: 0212
title: The beat is a light that travels, and it declares for itself
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0091, 0094]
tags: [ui, perf]
---

# The beat is a light that travels, and it declares for itself

## Context

[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) marks two
things apart, as [P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md) asks.

> **Forced.** Something is moving continuously while the console is live, and a scheduler may not
> stop it to make room.

> **Preference rather than force: continuous movement is a better carrier of this signal than a
> discrete flip.** … it is a 0/1 switch per dot, changing once a beat, two to four times a second at
> ordinary tempos. That proves liveness **across an interval**: an observer knows the panel was alive
> between two flips, and a stop is only apparent once a flip that was due does not arrive. A
> continuous movement proves it **at every instant**, which is what this signal wants.

**The forced clause was satisfied by accident.** P-0072's *Where it holds*, re-measured in the
commit that landed
[ADR-0210](0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md), says
the beat grid *"moves because the panel is being redrawn for something else rather than because
anything decided it must"* and declares nothing. So on a console with nothing pending, nothing
declares, no frame is asked for, and the panel goes still — which is exactly the state P-0094 says
must be visible as a fault rather than looking calm. The one thing on the panel that P-0094 wanted
was the one thing riding on somebody else's frames.

ADR-0210 asserted the two schedulability conditions over one declaring region and said what they
were written for: *"the arbitration arrives with the second declaring region, which P-0094 wants to
be the beat."* This is that region, and it is the first time either sum has more than one term.

## Decision

**The beat grid becomes a light that travels the grid, its position is the fractional beat the
oscillator already accumulates, and the transport row declares a cost and a staleness of its own for
as long as that grid is drawn — whether or not anything is pending.**

### The presentation: a light with a position, not a dot with a state

`TransportRow::on` was `Transport::beat()`, a dot index, and `.beat-grid i.on` was painted
`--c-pink` while the other three were `--c-line`. It is now `TransportRow::at`, a **position** in
dot pitches, and `view::beat_at` says how much of the light is on each dot: a **raised cosine one
dot pitch wide**, measured round the cycle.

Every number in it is read off the mock rather than chosen. The pitch is `.beat-grid i`'s
`width: 15px` plus `.beat-grid`'s `gap: 4px` — **19 px**, which is `view::BEAT_PITCH` and is what one
beat of travel measures. The falloff is exactly one pitch, so:

- **At the instant of a beat the grid is the mock's own picture.** All of the light is on one dot and
  the others are exactly `--c-line`: `<i class="on"></i><i></i><i></i><i></i>`, the halo at
  `.beat-grid i.on`'s `box-shadow: 0 0 9px var(--c-glowp)`, nothing else lit. **The mock did not have
  to be contradicted** — it is a frame of this drawing rather than a different one, and
  `docs/manual/style.css` now carries the travel between those frames as `@keyframes beat-sweep`.
- **At most two dots are lit, and the grid's total light is constant.** `f(d) + f(1 - d) = 1` is the
  raised cosine's own identity, so the four dots always sum to exactly one dot's worth. The light
  **moves along the row** rather than the row brightening and dimming as it goes — which is what
  makes a stop visible, because a still grid at half brightness would be indistinguishable from a
  light sitting between two dots.
- **It is measured round the cycle**, so the last dot and the first are one pitch apart and not
  three: the light leaves the right-hand end and arrives at the left in the same instant, each half
  lit, and there is no frame on which it jumps.

**A raised cosine and not a triangle**, which is `roll_at`'s reason one row up: it leaves and
arrives at zero *with zero velocity*, so a dot does not snap into being dark as the light goes.

**The clamp went with the index, and that is the shape of the change.**
`(-1e-18_f64).rem_euclid(4.0)` is `4.0` exactly, which as a dot index is one past the end of a
four-dot grid — a rectangle drawn beside it, or a panic — so it was clamped to the last dot. As a
*position* it is the downbeat: `4.0` and `0.0` are the same point on a cycle of four, and nothing is
written for it. `Transport::beat` is gone and `Transport::position` is in its place.

### The phase arrives as a value, and it is the beat's own

`src/` reads no clock (ADR-0156), so the phase arrives per frame the way ADR-0190's does. **It is
not `View::phase`**, and it is not a new field either: `Transport::beats` is the oscillator's
musical position, unbounded and monotone, and the fraction this row used to throw away *is* the
position of the light. `Transport::position` is `beats.rem_euclid(dots)` and nothing else crossed the
seam.

That also settles what the motion means. The light moves because the **session** moves. A grid
driven by `View::phase` would sweep at its own wall-clock rate over a session running at another
one, which is a beat grid that is not the beat; and it would go on sweeping after the session's clock
had stopped, proving that the panel repaints while asserting something false about the instrument.

### What it declares, and the two things it does not ask

`View::transport_declares` answers `Declared { region: "transport", cost: PANEL_PASS, staleness:
BEAT_STALENESS }` when **the transport row is laid out** and **there are values behind it**.

The first is
[ADR-0193](0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)
asked of a row rather than of a bay, and `Layout::visible` answers for `transport` exactly as it
answers for `mixer` — a fold is a fold. The transport is a direct child of the unnamed root, so the
only thing that encloses it is the root itself (ADR-0204), and there is no enclosing case to test
beside it. The second is `View::picture`'s seam one row up: a console with no engine behind it draws
no row at all, and there is no light on a grid that is not there.

**Nothing else is asked, and that is the whole content of the declaration.** A beat that declared
only while something was pending would be the panel's liveness signal going quiet exactly when
there is nothing else to say the panel is alive — the state P-0094 exists to make visible.

### The cost is the same constant, and the reason it is holds here too

`PANEL_PASS`, 1.26 ms. ADR-0210's argument is that `egui` is immediate mode, there is no retained
tree, *redraw the mixer bay* is not an operation this console has, so the price of servicing any one
region's deadline is the price of the whole pass. **That is a fact about the toolkit and not about
the mixer**, so it transfers verbatim: there is no way to redraw the transport row alone either, and
a smaller per-region figure would be a more precise-looking number for work this console cannot do.
`tests/schedulable.rs` holds every declaration to the one constant, and that assertion is now about
two regions rather than one.

### The staleness is the beat's, and it is the first one that is not the roll's

`view::BEAT_STALENESS` is **24.67 ms, about forty a second**: one beat at the mock's `128.0` BPM —
468.75 ms — in as many steps as the travel has pixels, so the light moves by at most one of them
between updates. `ROLL_STALENESS`'s shape and a different answer, because the two motions are
different sizes: the roll travels a few pixels of a word's pitch and twelve steps is smooth over it,
where the light crosses a whole dot and a gap every beat.

**It is stated at the mock's tempo, and it is the one number here the music moves.** A beat is 375 ms
at 160 BPM, so the same declaration is a pixel and a quarter a step up there.

### The two conditions, with two regions

| | Asserted | Read | Against |
| --- | --- | --- | --- |
| capacity | `Σ (cost / staleness) ≤ budget / frame interval` | **0.0889** = 0.0511 + 0.0378 | 1.0 |
| indivisibility | `max(cost) ≤ a small part of the budget` | **1.26 ms** | 4.17 ms |

The first sum has two terms for the first time. The second is still one number, because both regions
declare one whole panel pass — the day a region declares a cost of its own is the day `max` starts
choosing, and `budget::SMALL_PART`'s quarter is what to revisit then.

## The alternatives

**Share `View::phase`, which is what everything else that moves on this panel uses.** The tidier
answer — one phase panel-wide is
P-0075's
own rule, and `Phase`'s documentation says outright that the console *"is about to have the beat …
and this is where the second one is either free or a second clock"*. It lost on what the two phases
**mean**. P-0075 asks for one phase because *everything pending moves together*, and two controls
moving out of step looks broken; the beat is not pending and is not moving toward anything. Driving
it off elapsed wall time would make the grid's motion independent of the tempo the row is drawing
beside it, which is a beat grid that is not the beat. It is not a second clock either way: nothing
new is read, because the position was already in `Transport::beats`.

**Give `Transport` a phase field of its own.** Refused by `Transport`'s own rule — *"it carries what
cannot be derived and nothing that can … three numbers for one position is three chances for the lit
dot and the bar beside it to come from different arithmetic and disagree"*. The fraction was already
there.

**Keep the flip and declare it.** The smallest change: leave the presentation alone and give the
transport row a staleness so the flip stops riding on somebody else's frames. It satisfies P-0094's
**forced** clause and fails its stated preference, which is the whole reason the preference is
written down. It is also a poor declaration: a flip changes twice a second at 128 BPM, so a
staleness fine enough to land it on time asks for frames that draw the same picture eleven times out
of twelve, and one coarse enough to be honest lands the flip late. A presentation that only changes
at instants wants an event, and a region that declares wants something that is different on every
update.

**A needle or playhead sliding over the grid.** A thin bright line travelling across the four dots,
with the dots left as they are. Rejected on the mock: `.beat-grid` is 72 px wide and 6 px tall, and
a second mark inside it is either invisible or larger than the thing it is marking. It also leaves
two things to read where the dots already say where the beat is, and it is furniture the mock does
not have — so the mock would have had to be contradicted rather than extended.

**A fill growing inside the lit dot, once a beat.** The progress-bar reading, and the least new
drawing of any of these. It travels 15 px and restarts on every beat, in a box 6 px tall, so at
ordinary tempos it is a sliver moving a quarter of a pixel a frame; and *which* dot is the beat is
still a flip, so the discrete presentation is kept and a second one is added beside it.

**A wider falloff — two pitches, so the light is softer.** More than two dots lit at once, and the
total light stops being constant unless the width is a whole number of pitches, so the row breathes
as the light travels. A light that cannot be located is a glow, and a glow is what a still panel
would also be.

**Derive the staleness per frame from `Transport::bpm`, which the row is handed.** Honest about what
the light needs, and it lost on the arithmetic rather than on the drawing: a staleness that falls
with the tempo makes `Σ (cost / staleness)` a function of how fast the music is, so both conditions
could only be asserted against a fastest tempo nobody has written down. Inventing one inside the test
that noticed it was missing is exactly what ADR-0210 refused for the panel's share of the budget.

**Declare the beat at `ROLL_STALENESS` and have one rate for the whole panel.** It would keep both
sums at one term in effect and there would be nothing to arbitrate. It is a scheduler's decision
taken by a presentation, which P-0072 rules out in the sentence ADR-0190 already quotes: *"A
presentation declares what it needs; what the panel can afford is decided elsewhere."* And it is
false — 33.33 ms is a pixel and a half of travel, which is the step size that reads as a sequence of
positions.

**Make the beat grid a region of its own.** A `beat-grid` node under the transport row, so the
declaration names the thing that moves rather than the row it is in. The arrangement has no such
node and should not grow one for this: a region is a node the layout can answer *is this laid out*
for and a scheduler could know which rectangle it was spending on, and the grid is a presentation
inside a row exactly as the tally is a presentation inside a bay.

## Consequences

- **P-0094's forced clause holds in its own right**, and its *Where it holds* says so. Something is
  moving continuously while the console is live, it moves because a declaration asked for the frames
  rather than because something else did, and it does not stop when nothing is pending. **What is
  still absent is the scheduler** — there is nothing to refuse the economy the rule forbids, and
  that has not changed. The preference is now met as well: the light is somewhere at every instant.
- **A live console never declares nothing, and that is P-0072's first clause narrowing on purpose.**
  P-0094 anticipates it: *"It also rules out the converse economy — a panel designed so that nothing
  moves when nothing is pending, on the argument that a still panel costs nothing."* The price is
  1.26 ms every 24.7, which is about 5% of a frame, and P-0094 says what it buys: *"it is the
  cheapest thing the console has to say this is live … and it is the only statement that still works
  when whatever would otherwise report the fault is itself the thing that has stopped."*
- **`examples/panel.rs`'s zero now needs a fourth fold.** Its still-panel reading was reachable with
  the picture, the preview row and the mixer bay folded away; the transport row goes with them now,
  and the reading says which of the two declarations it is looking at.
- **Nothing arbitrates, and the reason has changed.** It used to be that choosing between one region
  and nothing is an abstraction with one call site. It is now that **both fit**: `Σ` is 0.0889
  against 1.0, so the frame that meets the sooner deadline meets the other one too, and
  `View::animating`'s *soonest staleness* is still the whole policy. A scheduler is what a budget
  that cannot afford everything needs.
- **`Transport::beat` is gone**, and the clamp that existed only because a dot index is discrete went
  with it. `Transport::position` is the one derivation, `Transport::bar` is unchanged, and
  `tests/transport.rs` asserts the position across a bar boundary and at both `f64` edges as it did
  the index.
- **The mock moves.** `.beat-grid i` carries `animation: beat-sweep 1.875s linear infinite` with each
  dot a quarter of the bar behind the one before, and the keyframes name the curve at its quarter
  points. `.beat-grid i.on` is untouched and is what a dot with all of the light on it looks like,
  which is what `size::BEAT_GLOW` cites and what the still markup keeps drawing.
- **What the window read after it, on one run.** 535 allocations and 673.9 kB a frame, 58.3 frames a
  second and 20.1% of a second drawing, with the panel-draw median at 0.997 ms against the 1.260
  `PANEL_PASS` declares — an Apple M4 Pro at 1440x900 logical, host clock, debug profile with
  dependencies at opt-level 3, nothing touching the window. Both of the example's own verdicts read
  *still one this window produces*. The allocation count was 524 to 538 over the nine runs of
  2026-08-26 and 524 in every one of ten on 2026-08-28, so 535 is inside the older spread and above
  the newer one; one run cannot separate the second halo from the swing that has already caught this
  measurement twice.
- **One more shape on a frame, at most.** The halo is scaled by how much of the light is on a dot and
  is not added at all where that is zero, so a grid of four draws one halo at the instant of a beat
  and two between two of them, never four.
