---
id: 0276
title: A version's Set id goes in the snapshot's name, and a run without one writes none
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0090, 0091]
tags: [store, history, library, environment]
---

# A version's Set id goes in the snapshot's name, and a run without one writes none

## Context

`karakuri_environment::history::Snapshots::record` addressed a version by `(slot, layer, index)`
and **the layout carried no Set id anywhere**, so the two sides of a library load on one slot were
indistinguishable and *a Set's history* was not answerable from these files. That the id is written
from now on was settled in commit `c4919bd`, on an argument rather than a design: every route that
edits is addressed by slot — `mcp::write_procedure` resolves `Slots::path(slot, layer, index)` and
refuses a node the slot does not hold, and an operator's own editor is pointed at that slot's
scratch copy — so at the moment of a write the program knows which Set the slot is running.

**What that commit left open is what this record answers**: *where* in the layout the id lives, and
what is written where there is no Set.

The constraint that decides the first is
[`history::list`](../../crates/karakuri-environment/src/history.rs): it walks day directories
newest first, stops opening them once it has enough, and **opens no file at all** — a row is a
name. Its own head calls that the shape of the thing rather than a shortcut, and
[ADR-0263](0263-the-library-bay-lists-most-recent-first-because-the-listing-is-the-operations-and-not-the-surfaces.md)
rests the ordering on it: the time is in the name to the millisecond, so no `mtime` is read and the
order is total without touching the filesystem clock.

## Decision

**The Set id is written into the snapshot's file name, behind `@`, after the procedure name:**

```text
<store>/history/2026/08/16/143052-271_slot0_L4_beat_strokes@star_vortex.kir
```

**The id is also the fourth field of the dedup key**, so a chain is a node *of a Set* rather than a
node.

**Where there is no Set, nothing is written**: no `@`, and `Version::set` reads back `None`.
`record` takes `Option<&str>` and `Version` carries `Option<String>`.

## Alternatives, and why they lost

### Beside the file — a sidecar per snapshot

Costs the lister one file open per row, which is the one property `list` is built on. It also
doubles the entries in a directory an operator is explicitly invited to work in by hand — `rm -rf
history/2026/07` is this module's whole retention policy — and a sidecar separated from its
snapshot by that `rm` is worse than no sidecar, because it is a claim with nothing under it.

### A directory per Set — `history/<set>/YYYY/MM/DD/`

Costs more than the open, and both costs are structural.

- **The retention policy stops being one.** `rm -rf history/2026/07` deletes a month only while the
  month is near the top of the tree. Under a directory per Set, a July is spread across every Set
  the operator has ever played, and the cleanup that is a feature nobody has to write, learn or
  trust becomes one they have to write.
- **The walk stops being lazy.** `list` is cheap because the day directories *are* the top of the
  tree: it sorts their names, takes the newest, and stops entering them once it has `most` rows.
  A reader that had to enter every Set directory before it could order two days pays for every Set
  on every listing, which is [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)'s
  cost-known-before-it-is-paid running the wrong way.

### In the dedup key only, and not on disk

This is the cheapest change and it answers nothing. `list` reads names, so an id that never reaches
a name is an id `Version` cannot carry, and *narrow this listing to one Set* stays unanswerable —
which is the whole of what the id was for.

It is nonetheless **half** of the decision: the key needs the id whether or not the name gets it,
because keyed without it a load onto a node whose source happens to match what the outgoing Set
last had is skipped, and the incoming Set's chain begins at its first *edit*. That is the hole
`history::seed` exists to close, one level up.

### A fixed `_`-delimited field, rather than `@`

Rejected on evidence rather than taste. A name is parsed back by splitting on `_`, and the
procedure name is the field allowed to contain one. A Set id may contain one too:
`karakuri_environment::accepted_save` spells a model's save `<stamp>_<name>`, so a `_`-delimited
fixed position is not merely awkward, it is **wrong for ids this program itself writes**. Two
variable fields need two separators, and `@` is outside `sanitize`'s alphabet — both fields go
through it, so the last `@` in a name this wrote is the separator and there is no other.

Placing it *after* the procedure name rather than before is what keeps every name already written
readable: a name with no `@` parses exactly as it did, which is the same argument `record` leaves a
renderer's `_0` off a name with.

### A sentinel for *no Set*

Refused. Any word that reads as a name is a name an operator is free to give a Set — `checked_id`
accepts letters, digits, `-` and `_`, and `unknown` is among them — so a sentinel is a collision
waiting for the one listing that matters, and it makes *there is no Set* and *there is a Set called
this* the same string. An `Option` says the first and cannot say the second by accident.

## What is written where there is no Set

Two cases, and both are answered by one sentence: **`record` is told what the slot is running at
the moment of the write, and a version that is filed is never re-filed.**

1. **A run launched with a pair on the command line writes `None`.** There is no id: nothing was
   loaded and nothing was saved, and the material is two paths somebody typed. The nearest thing to
   a name is `crates/karakuri`'s `Sources::material`, which is `"coil_vortex + star_flares"` — a
   readout for the mixer strip, not an id, and filing versions under it would put rows in the
   history under a Set no listing can ever match.
2. **A save mid-chain changes nothing, because a save does not move the slot.** It copies what is
   playing into the library under a new id. A **load** is what changes which Set a slot is running,
   and the panel already spells that difference: `played` rewrites the slot's name on a `LoadSet`
   and nothing rewrites it on a `SaveSet`. So the versions either side of a save carry whatever the
   slot was already carrying, and a Set a save created has no versions under its own id until
   somebody loads it.

**The second is a decision and the rejected alternative is re-filing the chain from the save
forward.** It loses on its own terms. The version that was saved is the one written *before* the
press, so a Set re-filed from the press onwards is a Set **whose history does not contain itself**;
and making it contain itself means renaming files that are already filed, which is the one thing
this module never does to a version it has written — and which `list`, opening nothing, could not
be made to do cheaply anyway. What the second case leaves is honest rather than empty: nothing has
ever been a version *of* a Set that was just saved — it **is** a version, and it is in the library.

## Consequences

- `Snapshots::record` takes `set: Option<&str>`; `history::seed` takes one per slot;
  `watch::Watch::snapshotting_to` takes the Set the slot is running beside the shared history.
- `history::list`'s `Version` carries `set: Option<String>`. **The narrowing is not built here** —
  which rows an operator is looking at is the surface's question, the way `Operation::ListSets`'
  two filters are applied where they are answered
  ([ADR-0262](0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)).
  A narrowing must treat a `None` row as matching no Set rather than as a wildcard.
- **`karakuri-cli` is still the only caller.** Its only id is `--load-set`'s, which fills slot 0 and
  is refused alongside `--set`, so every other slot writes `None` and nothing moves the answer
  mid-run: the re-point channel carries a re-wiring, and MCP publishes no load. A surface that
  loads a Set into a running slot has to move the id with the re-point, or every version after the
  load is filed under the Set before it.

  > **Annotated 2026-09-08, later the same day.** That clause has a second caller now:
  > `crates/karakuri` seeds one `Snapshots` for the run and hands it to every slot's watcher, and it
  > **is** the surface this consequence describes — a library load re-points a running slot. The
  > move it asks for is
  > [ADR-0304](0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md):
  > `watch::Aim` carries the Set the slot is running, so the id travels with the files it belongs
  > to.
- `Operation::WalkHistory`'s doc gave *"the store has no reader"* as half its reason for
  `Undecided`; that half was already false and is corrected. The payload is unchanged — the missing
  half is the control.
