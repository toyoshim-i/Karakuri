---
id: 0196
title: A map line names a state, and an old line is refused rather than redefined
status: accepted
date: 2026-08-27
supersedes: []
superseded_by: []
principles: [0074, 0058]
tags: [midi, vocabulary, surfaces]
---

# A map line names a state, and an old line is refused rather than redefined

## Context

`karakuri-midi` had a vocabulary of its own: eight `Action`s, declared engine-neutral, which is
why three of them were affordances rather than destinations — `ToggleOnAir`, `TogglePriming`,
`CycleBlend`. That is the worked example
[P-0074](../principles/0074-an-operation-says-what-it-wants-never-which-way-to-move.md) is written
around: *the two rules — be engine-neutral, and have no toggles — are not jointly satisfiable
unless the vocabulary owns the lists.*

Two things had to land before the crate could move onto
[`karakuri-operation`](../../crates/karakuri-operation), and both have:

- `SetLook` demanded a tone map beside every exposure, so `cc 20 -> exposure` could not become an
  operation at all. It is `SetTonemap` and `SetExposure` now
  ([ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
- There was nowhere for an operation to become a record. There is now, and it is
  [`karakuri-operation-record`](../../crates/karakuri-operation-record)
  ([ADR-0194](0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).

`Action` was never mostly toggles — five of its eight already named a destination, and
`Preview { slot: Option<u8> }` had the shape the vocabulary later copied. What is left is three
target words, and **the target words in a map file are the manual's badges verbatim**
(`docs/manual/operations.html`), so changing one is an edit to the specification rather than a
rename.

## Decision

**`karakuri-midi` produces `karakuri_operation::Operation` and `Action` is deleted.** The crate
gains one dependency, on a crate that has none — which is ADR-0180's charter and was written with
this crate as its example: *a map file's parser must not pull serde and sha2*. `cargo tree -p
karakuri-midi` shows `karakuri-operation` as a leaf beside `midir`.

Three decisions inside it.

### 1. The three affordance targets become value-naming ones, and an old file is refused

The grammar is the target, the deck, then a value word — the shape `preview mix` already had:

```text
note 32 -> residency 0 live       # was: on-air 0
note 36 -> residency 0 priming    # was: prime 0
note 40 -> residency 0 allocated  # was: nothing at all
note 44 -> blend 0 over           # was: blend 0
```

The value words are `Residency::name` and `BlendMode::name` — the same words the record carries and
the status line prints — read out of `Residency::ALL` and `BlendMode::ALL` rather than copied into
the parser, so a value added to the vocabulary is offered to a map file the day it lands.
`Residency::ALL` is new and exists for `BlendMode::ALL`'s stated reason: what a surface needs from
the vocabulary is *which values exist*, never a *next*.

**A file holding an old line is refused on that line, with the line to write instead:**

```text
line 36: `on-air 0` flipped a deck on and off rather than naming where it goes;
         write `residency 0 live` or `residency 0 allocated`
line 41: `prime 0` flipped a request on and off rather than naming where it goes;
         write `residency 0 priming` or `residency 0 allocated`
line 46: `blend 0` needs a mode — write one of `blend 0 add`, `blend 0 over`, `blend 0 max`
```

That is the previous `examples/surface.map` run through the new parser, verbatim: twelve lines
named, each with somewhere to go, and the nine that were already destinations still loading.

This is what the crate's own design already says to do about a bad line — every complaint carries a
line number, and *one bad line is a line an operator can fix*. Breaking the format is permitted
before v1 and has to be deliberate
([P-0058](../principles/0058-before-v1-compatibility-is-a-bill-not-an-argument.md)); this is the
deliberate part.

### 2. The pad count is the bill, and it is written into the example

Four decks by three residencies is twelve pads where two toggles were eight; four decks by three
blend modes is twelve where one cycle was four. With preview and tap, `examples/surface.map` is
**thirty notes**, and its own prose describes a four-fader surface with a row of pads under it —
which is not a device with thirty pads. The file now says so, in the section that grew: take the
ones your hands need, or put the rest on the controller's own banks; what is not on offer is a pad
that means *next*. That is P-0074's cost on this surface, and stating it is better than leaving an
operator to count.

### 3. The exhaustiveness moves to `written`, and `note -> tap` keeps its behaviour

`Live::run_surface` was one match over eight `Action`s and its documentation claimed *a control
added to one and not the other does not compile*. Against a forty-six-variant vocabulary that claim
would be **false** — a router arm nobody wrote is a wildcard nobody notices. So the MIDI path is
now *message → `Operation` → `Live::operate`*, and the compiler's guarantee lives where it is true:
`karakuri_operation_record::written` is one exhaustive match over all forty-six.

**Where it does not reach is said rather than papered over.** `Operation::TapBeat` needs the beat
tracker rather than a value and `written` answers `Owed::NotSettled` for it, so routing it through
`operate` would print a gap where a tap used to move the grid. `run_surface` therefore has exactly
one arm of its own — `TapBeat` calls `Live::tap`, the `b` key's own function — with the reason at
the arm and the note that the arm is what goes the day the record a tap owes is settled.

## Alternatives rejected

- **Keep the old spellings and redefine them.** `on-air 0` would go on parsing and would mean *put
  slot 0 live* instead of *flip slot 0*. This is the one an operator would feel: a map checked in
  last month loads clean, says nothing, and does something else in the middle of a set — and the
  failure appears on the pad that was working. A refusal with a line number costs an edit the
  operator can make in the moment; a silent redefinition costs a set.
- **A separate word per state — `live 0`, `prime 0`, `park 0`.** Three targets over one operation,
  where the vocabulary has exactly one, and `park` is the name for a *disagreement* between the
  request and the effective residency
  ([ADR-0186](0186-one-operation-names-one-of-three-residencies.md)) rather than a state to ask
  for. It also loses the shape `preview mix` already established, which is the reason the grammar
  reads as one rule and not as a list of special cases.
- **Keep the router's match over operations in `karakuri-cli`.** It would preserve the printed
  feedback per action and the shape everyone knows. It also keeps the second list the vocabulary
  exists to abolish, and its compile-time claim would have to be dropped or rewritten as a
  wildcard — which is the same thing with a comment on it. The exhaustiveness has a home now.
- **Keep the pad count down by mapping a subset in the example.** A file that shows two of three
  residencies teaches a grammar that does not exist, and the count is the honest consequence of
  P-0074. It is written down instead.
- **Give `Map::operation` a readback so a pad could still toggle.** Rejected at ADR-0192 and
  rejected again here for the same reason: it ends `Map::operation` being a pure function of one
  message, which is this crate's whole test story.

## Consequences

- **`Action` is gone**, and with it `karakuri-cli`'s eight-arm router and `midi.rs`'s `slot_of`.
  What replaces `slot_of` is `deck_of`, five arms and a wildcard — safe rather than merely
  convenient, because the record path is the backstop: `mix::change` refuses a slot the deck does
  not hold with `no_such_slot` and nothing moves, where the old path indexed a `Vec` and panicked
  on the render thread. What the router's check still buys is **silence**: the refusal is said once
  per slot per run rather than once per message.
- **A mapped message no longer prints what it did.** The old arms ended in `set_gain` and its
  neighbours, each of which reports — an `eprintln!` per MIDI message inside `Live::frame`, several
  hundred a second on a fader sweep, which is the blocking write per message `crate::midi`'s own
  *once per control, not once per message* rule exists to prevent. What a surface moved is read
  back from the deck.
- **The map file's `[lo, hi]` is now the surface's clamp.** `set_gain` and `set_opacity` clamped
  before building the record; `operate` does not, because
  `karakuri-operation-record` deliberately holds no clamps and `Operation::SetGain` is open on
  purpose. A map that asks for `gain 0 [-1, 1]` therefore records what it asked for. The engine
  still clamps on apply, so nothing on screen changes; what changed is that the bound is the
  operator's line rather than the CLI's floor, which is where ADR-0185 puts it.
- **Seven of the forty-six operations are reachable from MIDI**, where the count used to be read
  off `Target` as eight — two rows became one, and the vocabulary is no longer this crate's to
  count. `docs/manual/operations.html` recounts to the same **46 operations and 50 of the 200 ways
  in**: the two badges that changed changed their text and not their kind.
- **`karakuri-console`'s `panel::Op` and `karakuri-cli`'s key handler have still not moved.** They
  are the two surfaces left, and each is a change of its own — the order ADR-0186 and ADR-0192 took.
- **No new principle.** P-0074 is what this applies; the only rule it adds is local to the format
  and is stated in `karakuri-midi`'s module documentation.
