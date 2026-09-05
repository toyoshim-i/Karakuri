---
id: 0194
title: Where an operation becomes a record is a crate that depends on both, and the reading is a value handed in
status: accepted
date: 2026-08-27
supersedes: []
superseded_by: []
principles: [0090]
tags: [vocabulary, architecture, records]
---

# Where an operation becomes a record is a crate that depends on both, and the reading is a value handed in

## Context

[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) is *every control ends in the
same record*. What makes a console fader the same thing as a key press and a mapped MIDI knob is
that all three write the same `Record` and the deck is moved by the decode — so an `Operation` has
to become a `Record` somewhere, and **there was nowhere**.
[ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md) said so at
length and had the console's example build three records by hand with a note that *"the day the
conversion lands this function is deleted rather than moved"*. The roadmap has carried it as one of
the decisions nobody has taken since.

It is forced now:
[ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md) split
`SetLook` into `SetTonemap` and `SetExposure` precisely so `karakuri-midi`'s `cc -> exposure` could
become an operation, and the migration cannot happen while there is nowhere for that operation to
become a record.

**What was checked first, because it turned out not to be true.**
[ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md) says the conversions
live as *"one `From` impl per list in `karakuri-cli`, in the one place every control already ends"*.
`karakuri-cli` did not depend on `karakuri-operation` at all, and the only conversion in the
workspace was in `karakuri-console/examples/panel.rs`, whose own documentation says it is in the
wrong place.

### The survey, which is the decision

Every one of the 46 operations was read against the record vocabulary and sorted three ways. The
shape of the middle group is the whole question:

- **Six write a record that is a pure function of the operation** — `SetGain`, `SetOpacity`,
  `SetBlendMode`, `SetResidency`, `SetPreview`, `SetFreeRunTempo`.
- **Ten need a reading of what is current**, and they need four different things: the look that is
  running (`SetTonemap`, `SetExposure`); the transport of the deck they name (`ScrubDeck`); the
  grid's position quantised onto a musical instant, together with the quantum and the length
  (`FadeDeck`, `Crossfade`, `Wipe`, `SelectRenderer`); the beat tracker itself, which may refuse
  (`TapBeat`, `ScaleGrid`); and a session tempo run through the engine's anchor clamp (`SetSync`).
- **Thirty write no record**, for four different reasons: a surface's own state (`SelectDeck`,
  `SetTransition`, the folds, the window), a question rather than a change (`ListSets`, `ReadSet`,
  `ReadProcedure`, `SwapOutcome`), a record written where the work *lands* rather than where it was
  asked for (`SaveSet`, `WriteProcedure`), and no record in the session vocabulary at all.

**Two findings inside that last group are worth naming.** Four operations —`WriteParam`,
`AttachSignal`, `WireInput`, `SetProperty` — do have a record, and it is a **Set file's**:
`Record::Param`, `Record::Bind`, `Record::Edge`, `Record::Capacity`, `Record::Seed`,
`Record::Camera`. Every one of them has **no `slot`**, because a Set does not know what fader it is
under, and every one of those operations names a deck. So they are not a conversion waiting to be
written; they are a gap in the *session* vocabulary. `SetCompositing` is the same shape —
`Record::Merge` is what a Set says about its own layering, and *"nothing in a stream says that the
Set in slot 3 composites."* And `Crossfade` is four records and `Wipe` is five, so **one control is
not one record**; P-0090 is about where they end, not how many there are.

## Decision

**A new crate, [`karakuri-operation-record`](../../crates/karakuri-operation-record), depending on
`karakuri-operation` and `karakuri-store` and on nothing else.** One entry point:

```rust
pub fn written(operation: &Operation, current: &Current) -> Written
```

### The reading is a plain struct of values, and every field is optional

`Current` carries the look that is running and the transport of the deck the operation names. It is
**not a trait the caller implements**, and the survey is the reason rather than taste: part of what
these records need *is not readable from anything*. The quantum, the length of a fade and the wipe
shape are `Operation::SetTransition`'s, and that operation **writes no record at all** — it is a
surface's own setting deciding what the *next* move means, and it lives today in `karakuri-cli`'s
`Live` and nowhere else. A trait over "the deck" has nothing to ask for them. A value handed in can
carry them the day somebody decides whose they are.

**`Current::default()` means *I read nothing*, and an operation that needed a reading it did not get
answers `Owed::NotRead` rather than a record.** This is the load-bearing half. A default `Look`
would let every exposure nudge write a tone map operator nobody chose — which is exactly the failure
ADR-0192 rejected `cc 20 -> exposure aces` for: *"nudging exposure silently overwrites a tone map
somebody chose with `t` a moment earlier."* A conversion that can only be wrong in a way an operator
notices on stage does not get a default.

### Three answers, and they are the survey made executable

`Written` is `Records`, `Silent` or `Owed`, over **one exhaustive match on `Operation`** — in
`Record::vocabulary`'s shape and for its reason: an operation added to the vocabulary does not
compile until somebody has said what it writes. `Silent` carries which of the four kinds of nothing
it is; `Owed` carries which of the three kinds of gap. **`Owed` is not an error**: it is this
crate's `Undecided`, and four of its eleven rows *are* the vocabulary's `Undecided`.

`Records` is a list although every conversion built today answers exactly one, because `Crossfade`
and `Wipe` are four and five, which is what `karakuri-cli` already does for them.

### Nine conversions built, and the seven left out are left out for a reason

The six pure ones and the three whose reading is settled — the look pair and `ScrubDeck`. The seven
remaining are `Owed::NotSettled`, and each is a decision rather than work: **scheduling** needs
`karakuri_engine::transition::quantise` and the transition settings no record carries; **moving the
grid** needs the beat tracker rather than a value; **`SetSync`** needs the engine's `clamp_anchor`,
so whether the record carries the anchor that was asked for or the one that was clamped is a
decision about the bytes on disk. This crate cannot reach any of that, and that is deliberate: a
conversion crate that pulled `wgpu` in would be unreachable from every surface again.

### `SetFreeRunTempo` closes a gap P-0028 names

P-0028 says *"`--bpm` exists and the v0.2 vocabulary has no tempo record."* It has one now, and
`Record::Tempo`'s own documentation says what shape a *statement* takes rather than a correction:
*"a correction with `shift` 0.0 and `confidence` 0.0 is a free-running tempo being stated."* The
conversion is written to that sentence. **Nothing routes through it yet** — `--bpm` is a launch flag
and there is no key — so this changes no bytes today.

## Alternatives rejected

- **`karakuri-cli`, which is what ADR-0180 said.** It is the one place every control already ends,
  and it cannot be it: the package has **no library target**, so nothing else in the workspace can
  call into it. That is not an incidental fact — it is the stated reason
  `karakuri-console/examples/panel.rs` wrote three records again instead of calling
  `mix::gain_record`, and putting the conversion there would make ADR-0185's promise — that the
  example's `record()` is *deleted* rather than moved — impossible to keep. **Giving `karakuri-cli`
  a library target** was the repair and is worse: that lib is `wgpu`, `winit`, the engine and a
  GPU, so the console's dev-dependency on it would put a device into a crate whose `src/` has none
  by design ([ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)).
- **`karakuri-store`, with `karakuri-operation` as an optional dependency.** Cheap to write and it
  points the wrong way: the record layer would know the vocabulary, and a replay reading a stream on
  a headless machine has no surfaces and no asks in it at all. `karakuri-store` has no karakuri
  dependency today and neither does `karakuri-engine`; this would be the first, in the crate every
  other one reads. A feature flag makes it worse rather than better — a conversion that exists in
  some builds is one every caller has to ask about.
- **`karakuri-operation`, with `karakuri-store` under it.** Refused by that crate's charter and by
  arithmetic: `karakuri-midi` would pull `serde`, `serde_json` and `sha2` to parse a map file.
- **`From<Operation> for Record`.** Not available and not wanted. Not wanted because the conversion
  is not pure — ADR-0192's whole argument. Not available because it is one operation to *several*
  records for two rows, and `Option`/`Vec` in the impl is a `From` in name only.
- **A trait the caller implements, instead of `Current`.** Rejected above: no trait can be asked for
  the transition settings, because nothing holds them. It would also make the console's example —
  three faders and no engine — implement a readback for operations it never emits, and answering
  those honestly is exactly the `Undecided` shape this repository refuses to guess at.

## Consequences

- **`karakuri-cli` is wired to it, and is the second customer.** `set_gain`, `set_opacity`,
  `cycle_blend`, `show`, `toggle_on_air`, `toggle_priming`, `cycle_tonemap`, `set_exposure` and
  `scrub` now name an `Operation` and hand it to `Live::operate`, which takes the reading and writes
  what comes back through the same `Live::record` everything else goes through. **No key arm moved
  and no record changed**; this is the mix path routing through the vocabulary, not the key handler
  migration, which is still its own change.
- **`mix::gain_record` and `mix::preview_record` are deleted**, because the conversion is now the
  derivation and two of them is the drift `mix.rs` exists to end. `opacity_record`, `blend_record`,
  `residency_record`, `mask_record` and `transition_record` stay, called by `crossfade` and `wipe`,
  whose own conversions are not settled — and a test asserts the ones that remain answer what the
  conversion answers, so the interim cannot drift while it lasts.
- **`karakuri-console/examples/panel.rs` is not touched, and its `record()` is still there.** ADR-0185
  promised it is deleted the day this lands; it is a follow-up commit, because another session is
  working in that crate. Its documentation's pointers at `mix::gain_record` are stale until then.
- **ADR-0180's "one `From` impl per list in `karakuri-cli`" cannot be written, and it is the orphan
  rule rather than a preference.** `Blend` is `karakuri-engine`'s and `BlendMode` is
  `karakuri-operation`'s, and `karakuri-cli` owns neither, so no `impl From` is allowed there at all.
  They are plain functions — `mix::blend_mode`, `mix::residency`, `mix::sync`, `mix::tonemap` —
  exactly as `panel.rs`'s `blend_mode` already was. That bullet of ADR-0180 is history rather than a
  rule (`docs/contributing.md` §4).
- **`Tonemap`, `Sync` and `Residency` got a `name()`**, on `BlendMode::name`'s terms and for its
  argument — a value added to the enum does not compile until it has a name — because a record
  carries the *name* and the conversion had nowhere else to get it. Without it this crate would have
  been a fourth spelling of each list.
- **The two copies of every list are now checked against each other**, in
  `karakuri-cli`'s `mix.rs`, which is the only crate in the workspace that depends on the engine and
  on the vocabulary at once. `karakuri-operation` cannot do it and neither can this new crate; both
  are engine-free by charter. A level spelled `prime` here and `priming` there is a record that
  decodes to a refusal on replay and moves nothing in the mix, which would show up as a session
  replaying differently and nowhere earlier.
- **What is still owed, and it is now enumerable rather than vague.** Seven operations are
  `Owed::NotSettled` and each names its question; four are `Owed::Undecided` and those are the
  vocabulary's. The largest single one is **who owns the transition settings** — the quantum, the
  length and the wipe shape are `SetTransition`'s, that operation writes no record, and four
  conversions are blocked on it.
