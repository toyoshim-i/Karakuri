---
id: 0227
title: A pattern and a master-chain setting are library data, in two tiers, on the arrangement's shape
status: accepted
date: 2026-08-30
supersedes: []
superseded_by: []
principles: [0013, 0028, 0048, 0085]
tags: [store, format, console, sequencer]
---

# A pattern and a master-chain setting are library data, in two tiers, on the arrangement's shape

## Context

Two of the console's bays hold state nothing in this workspace keeps. The Sequencer's patterns are
the clearer case — *"nothing in this workspace holds a pattern"*, which is why that bay draws
nothing beyond its head
([ADR-0222](0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md)) — and the Master bay's
chain is the same question one step earlier, since the chain itself does not exist. This record is
about where both are kept, and the shape it follows is already built.

### What ships as a preset today, and where

`examples/`, and **it already holds something that is not material**. Twenty-two `.kir` files and
one `surface.map`: `crates/karakuri-midi/src/map.rs` `include_str!`s that map into its own test and
calls it *"what an operator copies before they have one"*. So the precedent for a preset that is a
*setting* rather than a procedure is in the tree, and it is the only one.
[P-0048](../principles/0048-what-ships-what-you-saved-and-what-you-are-editing-are-three-places.md)
names the tier: *"`examples/` is app presets and **nobody writes it**"*, and
[manual.md](../manual.md)'s *Where your work lives* table is the same three rows — app presets in
`examples/`, your presets in `<store>/sets/<id>.set.ndjson`, scratch in `<store>/scratch/`.

**The map's second tier is not a store directory**, and that is the one place the existing shape is
ragged: an operator's own map is whatever path they hand to `--midi-map FILE`. It ships as a preset
and it is saved nowhere in particular.

### What the store holds, and under what names

`crates/karakuri-store/src/store.rs`'s own header is the layout, four kinds under the root:

```text
<hash>.kir                        source, immutable
<hash>.meta.ndjson                regenerated metadata
thumbnails/<hash>.mp4
sets/<id>.set.ndjson
sessions/<stamp>.ndjson
arrangements/<name>.arrangement.json
```

The fourth line is
[ADR-0221](0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)'s, and it is
the precedent this record follows rather than the one it argues with. What that record decided, in
four parts: a **name the operator typed** is the handle, because
[P-0087](../principles/0087-name-the-property-never-the-shape.md) rules out a
content address that moves on every character and a slot number that moves when something else is
deleted; a **fourth place under the store**, because each of the three that existed refuses it for
its own reason; the **spelling is one component at a time** — the operator's name, what kind of
thing it is, the format it is in — *"which is what lets a listing tell an arrangement from an
editor's backup or a half-written `.tmp` without opening either"*; and the store keeps **bytes it
does not have to understand**, `write_arrangement` taking `&[u8]` exactly as `put_artifact` does
with `.kir` source, so the format belongs to whoever owns the loader.

**And the panel has drawn the shape one level up since before any of it existed.** The Library
bay's scope row in [the console mock](../manual/console.html) is `favourites · my sets · presets ·
folder · +`, with the note that *"the scope list is itself extensible"*.

### What exists of the two subjects

**A pattern: nothing.** ADR-0222 settled what a lane *is* — a fifth route into
`karakuri-operation`, emitting operations on the beat — and that decision is about the wire rather
than about the file. What the mock draws is `seq 1 · seq 2 · +` over sixteen steps and four lanes,
three of them deck faders and the fourth a Set parameter (`L2:0 twist` on deck B), with a `+ lane`
in the foot. No type in any crate holds any of it.

**A master chain: nothing, and one level that is not the chain.** `Deck::set_out` landed this
morning with `Operation::SetMasterOut { out: f32 }`, a `Record::MasterOut` classified
`Vocabulary::Session`, and a fader that is *"the bay's whole body, and the rest of the bay is not
built"* (`crates/karakuri-console/src/view.rs`). The three effects the console page draws under
that row — feedback, bloom, rgb shift — exist nowhere, which
[ADR-0224](0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md) recorded as
the deliberate cost of putting `out` and `exposure` in different places before there was anything
between them.

## Decision

The maintainer's, on 2026-08-30:

> シーケンサのパタン含めてマスターエフェクトも設定の保存と復元が必要ね。これもプリセットとユーザー
> データの二段構成かな。他のデータと同じように。

**The sequencer's patterns, and the master effects' settings too, need saving and restoring. Two
tiers as well — presets and user data. The same as the other data.**

So: **a pattern and a master-chain setting are data of the same kind as a Set and an arrangement.**
They ship as presets, the operator's own live under the store, and both follow ADR-0221's shape —
named by the operator, one path component, a place of their own under the store root, bytes the
store does not parse.

**Following that record rather than inventing a shape is most of this decision.** It is
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) taken at the width of a
file format: the naming rule, the refusal that keeps a name to one path component, the atomic
overwrite that makes saving over a name mean saving over it, the listing that skips what does not
claim to be one, and the `Store::open` that establishes a directory an older build did not write —
all of it exists, was argued once, and was built for a kind of file that is not material either.

**The one place ADR-0221 does not extend, and it is stated rather than glossed.** An arrangement's
preset tier is *code*: what ships is `karakuri_console::layout()`, the built-in default, and
ADR-0221 says outright that *the default is not a file and there is no reserved name*. A pattern is
not that. It is authored content the way a `.kir` and `surface.map` are — somebody sits down and
writes one — so its preset tier is a file that ships, in `examples/` with the material and the map,
and nothing in this program may write there
([P-0048](../principles/0048-what-ships-what-you-saved-and-what-you-are-editing-are-three-places.md),
whose *what it rules out* is the day a session rewrote three tracked examples).

## Why the two other homes are wrong

Both are places the state could plausibly go, both are reachable from what already exists, and
somebody will re-propose each of them.

### a. State a session record carries

The most plausible of the two, because the machinery is already pointing that way. Every control
already ends in a record
([P-0028](../principles/0028-every-control-ends-in-the-same-record.md)), the master out has one as
of this morning, and `Record::MasterOut`'s own documentation anticipates growth: *"there is nothing
else about the master chain a stream can say yet, so a record that carried more would be recording
defaults nobody chose. It grows the day an effect lands in the chain."*

**None of that makes the stream the library, and
[P-0013](../principles/0013-a-set-is-a-projection-and-a-session-is-the-timeline.md) is why.** *A
Set file is a state projection with no time in it. A session stream is the timeline.* **Keeping
them apart is what stops saving a Set from saving a performance** — and the mirror is what is
wanted here: keeping them apart is what stops a pattern from existing only inside a performance.
A pattern authored on Tuesday and played on Friday would exist, under this alternative, only if
somebody had been recording on Tuesday, and would be recovered by replaying a session rather than
by opening a file.

**And for a pattern specifically the stream would hold the same fact twice.** A lane is a route
into the vocabulary (ADR-0222), so its writes land in the stream as `Record::Opacity` and its kin,
sixteen a bar, exactly as a hand's do — that is the property ADR-0222 was arguing *for*. A pattern
record beside them would be the cause written down next to every one of its consequences, and a
replay would then have both the pattern and what the pattern did.

**What this does not overturn is `Record::MasterOut`'s sentence.** A chain's *levels while it is
being played* are session state and belong in the stream, on that record's own terms, the same way
a Set's gain does. A chain's *settings* — which effects, in what order, with what values, saved
under a name and put back tomorrow — are the library form, the way a Set file is the library form
of material whose moves are all in the stream. Both, for the same reason a Set has both.

### b. A Set file holds it

Also plausible: a Set file is the existing *saved-and-restorable* thing, and `--save-set` is the
existing gesture.

**It is refused by the format and the refusal is already written.** `Store::write_set` rejects any
line that is not Set state, twice over — `MetaInSet` and `TickInSet` — and `Record::is_set_state`
is the one place a record is classified.

**And it is wrong before it is refused, which is the part worth recording.** Three of the mock's
four lanes drive deck A, B and C's opacity, which is `Record::Opacity`, `Vocabulary::Session`, and
**no Set's** — ADR-0222 found exactly this when it killed the binding reading: *"there is no
binding on a deck fader anywhere in the engine."* `record.rs` says what filing one under a Set
would do, about the gain and word for word about this: *"A Set does not know what fader it is
under; one that carried its gain would restore that gain wherever it was next loaded, which is a
Set file reaching outside the Set."* A pattern is worse than the gain, because it is *mixed*: the
mock's fourth lane targets a Set parameter and its first three do not, so there is no one Set the
file could belong to even if the format allowed it. The master chain is the same argument at the
top of the fold — it is one level out from every Set on the panel, and a Set that carried the
chain's settings would reconfigure the master the moment it was loaded into any deck.

## What this leaves open, deliberately

- **The file form and the directory names are not decided here.** Only that they follow ADR-0221's
  shape: a name the operator typed, one path component, a place of its own under the store root,
  and bytes the store hands over whole. Whether a pattern and a chain setting are one place or two,
  what each file is called, and whether the format is JSON as the arrangement's is or NDJSON as a
  Set's is are all for the record that has something to serialise.
- **What a chain setting *contains* cannot be written down, because the chain does not exist.** The
  console page's callout is *giving L5 a writable form*, and it is the argument for it rather than
  a record that it happened; `karakuri-engine` names `master` only at the one level this morning's
  fader moves. **A record kept for a thing that does not exist would be inventing its contents** —
  it would fix an order, a per-effect parameter list and a naming scheme for effects nobody has
  built, and every one of those would be a decision taken with no material to take it against. It
  is the same restraint `Record::MasterOut` already exercises for the stream: one value and no
  operator beside it, because *"a record that carried more would be recording defaults nobody
  chose."*
- **A pattern's contents are open for a smaller reason**: the shape is visible in the mock — lanes,
  a target per lane, a step count, values per step, a mute, and banks — but *what a lane's target
  is spelled as* is the vocabulary's question and not the store's, and no operation names a
  sequencer today.
- **No operation and no route is decided here.** `karakuri-operation` gains nothing, and no badge
  moves on [every operation](../manual/operations.html).

## Consequences

- **`crates/karakuri-store`'s header gains a line, or two, when there is a format to name**, and
  `Store::open` establishes the directory the way it establishes `arrangements/` — so a store
  written by an older build gains it the first time this one opens it.
- **`examples/` gains a second kind of non-material preset**, joining `surface.map`. The rule that
  nothing in this program writes there is unchanged and is the one P-0048 exists to enforce.
- **The Library's `presets` scope is not this**, and the resemblance is worth refusing now. That
  scope is a scope of the Set listing — *"a folder scope reads Sets"*, and a `.kir` is explicitly
  not a row there. Where a pattern is browsed from is a question for whoever draws the Sequencer
  bay's bank pills, and the answer is not automatically the Library.
- **This does not unblock the Sequencer bay.** What that bay is waiting on is *"a step grid and a
  pattern record, neither of which anything in this workspace holds"*; this record settles where
  the second one is kept once it exists, which is one of the two decisions in front of it and not
  the machinery.
- **It is inside what
  [ADR-0226](0226-m5-closes-when-the-manual-is-implemented-and-the-meter-is-progress-rather-than-completion.md)
  makes the milestone.** That record settles that M5 closes when the manual is implemented, and the
  operations page names no operation for the Sequencer bay at all — so saving and restoring a
  pattern will arrive as rows on that page the way *Save the arrangement* and *Put a saved
  arrangement back* did, specified before they are built. This record is the half of that which
  ADR-0221 already answered for a different kind of file: where the file goes. What the rows say is
  the page's.
- **`docs/adr/INDEX.md` owes a row for this record**, not added here because that file is held by
  another session in flight. `docs/roadmap.md`'s Sequencer and Master entries owe a pointer for the
  same reason and are held by the same session.
