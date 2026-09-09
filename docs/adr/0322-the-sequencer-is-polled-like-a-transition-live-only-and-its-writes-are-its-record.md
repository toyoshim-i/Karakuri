---
id: 0322
title: The sequencer is polled like a transition, live only, and its writes are its record
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0092, 0090]
tags: [engine, console, sequencer, determinism]
---

# The sequencer is polled like a transition, live only, and its writes are its record

## Context

The mock names this gap in as many words, on the `step 6` readout:

> *"What is not settled is where the thing producing steps runs — the index is cheap and could be
> taken wherever the beat is read, and the sequencer's producer is the one piece of this bay's
> machinery no record places."*

`docs/roadmap.md` carries the same item as the estimate's one excluded piece — *the sequencer's
producer* — and says it is why this bay is last.

**Everything around it is placed and this is not.**
[ADR-0222](0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md) gives the arithmetic — a
step index is `floor(beats × steps_per_beat) mod length`, a pure function of `Oscillator::beats` —
and says a step onset is seen at the next frame.
[ADR-0255](0255-three-clocks-run-at-once-and-a-slower-ones-work-never-lands-on-a-faster-one.md)
says the beat clock is *selection among options already prepared*.
[ADR-0227](0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md) refuses the
session stream a pattern record — *"the cause written down next to every one of its consequences."*
None of them says which thread reads the beat, how often, or what a replay does.

## Decision

### It is polled on the render thread, per frame, against `beats`

```
step = floor(beats × mode.steps_per_beat()) mod mode.count()
for each lane of the armed pattern, not muted:
    if step != lane.last_step:
        operate(lane.target.operation(if lane.step_on(step) { lane.on } else { lane.off }))
        lane.last_step = step
```

in the frame loop, after the tick has been applied and before the frame is drawn.

**This is not a fourth clock**, which is the *Sequencer* note's sentence and ADR-0222's: it is
`beats`, subdivided, read where the beat is already read. **It is what a transition already is, one
row finer.** `karakuri-engine` has no beat callback and no scheduler: `Transition::value_at(beats)`,
`Selection::due(beats)` and `transition::quantise(beats, quantum)` are all polled against `beats` by
the frame loop, and `Oscillator::advance` takes its step count and `dt` as arguments — nothing in it
reads a clock, which `advance_uses_only_its_arguments` pins. So nothing in the poll is a clock read,
and [P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)'s exception is not
touched.

**A skipped step is dropped, not caught up.** If a stall carries a frame past two boundaries the
lane emits the current step and only that one. The ir-spec's own sentence is the argument: *"a step
onset is seen at the next frame, which is right for a sequencer that writes values and would not be
for one that fires events."* Emitting the skipped step would put a value in the stream that is
overwritten in the same frame.

### It emits nothing of its own, and its writes are its record

`target.operation(value)` goes into the same `operate` a hand's press goes into
([ADR-0321](0321-a-lanes-target-is-an-operation-with-its-value-elided.md)), so what reaches the
stream is `Record::Opacity` and `Record::Ride` — records that already exist, already replay, and
already have a `written` arm. That is ADR-0222's claim made concrete: *"a hand and a lane meet at
`Live::operate` where every other conflict is already resolved."* It is also
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) by construction, since a lane's
write ends in the same record every other control ends in.

### The sequencer does not run on replay

Its consequences are already in the stream, verbatim, on `tick`'s and `audio`'s stated terms —
derived from the world when live, read back verbatim on replay, and the engine cannot tell which
happened. So the producer belongs to the live loop and to nothing else: `karakuri-cli` already
refuses `--tempo-source` and `--record-session` alongside `--replay` for the same shape of reason,
and `crates/karakuri` has no replay path at all today.

**P-0092 is therefore satisfied by the pattern already established for audio, and nothing new is
needed for determinism.**

### The volume, measured rather than waved away

Four lanes at a sixteenth at 128 BPM is `4 × 8.53 = 34.1` records a second — **122,880 an hour**,
against `tick`'s 216,000 (`docs/ir-spec.md`). It is a real number, it is the same order as what the
stream already carries per frame, and it is taken rather than suppressed. **A lane that skipped an
emission identical to its last one would fail to reassert after a hand moved the control between two
identical steps**, which is the case the take-back is about.

### The take-back is the mute, and the bay declares

**A hand's write lands immediately and the lane reasserts at the next step; the way to keep the
hand's value is to mute the lane.** That is the mock's own sentence — *"Click to mute the lane and
keep the pattern"* — and it is rule 02's *take back sits next to it*, drawn on the lane label rather
than on the strip.

**The sequencer is the third declaring region** under
[ADR-0283](0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md): a
constant `staleness` and a `moves_in` equal to the time to the next step boundary. It is the first
region whose deadline is a beat **subdivision** — 117 ms at a sixteenth at 128 BPM, against the
transport row's beat — so it is what `moves_in >= staleness` will be tightest against.

**One thing this decision owes and does not pay**, recorded so the gap is not invisible: a lane
reaches `Deck::set_opacity`, which cancels unconditionally, so a live lane silently kills a
scheduled fade onto the deck it drives.
[ADR-0323](0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md) is that refusal, it is
engine-side, and it may land after the first slice.

### `written` answers `Silent(Surface)` for all five

Today all five answer `Written::Owed(Owed::Undecided)`, and the arm gives two reasons. **The first
is already refuted in the same file.** The arrangement family — `SaveArrangement`,
`RestoreArrangement`, `ResetArrangement` — is *also* library data under the store, on ADR-0221's
terms, and it answers `Silent(Surface)`. The test that pins it says why, and the sentence transfers
word for word:

> *"a save writes a file, and a file is not a record whose timing `OnLanding` could be about, nor a
> gap `NoRecord` could be about"*

So `Silent::Surface` already covers *a file under the store*, because the arrangement put it there;
nothing is widened to fit these five and they are the second family of the same kind. **The second
reason — *nothing about these has been settled at all* — is what these four records are.** Once
ADR-0227's refusal of a pattern record is read as settling the stream's silence rather than
deferring it, `Owed(Undecided)` is the wrong answer.

## Alternatives rejected

**A worker on a timer.** Refused before it is considered: a timer is a clock read into simulation
state, which P-0092 rules out, and the worker would have to read the oscillator anyway.

**A beat-clock scheduler that ticks the sequencer.** There is no such thing to hang it on, and
building one puts a second time authority beside `beats` — the engine polls, everywhere, and the two
would have to be kept in step. It collapses into the poll.

**Re-deriving the steps at replay.** The alternative to *the sequencer does not run on replay*, and
it loses twice. It needs the pattern in the stream, which ADR-0227 refuses; and it would make a
replay depend on a file under the store that may have been edited since, which turns a replay into a
re-run.

**Suppressing a repeated emission.** It would cut the 122,880 to roughly the number of edges in the
pattern. It loses on the take-back: after a hand moves a fader between two identical steps, a lane
that suppressed the repeat would never reassert, and the mute would be a take-back a hand could not
undo by waiting.

**Catching up a skipped step.** It is what a sequencer that fires events must do and it is wrong for
one that writes values: both writes land in the same frame and the first is invisible except in the
stream.

**Leaving `written` at `Owed(Undecided)`.** It is the status quo and it is defensible for exactly as
long as *what a pattern is* is unsettled. These records settle it, and an `Owed` that has stopped
being owed reads as debt somebody still has to pay.

## Consequences

**What was built the same day** (2026-09-09):

- **The poll is eight lines in `crates/karakuri/src/main.rs`'s frame handler**, taken from
  `deck.signals().oscillator().beats()` before the frame is composed, and every emission goes
  through `App::performed` — the same function a press goes through — so what reaches the stream is
  `Record::Opacity` and nothing this bay invented. `karakuri_pattern::Playhead::advance` is what
  answers *has the step index moved*, and it is indexed rather than iterated so a boundary
  allocates nothing.
- **Live only, and there is no replay path in this window to exclude it from.** That is the honest
  state rather than a test: `crates/karakuri` has no `--replay`, and the block is where an exclusion
  would go the day it gains one. What makes a replay correct is asserted where it can be —
  `editing_a_pattern_writes_a_file_and_no_record` in `karakuri-operation-record` — because a lane's
  consequences are `Record::Opacity` and its kin, which already replay.
- **The bay declares**, `karakuri_console::view::View::sequencer_declares`: `STEP_STALENESS` (one
  sixteenth at the mock's tempo, 117.19 ms, stated at that tempo for ADR-0212's reason) and
  `step_moves_in`, the distance to the next boundary, clamped to the rate. It declares **only while
  the pattern has a lane**, because the playhead is the picture that moves and it has nothing to
  stand over otherwise. `tests/sequencer.rs` holds both, and `tests/schedulable.rs` sums it.
- **`karakuri-operation-record`'s five arms moved from `Owed(Undecided)` to `Silent(Surface)`**,
  with the arrangement family's sentence at the arm and
  `editing_a_pattern_writes_a_file_and_no_record` beside
  `keeping_an_arrangement_writes_a_file_and_no_record`. The crate's own counts moved with them.
- **Tests watched to fail first**: the playhead answers once per step and a stall drops the step it
  missed; an eighth reads slot `2k`; a muted lane emits nothing and keeps its steps; a cell press is
  a state and not a flip; the bay's declaration and its invariant; and the press-handler seam, which
  fails naming the row when the window stops asking the control it drew.
- **`docs/manual/console.html`'s `step 6` tip lost *"where the thing producing steps runs"*** and
  says it is polled per frame against `beats`, like a transition, live only. The *Sequencer* note
  gained the mute as the take-back.

**The MCP column gave way, and the class stays shut.** ADR-0315 makes the members of `written`'s
`Silent::Surface` arm `gap` in the MCP column, *read off the arm rather than off a list*, so joining
that arm reaches these five: all five now read `gap` there, carrying ADR-0315's sentence word for
word — one refusal per mistake, worded once (ADR-0131). That answers the bay head's *"which of the
two gives way is not decided"*: the page did, and `gate.rs` is unchanged — the five stay
`ClosedUnclassed(Unclassed::Lanes)`, in a group no bay head opens.

**Two more of the bay's controls reach that poll now** (2026-09-09,
[ADR-0327](0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md)), and
neither changes anything here. A bank press is `SelectPattern` and resets the playhead, so the next
frame is a boundary in the new pattern; `+ lane` appends a **muted** lane, which the poll skips, so a
lane added mid-performance emits nothing until the label is pressed. **The take-back is still the
mute** and this is the same rule met at the other end: a lane arrives with every slot off, and an
unmuted one would write `off` at the next boundary — 117 ms — which is exactly the silent write this
record measured the volume of.

**And the refusal this decision owes is still owed.** ADR-0323's lookup is built and nothing calls
it, so a live lane on a deck's fader still kills a fade onto that deck — and `+ lane` makes that
reachable in two presses rather than one, on any deck the mixer draws.

**M6's agents may reopen it, and this is where a reader should start.** The badge says what is true
while a model cannot see the console; an agent that *programs* a lane is asking for a route into a
pattern rather than into a window, and nothing here forecloses one. What it would have to answer
first is ADR-0315's own question — whether a model has a view of the console — and then whether a
bank index means anything to a caller that cannot see which bank is armed.
