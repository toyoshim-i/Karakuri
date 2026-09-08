---
id: 0289
title: The rec pill is a record/stop toggle, and each start takes a fresh stamp
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0090, 0091, 0092, 0094]
tags: [console, ui, operations, transport, session, store]
---

# The rec pill is a record/stop toggle, and each start takes a fresh stamp

## Context

*Record the session* is one row on [every operation](../manual/operations.html) and one capsule at
the end of the transport row in [console.html](../manual/console.html) — `● rec`, drawn `.pill.on`,
tipped *"Recording this session to the store as it happens. Click to stop."*

Before this record the pill was one of the items `view::transport` listed as deliberately not drawn:
*"there is no session recorder behind this panel and no record stream is written from it."*
`karakuri-cli` has had one since `--record-session` landed; `crates/karakuri` had none.

**The roadmap's reading was that only half the control could be built.** Stopping was refused by
nothing; starting was refused *by a mechanism*, because
`karakuri_environment::session::Recorder::open` takes a **head** — a Set file's records, which are
what make the stream self-contained — and `karakuri_store::project`'s `key_for` says the projection
that would fold a session down to the deck state it ends at does not exist. So a head could only be
written *before the first frame*, from the material the run began on, which is the only instant at
which *what this run started with* and *what this deck is in* are the same statement.

**That reading was one step short.** A head is a Set file, and a Set file is *what is playing and at
what values* — which is exactly what `Keeping::save_set` already writes from the **live deck**,
through `Playing::at` and `playing_values`, at any frame. `karakuri-cli` builds its head from the
launch arguments because that is all it has at the instant it opens a recorder; this program has a
running deck and a keep path over it. So a head at an arbitrary frame is producible from material
that already exists, and the mechanism refuses nothing.

What is genuinely true is narrower, and it is a *statement about the replay* rather than a refusal:
**a Set file carries no running state.** It names the material and its parameter values and nothing
about how far anything has got — `console.html` says of one shipped deck that its material
*accumulates* — so a replay from a mid-performance head restarts that material from the top rather
than continuing the picture that was on screen. A recording begun mid-performance is a *correct*
session of everything from the press onward, played from a deck that begins again.

**The third fact is a live hazard and it does bite.** `Store::append_session` appends, and
`session::split` sets `started` at the first tick and never clears it — so a second head written
under an id that already has a stream lands in the middle of it and is read back as edits.

## Decision

**One capsule, two ends: a press starts a recording and a press stops the one running.** The pill
draws which end the next press is before it is made — the mock's plain `.pill` while nothing is
running, its `.pill.on` while something is — and
`karakuri_console::view::TransportRow::record` reads that same value to say what the press asks
for. One reading, so the capsule an operator is looking at and the operation the press names cannot
come apart.

**Each start takes a fresh `history::stamped_id()`.** The capsule types no name — this console's one
letter-taking flow is bounded to an arrangement's name
([ADR-0221](0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md),
[ADR-0287](0287-the-keep-pill-files-under-a-stamp-because-the-consoles-one-letter-taking-flow-is-an-arrangements-name.md))
— so `Recording::Start` carries `id: None` and the program stamps it. That is the same convention a
keep with no typed name already uses, and it is also what makes the append hazard **unreachable
rather than unlikely**: no press can name an id that already has a stream.

**Neither end happens on the frame, and the flush is a thread.** A start writes a Set file, reads it
back, creates `sessions/<id>.ndjson` and spawns a writer; a stop hands the last batch over and
*joins* that writer — and so does `Drop`, so a recorder merely let go of on the frame path stalls
the frame exactly as `finish` would. `karakuri-cli` finishes in `exiting`, where a stall is free; a
press is not that place ([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md),
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)). So both
ends go to a thread of their own and the outcome is said at the frame it arrives, which is
`Keeping::save_set`'s arrangement exactly: gather on the press, work on a thread, report afterwards.

**The pill says `Running` only while a recorder exists.** A start still opening reads `rec`, because
nothing is being recorded yet; a stop still flushing reads `rec`, because the stream stopped taking
records the instant the recorder left. There is no third state on the capsule: a *starting* wash
would be the panel drawing a promise instead of a fact, and P-0094 is about exactly that gap.

**A second press while a thread is out is refused and says so**, rather than opening a second
recorder or queueing. Two recorders would be two writers over one deck, and a queued press is a
gesture whose effect lands after the operator has stopped looking at it. A control that says no is
better than one that acts later, which is this row's own rule read one capsule along.

**The replay caveat is printed at the start.** The line the press writes says the head is the deck's
material as it stands, and that a replay begins that material from the top rather than continuing
the picture on screen. It is said rather than designed around: the maintainer chose the toggle
knowing it, and a program that knew this and did not say it would be the shape of quiet wrongness
this repository is arranged against.

## What it costs

- **A recording begun mid-performance does not replay the picture that was on screen.** It replays
  the same material, from the top, with every edit from the press onward at the right frame. For a
  closed-form renderer the two are the same; for an accumulating one they are not, and no amount of
  care here closes that — what would is the projection `key_for` says does not exist.
- **The head is slot 0's, and the other slots do not replay.** `karakuri-cli`'s `session_head` says
  why — *"a session stream cannot say what a deck held"* — and this takes the same slot for the same
  reason. A head written from whichever deck happened to be selected would make *which slot replays*
  depend on where a hand was.
- **An operator cannot name a recording.** Every session started from this panel is a timestamp,
  which is ADR-0287's cost read again one control along, and here it is also load-bearing rather
  than merely inconvenient.
- **Each start leaves a `<stamp>-material` Set in the library.** It is the head, filed
  `Asked::Operator` where an operator will look for it, which is `karakuri-cli`'s own choice — and
  it is also a row in the Library bay that nobody asked to keep.
- **The stream is this program's frames rather than a measured performance.** `STEPS_A_FRAME` says
  this window advances one step per frame drawn and reads no clock, so the ticks it writes are
  `steps: 1` and a replay steps where this run stepped. That is P-0092 satisfied and it is *not*
  `karakuri-cli`'s wall-clock timeline; a run whose frame rate sagged records the sag as fewer
  ticks rather than as a longer frame.
- **A frame between the press and the recorder opening is not in the stream.** The thread takes as
  long as two file writes take. The head describes the deck as of the press, so the gap is frames
  whose motion is missing rather than frames that replay wrongly — and the pill does not read
  `.pill.on` until the writer exists, so nothing claims otherwise.

## Alternatives rejected

**Build the stop and leave the start to a flag, which is what the roadmap proposed.** It reads as
the conservative choice and it is not: it leaves the panel with a control that can only ever be
pressed once per run, on a run launched with a flag this program does not have — so the pill would
be inert on every run anybody could start from it, which is the drawn-and-inert control this
repository has shipped once already.

**Reuse one id across starts, or let a start name the id a stop just closed.** The append hazard,
taken on purpose. `session::split` would read the second head as edits, and the file would look
complete.

**Flush on the frame and accept the stall.** `Recorder::finish` blocks on a writer thread that is
behind by up to two batches of 256 records; the honest worst case is a disk hiccup, which is the
case the bounded channel exists for. P-0094 does not have a size threshold.

**Give the pill a third state while a thread is out.** It would draw *what the operator asked for*
rather than *what is happening*, on a control whose whole job is to say which of those two the next
press changes. The refusal sentence is where a press that cannot land is answered.

**Put the recorder on `Gfx`, beside the audio input.** The input is there because a window remade is
a display remade and the input is re-opened with it. A recording is not the window's — it outlives a
surface being rebuilt, and a session that ended because a window was resized would be a file that
stops for a reason nothing in it records.

**Read the head off the launch arguments, as `karakuri-cli` does.** It is what that program has and
this one does not need: every number a Set file carries can have moved since the run started, so a
head written from the command line would describe a deck nobody is looking at. `playing_values`
already says this about a keep, and a session's head is a keep with a different name.

## Consequences

- `karakuri-console/src/view.rs` carries `Rec`, `Transport::rec`, `TransportRow::rec`,
  `TransportRow::on_rec`, `TransportRow::record`, `REC_LABEL` and `rec_into`. The pill takes the
  row's right padding and the health capsule and the frame readout are laid out backwards from it —
  which is why it is inside `transport_row` where the arrangement pill is beside it: there is one
  derivation of that end of the row, not two.
- **The mark is drawn rather than typed.** The mock's `&#9679;` is a glyph nobody here chose a font
  for, so it is a circle — `Mask`'s rule, and its diameter and the gap after it are the Outputs
  sink's `SINK_DOT` and `SINK_GAP`, because that is this console's other round mark before a word
  inside a capsule.
- **`Transport::rec` is `Option`, and `None` is *nobody said*.** It is `View::audio`'s distinction
  one group along: `Some(Rec::Idle)` is a program with a store that is not recording, `None` is a
  console that was never told and draws no pill. Every test in `karakuri-console` that does not say
  otherwise leaves it `None`, which is what keeps the row those tests describe the row it was.
- `crates/karakuri/src/main.rs` carries `Sessions`, `Stream`, `Ended`, `began`, `HEAD_SLOT` and
  `Keeping::head_material`; `App::recording` holds one, the `MouseInput` arm performs the press
  beside the keep's save, and `CloseRequested` waits for the flush where a stall is free.
- **The stream is written as well as opened**, or the pill would be a lie: `App::performed` pushes
  every record it applies, `measure_audio` pushes the frame's measurement and its tempo correction,
  and the frame's `tick` is pushed after `compose` — last, because *"a tick is a terminator rather
  than a header"*.
- **The console emits `Operation::RecordSession` and the row's panel badge is what has to follow**
  (`docs/contributing.md` §5 step 2): `karakuri-console/tests/panel_column.rs` fails naming this row
  until `panel <b>rec</b>` reads `has`.
- **The mock's tip is owed a sentence** (§5 step 3): it names one gesture, *click to stop*, and the
  control is both — and it says nothing about the replay starting the material again.
- **`karakuri_operation::Recording`'s two doc comments are now partly wrong.** `Start`'s says a head
  can only be written before the first frame and that a recording begun mid-performance would be
  *"not a partial session but a wrong one"*; `Stop`'s says *"nothing does this today"*. Both were
  written when nothing called either.
- **`op-when` for this row still reads `launch`**, whose legend is *"the only way in is a flag,
  before the run"*. It was true of a program whose only way in was `--record-session`; a press
  reaches it now, and what the press does is gathered on the press and written on a thread — which
  is `on a worker`, the word *Keep what a deck is playing* already carries for exactly that shape.
  It also contradicted the vocabulary before this record: `gate::standing` classes this
  `Closed(Class::InputsAndOutputs)`, a class an operator opens *during a run* at the Outputs row,
  and `console.html`'s pill there names *"starting or stopping a recording"* among the three things
  it refuses. An operation reachable only before the run has nothing for a runtime class to refuse.
