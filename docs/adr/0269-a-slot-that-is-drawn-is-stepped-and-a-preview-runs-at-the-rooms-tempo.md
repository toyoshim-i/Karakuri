---
id: 0269
title: A slot that is drawn is stepped, and a preview runs at the room's tempo
status: accepted
date: 2026-09-07
supersedes: []
superseded_by: []
principles: [0082, 0094]
tags: [engine, deck, governor, ui]
---

# A slot that is drawn is stepped, and a preview runs at the room's tempo

## Context

[ADR-0258](0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
decided that every deck slot is drawn into a cell of its own, on every frame, whatever its
residency, because **an operator has to see material before putting it on air** and the cell is what
that decision is made on. It settled the order the instrument is played in: look at the cell, decide
from what you see, then raise the fader.

The engine drew every slot, and stepped only some of them. `Frame::render` had three branches:
`Live` stepped and drew, `Priming` stepped on the frames the governor's rate allowed and drew on all
of them, and `Allocated` drew and never stepped. The deck's own documentation said what that meant
for the cell in as many words — *"the Allocated branch draws and never steps, so what it shows is the
still it stopped at"* — and the panel's startup legend said it to the operator: *"a slot that is not
stepping shows the still it stopped at and one that has never stepped shows black."*

**That is a departure from the design ADR-0258 recorded, and it is a departure whatever it costs.**
A cell drawn from a slot nothing is stepping is not showing the operator the material. It is showing
them a still — or, for a slot that has never stepped at all, whose element buffers
`Simulation::initialize` filled with zeros, a black frame with a smear at the origin. Either way it
is not the thing the decision is supposed to be made on, and the operator is being asked to choose
between candidates by looking at pictures of them standing still.

The same argument at a finer grain settles the rate. A cell running slower than the room is not a
preview of what would happen if that slot went live: the material would arrive somewhere else, on a
different part of the beat, the moment the fader came up. **A preview that is not in sync with the
beat is not a preview of anything.**

## Decision

**A slot that is drawn is stepped, on every frame, at the session's tempo. Every slot is drawn, so
this is every slot.**

`Frame::render` has two branches now. The `Live` one is unchanged: transport, `prepare`, `render`,
meter. `Priming` and `Allocated` are one branch, and it is `prepare_warming` followed by
`Set::render` — this frame's steps, off this frame's tick, and the draw into the slot's own target.
Nothing reaches the mix that did not before; what changed is that the picture in the cell is
running.

**Not at a reduced rate**, and that is the half of this decision that removes a mechanism rather than
adding one. `Residency::Priming` could be slowed by the governor to one step every *n* frames, with
`Deck::set_prime_one_in` as the manual door onto the same field. A slowed slot is drawn on every
frame and steps on one in eight, which is a cell showing an operator material at an eighth of the
room's tempo. There is nowhere left for a rate to apply, so the rate is gone: `SLOWEST_PRIME_ONE_IN`,
`Reason::Slowed`, `Decision::prime_one_in`, `Slot::prime_one_in` and `Slot::prime_phase` are all
deleted, and `Governor::admit` either fits a request whole or parks it.

### What `Residency::Allocated` means now

It meant *at rest*: compiled, buffers held, not stepping, keeping the `t` it stopped at. It now means
**off air and asked of nothing** — stepped and drawn exactly like a Priming slot, contributing
nothing to the mix, reading no level, and taking no transport.

**What separates it from Priming is that priming was asked for**, and nothing else. That is the
honest answer and it is written into the variants themselves. It is still a real distinction, because
it is the one an operator makes and the one a surface draws: a Priming slot is a request somebody
made and the governor granted, an Allocated slot is either a slot nobody asked about or a request the
budget has parked, and the mixer strip tells those apart (`Deck::is_parked`,
[ADR-0191](0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)). It is
no longer a distinction the frame path acts on.

**Residency now decides what reaches the mix, not whether the material runs.** That is the sentence
the manual and `architecture.md` carry.

### What the governor does about it, and it is nothing

A stepping off-air slot is work inside the frame that the compute budget does not account for. The
governor **may not refuse it**, and no refusal path is added for it.

The path that exists covers a different question. A park is the governor answering *may this slot be
warmed*, which is a request an operator made; a step that a slot takes because it is drawn is not
something anybody asked the governor about, any more than the draw itself is —
[ADR-0072](0072-auditioning-adds-a-draw-and-never-a-step.md) and ADR-0258 already put that draw
outside the budget and wrote the bill down rather than arguing it. A governor that could withhold the
step would be a governor that could take a cell back to a still, which is the state this record
exists to remove.

That is
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
losing for the reason written in it, for the second time on this surface: *a rule protecting a
performance may not be used to remove what the performance is played with.* The bill is paid and
written down
([P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).

**What an operator sees when the governor refuses is therefore unchanged, and it is on the strip
rather than in the cell.** A parked slot's chip rolls toward the residency that was asked for and
falls back; its cell goes on showing material running. A park withholds the grant, not the
simulation.

### Whether `measure_slots`' rewind still belongs

It does. The question is fair, because the state a rewind leaves — every element at the origin — is
the expensive one this record measures, and `Deck::measure_slots` is what manufactures it. Two things
answer it.

**It is a restoration here and not a reset.** `measure_slots` only measures a slot whose Set is at
`t == 0`, so the rewind undoes the measurement's own steps and nothing else. Skipping it would leave
every measured slot a few steps into a simulation nobody asked to advance, and would make the first
frame of a session depend on how many samples the probe happened to take.

**What it manufactures is no longer permanent.** Before this record, a slot rewound at startup and
left off air stayed at the origin for the whole session, and drew its entire capacity at one address
on every frame of it. Every slot steps every frame now, so that state is gone on the frame after the
rewind.

## What it costs, and this is a consequence rather than the reason

The reference workload — `examples/drift_shell.kir` + `soft_points.kir` at capacity 262144, 1280x720,
four slots with one Live — through
`karakuri-engine/tests/deck.rs`'s `the_cost_of_a_slot_and_of_the_composite_are_measured_and_reported`,
which prints these rather than asserting them. Host clock around submit-and-wait, so biased high
([P-0095](../principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md)).
Medians of 120 frames. **The machine was busy while these were taken and the figures below are the
quiet runs of two before and ten after**, which is stated rather than hidden: the four-slot lines
swung between 21 ms and 63 ms across the loaded runs while the one-slot lines held at 10.1 to 10.6,
so the contention shows up on four simulations and not on one. The one figure that did not move with
the load is the one this record turns on — the cold line read 207.5 and 209.7 on both before-runs,
loaded and quiet — because two hundred milliseconds is not noise.

| | before | after |
|---|---|---|
| bare Set, no deck | 7.3, 10.0 | 9.9 – 10.6 |
| deck of one | 7.3, 10.1 | 10.1 – 10.6 |
| deck of four, all Live | 21.2 | 21.0 |
| four, one Live, three warmed and then taken off air | 20.5, 20.7 | 21.1 |
| **four, one Live, three off air since the first frame** | **207.5, 209.7** | **22.4** |

**Stepping is cheaper than not stepping on this material, by about a factor of nine on the line that
matters.** Even in the loaded runs, where the absolute numbers mean nothing, the cold line came in
*below* the warm one instead of ten times above it. The mechanism is not subtle: an unstepped Set
draws its whole capacity at the origin, so every primitive lands on one clip position and the raster
back end serialises order-dependent blending at that one address. The cost tracks the number of
distinct destination texels. Three such slots are the two hundred milliseconds a frame the panel was
paying.

**The step itself is nearly free next to the draw**, which is the fourth row: three off-air slots
that were already warm cost 20.7 ms drawn and 21.1 ms drawn and stepped.

**This is this material's number and not the instrument's, and the record must not be read as
either.** Capacity 262144 with additive blending is deliberately aggressive — it is the `.kir`
default and the reference workload for the reason `docs/contributing.md` gives, that comparable
matters more than absolute — and the same shape at capacity 4096 costs about 2.4 ms unstepped. The
concentrated-overdraw cliff is real, and an operator can reach it with a legitimate picture: a
procedure that puts every element at one point is not a mistake. **It is not being fixed here and no
limit is being added.** Somebody with the hardware who wants that picture is allowed it.

**The governor's arithmetic on the panel moved, and not for the reason it looked like it would.** It
was tempting to read the startup park as circular — deck B refused for want of headroom, in a frame
that was 200 ms because of the unstepped draws the refusal was avoiding. It was not: the headroom is
computed from per-Set probe measurements taken at a fixed reference resolution before the first
frame, and the probe steps the Set before it measures it, so no frame time enters it. The
measurements are the same either side of this change (deck A 9.02 → 8.89 ms, deck B 8.841 →
8.790 ms, two runs each). What moved is `Engine::ask_to_prime`'s budget, which was
`committed + warming / (2 × SLOWEST_PRIME_ONE_IN)` and is now `committed + warming / 2` — half of a
cost is still not that cost, so the park is still arithmetic on every machine. Headroom went from
0.553 ms to 4.395 ms and the verdict is the same `NoHeadroom`.

## Alternatives rejected

**Step at a reduced rate.** Keep the governor's one-step-in-*n* and give the Allocated branch a rate
of its own. It is the cheap version of this decision and it buys a cell that is running, which is
most of what is wanted. It loses on tempo: a cell at an eighth of the room's rate shows material that
would arrive somewhere else the moment the fader came up, so it is a preview of a thing that will not
happen. It also inverts the ladder it was meant to preserve — a slot the governor *parked* would step
every frame while a slot it *granted at a slow rate* would step less, so asking for a slot to be
warmed would make it warm more slowly than not asking.

**Skip the draw for a slot that has not stepped.** The obvious fix, and the wrong one. Everything
above about concentrated overdraw is a draw of an unstepped Set, so not drawing one removes the whole
cost, changes nothing else, and is a two-line change. What it produces is a permanently black cell
for a slot that has material in it — which is exactly the state ADR-0258 exists to prevent, and
exactly the *Draw only the slots that are already drawing* alternative that record already rejected,
arriving again from the other end. **It is the option that keeps the departure and makes it cheap.**
An optimisation that is indistinguishable from the defect is worth naming as one.

**Leave it alone.** The instrument works, the operator can bring a slot up to see it move, and
priming exists for exactly the case of a cold candidate. It loses because bringing a slot up to see
what it looks like is putting it on air to find out whether it should go on air, which is the blind
choice ADR-0258 was written to remove; and because *priming exists for that* is a request the operator
has to make, and be granted, before the instrument will show them what is in a slot.

## Consequences

- **The priming rate is retired.** `SLOWEST_PRIME_ONE_IN`, `Reason::Slowed` and
  `Decision::prime_one_in` are gone from `karakuri-engine::governor`; `Deck::prime_one_in`,
  `Deck::set_prime_one_in` and `Residency::steps` are gone from `Deck`. `karakuri-cli`'s
  `report_governing` no longer prints a rate, and the panel's startup legend no longer explains one.
  `Governor::admit`'s peak cap — *a rate can spread a cost, it cannot shrink one* — and the
  comparison it guarded are now one test.
- **`Set::refresh_view` is gone.** It existed for a slot that nothing was preparing, and there is no
  such slot: the L4 uniform's viewport is written by `prepare`, on every frame, for every slot. The
  resize defect ADR-0072 found — an off-air slot drawing at the aspect ratio it had before a resize —
  is closed by the same fact rather than by a call.
- **What Priming and Allocated do is identical, and the levels are kept.** Collapsing them into one
  variant would delete the operator's request along with the behaviour, and the request is what a
  park is a park of. Whether three residencies are still the right vocabulary once two of them do the
  same thing is a question for the surface that draws them, and is not answered here.
- **[P-0082](../principles/0082-looking-never-writes-back.md) stands, and its example moves.** The
  file names deck.rs's Allocated branch as a place the rule holds, *where the Allocated branch draws
  and never steps*, and that sentence is now false of that branch. The rule is not: the step is the
  residency's and not the monitor's, so taking every cell off the console would stop no slot. What
  P-0082 rules out is a preview that moves the thing it was opened to judge — *you watch a moving
  picture, decide you like it, put it on air, and it is not where you were looking* — and the old
  Allocated branch produced that failure rather than preventing it, because what it showed was a
  still and the material moved the moment the fader came up.
- **ADR-0072's premise about `t` is gone from the deck, and survives where it was actually
  load-bearing.** *A slot taken down and brought back resumes where it stopped* is no longer true of a
  deck slot; it is still true of the Set `swap.rs` parks outside the deck while a candidate is on
  trial, which no slot draws and no residency reaches.
  `karakuri-engine/tests/deck.rs`'s `a_slot_taken_off_air_keeps_running_and_comes_back_on_the_beat`
  is the old test with its claim reversed.
- **ADR-0258's cold-cell end is unreachable, and its test is gone with it.** *A Set that has never
  stepped draws its zeroed element state, which is near-black, and priming is what exists to warm it*
  was asserted by `a_rejected_build_shows_what_is_still_running_and_a_cold_slot_shows_black`. The
  first frame a slot is drawn on is now a frame it has already stepped, so no deck can show a
  never-stepped Set and the assertion would have been about a fixture rather than about the engine.
  The rejected-build half is kept under the shorter name.
- **`Report::committed_ms` and `Report::headroom_ms` understate what the deck spends, and by more
  than they did.** They are the sum over *Live* slots, which is what they say they are and what an
  admission decision needs; they were never the deck's total, and the gap is now three slots' worth of
  step and draw rather than three slots' worth of draw. Making `committed_ms` the sum over every slot
  would turn `over_budget` — the governor's one warning — permanently on for any four-slot deck of
  heavy material, which is worse than a figure that says what it means. **Named and not closed**: what
  the deck actually spends is the per-slot `Decision::cost_ms` summed by whoever wants it, and nothing
  in the instrument wants it yet.
- **`Set::prepare_warming`'s lag correction has one caller-visible case left.** A slot that came up
  with the deck is never behind, so it reads the session's oscillator bit for bit and the split costs
  nothing. The case that remains is a Set installed part-way through a session, which starts at
  `t = 0` against a session clock that has run — and the discontinuity that leaves is on that slot's
  cell, where material reading `beats` moves when it goes on air. Named rather than closed, as it was
  before. `tests/priming.rs`'s `a_set_that_is_behind_the_session_warms_into_the_same_material` covers
  it, manufacturing the lag by handing the deck an oscillator that has already run; the same
  construction replaced the parked-slot one in `tests/transport.rs`.
- **The watchdog still does not judge an off-air slot, for a different reason.** *It renders nothing,
  so the frame interval is entirely the other slots' cost* was the argument, and it stopped being true
  at ADR-0258 and is further from true now. The conclusion is unchanged and the reason is attribution:
  the interval is one number for the whole deck, so a candidate on an off-air slot would be accepted
  or rolled back on its neighbours' cost.
- **Two tests are new, three that asserted the rate are deleted, and one is replaced.** The new pair
  is `tests/priming.rs`'s `an_off_air_slot_steps_every_frame_at_the_rooms_tempo` and
  `an_off_air_slots_cell_shows_material_running_rather_than_a_still`; the second warms its slot on
  air before taking it off, so what it catches is a cell that goes on showing the *same picture*
  rather than a black one — the failure that looks like a working preview until you watch it. The
  deleted three are `a_priming_slot_at_a_reduced_rate_advances_slower_than_wall_time`,
  `a_rate_change_takes_effect_from_a_defined_frame` and
  `priming_at_a_reduced_rate_reaches_the_same_state_as_being_live`;
  `the_priming_rate_does_not_change_the_signal_a_warming_set_reads` is replaced by
  `a_set_that_is_behind_the_session_warms_into_the_same_material`, which is the same property against
  the lag that is left.
- **Seven tests were watched failing against the old branch put back**, which is the whole of the
  evidence that any of them tests this: the two new ones, `tests/deck.rs`'s
  `an_off_air_slot_is_drawn_every_frame_and_steps_every_frame` and
  `a_slot_taken_off_air_keeps_running_and_comes_back_on_the_beat`, `tests/priming.rs`'s
  `an_off_air_slot_that_goes_live_shows_warmed_state`, `tests/governor.rs`'s
  `calling_govern_every_frame_changes_nothing_and_the_parked_slot_runs`, and `tests/binding.rs`'s
  `every_live_slot_reads_the_same_session_phase`.
- **The panel's startup legend said this wrong in three places** and now says it in one voice. That is
  the second time this file's narration has had to be corrected in the same direction: it said the
  three off-air slots cost nothing while they were being drawn, and *a draw each and no step* while
  they were being stepped.
