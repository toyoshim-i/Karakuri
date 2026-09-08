---
id: 0290
title: The level meter moves only when a frame is drawn, so it declares nothing
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0091, 0094]
tags: [ui, performance, console]
---

# The level meter moves only when a frame is drawn, so it declares nothing

## Context

[ADR-0283](0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
gave a live region a third number — `Declared::moves_in`, *when this region's picture is next
different from the one on screen* — and closed with an honest half naming what it did not reach:

> **The mixer bay's level meter moves every frame and declares nothing.** `Deck::level` is read into
> `Strip::level` and `meter_into` paints it, and with meters enabled it changes on every frame the
> engine renders. It has never declared a staleness, it does not now, and it rides on whatever rate
> the picture or the beat is asking for. So *the mixer bay does not change during the roll's rest*
> is true of the roll and false of the bay whenever a slot is live and metered. That is an
> under-declaration of the kind P-0091 exists to catch.

`docs/roadmap.md`'s M5.14 item 6 carries the same sentence as what is left of that item. This record
is that item worked out, and the answer is that **there is no number to write**: the meter is not
under-declaring, and the reason it is not is a fact about where the reading comes from rather than a
tolerance somebody chose.

### What the reading actually is, and when it moves

Four links, and the third is the one the earlier record did not follow:

1. `karakuri_engine::meter::Meters::record` copies a slot's reduction into a staging buffer while
   the deck's frame is being recorded, and `Meters::arm` maps it **after that frame's submit**.
2. `Meters::collect` takes delivery of whatever has finished. It is called from exactly one place —
   `Deck::begin_frame`, at the top of a deck frame — and it never waits.
3. So `Deck::level` moves **inside `begin_frame` and nowhere else**. It is a plain read of the
   retained reading between two frames, and a caller calls `begin_frame` once per composed frame.
4. `crates/karakuri/src/main.rs` composes exactly one frame per `RedrawRequested`, and reads
   `deck.level(slot)` into `view::Strip::level` at the top of that same handler.

**So the meter's picture is a function of the frames the panel is drawn on, and not of wall time.**
There is no moment between two frames at which what is on screen is not the newest reading taken —
because no reading is taken between two frames.

That is the difference between this presentation and the two that declare. `roll_at` is a raised
cosine over a `Phase`: it is different at 400 ms from what it was at 399 ms whether or not anybody
drew anything, which is why it can say when it next moves and why a deadline for it buys a picture
that would otherwise be wrong. The meter has no such curve behind it. The beat *looks* like the
meter — `main.rs` commits `steps: STEPS_A_FRAME` per composed frame, so the light also moves because
a frame was drawn — and it declares anyway, on
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)'s
forced clause: *something is moving continuously while the console is live*. That clause is what
makes the beat's circularity honest, and there is no equivalent clause for a meter.

### Where a meter declaration would decide anything, and what it would cost

`View::animating` takes the soonest `moves_in`, so a meter declaring some rate `M` changes what the
window does only where nothing sooner is already asking. There are four states and it is the last
two:

| what is on screen | what asks for frames today | with the meter declaring at the bay's rate |
|---|---|---|
| the picture or any preview cell | `main.rs`'s own `live()` — the loop asks for the next frame from inside the last one | unchanged: a frame every vsync |
| those folded, the transport row drawn | the beat, at `BEAT_STALENESS` 24.67 ms | unchanged: 24.67 ms is sooner than any meter rate worth having |
| those folded, the beat folded, a slot parked | the roll: **14** frames a second (ADR-0283, `tests/moving.rs`) | **31** a second |
| those folded, the beat folded, nothing pending | nothing: **0** frames a second | **31** a second |

Against ADR-0210's measured panel pass of 525 allocations and 694.3 kB, the bottom row is about
16,000 allocations and 21.5 MB a second bought on a console that is drawing no picture, no preview,
and no beat.

**And nothing is measured in those two states either.** With every sink folded away the loop is on
`ControlFlow::Wait`; nothing composes, so the deck does not step and `Meters::collect` is not
called. The frames a meter declaration bought would be frames that *cause* the readings they then
draw. That is what separates it from the beat one more time: the beat's motion is asked for because
P-0094 requires the console to look live, and this would be motion asked for so that a readout has
something to report.

### The lag that is left, and why no deadline closes it

`main.rs` reads the strips at the top of the frame and composes at the bottom, so what is on screen
is always one `collect` behind: when the window goes quiet there is one measurement in flight that
is never drawn. Drawing a frame to show it composes another frame and produces another one. The lag
is constant, it is one frame wide, and it is a property of the order inside the frame rather than a
staleness — a deadline cannot close it, and a region declaring one would be declaring for something
its own frames recreate.

## Decision

**The mixer bay's declaration is unchanged, and the level meter is not in it.** The bay declares
while something in it is pending, at `ROLL_STALENESS`, with `roll_moves_in(phase)` for when it next
moves. `Strip::level` is not a condition of that declaration and does not move that deadline.

**What changes is that the reason is written down where the declaration is**, in
`View::mixer_declares`, and held by `crates/karakuri-console/tests/metered.rs`:

- A metered console with nothing pending declares nothing and asks for no frame, at every phase of
  the roll — the still panel survives a slot that is metering.
- A metered parked console asks for exactly what an unmetered one asks for, at every millisecond of
  a period: the level is not in `Declared` and is not in the deadline. This is ADR-0283's 14 frames
  restated as a claim about a *metered* panel, which is what its own `settled()` strip already was.
- The meter's geometry is a function of the reading and of nothing else, asserted beside a fader's
  reach at two phases where the roll's curve differs: the reach moves and the meter does not.

**The premise, so that the day it stops being true is a test failure rather than a silent stale
readout: the meter has no ballistics.** A peak that is held and decays, or a fill that falls at a
rate rather than following the reading, is a function of the clock exactly as `roll_at` is — it
would move between two frames, and it would then have to declare. The third assertion above is that
premise, and it is the one to read first if a meter ever grows a fall time.

## Alternatives

**Declare a meter staleness — a new `METER_STALENESS`, `moves_in == staleness`, the transport
row's shape.** This is what the roadmap's sentence asks for read literally, and it is the shape an
*I move at this rate* declaration has. It loses on all three of what it would have to be true for.

- **The picture it would keep fresh does not exist.** It would decide something only in the two
  states above, and in both of them the deck is not stepping and no reading is being taken, so the
  frames buy the motion they report.
- **It is a second rate inside one region.** `Declared` says it: *"a second rate would be a second
  declaration; a second user of one rate is not"* — and the region is the unit, because the panel is
  redrawn whole. Two declarations named `"mixer"` would charge `PANEL_PASS` twice for one pass, and
  `tests/schedulable.rs` counts declarations against `REGIONS` for exactly that reason.
- **The rate cannot be derived.** `BEAT_STALENESS` is one `BEAT_PITCH` of travel and `ROLL_STALENESS`
  is `ROLL_TRAVEL` in twelve steps; both are the presentation's own geometry. A meter's fill has no
  rate of its own — the signal's is the engine's, which this crate does not know (ADR-0156) — so any
  number here would be a constant chosen because it looked reasonable, which is the *"presentation
  decision nobody has taken"* ADR-0283 named and which nothing has since taken.

**Give the meter the bay's rate as a fourth user of it**, which avoids the second objection above:
the condition widens to *pending or metering* and `moves_in` becomes `ROLL_STALENESS` while a slot
meters. It is the cheapest version of the same mistake and it is worth naming because it is the one
that reads as free. It is not: `main.rs` calls `Deck::enable_meters` for every slot at startup, and
`tests/moving.rs`'s own `settled()` strip carries a reading — so *every* console this program ships
is metered, and this would put the 31 frames back on every panel ADR-0283 took 17 off. It would
revert that record in the shipping configuration while appearing to add a line.

**A `Change` arm for a reading that arrived.** `crate::repaint::Change` is *one list of everything
that can change what the console shows*, and a new level is such a thing, so an arm for it reads
like the missing entry. It is the ADR-0283 *"compare at the read-back and set a flag"* alternative
one field wide, and here it fails in a way that record does not name: the caller reads the level
during a frame it is already drawing, so the arm could only ever ask for **another** frame — and the
frame it asks for produces the next reading, which asks again. That is a spin at whatever rate the
loop can manage, which is what `Repaint::asked` refuses to do to `egui`'s own delays. The list is
kept honest by saying so in `repaint`'s own module documentation instead.

**Say nothing and leave the roadmap's sentence standing.** Rejected because the claim is load-bearing
and is not true where it is written: what holds the meter honest is that the caller's engine clock
*is* the panel's frame clock, and that is a property of `crates/karakuri/src/main.rs` which nothing
in `karakuri-console` can see. A caller that stepped a deck on its own thread would have a meter
going stale through the roll's 566 ms rest, and this crate would have no way to know. Writing the
premise down is the whole of what is owed, and it is the same shape as ADR-0283's note that the
transport row's honesty is *"the harness's rather than the declaration's"*.

## Consequences

- **ADR-0283's honest half overstates its own gap and is left where it is**, because a record is a
  description of history (ADR-0151). Two sentences in it are wrong: *"it changes on every frame the
  engine renders"* is right but incomplete — the engine renders only on frames this panel is drawn
  on — and *"the mixer bay does not change during the roll's rest is true of the roll and false of
  the bay whenever a slot is live and metered"* is false, because during that rest nothing composes
  and no reading is taken. Its measurements are unaffected: the 31-to-14 count was taken on a
  metered panel and is a metered panel's count.
- **`docs/roadmap.md`'s M5.14 item 6 needs its *What is left of it* re-read.** The meter is not an
  under-declaration; what is left of that item is the texture cache alone.
- **The still panel survives a metered console**, and that is now asserted rather than incidental:
  `tests/metered.rs` holds `View::animating` to `None` for a settled metered bay at every phase.
- **The declaration count stays at two.** `tests/schedulable.rs`'s *"a third one wants both sums
  re-read"* is unmoved, and `Σ (cost / staleness)` is still 0.0889.
- **The premise is now a test.** Ballistics on the meter — a held peak, a fall time — is the change
  that turns this decision over, and it fails `the_meter_is_drawn_from_the_reading_and_not_from_the_clock`
  rather than shipping a readout that freezes for 566 ms with nothing saying so.
