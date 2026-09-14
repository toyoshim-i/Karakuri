---
id: 0355
title: A session head says what the deck held, in the records that already say it
status: accepted
date: 2026-09-14
supersedes: []
superseded_by: []
principles: [0085, 0092]
tags: [session, store, replay, format, deck]
---

# A session head says what the deck held, in the records that already say it

## Context

A session stream's only header was `{"t":"header","version":2}` and one Set file's records for
slot 0. `karakuri-cli`'s `session_head` printed the limitation out loud — *"a session stream
cannot say what a deck held, so the other N will not replay"* — and `docs/ir-spec.md` recorded
it as a gap in the format rather than in the driver: every mix record names a slot, and nothing
said how many slots there were or what was in them.

What that cost was not academic. `--replay` built a deck of one, and `mix::change` refused every
record naming slot 1 or beyond with `no_such_slot`, so a four-slot performance replayed as one
slot's performance with three quarters of its moves reported and skipped. The master chain was
seeded from nothing for the same reason: a Set file carries nothing for the chain
([ADR-0340](0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)), so a
session played through a chain replayed through none until a `master_chain` record happened to
arrive mid-stream. And the two writers — `karakuri-cli`'s `--record-session` and the console's
`rec` pill ([ADR-0289](0289-the-rec-pill-is-a-record-stop-toggle-and-each-start-takes-a-fresh-stamp.md))
— each assembled a head from whatever they had to hand, which is two answers to one sentence.

The mechanisms to close it already existed
([P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)):
`Record::Procedure` rebuilds a slot from a store address at the frame it changed on and a replay
already obeys it; every per-slot session record already names its slot; `Record::MasterChain`
is written whole; `HotSwap::install` puts a Set in a slot synchronously.

## Decision

**The head is everything before the first `tick` that is not a frame's own, and it says what the
deck held at frame 0.**

- **The head slot's material is one Set file's records.** A Set file is the only thing that
  carries a Set's params, bindings, seeds, edges, capacity, camera and layering, and it describes
  one Set. It is slot 0's — never the selected slot — because a replay numbers its slots the way
  the deck did, and *which slot is described in full* must not depend on where a hand was.
- **Every other slot is one `procedure` record per node.** The same record a hot swap writes and
  a replay already obeys. Those slots are built against the head's Set file: a run has one
  parameter table and one binding list, so a second copy per slot would be the same numbers said
  twice.
- **Then the deck, always and for every slot**: `canvas` first, then per slot `gain`, `opacity`,
  `blend`, `residency`, `mask` and `transport`, then `look`, `master_out` and `master_chain`.
  Written whether or not they differ from a fresh deck — a head that omitted the defaults would
  need a reader that knew them, and a reader that knows the engine's defaults is a second place
  they are written down.
- **`split`'s rule becomes "everything before the first tick except a measurement".** An `audio`
  or `tempo` line in front of the very first tick is frame 0's measurement — a frame writes its
  edits, then what it heard, then the tick that closes it — so sorting by position alone would
  move it into the head and replay frame 0 at what the bus invents. `Record::is_measurement` is
  the fourth question read off the one `Vocabulary` match.
- **The head reaches a reader as two lists, not one.** `Session::head` is the Set file's records,
  which `setfile::from_lines` builds a Set out of; `Session::opening` is the deck's, which a
  replay applies to the deck it has just built. The *file* has one rule; the struct has two
  fields because the head has two readers and they read two vocabularies. Keeping one list would
  have meant `from_lines` reporting a note per `gain` it skipped on every session ever recorded.
- **A replay builds a deck as wide as the head names**, installs each named slot, applies the
  opening records in order, and then reads frames. A slot the head names and a replay cannot
  build is fatal, where the same failure mid-stream is a note: a slot that fails at minute ten
  keeps what it had, and a slot the head could not build has nothing to keep — dropping it would
  renumber every record after it, so slot 2's gain would land on slot 1's material.
- **A rebuilt slot is built at the head's parameter table, never at the replay's flags.** A
  `procedure` record names a source and carries no value, so the values have to come from
  somewhere, and a replay is a function of the stream
  ([P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)). It was `args.overrides`,
  which made the picture depend on the flags the *replay* was typed with.
- **Both writers write the head through one function**, `karakuri_environment::session::head`,
  beside `split`. Each program reads its own deck — the readings live in different places — and
  neither spells a head.

## Alternatives rejected

**A slot-tagged Set file per slot in the head.** The complete answer: every slot's params,
bindings and seeds, exactly as the head slot gets them. It costs a `slot` field on `slot`,
`param`, `bind`, `seed`, `edge`, `capacity`, `camera` and `merge` — the whole Set vocabulary
grown a deck-shaped field that is meaningless in the file those records exist for, since a Set
file describes one Set and has no deck. Every reader of a `.kbset` would then have to ignore a
field, and `project::key_for` would have to fold on it. The price is paid in the format's most
load-bearing half to buy a case the `procedure` record already reaches most of.

**A single `deck` record naming addresses**, e.g. `{"t":"deck","slots":[{"l1":"sha256:…",
"l4":["sha256:…"]}]}`. One line, and it is a second way to say what `procedure` says: a record
naming a slot's material by store address, differing only in that it says four at once. Two
records for one sentence is what this format keeps removing, and it would need its own decoder,
its own refusal for a slot out of range, and its own rule for a stack's draw order — all of which
`procedure` already has and a replay already obeys.

**Keep the head at slot 0 and write the rest as body records after the first tick.** It needs no
format change at all: a replay that sized its deck from the stream could install slot 1 at frame
0. It replays the wrong first frame, every time — the deck a performance began on would arrive
one frame late, and a session of four slots would open on one. "What the deck held" is a fact
about frame 0, and a record that lands in frame 0's edit list is a fact about an edit.

**Write only what differs from a fresh deck.** A shorter head, and a reader that has to know the
engine's defaults to reconstruct the rest — a second derivation of `Deck::new`'s starting values,
kept in step with the engine's by nothing. The day a default moves, every session recorded before
it replays at the new one.

**Fold `is_measurement` into `is_set_state` rather than adding a question.** One less function,
and the wrong shape: `is_set_state` answers *may this line go in a Set file*, and a `tick` and a
`gain` answer that the same way for entirely different reasons. Four questions, four sentences,
one classification — which is what `Vocabulary` was made exhaustive for.

## What it costs

- **A value ridden onto a slot other than the head's, before the recording began, is not in the
  head.** Those slots are built at the head Set file's parameter table. `karakuri-cli` has one
  table for the whole run, so nothing is lost there; the console's decks each hold their own live
  values, and a knob moved on deck C before the press replays at deck A's value for that name.
  Closing it is a `ride` record per parameter per slot in the head, and it waits on both writers
  being able to read one — `karakuri-cli` cannot, because it has no per-slot table to read.
- **A slot's capacity and salt are the procedure's declared defaults**, not the run's. A
  `procedure` record carries neither, which is its own documented limitation; it is the same gap
  a mid-stream swap already has.
- **A head is longer.** Seven records per slot plus three for the deck — about thirty lines on a
  four-slot deck, once, against 216,000 `tick` records an hour.
- **The GUI records the canvas at the press and never again.** The frame follows the largest
  enabled output ([ADR-0247](0247-one-frame-is-rendered-and-scaled-into-each-output.md)), so a window
  resized mid-recording changes a size the stream does not carry. A replay renders at the size in
  the head, which is the size the press saw.
- **A slot the head names and a replay cannot build stops the replay**, where every other failure
  in that driver is a note. Said above; it is the alternative to renumbering a deck silently. The
  case it bites hardest is a *hole*: the console's `Playing::at` answers `None` for a slot whose
  build's sources never reached the store, so a head can name slots 1 and 3 and say nothing for
  2 — a deck four wide with a slot nothing can build. The replay refuses, naming the slot and
  saying its L1 was never named, rather than playing a deck of different slots under the same
  numbers.
- **The round trip cannot be run in a test.** Both writers need a deck and a deck needs a device;
  `karakuri-cli`'s live path needs a window, which is why `--record-session` is refused alongside
  `--render`, `--seq` and `--replay`. So the claim is checked at both ends instead: a hand-written
  two-slot head driven through the binary, and a source scan asserting that every program opening
  a `Recorder` wrote its head through the one function.

## Consequences

- `karakuri_store`: `Record::is_measurement`, the fourth question over `Vocabulary`.
- `karakuri_environment::session`: `Held`, `SlotHeld`, `head`, `Session::opening`; `split` takes
  the measurement rule and `Session::canvas` scans `opening` too.
- `karakuri-cli`: `held_deck` reads the deck, `app.rs` composes the head and no longer pushes a
  `canvas` or a `look` record of its own — the head carries both, and a second `canvas` in a
  stream is reported by a replay as one it ignored. `replay.rs` gains `slots_named` and
  `place_procedure` (one derivation of where a `procedure` record's address goes, shared by the
  head and by a frame), builds a deck as wide as the head, and `rebuild` takes the head's params
  and bindings.
- `karakuri`: `Keeping::head_opening` replaces `head_material` and gathers three things on the
  frame — the head slot's `Save`, every other slot's `Sources`, and the deck reading; `began`
  stores those sources before the head names them and fails the start if they do not land, since
  a replay now refuses a slot whose address the store cannot resolve.
- `docs/ir-spec.md` gains "The head — what the deck held at frame 0" where the gap paragraph was,
  and the two places that cross-referenced that paragraph say what is true now.
- `docs/manual.md` and the `rec` pill's tip in `docs/manual/console.html` no longer say the other
  decks will not replay.
