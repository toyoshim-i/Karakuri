---
id: 0323
title: A scheduled move is refused on a control a lane holds
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0083, 0094]
tags: [engine, sequencer, console, determinism]
---

# A scheduled move is refused on a control a lane holds

## Context

**A lane reaches the setter a hand reaches, and that setter cancels.** `Deck::set_opacity`'s first
line is `self.cancel(slot, Control::Opacity)`, and `Deck::cancel`'s own doc says why: *"Called by
hand on every manual write, which is the rule: an operator reaching for a fader is the one place an
automatic thing must not be writing too."* A lane is not a hand, and
[ADR-0322](0322-the-sequencer-is-polled-like-a-transition-live-only-and-its-writes-are-its-record.md)
makes it reach the same setter on purpose, because that is where a hand and a lane were supposed to
meet.

**So a live lane on deck A's fader silently kills any crossfade, fade or wipe onto deck A, within one
step of it being scheduled** — 117 ms at a sixteenth at 128 BPM. Nothing says so: the key does what
it always did, the record is written, and the fade is cancelled by the next step. That is exactly
what rule 04 forbids — *"a control you cannot use says why it cannot be used rather than going grey
without a reason"* — except worse, because the control does not even go grey.

The bay found this while its design was being written and no record carried it. It is separated from
ADR-0322 because it is an **engine-facing refusal** rather than a producer, because it can land after
the first slice, and because the alternative it refuses is a shape somebody will re-propose.

## Decision

**Scheduling a move on a control a lane holds is refused, and the refusal names the lane.**

The four operations that schedule are `FadeDeck`, `Crossfade`, `Wipe` and `SelectRenderer`; the
first three schedule a `Transition` on a `Control` and are the ones this reaches. `SelectRenderer`
schedules a `Selection`, which is not a control a lane can drive, and is untouched.

**A lane holds a control when the armed pattern has an unmuted lane whose target names it** — deck
and `Control`. That is a pure function of the pattern, so it is answered in `karakuri-pattern` and
not in a surface: a rule held in one surface binds none
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)), and the pointer, the keys, a
map and a model all schedule.

**The refusal is taken before any record is written, and that is not a detail.** If it were taken
inside `Deck::schedule`, the operation would still have written `Record::Transition` in the live run
and a replay — which runs no sequencer, ADR-0322 — would find nothing holding the control and run
the fade. The live performance and its replay would differ, which is
[P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md). So the ask is refused where
the operation is turned into records, on the same path that already completes an operation from the
running world, and the record that is never written is the whole of what a replay sees.

**One sentence, worded once**, on `no_such_slot`'s precedent and
[ADR-0131](0131-one-refusal-sentence-per-mistake-across-the-surfaces-that-face-a-person.md)'s rule —
so four routes cannot drift into four explanations. It names the lane, because
[P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md) says a refusal carries
what the next attempt needs, and the next attempt here is *mute that lane and ask again*: the mute is
the lane's take-back and it is one press on the label.

**This is the direction an operator is actually surprised in.** A fade that does not start, with a
sentence, is recoverable; a fade that starts and is killed 117 ms later is a defect the operator will
diagnose as the fade being broken.

## Alternatives rejected

**Accept it — the sequencer wins, silently.** The cheapest, and it is the status quo the code would
have shipped. It means the crossfade key does nothing while a lane runs, with no sentence, which is
rule 04 exactly. It also fails rule 02 from the other side: the operator cannot see who is holding
the control (see below), so there is nothing on screen to explain the fade that vanished.

**The lane's write does not cancel.** Split the setter so a caller says whether it is a hand: a fade
and a lane would then both run, the fade moving the fader while the lane writes it, last writer per
frame winning. **This is the right long answer and it is not this record's**, because it puts a *who
is writing* concept into the engine that nothing else needs yet, and because what the two writers
should do when they disagree is a second decision nobody has taken. It is named here rather than left
to be rediscovered: **if a fade over a sequenced fader is ever wanted, this is the shape**, and this
record is then the thing to supersede.

**Refuse the lane instead of the schedule** — pointing a lane at a control a transition is running on
is refused. It puts the refusal on the rarer act and it expires: the fade ends, the refusal stops
being true, and the operator has a lane they were told they could not have for a reason that lasted
two seconds.

**Cancel the lane instead of refusing** — a scheduled move mutes the lane it collides with. It is the
hand-wins reading, and it loses because a mute is a state a hand set: a fade silently unmuting or
muting a lane is a control moving without anybody touching it, which is what rule 02 is about.

## Consequences

**The refusal is not built and the lookup it needs is** (2026-09-09).

- **`karakuri_pattern::Pattern::holder_of_fader` answers *which lane holds this deck's fader*** as a
  pure function of the armed pattern, and `a_muted_lane_holds_no_fader` is what holds its one
  subtlety: **a muted lane holds nothing**, because it drives nothing — so muting is how an operator
  gets their fade back, which is the same take-back rule 02 draws on the lane label.
- **Nothing calls it yet.** The three scheduling operations still schedule, so a live lane on
  deck A's fader still kills a fade onto deck A within one step — the defect this record is about is
  present and written down rather than fixed. **It is reachable today**: the first slice seeds one
  lane over deck A's fader, and unmuting it is one press.
- **What is owed is the refusal itself**: taken where the operation becomes records — before
  `Record::Transition` is written, for the determinism reason above — carrying one sentence that
  names the lane.
- **And a test that the refused schedule writes no record**, which is the clause a replay depends
  on.

**What is owed and is not this record's.** **A channel fader does not say who is holding it.** The
mock draws `seq 1` as a source on a `.param` row's `.pval.src`, which covers the fourth lane, and
draws nothing of the kind on a channel fader, which covers the first three; `view.rs`'s own note says
these states cannot be entered today because *"no MIDI map and no sequencer lane exists to be one."*
Once a lane exists, three lanes in four hold a control that says nothing about who is holding it,
which is rule 02. **It is M5.9's** — M5.2 is closed — and the console page carries a purposeful note
saying so until the readout is drawn, because a gap with no note is indistinguishable from a
decision.
