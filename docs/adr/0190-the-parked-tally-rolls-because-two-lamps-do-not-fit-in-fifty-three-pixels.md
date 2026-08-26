---
id: 0190
title: The parked tally rolls, because two lamps do not fit in fifty-three pixels
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: [0072, 0075, 0077]
tags: [ui, decks, performance]
---

# The parked tally rolls, because two lamps do not fit in fifty-three pixels

## Context

[ADR-0188](0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md) decided
what a control has to say while a request of the operator's stands and has not been granted, and
wrote it as [P-0075](../principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md):
**where it is, where it is going, and that it has not arrived.**
[ADR-0189](0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md) then
deleted the clause that would have forced every frame of the animation to say all three on its own,
and left the choice of presentation on exactly two grounds: **space, and the design language**.

Both records say the same thing about where the decision gets made: *"the mock is still where the
choice between the two presentations gets made"*, and *"the tally's open question is unchanged in
substance and narrower in grounds"*. This is that choice, taken with the mixer strip's first
implementation of it, and the grounds turned out to be arithmetic.

The first user is the mixer strip's residency chip. The engine holds both halves already —
`Deck::residency` is the effective one, `Deck::requested_residency` is the request, and
`Deck::is_parked` is exactly `requested == Priming && effective == Allocated` — and the console
carried only the first of them.

## Decision

**The chip's word rolls part of the way toward the residency that was asked for and falls back,
about once a second, and never lands.** The pair of lamps is not rejected; it does not fit.

### The measurements, and every one of them is reproducible

Taken from `docs/manual/style.css` and the `size` constants in
`crates/karakuri-console/src/room.rs`, re-measured on 2026-08-26 through `egui`'s own text layout
at the console's default proportional face — which is what actually decides this, because the
question is whether type fits in a box.

- A strip is **53** wide inside `.strip`'s `padding: 7px 4px`, in a **61** track. The 61 is the
  mock's `.body-grid` third track of 268, less `.mixer-strips`' 6 + 6, divided into four tracks with
  three 4px gaps.
- The tally chips **as drawn**, galley plus two `TALLY_PAD_X`: `LIVE` **34.0625**, `PRIM`
  **37.59375**, `ALLOC` **44.53125**.
- The chip's box `TALLY_H` is **13.5** — `TALLY_SIZE` 9 at `LINE` 1.5 — against an ink row of
  **10.0**, so there is **3.5** of slack already inside the capsule.

#### A pair of lamps is impossible, not expensive

**The pair the rule exists for is `PRIM` beside `ALLOC`**, because that is the one disagreement the
engine can produce. Four ways of drawing it, each cheaper than the last, and all four are wider than
the row:

| Drawn as | Width | Against 53 |
|---|---|---|
| Two chips as they are drawn today | 82.125 | over by 29.125 |
| …with the strip's thinnest separator, `.strip-mode`'s `gap: 3px` | 85.125 | over by 32.125 |
| Bare glyphs, both paddings removed, same separator | 57.125 | over by 4.125 |
| `ALLOC` as a chip, plus a 6px dot and the 3px gap | 53.53125 | over by 0.53125 |

The third row is the one that settles it. **With every pixel of padding taken out of both chips —
which is no longer the mock's `.tally` at all — the two words still do not fit**, and the last row
says that even a word and a *mark* miss by half a pixel. The chip is centred in the strip
(`.strip`'s `align-items: center`), so an 85.125-wide pair hangs 16.0625 past the content box on
each side: through the strip's own 4px padding, across the 4px gap between tracks, through the
neighbour's 4px padding, and **about four pixels into the neighbouring strip's content box on both
sides**. Left-aligned instead of centred it would be 20.125 into one of them. Either way it is a
chip drawn over the deck next door.

There is no version of two indicators on this row. That is why this record says *impossible* rather
than *rejected*: nothing was traded away.

#### A second row is a flat 18.5 through four numbers that are one sum

Stacking a second tally under the first costs `TALLY_H` plus one `STRIP_GAP_Y` — 13.5 + 5 — and it
does not stop at the strip. `STRIP_H` is 215.5, and the same 215.5 is written into the mixer bay's
**316**, which is written into the right pane's minimum of **530**, which is written into the
window's minimum height of **632**. They are one sum by construction, and `tests/arrangement.rs`
recomputes it from the tree so they cannot drift.

So a second row makes the smallest window this console can be worked in taller by 18.5 — to buy a
presentation of one chip's transient state. The mixer bay is also the pane's floor: the manual pins
it (*"Four channel strips, all visible, nothing that scrolls out of reach mid transition"*), so the
cost cannot be absorbed by anything getting smaller.

#### The roll costs nothing

It needs the chip fixed at the widest word and a clip. Neither changes a rectangle in the strip, in
the bay, in the pane or in the window.

### What the roll is, and three things that came out of the measurement

At rest the current word sits alone and still. While a request is outstanding the destination word
comes up behind it and retreats — a raised cosine over 400 ms, then 600 ms still, reaching **0.4**
of the way and returning to exactly zero with zero velocity at both ends.

**It never lands, because landing is what arrival looks like.** A roll that reached 1.0 would state
the opposite of the truth once a second.

**1. The chip is sized for the widest of the three words, and that fixed a defect.** `view.rs`'s
`mixer` measured `tally_job(strip.tally, …)` — the current word only — so the capsule was 34.06 wide
on a live slot and 44.53 on an allocated one, and **resized itself by 10.47 whenever the deck moved
under it**. That was invisible while the chip was still; a word rolling through a box that resizes
as it rolls would not have been. Fixed by measuring all three and taking the maximum. It moves
nothing inside the chip: the capsule is centred in the strip and the word is centred in the capsule,
so the box grows symmetrically around type that was already on the strip's centre line.

**2. The travel needs a blank band, and the band is free.** At the natural 10.0 row pitch the two
words are both partly visible with nothing between them, and at 9px that is mud rather than two
words. The pitch is therefore **at least the box height**, which leaves a blank band of exactly
`TALLY_H - 10.0 = 3.5` — the slack the capsule already had — at *every* displacement, because it is
the difference of two constants rather than a function of how far the roll has got. It costs the
chip nothing: at rest the second word is a whole box below the first, which is outside the capsule.

**3. The clip is new.** `tally_into` painted a filled rect and a centred galley, and nothing called
`with_clip_rect` on it — there was nothing to clip. It is introduced deliberately and it is on the
**type alone**: `.tally.live`'s `box-shadow: 0 0 10px` is drawn to spill, and a clip taken before
the shadow would trim the one shape in this chip that is meant to leave it.

### The phase is one value, panel-wide, and it arrives from the harness

`view::Phase` is **how long the panel has been animating**, written per frame by `examples/panel.rs`
the way `Transport` and the strips are written.

**One phase, not one per animation.** P-0075 asks for it outright — two controls moving out of step
looks broken rather than informative, and N animations are N deadlines where one phase is one. Two
parked slots are the same roll because they are read off the same number.

**It is an elapsed interval and not a wrapped fraction, which is the part worth arguing.** A value
already reduced to *where we are in the cycle* fixes the cycle: a second presentation at another
rate could not be derived from it at all and would need a phase of its own — the drift the single
phase exists to prevent. So the carrier is the whole interval and each presentation takes its own
`Phase::cycle(period)` out of it. The console has one moving thing today and is about to have the
beat; this is where the second one is either free or a second clock.

**It is a `Duration` and not an `Instant`, and that is the seam rather than a preference.**
`crates/karakuri-console/src/` reads no clock; the transport row's doc already argues the general
case — *"an `Instant` here would put a clock in it, and then the row would be reading wall time in a
repository whose first principle is that nothing does"* (P-0002) — and every `Instant::now` in the
crate is in `examples/panel.rs`, which owns the window. An `Instant` is a reading; a `Duration` is a
number, and a number is what a caller writes and a **test chooses**. `tests/parked.rs` asserts the
displacement at 100 ms and 200 ms without a window, a device or a clock.

### The strip carries both residencies and derives the rest

`view::Strip` gained `requested` beside `tally`, and **no `parked: bool`**. A third field would be
the same fact stored twice, and the copy is the one that goes stale — a harness could write two
residencies that disagree and a flag that says they do not, and nothing in the crate could tell. One
derivation with several readers is this crate's habit, and it is P-0075's *derived every frame,
never stored* read one level down, in the surface rather than in the engine.

`Strip::pending()` answers **the destination word** rather than a `bool`, because P-0075's second
clause is that the destination is identifiable from the surface: the thing worth deriving is where
the slot is going, not that there is somewhere.

**It is written as an inequality rather than as the engine's pair**, and the two agree slot for slot
today: `deck.rs` closes the other cases itself — the governor *"may hold a slot below what was asked
for, and may never put one above it"*, and *"effective Live and requested Live are the same set of
slots"* — so `requested != effective` is exactly `is_parked` on every frame the engine can produce.
They differ only in what a **second** kind of disagreement would do to them. Matching the pair, a
surface would draw a settled chip over any other outstanding request: an under-draw, silent, and the
exact failure P-0075 exists to name. The presentation was never about *parked*; it is about a
request that has not landed, which is [P-0060](../principles/0060-name-the-property-not-the-shape.md)
— the property rather than the one shape it currently takes.

`park` is still the word, and it is still the status line's: `karakuri-cli` spells it out in prose
for an operator, which is ADR-0188's *the destination named in prose*.

### What it costs, and the panel is not still while a slot is parked

The roll declares **one staleness for the whole period**: the 400 ms travel in twelve steps, so
`ROLL_STALENESS` is **33.33 ms** — about thirty frames a second, for as long as anything is pending.
`View::animating` is the declaration and `repaint::Change::Animating` turns it into a
`Repaint::After` deadline. **The view is what knows the rate**, so the harness is told rather than
guessing; a rate written into the window loop is a presentation's number kept where the presentation
is not, and changing the roll would leave the window servicing the old one with nothing failing to
compile.

Against [ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s measured 184
allocations and 226.2 kB per drawn frame, that is roughly **5500 allocations and 6.8 MB a second**
while a slot is parked. ADR-0188 estimated 2800 and 3.4 MB for a roll, on the assumption of a fine
deadline while the word moves and a coarse one while it rests; **this declares one number instead**,
and pays about twice, for a reason worth having: two numbers is two live regions wearing one name,
and choosing between them frame by frame is a scheduler's job (P-0072's second half) rather than a
presentation's. A presentation declares what it needs; what the panel can afford is decided
elsewhere. The number is a deliberate over-declaration that a scheduler can refine downwards.

**Nothing pending is `Repaint::Never`**, and that is asserted rather than hoped: a console with no
parked slot costs exactly what it cost before any of this existed, which is P-0072's first clause
and is `tests/parked.rs`'s second test.

`egui` is immediate mode, so what repaints is **the panel** and not the chip, and a parked deck can
stand for the length of a set. That is the honest statement of the cost and it has not changed since
ADR-0188 made it.

### The alternatives

**A pair of lamps.** Not rejected — **impossible**, and the table above is the whole argument.
Recorded rather than dropped for the reason
[ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md) records the
three blend segments it could not draw: the next person to look at this row will reach for two
indicators, because P-0075's own worked example is two indicators, and the answer is a number rather
than a taste. It stays admissible anywhere with room — which is the point of P-0075 stating clauses
instead of a picture.

**A blinking pending mark, with the destination in a tooltip.** Cheaper: about 370 allocations and
450 kB a second (ADR-0188's figure), and no clip. It loses on P-0075's second clause, which is
written as *from the surface itself* precisely so this cannot pass: **the console draws no tooltips
at all** — a tooltip needs `egui` to own a widget where this console paints, and who owns the pointer
is undecided — so half of what is being said would be delivered by a mechanism that does not exist.
Even when it does, a compact control that has to be interrogated to be read is what that clause
refuses.

**A second tally row.** 18.5 through four numbers that are one sum, above. It also spends permanent
height on transient state.

**A coined word.** ADR-0188 rejected it and nothing here reopens it; the short of it is that `park`
names one pair where the rule is about the relation.

**A fine deadline while the word moves and a coarse one while it rests.** ADR-0188's own sketch, and
it halves the cost. Rejected here, above: a presentation that declares two numbers is two live
regions, and the arbitration is the scheduler's.

**Reducing the reach to zero at rest by leaving the second word out of the paint.** Not an
alternative — it is what is done. The destination galley is laid out only while something is
pending, so a settled strip costs exactly the galley it always cost.

## Consequences

- **P-0075 holds somewhere.** Its *Where it holds: nowhere yet* is corrected: the mixer strip's
  residency chip is the first user, the phase is a value on `View`, and the price is declared.
- **P-0077 is still not held, and the beat is still the only continuous thing.** Read from the
  source rather than assumed: the roll runs **only while a slot is parked**, so a console with
  nothing pending has no continuous motion of its own and the beat grid remains what a stop would be
  visible against. P-0077's forced clause — *something is moving continuously while the console is
  live, and a scheduler may not stop it* — is untouched by this change, and its *nowhere yet* is
  narrowed rather than closed: one region now declares a cost and a staleness, which was one of the
  three things it said were missing.
- **P-0072 has its first client, and still no scheduler.** One region declares a staleness; the two
  schedulability conditions and the arithmetic over several regions are absent because a scheduler
  that arbitrates between one region and nothing is an abstraction with one call site.
- **The mock gains an animation, which it had none of.** `docs/manual/style.css` had no
  `@keyframes`, no `animation` and no `transition` in 570 lines; it now has one keyframe rule and
  one selector that uses it, and `console.html`'s deck C is drawn parked. ADR-0188 said this record
  would be where that happened.
- **The mock's chips stay shrink-to-fit and the console's do not**, and that is a real difference
  with a reason: a still page draws each chip once, so *sized to the current word* and *sized to the
  widest* are the same picture there and only one of them is the same picture over time.
- **The chip is still not a control.** Nothing here is clickable, no claim rule was added, no
  `Operation` is emitted and `input.rs` is untouched. **That order is deliberate**: a control that
  could not yet say it is pending would look dead for exactly as long as one commit. What a press
  asks for is the next change, and P-0076 already says it asks rather than forbids.
- **The console still draws no tooltips**, and the manual's third rule still wants one. P-0075's
  second clause is met on the surface instead, which is why the roll was worth its clip.
