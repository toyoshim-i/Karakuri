---
id: 0278
title: An operation no record can be written for leaves the window before it is written
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: []
tags: [console, transport, operations, records, m5.4]
---

# An operation no record can be written for leaves the window before it is written

## Context

Every control on this panel emits an `Operation`, `karakuri_operation_record::written` turns it into
a `Record`, and `crates/karakuri/src/main.rs` moves the deck with it. That is P-0090, and
`App::performed` is the one place it happens.

**Two operations cannot take that road, and they are both this bay's.**
`written(TapBeat)` and `written(ScaleGrid { .. })` answer `Written::Owed(Owed::NotSettled)`. They are
not *silent*: a tap **does** end in a `Record::Tempo`, and `karakuri_environment::audio` writes it.
What it cannot do is come out of `written`, which is a pure function of the operation and a
`Current` — a tap's record is the *beat lock's* answer, made of a tapped tempo, a phase error against
the oscillator and the output lag, and no reading a `Current` carries can produce it.
`Owed` is a question rather than an error
([ADR-0194](0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)), and
`unwritten` prints it out loud so a press that owes a record nobody has designed does not read as a
press that did nothing.

**While the keyboard was the only route, that cost nothing.** `b`, `,` and `.` did not emit at all:
each arm called `tapped` or `scaled` itself and returned before `App::performed`, with the reason
written at `tapped` — *"routing this key through `App::performed` would print 'nothing moved, and
nothing here decides it' about a press that moved the grid."*

**A pill cannot do that.** M5.4 draws `tap` and `½ ×2`, and a control on this console is a rectangle,
a claim and an **operation**: `input::claim` claims the press, `Readout::pointer` asks the derivation
what it wants, and what comes back is an `Acted::Emitted`. `Readout` holds no device and no clock
(ADR-0156), so it cannot perform a tap where it decides one; and if it emits, the press arrives at
`App::performed` and meets the line the key arms were written to avoid.

## Decision

**`App::performed` takes the two out before anything else in its `Emitted` arm, and the three key
arms join them.**

`tracked(gfx, started, operation)` answers `Some(line)` for `TapBeat` and for `ScaleGrid`, performing
each against the session this program opened, and `None` for everything else. The `Emitted` arm calls
it first and **returns** when it answers — before the arrangement, before `attached` and `nudged`,
and before `written` and `unwritten` are reached at all.

**And `b`, `,` and `.` now emit like every other key on this panel.** Each builds its operation, hands
it to `App::performed`, and the same `tracked` performs it. **The pill and the key are one route**,
which is what M5's opening asks of every operation: *each operation is named once, and every surface
routes into that name.*

**`App::performed` takes `started: Instant`.** A tap is an instant measured from the run's origin —
`Audio::tap(signals, at, since_start)` — and `App::started` is that origin. Fourteen call sites
carry it, and `Instant::now()` is read inside `tracked` rather than passed in, because the last place
the instant a tap means is still true is where the tap is performed.

**The gap it names is not closed here.** The day a `Current` can carry a correction, `written` answers
for both operations, `tracked` goes, and the two leave `performed` by the door every other control
leaves by. That is a decision about `karakuri-operation-record`, and this record does not take it.

## Alternatives rejected

**Close the `karakuri-operation-record` gap first**, so both operations answer `Records` and no
special case is needed. It is the right end state and it is a decision nobody has taken: it means
saying what a `Current` carries about a beat lock — a tapped tempo, a phase error, an output lag —
and that is the vocabulary's own design rather than this bay's. Two controls should not be held open
behind it, and the roadmap already says so: *"That is a gap in `karakuri-operation-record` and not in
the panel, and it does not stop either control."*

**Let the two fall through to `unwritten`.** One line of code, and it prints *"nothing moved, and
nothing here decides it"* immediately after a press that moved the grid an octave. That is worse than
silence: it is the window contradicting itself, on the one control an operator watches to tell a lock
from a coincidence.

**Perform the tap where the press is decided**, in `Readout::pointer`. It has no device, no deck and
no clock, and giving it one is exactly what ADR-0156 is about; `Readout::pointer` being callable
without an event loop is what makes the panel's gestures testable at all.

**Keep the key arms as they were and give the pill its own route.** Two spellings of one operation,
which is what P-0090 exists to stop — and the shape a `has` badge on the panel column would then be
claiming for a route the keyboard does not take.

**A fourth `Acted` variant** — *performed, do not write* — so `performed` could branch on the answer
rather than on the operation. `Acted` says what routing a pointer event *did*
(`Operated`, `Emitted`, `Opened`, `Pointed`), and this is not a fact about the press: it is a fact
about the operation, which is why the branch is a `match` on the operation and why it disappears with
the gap rather than with the control.

## Consequences

- **`tracked` is the only place a tap or an octave is performed in this program**, and both surfaces
  reach it. `tapped` and `scaled` are unchanged, including the sentences they print.
- **`App::performed`'s signature grew a `started: Instant`**, and
  `crates/karakuri/src/main.rs`'s own `the_three_mix_keys_...` scan quotes the call in full, so the
  quoted string moved with it.
- **The `Owed(NotSettled)` pair is asserted from the window's side**, in
  `a_press_on_the_tracker_group_reaches_the_operation_its_key_reaches`: if either operation stops
  being owed, the arm that takes it out of `performed` is describing something that is no longer
  true and the test says so.
  `an_operation_whose_record_is_owed_is_said_rather_than_swallowed` already named `TapBeat` as the
  operation the third answer exists for, and it still does.
- **`Record::Tempo` still has no arm in `apply`.** Nothing changed about that: the tap's record is
  written and applied inside `karakuri_environment::audio`, and the record `written` builds for
  `SetFreeRunTempo` is still routed nowhere — which is *Set the free-run tempo*'s own item and is
  reported against M5.4 rather than closed here.
