---
id: 0291
title: The tempo figure is the track, and the band is a guard on the hand
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0084, 0090, 0094]
tags: [console, transport, audio, manual, m5.4]
---

# The tempo figure is the track, and the band is a guard on the hand

## Context

M5.4's *Set the free-run tempo* row is **what an operator with no beat in the room has**, and until
now this program did not give it to them. `karakuri_operation::Operation::SetFreeRunTempo` existed
with its six words — *"What the grid runs at with nothing driving it"* — and
`karakuri-operation-record` already answered a `Record::Tempo` for it. What was missing was a
control, and three decisions it could not be built without.

**The state the operation is for is the state this program runs in.** `crates/karakuri` opens no
audio device unless it is asked to: it runs at `Signals::default()`'s 120.0, and nothing on the panel
can change it. `tapped` refuses — *"a tap sets the grid this room is being tracked against, and there
is no room"* — and `scaled` refuses in parallel. So the row beside those two is the one control in
the transport that works in the program's ordinary state.

**What was open was not the plumbing.** `Signals::correct` and `audio::apply_tempo` are public and
reachable, `written` answers a record, and `apply` was one arm short. What was open was **the shape
of the control, what a press means while a room *is* being tracked, and the track's ends** —
and the last of those had nothing behind it in the repository: `karakuri_audio::tempo::BPM_RANGE`
says of itself that it is *not* the range of answers (*"a grid at 240 bpm is a perfectly good
grid"*), `karakuri_signal`'s is a clamp against a frozen phase at `1.0..=1000.0`, and
[ADR-0277](0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md)'s derivation
of a track — *one pixel is one key press* — is unavailable here, because **this row's key column is
`gap`**: there is no key to make a pixel out of.

The maintainer settled the three, and `docs/manual/console.html`'s `.bpm` carries the settlement as
a `data-tip`: the figure is the track, a press names a value outright, the band is ±15% of what the
grid is running at, a press outside it is **ignored rather than clamped**, and a set is **accepted
while a room is being tracked** rather than refused. `docs/manual/operations.html`'s row stopped
saying `op-when launch` and says `immediate`.

## Decision

### 1. The figure is the track, and its middle is the number it draws

`.bpm` is the control. There is no track painted under it and no capsule round it, because the mock
draws neither — `karakuri_console::view`'s own rule is that a shape which looks like a control and
is not does not get drawn, and the inverse of it is that a *reading* which is a control does not get
a second shape invented for it.

**The middle of the figure is the tempo the figure is drawn at.** That is what makes the manual's
*"the number under your finger is the one you get"* a property rather than a phrase: pressing where
the ink is asks for what is already running, and the value moves away from there in the direction the
hand moved. It is also why nothing has to be marked — a mark would be saying where the current value
is, and the number *is* where the current value is.

### 2. The span is two bands either way, and the guard is the outer half

`view::TEMPO_BAND` is `0.15` and `view::TEMPO_SPAN` is `TEMPO_BAND * 2.0`: the figure runs from
`0.70 ×` the tempo at its left edge to `1.30 ×` at its right, linearly, so **the band is the middle
half of the number and the guard is the outer half**. At the mock's 128.0 the figure is 49.25 pixels
wide — 24.6 of band and 12.3 of guard either side.

**The figure has to reach past the band, or the band could not be missed.** A guard nothing can land
in is not a guard: it is an assertion the surface satisfies by construction, and no test could ever
show it refusing. So the only question is by how much, and the answer is taken from the one number
that was given rather than from a new one: **one more band**. A press that misses by up to as much
again as it is allowed to move is refused; a press that misses by more is off the number and was
never this control's.

**Linear, which is the offset track's arithmetic and not the exposure's.** A tempo is a ratio — that
is why the control beside it is an octave and why this band is a percentage — but the band is stated
as ±15% *of the tempo at the press*, and at the instant of a press that is a fixed number of beats a
minute. The whole geometry is settled by the one number the row was measured with, so equal distances
along the figure are equal numbers of beats a minute. Over a span this narrow the logarithmic and the
linear curve are within a percent of one another in any case, and the linear one is the one whose
middle is exactly the number drawn there.

### 3. A press outside the band is ignored, and *ignored* is the decision

Not clamped. `unit_of_offset` draws a value past either end **at** that end, because the offset's
ends are the range of the value; these are not ends of anything — they are how far a hand is trusted
to have meant it. Clamping would turn a press the operator did not mean into a 15% move of the grid,
which is the loudest thing this row can do.

**And the guard is part of the hit test, not only of the answer.** `TransportRow::on_tempo` is false
there, so the press is `egui`'s and falls through — `TrackerGroup::half`'s rule word for word
(*inert is not claimed*) and `input`'s *a control claims what it acts on and no more*. A control that
took the press and then did nothing is the one outcome an operator cannot tell from a panel that has
stopped ([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).

**It bounds a press and not a tempo.** 240 is reached from the tempo the mock draws by walking the
band **five times**, with the figure naming 240 outright on the last press because by then it is
inside the band; `tests/tempo_figure.rs` asserts the walk at that length.

**The tip says two presses, and that is not arithmetic that works from 128.** ±15% a press is 147.2,
and the `×2` beside the figure is drawn **inert** at this tempo — `Tracker::double` is
`karakuri_audio`'s `BPM_RANGE` against twice the grid and 256 is outside it — so no pair of presses
reaches 240 from where the mock stands. The band is the maintainer's number and the sentence around
it is reported rather than met by quietly widening it. Two presses *is* true higher up: from 190 it
is 218.5 and then 240.

### 4. The band lives where the press becomes an operation, and nowhere else

It is in `karakuri-console`, on the laid-out row, and it is **not** in `karakuri-audio`,
`karakuri-environment`, or any path an estimate travels. The tracker folds every candidate into a
window centred on the grid and the beat lock re-acquires on evidence; a ±15% bound anywhere in that
path is a grid that cannot follow a song, which is a far worse failure than a hand that can ask for
anything. The maintainer's words: *人手の時の操作ミスのゲート。自動判定に入れられたら曲に追従できなく
なるのでやめて*.

### 5. A set while a room is tracked is accepted, and the lock is told two things

The grid moves through the record — `Operation::SetFreeRunTempo` → `Record::Tempo` → `apply` →
`audio::apply_tempo`, which is *"the only way a correction reaches the oscillator, live or on
replay"* — so the live path and a replay are one path
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)). What the record does not
carry is the session state around it, and `Audio::set_tempo` hands over exactly two things:

- **The tracker's window**, moved to the new tempo at once rather than at the next frame, which is
  `Audio::octave`'s own sentence: an estimate published in between would be folded into a window
  centred on the tempo the operator has just left.
- **The beat lock's run of evidence**, which is `BeatLock::retarget`:

**`agreement` and `disagreement` are discarded and `candidate_bpm` becomes what was named** —
`BeatLock::octave`'s reason exactly. Those counters count consecutive estimates about a grid that has
moved, and the tracker goes on publishing from its old window for up to a quarter second; a
`RELOCK_EVIDENCE` run part-served by opinions formed before the press would take the grid back in
less than the two seconds that number is built on, and the operator would watch the control undo
itself. Cleared, a room that really is at another tempo pays a whole fresh run of eight for it.
`a_tempo_named_by_hand_costs_the_room_a_whole_run_to_take_the_grid_back` measures both: eight revisions
with the clearing, three without it.

**The state is left exactly as it is, and that is the one place this parts company with a tap and an
octave.** Both of those set `State::Locked`, and both are a performer saying what the grid is locked
*to*. This says what it **runs at**, and says nothing about whether anything is driving it:

- **Not `Locked`.** `BeatLock::locked` is a readout — `Status::locked` is read every frame and drawn
  — and *locked* about a room with no beat in it is the confident wrong judgement
  [P-0084](../principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)
  refuses. It would also cost the operator the thing they set the tempo for: a beat arriving
  afterwards would need `RELOCK_EVIDENCE`'s eight revisions to take a grid that `ACQUIRE_EVIDENCE`'s
  three would have taken from a free-running one, so naming a rough tempo before the music started
  would make the music *slower* to lock than saying nothing.
- **Not `Free` either.** A room that is being tracked stays tracked: dropping to `Free` would let the
  next three agreeing estimates **jump** the grid, where a locked one trims toward them.

**What the tracker then does is the tracker's.** A room genuinely at another tempo, with the grid
hand-moved inside the same octave, will take the grid back after a full run of disagreement — the
same mechanism a tap is subject to. That is *accepted and recomputed around*, which is what was
asked; it is not a control undoing itself, because the run it costs is one that starts at the press.

## Alternatives rejected

**The figure spans exactly the band.** The nicest control of the three — every pixel of the number
live — and it makes the ±15% unfalsifiable: no press on the figure could be outside a band the figure
is a picture of, the guard would be dead code, and the manual's *"a press outside it is ignored"*
would describe nothing. A rule that cannot fail is not the rule the tip states.

**The figure spans an octave, `×½` to `×2`, logarithmically**, so that its ends are what the two
octave chips beside it reach. Attractive — the figure would be the continuum between the two marks —
and it loses on the hand: the band is then 21.8% of the number, twelve pixels at the mock's tempo,
which is smaller than the target `OffsetTrack::grip` was widened to find.

**A fixed track with ends**, laid out like the offset's. There are no ends: the two ranges in the
workspace both say they are not the range of answers, and inventing a pair here is exactly what
`karakuri_audio::tempo` refuses to let the fold do. A track needs a range and a step and this row has
neither.

**Hit-testing a figure was rejected once**, by
[ADR-0277](0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md): *"the
capsule is as wide as the number in it, so the target would move under the hand as the hand set it,
and worst at the moment it was being used most precisely."* That objection is answered rather than
ignored. It was made where a **track** was available and a figure was the lazier option; here nothing
else is available, and the mock's tip says which shape this is. And the movement it warns about is
almost absent: the figure is the **first** item in the row, against `.transport`'s left padding, so
it does not walk sideways as the row does; digits are equal-width, so the box changes size only when
the number changes glyph count — crossing 100, once, between two presses and never during one.

**Clamping a press outside the band**, which is what the offset's own `unit_of_offset` does. It turns
the failure it is guarding against into the largest move the control can make.

**Refusing the operation while a room is being tracked.** It is the shape the roadmap expected —
*"a control that undoes itself two seconds later is worse than one that says no"* — and the
maintainer decided against it: *ロック中でもターゲットは動くので受け入れて再計算*. A refusal would also
have to be taken by a surface that cannot see a device (ADR-0156), or by the lock, where it would be
the second half of a band that must not be there.

**A `Reason::Set` variant on the lock.** `Reason` is *"why a correction happened — for the status
line"*, and this correction is not the lock's: it comes from the record. `BeatLock::update` clears
`reason` at the top of every frame, so a variant here would name something nothing observes.

## Consequences

- **The transport row is four readouts and two controls.** `view::transport`'s own documentation and
  `tests/transport.rs`'s `the_readouts_in_the_transport_row_are_not_controls` both said the tempo was
  a readout, and both moved; the tempo left that test's probe list, and what it is instead is
  `tests/tempo_figure.rs`.
- **`crates/karakuri/src/main.rs`'s `apply` grew its `Record::Tempo` arm**, which is the live half of
  `apply_tempo`'s *only way in*. A tap and an octave still do not come through it: theirs is the beat
  lock's answer and `written` cannot build it
  ([ADR-0278](0278-an-operation-no-record-can-be-written-for-leaves-the-window-before-it-is-written.md)).
- **`retargeted` sits beside `nudged` and does not return early.** It is the third performer that
  reaches the audio session and the only one whose operation goes on to `written` and `apply`.
- **`Audio::set_tempo` takes no `Signals`**, which is what says it moves nothing.
- **The console's registration is one more row of `input::PROBES` and one more entry of
  `press_handler::ASKED`** ([ADR-0274](0274-a-control-is-a-row-in-the-consoles-own-table.md)), asked
  through the derivation the `rec` pill already binds. `CONTROLS` rises to 41.
- **The row's panel badge on `docs/manual/operations.html` becomes `has`**: a control emits the
  operation, which is what [ADR-0213](0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
  defines the badge as. The key column stays `gap` — this row still has no key, and that is the
  reason its track could not be derived the way the offset's was.
- **The MIDI column is still empty and is now the interesting one.** The tip says so: a control
  change can set a tempo, and this is the one thing in the row it could set without naming a word
  from a closed list.
