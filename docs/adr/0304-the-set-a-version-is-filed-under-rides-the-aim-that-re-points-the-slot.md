---
id: 0304
title: The Set a version is filed under rides the aim that re-points the slot
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0087, 0096]
tags: [store, history, environment, panel]
---

# The Set a version is filed under rides the aim that re-points the slot

## Context

The panel program did not write an edit history at all. `mcp::write_procedure` ran
`compile::check`, wrote the file and touched nothing else; `crates/karakuri/src/main.rs` never
constructed `history::Snapshots` and never called `record`. Only `karakuri-cli` did — so a
`--watch` run accumulated versions under `<store>/history/` and a panel run, **which is where MCP
is served from**, accumulated none. The one surface where something other than the operator's own
hands writes a procedure was the one surface keeping nothing
([P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md), whose
*every version that compiles is kept* is the rule, and
[ADR-0089](0089-history-is-gated-on-compiling-not-on-landing.md), which puts the gate at the
watcher's compile-success point because a hand at an editor and a model's write both pass through
it).

Wiring it is three calls — `Snapshots::shared`, `seed`, `snapshotting_to` — and the whole of the
decision is in the fourth thing:
[ADR-0276](0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md)
files a version under the Set the slot is running, and **this program is the surface that record's
Consequences named**: *"A surface that loads a Set into a running slot has to move the id with the
re-point, or every version after the load is filed under the Set before it."* `l` over the Library
bay is exactly that load. `karakuri-cli` never had the problem — its only id is `--load-set`'s,
settled before the deck is built.

So: a slot launches on the pair the command line settled, which is no Set at all, and becomes a Set
the moment somebody loads one. **Where does that answer live?**

## Decision

**It is a field of `watch::Aim`** — `set: Option<String>` — moved into the watcher by
`Watch::repointed` along with the files, the layering, the fold, the salts and the camera, and
restated by `restated` on every re-aim.

- `crates/karakuri` builds **one** `history::Snapshots` for the run in `main`, seeds every slot's
  working copies into it under `None` before the window opens, and hands it to every watcher
  through `Engine::new` and `watched`.
- `loading` sends `set: Some(id)` on the aim that re-points the slot, beside the scratch files it
  has just written. Nothing else in the program writes it.
- `Watch` keeps it inside the `snapshots` pair, which is its only reader.

**One holder per program, and the compiler is what says so.** `Watch::repointed` destructures an
`Aim` with no `..` and `restated` rebuilds one with no `..` at either end, so a field added to an
aim cannot be left behind by the watcher that receives it or defaulted by the surface that sends
it. That was already the rule for the other thirteen fields; the id joins it rather than being
carried beside it.

## Alternatives, and why they lost

### A per-slot `Option<String>` beside the deck, written where `played` writes the readout

This is what the plan said to build, and it is the losing option. It is one holder too many.

**The watcher cannot read it.** A watcher runs on a worker thread and the only thing that reaches
it mid-run is an aim — that is `Watch::aimed_by`'s whole purpose, and `Deck::install` is
deliberately unreachable from a surface. So a field on `Gfx` would have to be *sent*, on the aim,
at the same moment the files are; it would be a copy of a value that had already travelled, kept in
step by hand.

**And the two copies part company at the second route.** `played` is the load. `wire_input` is not:
a rewiring restates an aim through `Aiming::at` and never passes through `played` at all, so the
`Gfx` copy would have to be read again there — a second derivation of *what is this slot running*,
in the one place a stale answer is silent, which is what
[P-0087](../principles/0087-name-the-property-never-the-shape.md) rules out. `Aiming::at` is
already the answer to *where is this watcher pointed*, kept for exactly this reason for the other
fields, and adding the id to it costs nothing new.

**The symptom of getting it wrong is not a picture.** Every other field on an aim comes back as a
wrong camera or a repainted element — visible, eventually. This one comes back as a version filed
under the wrong Set, in a directory nobody opens until after the show, and `history::list` opens no
file, so the name is the whole of the row and there is nothing to check it against.

### `Gfx::material`, which is already one name per slot and rewritten on the load

Refused, and ADR-0276 refused it first. That field is the mixer strip's **readout**: at launch it
is `"coil_vortex + star_flares"` — the pair, not an id — so filing versions under it would put rows
in the history under a Set no listing can ever match. It is also written *after* `loading` returns,
which makes it a report of a load rather than part of one.

### Work the id out at the write, on the worker thread

There is nothing to work it out from. A watcher holds paths, and the paths are scratch copies named
`A0-<proc>.kir` — the deck letter and the node's place, which is what keeps two decks from sharing
a file and says nothing about which Set the node came from. A content hash of the source is an
address and not an identity, and `--watch` moves it on every save. The id is known at the moment of
the press and nowhere else, which is `Snapshots::record`'s own argument for taking it rather than
deducing it.

### A field on `Watch` beside `head` and `rest`, rather than inside the `snapshots` pair

The narrower version of the same test, and it loses on it. The history is the id's only reader: a
watcher given no store keeps no history, and a field that means nothing whenever `snapshots` is
`None` is a second answer to *what is this slot running* with nobody asking the question. Inside
the pair there is exactly one holder, it exists exactly when it has a reader, and a re-point that
arrives at a watcher keeping no history drops it, which is the right thing to do with an answer
nothing will ask for. `Watch::new` also keeps its argument list: the id arrives through
`snapshotting_to`, where ADR-0276 put it.

## What a run writes now

**A panel run launches under no Set**, on every slot, and that is the true answer rather than a
placeholder: the material is two paths somebody typed. A library load turns one slot's answer into
the id the operator picked out of the bay, and the versions written after it are filed under that
id — the first of them on the load itself, because the dedup key is `(slot, layer, index, set)` and
a Set this slot has never played has no `last` entry to be skipped against.

**A save still does not move it**, unchanged from ADR-0276: a save copies what is playing into the
library under a new id and does not move the slot, so a Set a save created has no versions under
its own id until somebody loads it.

**A window remade goes back to the launch pair**, deck and readout together, and the aims with
them — so a slot's id returns to `None` along with the material it names. The run's `Snapshots`
does *not* go back: it lives on `App`, so the dedup memory survives a remake and the first rebuild
after one does not file every untouched procedure a second time.

## Consequences

- `watch::Aim` carries `set: Option<String>`. `Watch::repointed` moves it into `Watch::snapshots`'s
  pair; both that destructuring and `restated`, in `crates/karakuri` and in `karakuri-cli`, still
  have no `..`.
- `crates/karakuri/src/main.rs`'s `seeded` builds the run's one `Snapshots` and files every slot's
  working copies into it under `None`, called from `main` before the window opens; `Engine::new`
  and `watched` take the `Shared` and every watcher is given `snapshotting_to`. It is a free
  function for `running_from`'s reason — `main` cannot be entered from a test and the seeding is a
  claim worth asserting. `watched`'s documentation said *"`Watch::storing_to` **is** given now and
  `Watch::snapshotting_to` is not"* and now describes what it does
  ([ADR-0031](0031-a-document-describing-replaced-behaviour-is-worse-than-none.md)).
- `karakuri-cli` reads the id for `snapshotting_to` off the aim it has just built rather than
  matching on the slot a second time at that line. Its launch aim states it, and nothing on that
  surface moves it.
- `mcp::write_procedure`'s *"The version it replaced is in the run's edit history under
  `<store>/history/`"* is true on the panel now; it was written for a run with a `Snapshots` and
  the panel had none. Its `--watch`-conditional sentences are unchanged and still true: they
  describe a run with no watcher, and this program passes `watching: true` because every slot of it
  is watched.
- **Nothing walks the history yet.** `Operation::WalkHistory`'s payload is still `Undecided`, no
  route is added, and no badge moved — recording is not an operation. What that row waits on is the
  manual naming a control, which is M5.3's.
- `docs/manual/operations.html`'s *Walk the edit history* tip said *"only the command-line player
  takes the snapshots today — the panel program is handed no snapshotter"*, and says what both
  programs do instead. `docs/roadmap.md` carried that claim in M5.10's *Every write that compiles is
  a version*, in M5.3's row list, its task 2 and its *what is not here*, in the Staging lane's
  blocked-on list and in *Rows the manual has not given a home*, and each says what is true now and
  what is still owed there — which in every case is the control that reads them.
- `watch.rs`'s `a_re_point_files_the_versions_after_it_under_the_set_it_loaded` is the test: a
  watcher snapshotting under `None` is re-pointed with an aim naming a Set, and the version it then
  writes is read back off `history::list` under that Set. It was watched to fail with the move left
  out of `repointed` — the rows come back `set: None` — before it was kept. `crates/karakuri`'s own
  load and rewiring tests assert the id on the aim `loading` sends, on the aim it keeps, and
  through `restated`, and `the_launch_versions_are_filed_before_a_window` asserts the seed — every
  node of every slot, filed under no Set, before there is a window — watched to fail against a
  `seeded` that returns the history without filing anything.
