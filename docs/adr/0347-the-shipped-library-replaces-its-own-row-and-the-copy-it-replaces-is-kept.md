---
id: 0347
title: The shipped library replaces its own row, and the copy it replaces is kept
status: accepted
date: 2026-09-13
supersedes: []
superseded_by: []
principles: [0083, 0090]
tags: [store, presets, console, cli, setfile]
---

# The shipped library replaces its own row, and the copy it replaces is kept

## Context

Three preset rows — `drift_cloud`, `beat_threads`, `beat_glow` — could not be loaded. Pressing one
in the Library bay was refused; loading the store's copy failed in the checker:

```
contract: `point_size` is now spelled `point_rate`
contract: `point_rate` is not assigned on every path through `vertex`
```

The `.kset` files and the parts they name are correct: `--take-in` of each into a fresh store
succeeds and the Set renders. The store's copies are stale, and the dates say why:

| Date | Event |
|---|---|
| 2026-08-31 | `6f64027` adds the twenty-three `.kset` files |
| 2026-09-01 | `drift_cloud`, `beat_threads`, `beat_glow` are taken in |
| 2026-09-02 | `96c9cc7` renames the renderer output `point_size` to `point_rate` |
| 2026-09-09 | the other twenty are taken in |

A `.kset` names its parts by relative path; the `.kbset` the store writes names them by content
([ADR-0231](0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md)).
A take-in is therefore a snapshot, and that is intended: a deck's material must not change because
a file was saved.

The defect is that the snapshot could not be retaken. `setfile::unbundle` refuses a taken id, on
the grounds that an id arriving inside somebody else's file is not the instruction a typed id is.
That holds for the `folder` scope, which lists a directory dropped on the window
([ADR-0275](0275-a-folder-is-chosen-by-dropping-one-on-the-window-and-the-drop-is-the-windows-rather-than-a-bays.md)).
It does not hold for `presets`, which is the library this run resolved at startup
([ADR-0230](0230-where-the-programs-data-lives-is-told-rather-than-baked.md)) and whose id is the
word the row was already listed under. One press serves both scopes, and `unbundle` was called
without the difference between them, so the shipped library was the one library that could not be
updated, and its refusal advised editing a file inside the installation.

## Decision

`setfile::unbundle` takes a `CameFrom`, supplied at every call site — the shape
[`Asked`](../../crates/karakuri-environment/src/lib.rs) uses, so no route reaches the permissive
branch by default.

- `CameFrom::Somebody` — an arriving `.kbset`, a dropped folder's row, a path typed at
  `--take-in`. A taken id is refused before anything is written. Unchanged.
- `CameFrom::TheShippedLibrary` — a preset row. A taken id is replaced.

On replacement the held Set file is read and written back under `<id>-<stamp>`
(`history::stamped_id()`, e.g. `drift_cloud-20260913-220315-492`), with its `set` record rewritten
to that id. Nothing is deleted; the retired copy is an ordinary Set that lists, loads and can be
starred, and the plain id names what the library ships.

Where the records to be written equal the records already filed, nothing is written and the report
says so, so repeated presses do not accumulate dated copies.

A preset press now always ends with the preset loaded. Previously the refusal meant nothing was
taken in and therefore nothing was loaded.

## Alternatives rejected

**File the shipped copy under the stamp and leave the old one under the plain id.** The plain id
would go on naming the stale Set, so the row an operator presses is still the one that fails to
load.

**Retire into `history/`.** `Snapshots::record` names a slot, layer, renderer index and the Set a
slot was running
([ADR-0276](0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md),
[ADR-0304](0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md)); a
take-in has no run and no slot. A history entry also holds one procedure's source, so a Set retired
there could not be recovered as a Set.

**Reconcile the store against the library at startup.** Store contents would change without an
operator acting, including under a Set the operator had edited.

**Let `--take-in FILE` replace as well.** `karakuri-cli` has no `--presets` and cannot distinguish
a shipped file from any other typed path, so the permission would amount to *any typed path may
overwrite any id*.

## Consequences

- `setfile::unbundle` gains a `CameFrom` parameter. Its callers answer it: `karakuri-cli/src/args.rs`
  with `Somebody`, `karakuri/src/bridge/filesystem.rs`'s `taking_in` via the `Taking` it holds.
- `Taking` gains `came_from`, the first reader of the difference between its two variants.
- `--take-in` cannot update a store; the console press can. An operator who does not open the
  console has no route to a stale preset in this build.
- `docs/manual/operations.html`'s take-in row and `docs/manual/console.html`'s `presets` scope tip
  and note state the exception.
- `karakuri/src/tests/mod.rs`'s `a_preset_whose_id_this_store_holds_is_refused_rather_than_overwritten`
  is replaced by `a_preset_this_store_already_holds_is_loaded_rather_than_refused` and
  `a_preset_the_library_has_moved_on_from_is_replaced_and_the_old_one_kept`. The second copies
  `examples/` and edits a `.kir` in it, which is how a library and a store's copy diverge in
  practice.
- The folder refusal is now asserted at the press, in
  `a_folder_row_is_taken_in_by_the_same_press_a_preset_row_is`. At the `unbundle` level
  `an_unbundle_refuses_an_id_already_taken_and_leaves_the_set_alone` is unchanged and is joined by
  `a_shipped_row_replaces_a_taken_id_and_what_was_there_is_kept` and
  `a_shipped_row_this_store_already_holds_writes_nothing_and_says_so`.
- All four new tests were watched to fail against the rule they replace: with `came` forced to
  `Somebody` the replacing tests meet the old refusal, and with it forced to `TheShippedLibrary`
  the folder assertion fails.
- The three Sets that prompted this are not repaired by the change. Pressing each row once repairs
  it and files `drift_cloud-<stamp>`, `beat_threads-<stamp>` and `beat_glow-<stamp>` in `my sets`.

## Evidence

- `--take-in` of each `.kset` into a fresh store, then `--load-set` of each: renders at 262144,
  81920 and 81920 elements.
- `grep -l point_size .karakuri/*.kir` returns three artifacts; `grep -l point_size examples/*.kir`
  returns none.
- Run against a copy of the affected store: the three are replaced with the old copies kept, a
  fourth (`swirl_cloud`) reports already current and writes nothing, and all three render.
