---
id: 0207
title: A continuous control says one thing per frame
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0091]
tags: [midi, surfaces, frame-path, records]
---

# A continuous control says one thing per frame

## Context

**A mapped fader breaks the first rule this repository has.**
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) says
the frame path allocates no heap memory, and it says it with no clause for a small allocation.

The path is short and every step of it is written down where it happens. `Live::run_surface` is
called from `Live::frame`. `Router::route` emits one `Operation` per MIDI message, and nothing
between the port and the recorder drops a repeat — `Port::drain` moves whatever arrived, and a knob
held against its stop keeps sending, so what bounds a frame's traffic is the **message count and
not the 7-bit value range**. `crate::midi` puts the number on it: *"A fader sweep is several hundred
messages, this runs inside `Live::frame`"*, which is why `INBOX` reserves 256.

**Two of the four continuous targets allocate, and two do not.** `gain` and `opacity` become
`Record::Gain` and `Record::Opacity`, which are scalars and touch no heap. `exposure` becomes
`Record::Look` carrying `look.tonemap.name().to_string()` and `mask-position` becomes `Record::Mask`
carrying `mask.kind.name().to_string()` — each a `String` copied from a `&'static str` of at most
eight bytes (`reinhard` is the longest tone map name; the wipe kinds are shorter). `Live::record`
clones the record for the recorder, so with `--record-session` attached it is **two** such
allocations per message rather than one.

Neither end of that is where the allocation is. `karakuri_midi::map` says of its own routing that
nothing there allocates — every operation a map line can name carries scalars only — and
`session::Recorder::push` moves a `Record` into a `Vec` that already has room. What allocates is the
record in the middle, and it was written when the only way to reach it was a key press.
`crate::mix` has carried the measurement and the open question since the map arrived on the frame
path, ending *"nobody has decided whether a byte-sized allocation per message is inside that rule or
a hole in it"*. This record decides it.

## Decision

**Coalesce continuous controls per frame, in `karakuri-cli`'s router.** The last value a control
sent within a frame is the one that becomes an operation; the earlier ones are dropped. A sweep of
several hundred messages becomes one operation, so it builds one record — or two — rather than
several hundred, and the rule is met by removing the cause rather than by widening the rule.

### Where it goes, and why not a step earlier

`Router::route` in [`crates/karakuri-cli/src/midi.rs`](../../crates/karakuri-environment/src/midi.rs). It
already holds the per-run state a frame's worth of accounting belongs beside, and it already has the
habit: **everything it says is said once per control** rather than once per message, through
`seen_unmapped` and `seen_no_slot`, for this exact reason — *"a line per message is a blocking I/O
storm on the render thread, which is the first rule this repository has."* Coalescing is that same
sentence about the operations instead of the notices.

**Not in `karakuri-midi`.** `Map::operation` is *"a pure function of one message"*, said in its own
module documentation and in the crate's, and
[ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md) and
[ADR-0202](0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md) both rest on it —
it is why `cc -> exposure` names an exposure rather than a whole look, and why a shape target has no
spelling. A map that remembered what the last message was would be a map that reads something back,
and the crate's whole test story is `parse` then `operation` with no world to set up.

What the map does gain is one more pure function of one message: `Map::is_continuous`, which is
`Target::continuous` — the predicate `parse_line` already refuses a `note` on a fader's target with,
and the reason a `cc` cannot put a deck on air. The alternative was a list of continuous operations
in the router, which is a second answer to the same question, kept in step by hand against a
forty-eight-variant vocabulary.

### Pads do not coalesce

**Two presses in one frame are two operations that both mean something.** `residency 0 live` then
`residency 0 allocated` is not the second one alone in intent, and a note is a press rather than a
position. `Target::continuous()` already draws exactly this line — it is why the grammar refuses
`note -> gain 0` and `cc -> tap` — so the router asks it rather than keeping a second list.

### Per frame, not a filter on change

The console's fader is the other precedent and it is deliberately **not** what this does: it emits
only when the *value* changed, because *"a pointer dragged on past the end of a track asks for the
end sixty times a second and the value is already there."* That works because the console holds the
value it last drew. This router holds nothing between frames and could not: **MIDI out is not
built**, so nothing tells a surface that a transition moved the mask front under a hand that is not
moving. A fader re-asserting the position it last sent is therefore asking for somewhere the deck
may no longer be, and a change filter would swallow it. The state kept is one frame wide and is
cleared at the top of every `route`.

### The key is what moves, not which knob moved it

A control is keyed by the operation's variant and the deck it names — `std::mem::discriminant` and
the `deck_of` this module already had — rather than by the `(channel, controller)` on the wire. Two
lines can put two knobs on one `gain 0`, and a line naming no channel puts one knob on sixteen
channels; what reaches the deck is one value either way. It also bounds the accounting: every key
comes from a target and a target comes from a map entry, so a `Vec` with `map.len()` reserved can
never grow, and the fix does not itself allocate on the frame path.

## Alternatives rejected

- **Stop the two records carrying a `String`.** `Record::Look` would name its tone map operator and
  `Record::Mask` its wipe kind as an enum, and the allocation disappears at the source. It puts the
  engine's list of names into `karakuri-store`, whose own rule is that **what a name is allowed to
  be is the engine's to say** — the reason `Record::Blend` carries its mode as a `String` in the
  first place. It is also a format change to fix a frame-path cost, which is the wrong document to
  pay in.
- **Give P-0001 a clause for a small allocation.** One paragraph, and every number in `mix.rs` is
  already inside any threshold anyone would write. But there is no measured threshold to write:
  `contributing.md` says every performance number here is host-side and biased high, and the figures
  in `mix.rs` are read off the code rather than measured on a surface at all. An exception in the
  **first** rule, with no measurement behind it, is an exception the next caller cites.
- **Coalesce in `karakuri-midi`.** The state would sit next to the table it needs, and `Map` is
  where a control's identity is already known. It ends `Map::operation` being a pure function of one
  message, which two records rest on and which is the whole reason this crate can be tested against
  bytes.
- **Filter on change instead.** Strictly fewer records, and it subsumes the per-frame case. It is
  wrong until MIDI out exists: see above.
- **Drop the earlier operation and push the later one.** The simpler edit, and it moves a swept
  fader behind every pad hit that happened during the sweep. A frame's operations are applied in the
  order they are given, so the fader keeps the position it first spoke in and carries the value it
  ended at.

## Consequences

- **The record stream is shorter for a swept fader, and this is a real loss to state plainly.** A
  sweep that wrote several hundred `look` or `gain` records now writes one per frame it moved in.
  What is *not* lost is anything reconstructible: `Live` applies each operation into the deck and
  the frame draws what the deck holds afterwards, and a replay applies every record between two
  `tick`s **before** drawing the frame they close — `session::Frame::before`, *"applied before the
  frame renders."* A value overwritten within a frame was never on screen on either side of the
  recording. What a session loses is sub-frame detail nothing could ever have replayed, and what it
  keeps is every frame the fader was in a different place.
- **A session recorded through a surface is no longer a different size from one recorded through
  the console**, which emits per change rather than per message. The two were previously off by the
  device's message rate.
- **P-0001 holds on the MIDI path without a clause**, and `mix.rs`'s open question is closed by this
  rather than by a threshold. The measured figures stay on the page: what a sweep *would* allocate
  is why the coalescer is there, and a reader who removes it should meet the number.
- **`Map::is_continuous` is public API on `karakuri-midi` with one caller.** It is a pure function of
  one message like `Map::operation`, and it exists so that the fader/pad line is stated once. A
  second caller is the console's map-learning UI, whenever that lands.
- **Nothing changes for `gain` and `opacity` except the record count.** They allocated nothing before
  and allocate nothing now; they are coalesced because a control is a control, not because they cost
  anything.
