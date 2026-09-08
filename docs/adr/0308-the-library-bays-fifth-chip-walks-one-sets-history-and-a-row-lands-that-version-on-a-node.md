---
id: 0308
title: The Library bay's fifth chip walks one Set's history, and a row lands that version on a node
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0082, 0090, 0091, 0096]
tags: [console, ui, library, operations, store, history]
---

# The Library bay's fifth chip walks one Set's history, and a row lands that version on a node

## Context

*Walk the edit history* was the last row of *The library* with a `plan` panel badge over a cell
reading `—`, and everything under it was built. `karakuri_environment::history::list` reads
`<store>/history/` back, most recent first, opening no file
([ADR-0276](0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md)),
and a panel run writes into it: one `Snapshots` for the run, seeded before the window and handed to
every slot's watcher, with the Set a version is filed under riding the aim a library load sends
([ADR-0304](0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md)).
[roadmap.md](../roadmap.md)'s M5.3 task 2 named what was left in one clause: *"the manual naming the
control and the bay drawing it"*.

**The shape was named there too**, and this record is what it cost to build rather than a choice
between shapes: a fifth scope chip beside `all`, `my sets`, `presets` and `folder`; rows that are a
Set's versions; the order stated in the row's own sentence; landing on a row is a load; and **a
narrowing must treat a `None` row as matching no Set rather than as a wildcard**.

**What the page said and could not both mean.** *Walk the edit history*'s tip carried two sentences
that pull apart once a control exists: *"Landing is not this row either — it is put a node's
previous version back, under Procedures. This one is the list"*, and *"A walk needs a cursor and a
direction, or a count of steps, or a revision to land on, and the answer among the three is a
revision to land on"*. If landing is the other row, the revision belongs to the other row. Deciding
which of the two carries a payload, now that a surface can say one
([ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)), is
the first half of this record.

## Decision

**Four things, and the page moved first (`docs/contributing.md` §5).**

### 1. `history` is a fifth scope chip, and it asks its own row

`view::Scope::History` joins `Scope::ALL`, last, and `console.html` draws it. A press marks it
exactly as a press on the four beside it does, and what it **asks for** is
`Operation::WalkHistory { step: Undecided }` where the other four ask for
`Operation::SelectScope { scope: Undecided }`.

**The payload is what settles that.** `SelectScope` cannot say *which* chip — deliberately, because
the scope list grows when a directory is added — so a `SelectScope` emitted here would be
indistinguishable from a press on `all`: a record saying *a library was chosen* for a press that
asked for an edit history. One press names one operation, and the one it names is the row that
describes what was asked for.

### 2. The rows are one Set's versions, and the Set is the pulldown's deck's

The listing is narrowed to the Set the **load pulldown's** deck is running
([ADR-0305](0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)'s
`View::target_deck`), read on the host off `engine.aimed[slot].at.set` — the id ADR-0304 puts on the
aim. Most recent first, which is `history::list`'s own order and not one applied at the bay
([ADR-0263](0263-the-library-bay-lists-most-recent-first-because-the-listing-is-the-operations-and-not-the-surfaces.md)
read on a second listing).

**A `None` row matches no Set.** A version written where the slot was running nothing is a version
of nothing, so a deck playing the pair the run launched with lists **none** of them rather than all
of them, and `why_nothing` says so in words: the deck is playing a typed pair rather than a Set, so
its versions are filed under none.

**A row is one line in the name column.** The mock draws a `.dim` time beside each Set and the panel
draws none for want of a spelling; a history row does not get one either, because a row of this
scope and a row of `all` are rows of one list and a column drawn for one of them would make them two
kinds of thing. So the row is the name the store filed the version under, less the `@<set>` every
row of one walk shares — `20260908-143052-271_slot0_L4_beat_strokes`: when, which slot, which layer
and index, and what the procedure called itself, which is what `history::Version`'s own head says a
row is for.

### 3. Landing is *Put a node's previous version back*, and its payload is a `Revision`

```rust
pub enum Revision {
    Previous(NodeAt),
    Picked(String),
}
RestoreProcedure { deck: u8, revision: Revision }
```

**Two arms because two surfaces can ask and each says a different half** (ADR-0192). A staging lane
row is a node with one unsettled version on it: it names the node and means *the one this replaced*
— one step, never a cursor — and it holds no listing to pick out of. A row of this walk is a version
an operator picked, and what that row carries is the name the store filed it under, which **is** the
node's address as well as the moment.

**A press on a row emits `RestoreProcedure { deck: the pulldown's, revision: Picked(the row) }`**,
and the host writes that snapshot's bytes over the node's working copy under `<store>/scratch/`. The
watcher already looking at that file builds it, swaps it at a frame boundary and rolls it back on
its own if it cannot hold the budget — the path an edit already takes, so **nothing is installed**
([ADR-0228](0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)). **The
version it replaces is kept by the same act**, because the rebuild compiles it and compiling is the
gate ([ADR-0089](0089-history-is-gated-on-compiling-not-on-landing.md)) — so a landing you did not
mean is itself a row of the listing you landed from.

### 4. `WalkHistory` keeps `Undecided`, and the three shapes are answered rather than open

A cursor and a direction is the console's own mark, which has no row on the page at all; a count of
steps is something no surface offers; and a revision to land on is the landing's, where it now is.
What is left for a walk to carry is *which history*, and no surface spells it: the id rides the aim,
and the console holds a deck letter rather than a Set id.

## Alternatives rejected

**Narrow to the *selection* rather than to the pulldown's target.** The cheaper reading, and the one
a reader who has not read ADR-0305 will assume: the ring on a strip is the deck mark this console
has always had. It loses to what the two marks are for. The whole of ADR-0305 is that the deck a
**load** lands on and the deck the **keys** are addressed to are two values, so that deck C can be
prepared while deck A is playing; a listing narrowed by the ring would put the history of the deck
being played in front of an operator preparing another one, and the landing — which lands on the
pulldown's deck — would then be aimed at a deck the rows above it were not about. One mark decides
both, and it is the one already drawn in that bay's own foot.

**Narrow by the deck rather than by the Set.** The variant of the same mistake one level down, and
it is what a `WalkHistory { deck: u8 }` payload would have asserted. It is a wrong answer rather
than a missing one: `history::list` narrows on `Version::set`, and a version of Set `x` written
while `x` ran on slot 2 is a row of `x`'s history whoever asks. Two decks running one Set have one
history between them. The deck is how this console arrives at an id and never what the listing is
about — the same distinction ADR-0304 drew when it refused `Gfx::material`.

**A version list that opens a file per row.** Tempting, because a row could then say what the
version *is* rather than what it was called — a diff, a line count, the first line of the source. It
is refused by the one property `history::list` is built on: it walks day directories newest first,
stops entering them once it has enough, and **opens no file at all**, which its own head calls the
shape of the thing rather than a shortcut. A read per row puts a file open on the path a scope press
takes, which is [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) run the wrong way,
and it buys nothing the name does not already carry: ADR-0276 put the moment, the node and the Set
in the name precisely so that a row is a name.

**Landing as `WriteProcedure` carrying the bytes.** The operation that already exists, and it would
need no new payload at all. It loses to the same argument the staging lane's route lost to, at
`Operation::RestoreProcedure`'s own definition: *"A write takes a `source: String` because whoever
asks for one is holding the text"* — a model has it in the conversation, an editor has it in a
buffer — and **this surface holds neither**. A bay whose rows are names would have to read four
kilobytes of IR off a disk to build the ask, which puts the store inside the console (ADR-0156) and
makes a press pay for a file open twice: once to fill the payload and once to write it.

**A `version: Option<String>` beside the existing `node: NodeAt`.** The smaller change, and it
keeps one shape for both surfaces. It is refused because it can spell a disagreement: a version of
`L4:0` addressed to `L2:1` is a payload with two answers to *which node*, and which one wins is
whichever the performer happens to read. The enum puts the address inside the arm that owns it, and
the disagreement cannot be written down.

**A third operation for the landing.** *Land on a version* as its own row. Refused on
`docs/contributing.md` §5's terms and on the page's: the row that says what this does already exists
and already says landing is its own — *"Landing is not this row either — it is put a node's previous
version back"* — so a third row would be the page contradicting itself to avoid changing a payload.

**A path in the payload.** `Revision::Picked` carrying `PathBuf`, which is what the performer
actually needs. Refused by the rule the one path in the vocabulary already obeys: a path may sit in
a payload only where every route that fills it derives it from something the program produced. The
console reads no store, so it would be spelling one. The name is matched back against the listing
that produced it, which is `SetTransfer::Take`'s own arrangement.

## Consequences

- **`karakuri-operation` gained `Revision` and `RestoreProcedure` changed shape**;
  `RestoreProcedure { deck, node }` is now `RestoreProcedure { deck, revision }`. `gate.rs`'s class
  is unchanged (`Standing::Open`) and so is `karakuri-operation-record`'s answer
  (`Silent(OnLanding)`), because neither reads the payload. `Operation::TITLES` and every title
  string are untouched, so `the_manual_and_the_vocabulary_agree` is unaffected.
- **`Operation::WalkHistory`'s payload is unchanged and its doc is rewritten.** It said *"no surface
  can say a revision yet"*; a surface can, and the revision is on the other row.
- **Two panel badges moved to `has`**: *Walk the edit history* reads `panel library history chip`
  and *Put a node's previous version back* reads `panel library history row`. The second was
  `plan staging`, and **the staging lane's own route is still unbuilt** — the badge names the route
  that exists and the tip carries the other. That is a second instance of M5.3's known exit defect:
  a row grouped under one bay whose `has` came from another. M5.9's *Rows* paragraph says so.
- **`view::Scope` has five members and `Scope::ALL` is five long**, so `input::PROBES`' scope row
  claims one more control and the legend `crates/karakuri/src/main.rs` prints rises with it — the
  count is `Scope::ALL.len()` and was never a typed four.
- **`view::Scope::lists_sets`, `View::sets` and `View::versions`** are how four existing controls
  stay right: the star, the `read` chip, the `load` button and the carry all read a row as a Set id,
  and under this scope they are handed an empty listing, which is the refusal each of them already
  had. `input::claim` keeps **one** row for the list's rectangle, and `bay.take` and `bay.land` are
  told apart by which of the two listings goes in with the point rather than by an arm asking the
  scope.
- **`l` says so rather than loading a version**, and so does a reading left open when the scope is
  marked. The key loads a Set and a row here is not one; the arm names the press that does land one,
  and `read_reading` puts a stale block away rather than trying to read a version's name as a Set
  id.
- **`karakuri_environment::mcp::Slots::file` is public**, taking the layer as the word a snapshot's
  name spells it with. It delegates to the private `path`, so turning `(slot, layer, index)` into a
  file stays one walk. `crates/karakuri`'s `restored` builds the `Slots` it hands `put_back` **out
  of the aims** — `Aiming::at`, where each watcher is pointed now — rather than out of the launch
  copies, so the answer follows a library load.

  > **Annotated 2026-09-08.** **The stale half is fixed and this second `Slots` is gone**
  > ([ADR-0309](0309-a-slots-files-are-published-where-its-aim-is-sent-and-the-server-reads-them-live.md)).
  > Building one here was the workaround for the MCP server's own being the launch working copies,
  > taken before the window and never written again — so a model reading, writing or rewiring a deck
  > a Set had been loaded onto was answered about the material that deck had stopped running, and
  > answered *successfully*. `mcp::Slots` is now a shared handle the host writes on every re-point
  > and the server resolves through on every call, `Aiming::publish` is the one place that writes
  > it, and `restored` reads `Engine::pointing` rather than deriving a second one. What this
  > consequence says about `Slots::file` being public and delegating to `path` is unchanged.
- **The cap is on the walk and not on the Set.** `HISTORY_MOST` rows are asked for and the narrowing
  happens after, so a store whose day directories hold several Sets' versions lists fewer of each.
  `Listing::stopped_short` and `Listing::unclaimed` are said out loud beside the count rather than
  left for the foot's `n of m` to imply.
- **A landing is refused on five counts and each names what is in the way** (P-0083): a slot the
  deck has not got, a deck playing no Set, a version that is not one of that Set's, a node that deck
  does not hold, and a file that is gone or will not be written. The fourth is ordinary — `rm -rf
  history/2026/07` is this store's whole retention policy.
- **`view::Scope::Presets`' doc said a preset load *"leaves a row in `Scope::MySets`"*** and now says
  `Scope::AllSets`, which is what
  [ADR-0299](0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md) decided:
  `my sets` is the starred subset, so a take-in appears there only if somebody presses the star.
  Corrected in passing rather than as part of this decision.
- **Six tests hold it, and five existing ones had their premise moved.**
  `crates/karakuri-console/tests/library.rs` carries four new ones — the chip is fifth and asks for
  the walk, a listing is the rows the host handed in with the foot counting them, a press on a row
  lands that version on the pulldown's deck while the carry answers nothing, and a deck running no
  Set draws no rows. `crates/karakuri`'s two are CPU tests:
  `a_history_listing_is_one_sets_versions_and_a_none_row_is_nobodys` and
  `a_landing_writes_the_versions_bytes_over_the_nodes_working_copy`. Each was watched to fail —
  against a chip emitting `SelectScope`, a reversed paint, a foot counting `rows` twice, a landing
  hard-coded to deck 0, a `View::versions` with no scope in it, a `listed` that ignored the total, a
  `None` row read as a wildcard, a walk that listed under no Set, a landing resolved to the wrong
  node, and a missing file swallowed.
- **`karakuri_environment::history` did not change**, and neither did what a run writes. This is the
  reading half only.
