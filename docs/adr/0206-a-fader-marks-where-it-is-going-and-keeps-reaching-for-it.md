---
id: 0206
title: A fader marks where it is going and keeps reaching for it
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0087, 0091]
tags: [ui, decks, transitions]
---

# A fader marks where it is going and keeps reaching for it

## Context

[P-0075](../principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
says that a control whose request has been made and not granted has three things to say and must
say all three — **where it is, where it is going, and that it has not arrived** — and that the
destination is identifiable *from the surface itself*, "not from a tooltip alone, and not from a
log."

**An armed fade is exactly such a request, and until today the only place it was said was a log.**
`karakuri-cli`'s status line prints `g>`/`o>`/`w>` with the destination, and the reason is written
at the code: *"An armed fade is invisible otherwise: with the default quantum it is due up to a bar
after the key, and the only thing that said so was one line at press time. A control that changes
something invisible is indistinguishable from a control that is broken."* The console drew nothing
at all — a fader scheduled to cross the whole track in eight beats looked exactly like a fader
nobody had touched.

[ADR-0190](0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md) met
the same rule on the residency chip, and P-0075's *Where it holds* named that chip as its only
user. This is the second, and it is a **value on a track** rather than a word in a capsule, which
is why the presentation is a different one and why P-0075 states clauses rather than a picture
([P-0087](../principles/0087-name-the-property-never-the-shape.md)).

### What the deck can answer, asked before anything was drawn

`Deck::transitions_on(slot)` — *"what is moving on this slot, for a status line"* — hands out every
`Transition` on that slot, and each carries `control()`, `to()`, `start()`, `beats()` and `curve()`.
So *is this control armed, and to what* is a question the engine already answers, and no engine
change was needed or made.

Three facts about that answer shaped the surface:

- **There is at most one per `(slot, control)`.** `Deck::schedule` cancels whatever was moving that
  pair before it pushes, so a surface finds one destination or none.
- **Armed and running are one state.** A transition is in the list from the instant it is scheduled
  until the beat it finishes on, and `Transition::value_at` deliberately writes nothing before its
  start — *"a fade scheduled for the next bar must not take the control away from the operator in
  the meantime"*. Both are *a request that has not arrived*, which is the relation P-0087 is about,
  so both are drawn the same way; what separates them is that the value under the knob is moving in
  the second case.
- **The console cannot say when.** `start` and `beats` are on the beat clock and
  `crates/karakuri-console/src/` has no beat count and no clock at all (ADR-0156, P-0092). So the
  strip carries the destination and nothing else about the move.

## Decision

**The knob and the fill go on saying where the control is; a hairline mark on the track says where
it is going; and the fill keeps setting off toward the mark and falling back, once a second, never
covering the gap.**

`view::Strip` gains `gain_to` and `opacity_to`, both `Option<f32>`, written per frame by whoever
owns the deck — the seam every other field on that type is on. `Strip::gain_pending` and
`Strip::opacity_pending` derive the third clause from the pair, exactly as `Strip::pending` derives
it from the two residencies, and **nothing stores *a move is pending***. `view::Reach` is the two
rectangles, `StripBox::trim_reach` and `StripBox::fader_reach` measure them off `trim_at` and
`fader_at`, and `roll_at` — the tally's own curve, at the tally's own rate — is the displacement.

### The measurements, and every one of them is reproducible

Taken from `docs/manual/style.css` and the `size` constants in
`crates/karakuri-console/src/room.rs`, re-measured on 2026-08-28 through `egui`'s own text layout at
the console's default proportional face, in a strip at the plausible 1920x1080 window every other
test in the crate uses.

- A strip is **53** wide inside `.strip`'s `padding: 7px 4px`, in a **61** track — ADR-0190's
  numbers, unchanged.
- The tall fader is **17 x 104** with its fill **3** inside it (`.vfader b`), so the knob's centre
  travels **98**, and the knob is **21 x 9** (`.vfader s`'s `height: 9px` and `left: -2px;
  right: -2px`).
- The trim's track is **36.84375 x 5**: the strip's 53, less `.trim`'s `padding: 0 3px`, less the
  `g` at **5.15625** (`.trim .lbl`'s `font-size: 9px`), less `.trim`'s `gap: 5px`. Its fill runs the
  track edge to edge and its knob is **9 x 11** (`.fader s`).
- **One pixel is 0.0102 of the fader's range and 0.0271 of the trim's.**

#### The mark is the knob's own footprint, reduced to a line

It is one pixel thick and **as long as the knob is wide** — 21 across the 17-wide fader,
11 down the 5-tall trim — so it always overhangs the track by exactly what the knob overhangs it by
(`VFADER_KNOB_OUT`, 2 either side) and always has that much strip to be read against. A mark the
width of the *track* would cross the fill's own lavender end in lavender on a move down from the
top of a full fader, and vanish.

It is painted **over** the knob. Under it, a mark would be swallowed for every move shorter than
the knob is long — 9 pixels on a 98-pixel travel is **0.0918** of the fader, and 9 on 36.84 is
**0.2443** of the trim — which is to say it would disappear exactly as the control gets close, and
say *arrived* at the one moment it must not. A hairline over a 9-pixel knob hides nothing.

#### The reach is a fraction of the gap, and that is also its limit

The band runs from the value's own edge toward the mark, `roll_at(phase)` of the way: `ROLL_REACH`
is **0.4** at the top of the 400 ms travel and **0** for the 600 ms it rests, at which point it has
no area and nothing is painted at all. So two thirds of an armed strip's frames are the strip's
settled appearance plus one hairline.

**In proportion to the move**, which is the honest picture and the limit worth writing down: a fade
across the whole fader sets off 39 pixels, a fade of a tenth sets off 3.9, and a fade of a hundredth
sets off **0.39 of a pixel**. Below about a hundredth of the travel the entire disagreement — value
against destination — is one pixel, and what carries the message there is the mark rather than the
motion. A fader cannot show a difference it cannot show; the status line names the number, and a
number on the strip is a separate question this record does not open.

#### Nothing in the strip, the bay, the pane or the window changes size

Two rectangles per armed fader, inside the fader's own track and the two pixels the knob already
stands proud on. `STRIP_H` is 215.5, and the 215.5 in the mixer's **316**, the right pane's **530**
and the window's minimum height of **632** are one sum by construction. This adds to none of them.

### What each rejected alternative was, and why it lost

**A still ghost knob at the destination.** The obvious answer, and it fails the third clause: a
second knob standing still is indistinguishable from a limit, a saved position or a marker somebody
left, and says nothing about a request that has not been granted. It is also unbuildable at the
sizes involved — a 9-tall knob on a 98-pixel travel overlaps the real one for every move under
**0.0918**, and a 9-wide knob on the trim's 36.84 for every move under **0.2443**, which is a
quarter of the control. Two overlapping knobs at the small moves an operator makes most is mud where
it matters most.

**A blinking mark, with the destination in a tooltip.** ADR-0190 rejected this for the chip and the
reason holds unchanged: **the console draws no tooltips at all**, so half of what is being said
would be delivered by a mechanism that does not exist. A blink *without* the tooltip — a mark on the
track that flashes — meets all three clauses, and loses on two grounds. It cannot say **which way**
the control is going, which a track can and a capsule cannot, so it throws away the one thing this
surface can say that the tally's roll could not. And it is a second idiom for one property on one
panel: this console already says *pending* as **a move that sets off and falls back**, and a second
vocabulary for the same relation is what P-0075 warns is invented wherever a shape does not fit.

**Rolling the number instead** — `.strip-num` rolling from `0.30` toward `0.80`, which is ADR-0190's
presentation reused exactly, at no new geometry. It lost on coverage: **only opacity has a number**.
The trim has a `g` and a track and no figure at all, and mask position has neither, so this presents
one of the three controls a transition can move and leaves the other two to invent something else —
which is a per-control idiom on one strip. It also says the value without saying the distance: a
number tells an operator it is going to 0.80 and not how far the knob has to travel to get there.

**The status line's own words, on the strip.** `o>0.80` is **30.84** wide at `.strip-num`'s 10px and
`g>0.44` is **30.75**; the two together are **61.59** against a 53-wide strip, before any separator,
and a slot can have a third armed on its mask. The status line's presentation exists because a
terminal has no geometry; a fader has nothing else.

**A second row for the destinations.** A `.strip-num`-sized row costs 15 plus one `STRIP_GAP_Y`,
so **20** through the four numbers that are one sum — `STRIP_H` 215.5 to 235.5, the mixer bay's 316
to 336, the right pane's 530 to 550, and the smallest workable window from 632 to 652. Permanent
height for transient state, which is where ADR-0190's second tally row went.

**Moving the fill itself to the destination and back**, rather than adding a band. It is the most
fader-like motion available and it is the first clause broken: the fill is one of the two things
that say where the control *is*, and a fill that retreats while the knob stands still is a strip
showing two values with no way to tell which is the deck's. The band is drawn **over** the fill and
**under** the knob for the same reason.

**A finer rate for the reach than for the roll.** A fader's move is continuous where a word's roll
is not, so a case can be made for servicing it faster. Rejected on ADR-0190's own argument: a
presentation that declares two numbers is two live regions wearing one name, and choosing between
them frame by frame is a scheduler's job (P-0072's second half). One rate also happens to be what
P-0075 asks for outright — *everything pending moves together* — and two rates on one strip is
precisely the "two controls moving out of step" it says looks broken rather than informative.

### What it costs

**Nothing while nothing is armed**, which is asserted rather than hoped: `View::animating` answers
`None` and `repaint::Change::Animating` turns that into `Repaint::Never`.

While something is armed it declares `ROLL_STALENESS` — **33.33 ms**, the roll's own number — and
because the two presentations share a rate, a parked slot, one armed fade and eight armed fades are
**one deadline**, not ten. Against ADR-0190's figure that is the same roughly 5500 allocations and
6.8 MB a second the tally's roll already costs, and an armed fade on a panel that already has a
parked slot costs nothing further at all. Per frame it adds at most two shapes per armed fader —
four per strip, sixteen across a full bay — and zero on a strip with nothing scheduled.

**Two moves are not drawn, and neither is silent by choice.** A gain move between two values above
unity — 1.5 to 2.0 — is two positions the trim draws in one place, because the track shows `[0, 1]`
on a mix that is HDR (`StripBox::trim_at` already carries that gap). The derivation answers `None`
there rather than declaring 30 Hz for a picture that cannot change, which is
[ADR-0193](0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)'s
argument arriving through the other door. It closes when a hand can push the control past the top.

## Consequences

- **P-0075 has a second user.** Its *Where it holds* named the residency chip as the only one; it
  now names the mixer strip's two faders as well, and the pair of clauses that decided between the
  two presentations — space, and the design language — decided this one on **direction**, which is
  something a track can say and a capsule cannot.
- **`Control::MaskPosition` is an under-draw, named rather than closed.** A wipe is one scheduled
  move on a mask's front, and the strip has no control and no readout for that number: the mask
  `.mini` names a *shape*, and position, angle and softness are an inspector row that does not exist
  yet. An armed wipe is therefore invisible on this panel. It is not that it does not matter — it is
  that a destination with nowhere to be drawn would have to be drawn on the shape chip, which would
  say a shape was changing when none is. **It closes the day a mask-position control lands**, and
  `Strip::opacity_to`'s documentation says so where the next person will be standing.
- **The mock draws it and the example cannot.** `examples/panel.rs` reads
  `Deck::transitions_on` and writes both fields, so the seam is closed end to end — but nothing in
  that file *schedules* a transition, and scheduling one to demonstrate the mark would be the
  example writing to a deck without a record. So `docs/manual/console.html` is where a human sees
  this: deck B is drawn with a fade armed on its fader, which is the mock's **second** animation
  after the parked chip's.
- **P-0072 has a second client and still no scheduler.** Two presentations now declare, they
  declare the same number, and the arithmetic over several regions is still absent because there is
  still one region.
- **Neither mark is a control.** Nothing here is clickable, no claim rule was added, no `Operation`
  is emitted and `input.rs` is untouched — the tally's order exactly (ADR-0190), and for its reason:
  a readout of a pending state comes before any press that could arm one.
- **`ROLL_PERIOD`, `ROLL_TRAVEL`, `ROLL_REACH` and `ROLL_STALENESS` now have two readers**, and
  their documentation says so. The names still say *roll*; what they describe is *how far anything
  pending on this panel gets toward where it is going before it falls back*, and a second period
  here is how one phase would quietly become two.
