---
id: 0297
title: The panel's tick is measured, and the fixed step a frame ran the room at the display's rate
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0092, 0095]
tags: [determinism, session, engine, console, audio, architecture]
---

# The panel's tick is measured, and the fixed step a frame ran the room at the display's rate

## Context

`crates/karakuri/src/main.rs` advanced the session by a constant:

```rust
const STEPS_A_FRAME: u8 = 1;
```

and its documentation said why:

> [P-0092] says simulation time comes from a record and never from a clock. There is no record here
> — this program is not a session — so the honest third option is neither: a fixed count per frame …
> It is not a performance, and a `karakuri-cli` that measured an interval and wrote a `tick` is what
> a performance is.

**Both halves of that were wrong, and the second one had stopped being true rather than never having
been true.**

**P-0092 does not say *never from a clock*.** Its first sentence is the rule in full:

> **Time comes from a record**: live, the engine derives the step count from real time and writes it
> in; replaying, it reads the number back and derives nothing. A clock may be read to judge *cost*
> and no value derived from one may reach simulation state — that is the whole of the exception.

The clock read is the **live path of the rule**. What the rule forbids is a clock reached from inside
the simulation, and a replay deriving the count a second time from the replaying machine's timing —
*"which makes a replay a re-run and lets the faster machine see a different performance"*. A fixed
count is not a third option between measuring and reading back; it is a refusal to take the
measurement the record is shaped to carry.
[ADR-0006](0006-the-step-count-is-a-record-not-a-measurement.md) says the same thing in the same
words — *"Live, the engine derives it from real time and **emits** it"* — and so do
`docs/ir-spec.md`'s *On `dt` and simulation time* and `karakuri_engine::frame::Committed::steps`'s
own doc comment: *"the **one** thing that legitimately differs between a live run and a replay:
measured from a clock there, read from a `tick` here."*

**And *this program is not a session* stopped being true with
[ADR-0289](0289-the-rec-pill-is-a-record-stop-toggle-and-each-start-takes-a-fresh-stamp.md)**, which
gave the transport row a `rec` toggle that opens a recorder and writes a stream from this window.
That record carries the misreading forward in its own consequences — *"`STEPS_A_FRAME` says this
window advances one step per frame drawn and reads no clock … That is P-0092 satisfied"* — and it is
history and stays as it is (ADR-0151).

### What it cost, and it was not about recordings

`DT` is 1/60 s, this window is `PresentMode::Fifo` — `Cost::wait`'s own sentence, *"so at 60 Hz this
is most of the 16.6 ms"* — and `karakuri_signal`'s oscillator *"advances by simulation steps, never
by wall clock"*. One step per frame **drawn** therefore made simulation time advance at **the
display's refresh rate divided by sixty**:

| what the window is doing | frames a second | simulated seconds a second, fixed | measured |
|---|---|---|---|
| a 30 fps sag | 30 | **0.5000** | 0.9983 |
| every sink folded, the beat declaring at `BEAT_STALENESS` | 40.53 | **0.6755** | 0.9983 |
| a 60 Hz display | 60 | 1.0000 | 0.9983 |
| a 75 Hz display | 75 | **1.2500** | 0.9983 |
| a 120 Hz display | 120 | **2.0000** | 0.9983 |
| a 144 Hz display | 144 | **2.4000** | 0.9983 |

Taken on 2026-09-08 by driving the derivation with stated frame arrivals over ten seconds at each
rate — `karakuri-environment`'s `clock::tests::the_room_advances_at_one_second_a_second_whatever_the_display_does`,
which is the test that holds the table. A display's refresh rate cannot be changed from a test, so
what is measured is the half that decides; the other half is `PresentMode::Fifo`, which is why
frames arrive at the display's rate at all. The 0.9983 is the one step still sitting in the carry
when the ten-second window closes, and it is the same 0.9983 at every rate — the residue of the
window rather than of the display.

**The 60 Hz column is the reason this was invisible.** The room ran at exactly the right speed on the
machine it was written on, and the beat grid `README.md` promises follows the room ran at double
speed on the desk next to it.

## Decision

**The panel derives `steps` from the interval it measures and writes it into the record**, which is
P-0092's live half and is what `karakuri-cli` has always done. One read per composed frame, and the
number reaches four places from that one reading: the audio frame's advance, `Committed::steps`,
the deck, and the `tick` that closes the frame while a recording is running.

**The derivation is one implementation, and it moved to `karakuri-environment`.**
[ADR-0215](0215-the-package-is-karakuri-environment-and-a-module-belongs-if-what-it-deals-with-is-outside-this-process.md)
had already decided this and named it as owed: applying the charter to `Clock::steps(&mut self, now:
Instant)` it concluded *"wall-clock time comes from outside this process"*, and its consequences
close with **"`Clock` moves and `Live` does not"**. It had not moved because one program needed it.
`karakuri-environment/src/clock.rs` is that move carried out, with the three tests that were beside
it in `karakuri-cli`.

**Copying the derivation was the alternative and it is refused.** A frame's step count is the live
half of a determinism rule; two copies of it are two answers to that rule, agreeing until the day
one of them is edited. This is not *the CLI does it, so the panel should* —
[ADR-0242](0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)
retired that argument, and it also says what still binds both programs: *"Determinism, the record
stream, the render-thread rules … those are about the system, and `karakuri-cli` is in the system.
The line is who the rule is written for."* P-0092 is written about the machine. It reaches both.

**The cap is `karakuri_store::record::MAX_STEPS`, and what it means is ADR-0006's.** Past four steps
the simulation is allowed to fall behind rather than catch up, *"because unbounded catch-up turns a
load spike into a death spiral"*, and `t` then *"diverges from wall clock permanently and never
resynchronizes"* — which that record put in the specification *"because otherwise it is reported as a
bug"*. Nothing here softens either half, and no second copy of the number is kept: `karakuri-cli`
held its own `const MAX_STEPS: u8 = 4` beside the clock and the clamp now reads the number the record
format publishes, because the value is being written into a `tick` and a `tick` a reader will not
accept is unreplayable.

**A folded panel still steps nothing.** `Clock::last` moves inside `Clock::steps` and nowhere else,
and `steps` is called from one place — after the last point at which the redraw handler can abandon a
frame, and before the `compose` that always follows it. So a window drawing no frames advances no
simulation, which is what
[ADR-0290](0290-the-level-meter-moves-only-when-a-frame-is-drawn-so-it-declares-nothing.md) turns on:
*"With every sink folded away the loop is on `ControlFlow::Wait`; nothing composes, so the deck does
not step."* What changes is what happens when such a window is drawn again — the gap is counted whole
up to the cap, rather than as one step — and that is the anti-spiral clamp working rather than a new
behaviour.

**Where the clock is read is ADR-0078.**
[ADR-0078](0078-a-frame-that-is-discarded-must-not-already-have-been-recorded.md) is the defect this
ordering exists against: a loop that read the clock, wrote the `tick` and then found the swapchain
had nothing made a replay diverge from the performance on every resize. The read sits below the four
arms that return — a surface to remake, one to ask again for, an idle window, a validation fault —
and everything below it composes.

**The output lag is built from the same interval, and no longer from the still-panel reading.**
`measure_audio` was handed `Costs::rate_now()` inverted, on the argument that the transport row's
`fps` and the beat correction's lead should be one number. They are not one question. `rate_now` is
*frames drawn on an untouched window over the stretch since something touched it*: it is `None` on
every frame near a pointer, a key or a resize, and its stretch goes on running while frames that were
asked for count as zero. The lag wants the interval between this frame and the last one, on the
frames an operator is working — which is exactly the interval the step count was derived from. One
measurement, two consumers, and the row goes on reporting the still-panel rate because that is the
different question it was always asking.

## What it costs

- **A recorded session's ticks are no longer all `steps: 1`.** ADR-0289's *"a run whose frame rate
  sagged records the sag as fewer ticks rather than as a longer frame"* is reversed: the sag is now
  recorded as the same number of ticks carrying more steps each, and the replay reproduces the
  performance rather than the frame count. That is the whole of why the number is in the record.
- **A frame can now advance zero steps**, and on a 120 Hz display half of them do. It is the carry
  spending a half-step frame on the next one, `Record::Tick { steps: 0 }` is a well-formed record,
  and `Set::prepare` already clamps and accepts it. What it is not is a dropped frame.
- **The first frame of a run is capped.** The clock starts in `App::new`, before the device, the
  compile and the first Sets, so the run's opening gap is counted whole up to four steps. Starting it
  at the first frame instead would hide the same gap by pretending it did not happen.
- **`karakuri-cli` lost its own `Clock` and its own `MAX_STEPS`.** The move is a change to a file
  this decision does not otherwise touch; nothing else in that program named either.

## Alternatives rejected

**Derive the step count from `Costs::rate_now()`, which is the measurement the panel already had on
the frame path.** It is the shortest change and it is wrong twice: the reading is `None` on any frame
near an event, so a fader being dragged would produce no interval at all, and it is an average whose
denominator counts seconds the frames it counts were excluded from. A step count has to come from
the interval between two frames, and the panel had no such measurement — the `Instant::now()` this
adds is a new clock read, and it is the one P-0092 puts in the live path.

**Keep the fixed count and let the recording carry it, on the grounds that a replay of a `steps: 1`
stream reproduces the run exactly.** True and beside the point: the defect is what an operator sees
on a 120 Hz display, not what a replay does. A reproducible wrong tempo is still a wrong tempo, and
the record stream is the mechanism, not the goal.

**Scale by the display's reported refresh rate instead of measuring.** It is a number the platform
says rather than one this program took, which is P-0095's *"establishes for itself that its clock
works … rather than against what the platform says it supports"*, and it answers nothing about a
frame that missed vsync.

**Leave `STEPS_A_FRAME` in the live path and add a clock beside it for the recorder.** Two answers to
how far one frame advanced, one of them in the picture and one in the file — which is a replay that
does not reproduce the run it recorded.

## Consequences

- `crates/karakuri-environment/src/clock.rs` is new and `lib.rs`'s *"`Clock` is owed a move it has
  not had yet"* is now that move, taken. The charter's module list gains a fourteenth entry.
- **`STEPS_A_FRAME` survives as a fixture in `mod gpu`, and the correction is a comment where the
  constant was.** The tests that compose a frame to assert what was *drawn* — a cell that took a
  pass, a rectangle it was aimed at — time nothing, so a stated count is a fixture there rather than
  a measurement withheld.
- **It is not a `#[cfg(test)]` constant up among the live ones, and that is worth writing down.**
  That was the first shape of this change and it broke five tests at once: several tests in
  `crates/karakuri/src/main.rs` scan the file for its own code — the keys the window loop binds
  against `KEYS`, the receiver `on_window_event` is bound to, the re-read after an arrow key — and
  every one of them bounds its scan at **the first `#[cfg(test)]`**. A test-only item at line 4,100
  moves that boundary nine thousand lines up the file and silences the lot. They failed loudly and
  named what they could no longer find, which is the behaviour ADR-0044's *"a test that survives
  mutation is not a test"* is asking for; the note where the constant was says so, so that the next
  person to want a test-only item in that region knows what it costs.
- **P-0095 is met by there being one way a `tick` is taken.** `Record::Tick`'s own documentation says
  it is *"emitted from real time when live, read back verbatim on replay"*; until this record the
  panel wrote a number nothing measured into a field documented as measured, which is that
  principle's *"handing an inferred number out as a measured one"*. The record format is unchanged
  and needs no method field: the provenance is a property of the stream — derived when live, read
  when replaying — and there is no longer a third kind of tick for a reader to fail to distinguish.
- **The sentence the `rec` press prints says how the ticks were taken.** ADR-0289 put the replay
  caveat on that line rather than designing around it, and this is the second half of the same
  statement: *"Its ticks carry the step count measured between one frame and the last, capped at
  four, so the replay runs at the speed this run ran and not at the speed the machine playing it
  draws."* That is P-0095 where an operator can read it — the record format carries no method field
  and does not need one, but the person who will type `--replay` is told what the numbers in the
  file are.
- **This program's own narration is corrected in two more places**: the startup reading said the four
  slots run *"one step a frame apiece"*, and the transport row's doc said the deck advances the
  oscillator *"one step a frame (`STEPS_A_FRAME`)"*.
- **ADR-0290's `main.rs commits steps: STEPS_A_FRAME per composed frame` is now history**, and its
  argument is untouched by that: the meter moves inside `Deck::begin_frame` and a caller calls
  `begin_frame` once per composed frame, whatever that frame's step count is.
- **`docs/manual/console.html` and `docs/roadmap.md` are not touched here** and may describe the beat
  as a function of frames drawn; whoever maintains them owns that reading.
