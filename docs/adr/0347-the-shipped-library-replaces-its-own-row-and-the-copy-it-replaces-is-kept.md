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

**Three preset rows could not be loaded, and nothing in the program could say why.**
`examples/drift_cloud.kset`, `examples/beat_threads.kset` and `examples/beat_glow.kset` are three of
the twenty-three Sets that ship in `examples/`. Pressing any of them in the Library bay refused;
loading the store's copy of one instead failed in the checker:

```
contract: `point_size` is now spelled `point_rate`
contract: `point_rate` is not assigned on every path through `vertex`
```

Neither the files nor the parts they name are wrong. `--take-in examples/drift_cloud.kset` into a
store that has never held it succeeds, and the Set renders. What is wrong is the *store*, and the
dates are the whole of it:

| | |
|---|---|
| 2026-08-31 | `6f64027` adds the twenty-three `.kset` files |
| 2026-09-01 | `drift_cloud`, `beat_threads` and `beat_glow` are taken in |
| 2026-09-02 | `96c9cc7` renames the renderer output `point_size` to `point_rate` |
| 2026-09-09 | the other twenty are taken in |

The three that were taken in first hold sources written the day before the rename. The twenty taken
in after it hold sources written after it. Nothing repaired the first three and nothing could:
pressing the row again is the only route from the library into the store, and it refuses.

**A take-in is a snapshot, and that is not the defect.** A `.kset` names its parts by relative path;
the `.kbset` this store writes names them by content
([ADR-0231](0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md)).
The two part company the moment a part on disk is edited, and they are *meant* to: a deck's material
must not change under it because somebody saved a file, and the resolved form is what makes a swap
atomic. Nothing here proposes that the store track the library.

**The defect is that the snapshot could never be retaken.** `setfile::unbundle` asks what the store
holds before it writes a byte and refuses a taken id, and the reason it gives is a good one:

> an id you type is an instruction, and an id that arrived inside somebody else's file is not.
> Overwriting on a name you chose is you replacing your own preset; overwriting on a name a
> stranger's file chose is a preset an operator built disappearing because somebody they have never
> met picked the same word.

Every clause of that is true of the `folder` scope, which lists a directory somebody dropped on the
window ([ADR-0275](0275-a-folder-is-chosen-by-dropping-one-on-the-window-and-the-drop-is-the-windows-rather-than-a-bays.md)).
None of it is true of `presets`. That scope is the library this run resolved at startup and printed
the answer for ([ADR-0230](0230-where-the-programs-data-lives-is-told-rather-than-baked.md)); the id
in the file is the word the row was already listed under; and the file is not a stranger's, it is
this program's own installation. One press reaches both scopes —
`docs/manual/operations.html`, *"Taking one in is not a second row — opening a preset is this
row"* — and `Taking::Presets` and `Taking::Folder` were handed to `unbundle` with the difference
between them dropped.

So the rule was applied one scope too wide, and what it produced there is the opposite of what it
was written to protect: the shipped library became the one library in the program that could never
be updated, and the sentence it refused with — *"Edit the `set` record's id, or move the set you
have"* — asks an operator to edit a file inside the installation.

## Decision

**Where the file came from is an argument at the call site, and it decides what a taken id means.**
`setfile::unbundle` takes a `CameFrom`, in the shape
[`Asked`](../../crates/karakuri-environment/src/lib.rs) already has for the same class of question:
a value every caller must supply, so that no route reaches the permissive branch by saying nothing.

- `CameFrom::Somebody` — a `.kbset` that arrived, a row of a dropped folder, a path typed at
  `--take-in`. A taken id is refused, in the sentence it was always refused in, before a byte is
  written. Unchanged.
- `CameFrom::TheShippedLibrary` — a row of the preset library this run resolved. A taken id is
  **replaced**, and the copy that was there is **kept** first.

**What is kept is a Set and not a diff.** The held Set file is read and written back under
`<id>-<stamp>` — `drift_cloud-20260913-212700-004` — using `history::stamped_id()`, which is
already this store's spelling for *a thing an operator will look for by when it happened*, and its
`set` record is rewritten to the retired id so the file does not carry two answers to what it is
called. It lands in `sets/` and is a Set like any other: it lists, it loads, it can be starred.
Nothing is deleted, and the plain id then names what the library ships today.

**An unchanged preset writes nothing and loads.** The records that would be written are compared
with the records that are there, and where they are equal the take-in reports that and stops
without writing a file or retiring anything. This is what keeps the decision from filling `my sets`
with dated copies: a retired copy appears exactly when the shipped material actually moved.

**A press on a preset row now always ends with that preset loaded**, which it did not before —
`taking_in`'s refusal meant nothing was taken in and so nothing was loaded, and that is the sentence
an operator met on all three of these rows.

## Alternatives rejected

**Keep both under two ids, the shipped one taking a new name.** This is the shape the question
arrived in — *"両方残ってても良い"* — and the decision keeps its substance while inverting which side
takes the stamp. If the new one is filed under a stamped id the plain word goes on naming the stale
copy, so the row an operator presses is still the one that will not load, and the defect survives
its own fix. The word has to name what ships.

**Retire the old version into `history/` instead of `sets/`.** `history/` is the closer-sounding
home and the wrong one. `Snapshots::record` takes a slot index, a layer, a renderer index and the
Set the slot was running, and writes all four into the name
([ADR-0276](0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md),
[ADR-0304](0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md)) — it is
a version *of a node of a running arrangement*, and a take-in has no run and no slot. Worse, what it
holds is one procedure's source: a Set retired into it could not be got back as a Set, which is the
one thing keeping it is for. Writing a slot-less name into that directory would have meant a second
naming form in the module whose whole discipline is that a name is read rather than a file opened.

**Compare against the library at startup and repair silently.** A store that reconciles itself with
`examples/` on every run is a store whose contents change without an operator acting, and the
material a deck loads would move under a Set the operator had edited under that id. The press is the
act; nothing happens without one.

**Let `--take-in FILE` replace too, so the CLI can repair a store.** `karakuri-cli` has no
`--presets` and no notion of a resolved library, so it cannot tell the shipped file from any other
path — the permission would be *any typed path may overwrite any id*, which is the rule this record
narrows rather than widens. The CLI keeps the refusal it had; the console press is the route, and
that asymmetry is a consequence below rather than a thing hidden here.

## Consequences

- `setfile::unbundle`'s signature gains a `CameFrom`. Its two callers answer it:
  `karakuri-cli/src/args.rs` with `Somebody`, and `karakuri/src/bridge/filesystem.rs`'s `taking_in`
  by asking the `Taking` it was already handed.
- `Taking` gains `came_from`, which is the first thing that has ever read the difference between its
  two variants for anything but a noun in a refusal.
- **`--take-in` cannot update a store and the console can.** An operator whose store holds a stale
  preset and who does not open the console has no route to it in this build. Nothing depends on it
  and no roadmap item is owed; it is written here so the next person meets it as a decision.
- `docs/manual/operations.html`'s take-in row and `docs/manual/console.html`'s `presets` scope tip
  both state the exception, and `console.html` gains a note arguing it.
- `karakuri/src/tests/mod.rs`'s
  `a_preset_whose_id_this_store_holds_is_refused_rather_than_overwritten` held the old rule at the
  press and is gone. `a_preset_this_store_already_holds_is_loaded_rather_than_refused` and
  `a_preset_the_library_has_moved_on_from_is_replaced_and_the_old_one_kept` are what stand there
  now, the second driving a copy of `examples/` it then edits, because editing a `.kir` beside a
  `.kset` is how a library and a store's copy of it come apart in practice.
- The refusal is now asserted at the press as well, inside
  `a_folder_row_is_taken_in_by_the_same_press_a_preset_row_is`. It was not before, and the press is
  exactly where the two scopes were being confused: `unbundle`'s own
  `an_unbundle_refuses_an_id_already_taken_and_leaves_the_set_alone` stands unchanged and is the
  `Somebody` half at the other level, joined by
  `a_shipped_row_replaces_a_taken_id_and_what_was_there_is_kept` and
  `a_shipped_row_this_store_already_holds_writes_nothing_and_says_so`.
- All four new tests were watched to fail against the rule they replace, in both directions: with
  `came` forced to `Somebody` the two replacing tests meet the old refusal, and with it forced to
  `TheShippedLibrary` the folder assertion fails.
- The three Sets that prompted this are not repaired by the change itself. Pressing each row once is
  what repairs them, and doing so files `drift_cloud-<stamp>`, `beat_threads-<stamp>` and
  `beat_glow-<stamp>` in `my sets`.

## Evidence

- The three failing Sets, and that the files are not at fault: `--take-in` of each `.kset` into a
  fresh store, then `--load-set` of each, renders — 262144, 81920 and 81920 elements.
- The stale sources are in the store and nowhere else:
  `grep -l point_size .karakuri/*.kir` returns three artifacts, and
  `grep -l point_size examples/*.kir` returns none.
- The dates above are `git log -1 --format=%ad -- examples/<id>.kset`, the mtimes of
  `.karakuri/sets/*.kbset`, and `96c9cc7`.
