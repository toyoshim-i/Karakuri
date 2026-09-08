---
id: 0299
title: my sets is the starred subset, and the star is kept beside the Sets
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0083, 0090, 0091, 0096]
tags: [library, store, operations, console]
---

# `my sets` is the starred subset, and the star is kept beside the Sets

## Context

The Library bay draws a row of scope chips and a listing under it.
[console.html](../manual/console.html) draws a star at the left of every row — `★` on two of them,
`☆` on three — and tips it *"A favourite. Kept beside the Sets in the store rather than inside the
Set file … Click to unstar."* It also has a note, *What keeps a favourite, and where it does not
travel*, which decides **where the value lives** and says outright what it does not decide:

> The star is a control over a fact **nothing in this workspace keeps**: no field on a Set listing,
> no record that carries one, no operation that names one. … What this page settles is the half a
> control cannot be specified without: the value it reads, and where that value lives.

So the star has been specified as a control, with nowhere to write and nothing to name it, for as
long as the page has been drawn. `crates/karakuri`'s `why_nothing` says the same thing back to an
operator who marks the scope: *"nothing in this workspace keeps a favourite … so this scope is empty
because there is nowhere for a star to be rather than because nothing is starred."*

**What was not settled is what `my sets` is.** The console reads it as *what this store holds*,
which is `Store::list_sets`, and the maintainer has said it is not:

> `my sets` はお気に入りなので、例えば通常の一覧表示の左に☆を付けといて、クリックすると★になって
> `my sets` にも表示されるのが良いと思う。

## Decision

**`my sets` is a favourites view.** It is the starred subset of what `<store>/sets/` holds, and
never the listing of what `<store>/sets/` holds. The ordinary listing carries a `☆` at the left of
every row; one press makes it `★` and the Set appears under `my sets` as well.

**The act is one operation naming a state.**
`Operation::SetFavourite { id: String, favourite: bool }`, on the page as *Star a Set, or take the
star off*. Not a toggle, on the vocabulary's general rule — a map with a button per direction, a
model that says which one it wants and a key all have to be able to say *star this* and mean it. The
fact is a **favourite** and the control is a **star**, which is `console.html`'s own pair.

**`Standing::Open`.** P-0094's question asked of a star comes back empty on every count: at its
worst, on the frame it goes wrong, with the operator's attention on the room, it has written one
line into a small file beside the library and moved no deck, no fader and no pixel.

**`Written::Silent(Silent::Surface)`**, which is `Operation::SaveArrangement`'s answer for
`SaveArrangement`'s reason. `console.html` refused the other two homes by name: in the Set file a
star travels with the material and has to be *written*, so a starred Set would jump to the top of a
listing ordered by when it was made; in a session it is a thing that happened at a moment, so a
press with `rec` off would keep nothing and a replay would hand a favourite back as an event rather
than as something that is true. The session vocabulary having no row for a favourite is therefore
**the decision** and not a gap, which is what `Silent::Surface` says and `Silent::NoRecord` would
deny.

**The mark lives in `<store>/favourites.json`**: one document, a JSON array of the ids that are
starred, at the store root beside `sets/`. `Store::favourites` reads it and `Store::set_favourite`
writes it, both off the frame (P-0091) — the listing is built at startup and on the press that
changes the scope, and a star is a press.

- **`.json` and not `.ndjson`, for `arrangements/`'s reason**: it is one document rather than a
  stream of records. Taking a star off is a *removal*, and a stream would need a tombstone and a
  projection to express one.
- **Not a `Record`**, because the session vocabulary carrying a favourite is exactly what
  `console.html` refused.
- **Not the metadata card.** The card the store keeps is `<hash>.meta.ndjson`, one per **artifact**,
  and there is no per-Set card anywhere in this workspace — see *What the records state wrongly*
  below. A card is also **derived**: `meta::card` builds it from a `Checked` and the store
  *"regenerates it from the `.kir` plus a compile pass"*, so a fact nothing can derive would be
  erased by the next regeneration.
- **A file and not a directory of markers.** Both keep the same fact and both leave the same stale
  entries behind; a file is one read and one atomic write, and the whole answer is a `BTreeSet` a
  caller can hold.

**A star is refused on an id `sets/` does not hold, and taking one off is not** —
`StoreError::NoSet`, carrying the id back (P-0083). The star is a control on a row and a row is a
Set this store holds; the asymmetry is the answer to a stale mark, below.

## What this closes

[The roadmap](../roadmap.md) has carried this under *The decisions nobody has taken*:

> **Whether loading a shipped preset should write into the operator's own library.** ADR-0229 makes
> a load a packaging step that stores the bundle, so opening a shipped Set adds an entry to
> `<store>/sets/` … **The first symptom if it is wrong** is a `my sets` filling up with things the
> operator did not make.

**The symptom cannot happen, so the question is answered rather than deferred.** A preset load and
a recording's head both land in `<store>/sets/` and **neither appears in `my sets` until somebody
stars it**. The load stays what ADR-0229 made it — copy-on-load, exactly as `scratch.rs` performs it
for material, and what keeps `examples/` unwritten (P-0096) — and what it adds is a row in the
listing rather than a row in the operator's favourites.

It closes the other half of ADR-0289's cost bullet by the same stroke: *"Each start leaves a
`<stamp>-material` Set in the library … a row in the Library bay that nobody asked to keep."* It is
still a row in the library, and it is no longer a row in `my sets`.

**This is not P-0096 restated.**
[P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md) is about
**who may write** the library, and it is untouched: a preset load and a recording head still write
`<store>/sets/`, a model's save still lands in `<store>/sandbox/`, and nothing about the four
addresses moves. This record is about **what a view shows**, which is a different question that
happens to have the same symptom.

## Alternatives rejected

- **Keep `my sets` as the listing and let `favourites` be the filter over it**, which is what the
  page draws today. It is the maintainer's decision read the other way round, and it leaves the
  roadmap's symptom exactly where it is: `my sets` goes on filling with presets and stamps.
- **A field on a per-Set metadata card.** There is no per-Set card, and the per-artifact one is
  derived and regenerated — see the Decision.
- **A star in the Set file**, refused on `console.html`'s own terms and not re-argued here.
- **Prune the file against `list_sets` on every write.** It makes a star that lands while `sets/` is
  briefly unreadable delete every other star, and losing what somebody chose is a worse failure than
  keeping an id that names nothing.
- **A `favourites/` directory of empty marker files.** The same fact, the same stale entries, a
  directory read instead of a file read, and a create or a delete instead of one atomic write.

## Consequences

- **64 operations, not 63**, and the split `gate.rs` asserts moves from 40 closed / 23 open to
  **40 closed / 24 open**. The floor in `karakuri-operation`'s own test and in
  `tests/the_manual_and_the_vocabulary_agree.rs` moves with the page, as it always has.
- **The page moves first and this record does not move it.**
  `the_manual_and_the_vocabulary_name_the_same_operations` fails until
  [operations.html](../manual/operations.html) carries the row, and that failure is the mechanism
  working: the page is the specification. The row's four badges are `plan` panel *library star*,
  `plan` key, `gap` MIDI and `plan` MCP — **MIDI is a `gap` and not a plan**, because every target a
  map line can name carries a slot, a range or a word from a closed list and a Set id is none of the
  three, which is the sentence the `read` and `load → A` pills beside this control already carry.
- **The two scope chips are one question now.** `favourites` was *"this library filtered rather than
  a fifth place a Set can be"* and `my sets` was the library; under this record they are the same
  question and the surviving word is `my sets`. **What lists everything the store holds still needs
  a chip**, and naming it is `console.html`'s — this record fixes that there is one and that it is
  not `my sets`.
- **A favourite does not travel with a Set**, which is `console.html`'s stated cost and is unchanged:
  copy the library and the stars come with it; hand somebody one Set and it arrives unstarred.
- **A stale mark is a line and not a hazard.** Nothing in this program deletes or renames a Set, so
  a mark goes stale only when a hand removes the file from `sets/`. The id stays; a listing is the
  intersection of the marks with `list_sets`, so it draws no row; the star can still be taken off
  without the Set coming back first; and putting a Set back under the same id brings its star back,
  which is the right answer when the id is a name an operator typed.
- **No new principle.** P-0096 is untouched for the reason given above, and the rule this states is
  narrow enough to live in the operation's own prose and in `store.rs`'s layout comment.
