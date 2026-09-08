---
id: 0283
title: A region declares when its picture next changes, not that something is pending
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0091, 0094]
tags: [ui, performance, console]
---

# A region declares when its picture next changes, not that something is pending

## Context

[ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md) gives a live region two
numbers — **what its update costs, and how stale it may get** — and
[ADR-0165](0165-the-repaint-decision-is-one-closed-list.md) turns the second into the deadline the
window sleeps on. Neither number says whether the region has **changed**. So the panel is redrawn at
whatever rate the soonest declaration asks for, and a bay whose content has not moved is read back
off the deck and repainted at that rate along with everything else.

That is the finding, and it is the maintainer's:

> 一応Bayごとに希望最低フレームレートを持つようにしたとは思うけど、**表示内容に変化がないのに毎回読んで描きなおすのは宜しくない。**

It is M5.14's item, and it was taken first because every control that lands writes the same
unconditional read-back, so the number of sites grows while it waits.

### Which declarations were honest, which was not

Two regions declare. They are not the same kind of claim, and separating them is what decided the
size of this.

**The transport row is honest, and it is an *I move at this rate* declaration.** `BEAT_STALENESS` is
24.67 ms — one `BEAT_PITCH` of travel, so the light moves by one pixel and no more between updates
(ADR-0212). The light sits at `Transport::beats`; `compose` calls `Frame::render` on **every** frame,
sink or no sink, and `crates/karakuri/src/main.rs` commits `steps: STEPS_A_FRAME` on every one of
them, so the session advances once per composed frame and every frame this declaration buys draws
the light somewhere it was not. Nothing here needed changing, and
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
is the rule that says it must not be.

Its *condition* is nevertheless written in the shape of the other kind — `the row is laid out and
there is an engine behind it` — so its honesty is the harness's rather than the declaration's. A
harness that wrote a frozen `Transport` would get forty identical rows a second and nothing in
`karakuri-console` could tell. That is a gap in what is asserted, not a defect in what is declared,
and `tests/moving.rs` is where the rule is now held.

**The mixer bay is honest for 400 ms of every 1000 and is drawn for the other 600.** It declares
`ROLL_STALENESS`, 33.33 ms, while a residency request has not landed or a fade has not run
([ADR-0190](0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
[ADR-0206](0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)) — and the three
presentations inside it are one curve, `roll_at`, which is **exactly `0.0`** for the whole of
`ROLL_PERIOD - ROLL_TRAVEL`. Over a second a parked panel was woken **31 times and 17 of those
frames were drawn while the curve was flat**, each one repainting the chip in the position it was
already in. Counted rather than described: `tests/moving.rs`.

ADR-0190 saw this coming and priced it — *"this declares one number instead, and pays about twice …
a deliberate over-declaration that a scheduler can refine downwards"*. This is the refinement, and
it did not need a scheduler, because the region itself knows: the curve is a pure function of the
phase the view already holds.

### Where ADR-0164's shape stops

Its arithmetic — `Σ (cost / staleness) ≤ budget / interval`, `max(cost) ≤ a small part of the
budget` — is an **admission** test, and it is right: it asks whether the panel could afford every
declaring region at its worst, forever. Both of its numbers are therefore constants of the
presentation, and neither is a function of the frame.

What it has no word for is **release**: given that the region was admitted, is a frame owed *now*.
`View::animating` had to answer that question with the only number available, so it answered *how
often must this be drawn while it moves* to a question that was *is it moving*. That is the whole of
the gap, and it is one number wide.

## Decision

**A declaration carries a third number, `Declared::moves_in`: when this region's picture is next
different from the one on screen.** `staleness` is unchanged, stays the constant, and stays what
`tests/schedulable.rs` sums; `moves_in` is a function of the frame and is what
`View::animating` — and through it `repaint::Change::Animating` — hands the window.

**The invariant is `moves_in >= staleness`, and it is the whole of the safety argument.** Change
detection may take a frame away and may never bring one forward, so nothing here can reach a rate
the two conditions never admitted. It is `Repaint::soonest`'s direction argued on the other side of
the seam, and `tests/schedulable.rs` asserts it across a whole period.

**The two regions answer it as they are:**

- The transport row answers `BEAT_STALENESS`, on every frame there is. Its two numbers are one
  number, which is what an honest *I move at this rate* looks like when it is written down.
- The mixer bay answers `roll_moves_in(phase)`: `ROLL_STALENESS` while the word travels, and the
  remainder of the rest while it does not — floored at one step, so a rest with less than a step
  left of it does not produce a deadline tending to zero as the period wraps.

**The region stays in both sums throughout, rest included.** A parked slot is a live region for as
long as it is parked; a schedule admitted on the 40% of the period that moves would be a schedule
that could not afford what it admitted.

### What it measures

Over one `ROLL_PERIOD`, on a panel with a parked slot, the mixer bay on screen and no transport row
— which is the state ADR-0193 measured the old defect in, and is the only state in which the
mixer's declaration decides anything, because the beat's is sooner whenever the row is drawn:

| | frames asked for in a second |
|---|---|
| before | 31 |
| after | 14 |

Twelve of the fourteen are the travel's twelve steps, one is the origin, and one is the frame that
discovers the rest. The rest is then a single sleep of 566.671 ms. Against ADR-0210's measured panel
pass of 525 allocations and 694.3 kB, the seventeen frames that stopped being drawn are about 8900
allocations and 11.8 MB a second that were being spent to redraw a still chip.

**This is not a frame-rate fix and it is not offered as one.** The loop spends about 1.26 ms of a
16.6 ms frame drawing the panel, and with a live picture in the Program bay it draws every vsync
regardless. What this buys is three things:

1. **Fewer wasted repaints**, on exactly the panel ADR-0164's still-panel clause is about.
2. **A declaration that means something.** *This region is live* and *this region is moving* were
   one sentence and are now two, and the second is the one a window acts on. A reader can now tell
   which kind of claim each region is making, because the two numbers are different where the claim
   is *I am pending* and the same where it is *I move*.
3. **The precondition for caching a bay to a texture**, which the startup legend names as undecided
   and `Cost::buffers` has a measurement against. A cache needs *has this bay changed since I
   painted it*, and nothing on this panel could answer that. This does not build one — the cache
   would also want the answer for the frames the panel *is* drawn on, which is a different and
   larger question — but it is where the answer starts existing.

### Where it does not reach, and this is the honest half

**The mixer bay's level meter moves every frame and declares nothing.** `Deck::level` is read into
`Strip::level` and `meter_into` paints it, and with meters enabled it changes on every frame the
engine renders. It has never declared a staleness, it does not now, and it rides on whatever rate
the picture or the beat is asking for. So *the mixer bay does not change during the roll's rest* is
true of the roll and false of the bay whenever a slot is live and metered. That is an
under-declaration of the kind P-0091 exists to catch, it is older than this record, and it is named
here rather than closed: closing it means deciding what a meter's staleness is, which is a
presentation decision nobody has taken.

## Alternatives

**Compare at the read-back and set a flag.** `mixer` and `inspector` in `crates/karakuri/src/main.rs`
assign unconditionally — `strip.gain = deck.gain(slot)` — and compare only where a `String`
allocation is at stake, so *did this change* cannot be recovered after the fact. Making each
read-back answer it is the version the finding points at, and it is what *"the longer it waits the
more sites there are to convert"* describes. It loses on three counts, and the first is decisive.

- **It cannot remove a frame.** The loop decides the next frame at the end of this one, from a
  deadline; the read-back for that frame happens after the deadline has already fired and the
  swapchain image is already in hand. A flag set there can only ever *add* a frame — which the
  event arms of `Change` already do, correctly — and the frames being spent are the ones a deadline
  asked for.
- **It compares what was read, not what is drawn.** `Strip::mask_angle` is read for the press and
  painted nowhere, so a change in it must earn no frame; `Phase` changes every frame and moves
  nothing for 600 ms of every second, so a change in it must not earn one either. Only the view
  knows which values reach pixels, and a diff at the seam gets both of those wrong in opposite
  directions.
- **It costs the frame path.** A per-field comparison of every strip on every frame needs a copy of
  the previous frame to compare against — a second `Vec<Strip>` with its `String`s, or a digest —
  and it is paid on every frame including the ones that were going to be drawn anyway. The
  declaration costs nothing new: `roll_moves_in` is arithmetic the painter already does.

It is the right shape for a **texture cache**, where the question genuinely is *is the picture I
already painted still correct*, and it is where this will be re-proposed.

**A fine deadline while the word moves and a coarse one while it rests.** ADR-0190's own sketch,
rejected there — *"a presentation that declares two numbers is two live regions, and the arbitration
is the scheduler's"* — and this record is that rejection reversed. It is reversed rather than
ignored because two of its three premises have changed.

- *Two numbers is two live regions.* It is one region in two states, which is what
  `mixer_declares` already was: it answers `Some` or `None` for one region depending on what the
  deck holds, and ADR-0193 made the *set* of declarations a per-frame answer as well. The region's
  name, cost and staleness are one each throughout.
- *Choosing between them frame by frame is a scheduler's job.* `View::animating` has chosen frame by
  frame since ADR-0193; what a scheduler would arbitrate is which region a frame is **for**, and
  there is still nothing doing that, because both regions still fit — `Σ (cost / staleness)` is
  unchanged at 0.0889 against 1.0.
- *The coarse number is a second constant that can drift from the first.* It is not a constant at
  all: it is `ROLL_PERIOD` and `ROLL_TRAVEL` read for where the curve leaves zero, which are the
  same two constants `roll_at` is drawn from and `ROLL_STALENESS` is derived from. There is nothing
  to keep in agreement.

**Declare the rest as a staleness and let the arithmetic see it.** One number rather than two, and
it makes `Σ (cost / staleness)` a function of the phase — which is exactly what ADR-0212 refused for
the tempo, and for the same reason: a schedulability condition that could only be asserted at the
moment somebody happened to sample it is not a condition. Admission is taken on the worst case and
servicing on the frame, which is what the two fields are.

**Stop declaring at all during the rest.** Simplest, and it is a panel that goes to sleep and never
wakes: the rest ends at a time only the region knows, so it has to be the region that names it. It
is also how a chip would stop rolling for good, which is
[ADR-0189](0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md)'s
stopped animation.

**Gate the beat the same way.** Rejected on P-0094's forced clause, and it is the one thing here
that would be a fault rather than a saving: a console that stops moving because nothing has
*changed* is indistinguishable from a console that has stopped. `tests/moving.rs` holds it in both
directions, and run against that defect —
`transport_declares` handing back `roll_moves_in(self.phase)` — it fails with *"a live console asked
for Some(33.333ms) at 0 ms into the roll, and the beat declares 24.671ms whatever else the panel is
doing"*.

## Consequences

- **P-0091's `Declared { region, cost, staleness }` is now four fields**, and the principle's
  *Where it holds* says so. The two the principle names are still the only two it names: the third
  is not a cost and not a tolerance, and nothing in the principle is being widened to fit it.
- **The still-panel reading prints a deadline and no longer a rate.**
  `crates/karakuri/src/main.rs` used to print *"the soonest staleness declared is X ms — about Y
  frames a second"*, and the reciprocal of a mixer deadline is no longer the rate anything runs at.
  The rate the window actually drew at is the line above it, which is measured rather than derived.
- **ADR-0193's 28.0-to-28.3 figure is now unreproducible**, and that is this record and not a drift:
  the same three folds with a slot parked ask for fourteen frames a second rather than thirty-one.
  The measurement is left where it is, because it is a description of what that record found.
- **Nothing arbitrates between two declarations, still.** Both fit, and the day one does not is the
  day `moves_in` gives a scheduler something the staleness could not: which region is actually
  asking.
- **The meter is a named gap** rather than an unnoticed one — see *Where it does not reach*.
