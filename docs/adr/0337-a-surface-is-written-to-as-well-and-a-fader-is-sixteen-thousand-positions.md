---
id: 0337
title: A surface is written to as well, and a fader is 16384 positions
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0090, 0092, 0094, 0083, 0096]
tags: [midi, devices, engine]
---

# A surface is written to as well, and a fader is 16384 positions

## Context

[ADR-0335](0335-the-panel-opens-the-first-surface-there-is-and-the-map-is-two-tiers-under-the-store.md)
gave the panel a port and a map, and
[ADR-0336](0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md) gave it a way
to change the map while it runs. Both are the wire coming *in*. `docs/roadmap.md`'s M5.12 named
the two things left, and named them together because they are the same half of the problem —
**the surface itself rather than a route into it**:

> MIDI *out*, so a surface's LEDs and motorised faders follow the deck, and that matters the
> moment two things can move a fader; and 14-bit control changes, so a fader is more than 128
> positions.

Both are M2's, deferred twice. And the sentence that makes them one record is in
`karakuri_environment::midi::Router::emit`, which has been carrying the reason since the
coalescing landed:

> **Coalesced, not filtered for change.** A repeat is dropped within a frame and never across
> two … The console holds the value it last drew; this holds nothing between frames and could
> not, **because MIDI out is not built**: a transition can move the mask front under a hand
> that is not moving, so a fader re-asserting the position it last sent is asking for something
> the deck may no longer be at.

A surface that cannot be written to is a surface whose faders are always a guess about where
the deck is. Everything else on this console is a readout as well as a control — a strip's
fader draws where the gain is, and a residency pill lights — and the one surface that was
write-only was the operator's own hands.

**Three things had to be decided.**

## 1. What a 14-bit line says, and what a half of one means on its own

**Decision: the grammar gains `cc14 <msb> <lsb> -> <control>`, and both numbers are written
out.**

The MIDI convention pairs controller `n` with controller `n + 32`, and the obvious spelling
is `cc14 <n>` with the second number implied. It is refused: the convention is a convention,
controllers exist that do not honour it, and a line that names both numbers is a line an
operator checks against their device's manual rather than against a piece of folklore they
have to know. It also makes every refusal able to **name both numbers**
([P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)) — `cc14 7 7` is
one controller twice, `cc14 7 200` is past what the wire carries, and `cc14 7` is told what to
write instead.

**The value is `(msb << 7) | lsb` over 16383**, scaled onto the target's range by exactly the
function a 7-bit line's value is — one `scale`, one span, so a `cc14` line and a `cc` line
cannot come to disagree about what a range means. Both ends stay exact, which is the property
[`Map`'s own documentation](../../crates/karakuri-midi/src/map.rs) has always held faders to.

### The lone MSB is the 7-bit reading, and the LSB refines it

**Decision: an MSB alone moves the control coarsely — at `msb / 127`, the exact reading
`cc <msb>` would give — and the LSB that follows re-states the control at
`((msb << 7) | lsb) / 16383`. A lone LSB with no MSB held moves nothing.**

The two halves are two messages and a frame boundary is free to fall between them, so
something has to be decided about the gap. Three readings were on the table and only one of
them never leaves a fader stuck:

- **`(msb << 7)` — the MSB with a zero LSB.** The tidy one, and it is wrong at the top: MSB 127
  alone is `16256 / 16383` = 0.992, so a fader pushed to the top of its travel on a device that
  sends only coarse halves **never reaches unity**. A gain that cannot reach 1.0 cannot be
  matched against another slot, which is the same argument `GAIN_RANGE` is chosen by.
- **Hold the MSB and send nothing until its LSB arrives.** The alternative, and it is
  **recorded rather than taken**. It needs a deadline — *how long* — and this route has no
  clock at all by charter (`karakuri-midi`'s *Latency is not compensated here*). Worse, a
  device that sends the coarse half only, which is an ordinary thing for a coarse control on a
  14-bit-capable surface, leaves that fader stuck at its last position for as long as the
  window lasts and then jumps.
- **The MSB is the 7-bit value; the LSB refines it.** Taken. Both ends are exact whether or not
  the LSB arrives, a coarse-only surface is a 128-position fader on the same line, and the
  refinement moves the control by less than one 7-bit step — so what an operator sees is a
  fader that lands and then settles, never one that waits. Within a frame the two coalesce
  anyway (ADR-0207), so the coarse value usually never reaches the deck at all.

**Nothing waits and nothing is timed**, which is the whole of the claim: every message is acted
on as it arrives, so no pair that did not finish can leave a control between two values.

### The state is the caller's

`karakuri_midi::Map` is *a pure function of one message*, which is that module's whole test
story, and the MSB last seen has to live between two messages. So the map answers
[`Map::wide`] — which half arrived, and which pair it belongs to — and
`Map::operation_wide`/`Map::parameter_wide` take the assembled value;
`karakuri_environment::midi::Router` holds the halves, beside the frame's coalescing it already
holds. The alternative was `&mut self` on `Map::operation`, which would have made the table
stateful for a fact about the wire.

**A learn writes a `cc` line and never a `cc14`, and that is owed.** Inside one drain, two
faders whose controller numbers are 32 apart are indistinguishable from one pair, and a learn
that guessed wrong would bind two faders to one control with nothing said — which is the one
thing ADR-0336 says a learn must not do. What an operator gets instead is the coarse half
working immediately and one word to change by hand, which the manual says.

## 2. Which way MIDI out is driven, and what it sends

**Decision: the map read the other way, compared against what was last sent, on the frame the
change lands.**

`Map::echoes` is every mapped control as an [`Echo`]: what to read (`Echo::control`, an address
and not a value), and how to say it (`Echo::position`, then `Echo::wire`). The router reads
each one through a new `Feedback` trait, compares the *position* — the number the wire carries
— with what that control was last shown at, and writes bytes only for the ones that moved.

Two alternatives, and both were live:

- **Poll the surface for feedback.** Ask the controller where its faders are and reconcile.
  Refused: it is the surface deciding, which is [P-0090](../principles/0090-a-surface-offers-it-never-decides.md)
  inverted, and most controllers cannot answer at all. What a surface knows is what it was last
  sent.
- **Send everything on every frame.** Simpler by a field, and it fills a device's queue with the
  answer it already had — a 30-line map at 60 Hz is 1800 messages a second down a wire that
  carries about 1000. The comparison is what makes the sentence *on the frame the change lands*
  true rather than aspirational.

**What is sent**: a `cc` per mapped continuous control at its 7-bit or 14-bit position, and a
note per mapped pad — velocity 127 where the deck is in the state that pad names, velocity 0
where it is not, which is [`Message`]'s own reading of a release, so a surface that echoes what
it is sent stays consistent with what this crate would read back from it. **What is not sent**:
a control no map line names, because the map is the list and MIDI out cannot reach further than
MIDI in does; `tap`, because a beat has no state; and a control this program cannot read, which
is skipped rather than darkened — a pad going dark because a value could not be read would be
the surface asserting something about the deck.

**Nothing writes a record.** No `Operation` is produced and no `Record` is written: what changes
is the wire. [P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md) is about the
stream, and a session recorded from a controller still replays with neither controller nor map
attached.

**Every source is shown and not only the surface's own**, which is the point of the row: a key
press, a model over `--mcp`, a pointer on a strip and a transition all move a fader.

## 3. The send does not block, and the port is paired by name

**Decision: a bounded channel to a thread that owns the connection, dropping and counting when
it is full — [ADR-0067](0067-the-session-writer-never-blocks-never-grows-and-never-silently-drops.md)'s
shape.**

`MidiOutputConnection::send` is a call into a driver whose timing this process does not own, and
**nothing on the frame path waits**
([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
So the frame does not make that call: `Out::send` is a `try_send` into a bounded queue and a
thread does the writing.

**Bounded rather than unbounded, which is the opposite of the input side and deliberately so.**
`Port`'s queue is unbounded because *the last value of a fader is the fader's position and a
dropped one leaves it somewhere the operator is not holding it*. Out is the other way round: the
queue is a picture of one moment, so a backlog is a fader travelling through positions it is
already past. A drop costs one stale control until the next change moves it, and the caller
re-states a control whenever the deck changes it.

**The drop is counted and said once a run**, because a surface that quietly stopped following
the deck is the failure nobody notices — the same reason ADR-0067 counts, and the same *say it
once per control* rule the router's notices already keep.

**The output port is the input's, matched by name** — the whole name first, then its first
word, because a device's two ports carry the manufacturer's name and usually differ past it
("nanoKONTROL2 SLIDER/KNOB" in, "nanoKONTROL2 CTRL" out). **A surface with no output port is a
state and not a fault**: it is what every run before this was, the run goes on unchanged, and
nothing is said. `Surface::open` names the output in its legend where there is one, on the same
terms it names the input.

**Both programs.** `karakuri-cli` opens a surface and drains it in `Live::run_surface`, and the
panel in `App::mapped`; each gained the send beside its drain. A map file means one thing in
both programs or it means nothing, which is ADR-0335's own sentence about this column.

## Alternatives rejected

**Poll the surface for feedback.** §2. The surface deciding, and most controllers cannot answer.

**Send every mapped control on every frame.** §2. It fills the wire with what the device already
knows.

**A 14-bit control spelled as two `cc` lines.** The grammar already has `cc`, so a pair could be
two lines and a rule that says *a line for `n` and a line for `n + 32` on one control is a
pair*. Refused three ways: the map's own key is `(channel, controller)` and two lines reaching
one control is already a legal thing that means *two knobs on one fader*, so this would make one
spelling mean two things depending on arithmetic; a readout would have to say which of the two
lines a control is on; and a learn writing one of the two halves would silently turn a working
7-bit binding into half of a pair. One line names one control, which is what every other line in
this grammar does.

**`cc14 <n>` with the LSB implied at `n + 32`.** §1. The convention is not a rule, and a refusal
could then only name half of what it was refusing.

**Hold the MSB for a window and send the pair or nothing.** §1, recorded there. It needs a clock
this route does not have, and it strands a coarse-only device.

**`Map::operation` takes `&mut self` and holds the halves.** §1. It makes the table stateful for
a fact about the wire, and costs `karakuri-midi` the charter its tests are built on.

**MIDI out writes a record.** Refused where every version of this is: which knob is which is a
property of the room's hardware (ADR-0007, P-0092). It is worth stating because the *shape*
invites it — the out side reads the deck every frame, which looks like a place to record from —
and what would be recorded is the room's wiring rather than the performance.

## Consequences

- **`karakuri-midi` gains `Out`, `Control`, `Echo`, `Shown`, `Wide`, `Half`, `Map::wide`,
  `Map::operation_wide`, `Map::parameter_wide` and `Map::echoes`**, and `Map`'s entries gained
  a private `Entry` with the LSB half beside the target. **A pair is one mapping and not two**:
  `Map::len`, `Map::lines` and `Map::bound` all talk about the line an operator wrote.
- **`Map::operation` and `Map::parameter` kept their signatures** and are now two entry points
  onto one `operating`/`parametered` pair, which is what keeps a line meaning the same thing at
  7 bits and at 14.
- **A controller cannot be a plain line and half of a pair, and the file says so on load.** The
  plain line wins — it is the whole of what it says — and the pair keeps its coarse half. Which
  of the two the file meant is not something the order of the lines should decide.
- **`karakuri_environment::midi` gains `Feedback`, `Lit`, `Router::shown`, `Router::paired`,
  `Router::relit`, `Surface::show`, `Surface::out_name`, `paired_out` and `Paired`**, and
  `Router` gained `halves`, `echoes` and `lit`. `Interface` is untouched: the way in resolves a
  position and the way out reads a value, and they are answered by different things — `Lit`
  needs the room's look, because `exposure` is a map line and is not a deck's.
- **A learn rebuilds the echo table**, because a control just bound has never been shown and the
  control it took over may have been lit on another knob a moment ago.
- **`Router::emit`'s note about MIDI out is now stale in its reason and right in its rule.** It
  says a fader cannot re-assert a position because nothing shows it where the deck is; that is
  no longer true, and the rule — *coalesced within a frame, never filtered across two* — is
  unchanged, because a transition still moves a control under a hand that is not moving.
  Filtering across frames is a separate decision and is not taken here.
- **`docs/manual/console.html`'s `map` pill tip is untouched.** It says which file is loaded and
  where map files live; it does not state the grammar's line kinds, so the grammar growing does
  not make it wrong. [ADR-0330](0330-the-console-paints-its-own-hover-layer-and-the-tips-are-the-manuals-own-words.md)'s
  rule is that the console quotes the page, and there is nothing here to quote that the page did
  not already say.
- **No badge moves and no row changes.** MIDI out is not an operation and 14 bits is a
  resolution: [every operation](../manual/operations.html)'s MIDI column measures what a map
  line can *name*, and both halves of this record are about the same lines saying the same
  things.
- **M5.12 is closed and this is the phase that followed it**, named in its stub rather than
  rescheduled — the roadmap's own sentence about what M5.12 left owed.
- **14-bit learn is owed and is written down as owed**, in the manual and in §1. What would
  close it is a signal that distinguishes a pair from two faders, which one drain does not
  carry.
