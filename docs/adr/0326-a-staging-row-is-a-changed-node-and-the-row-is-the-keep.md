---
id: 0326
title: A staging row is a changed node, and the row is the keep
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0083, 0087, 0090, 0094]
tags: [console, ui, staging, operations, store, m5]
---

# A staging row is a changed node, and the row is the keep

## Context

`roadmap.md`'s M5.7 carried one undecided item and it was item 4:

> **An undecided answer: what the lane draws when one build changes two nodes.** A rebuild
> restates the whole stack, so a save touching two files is one `Built` with two changed hashes
> and one `swap::Event`. One row cannot name two nodes, and two rows means the lane's row model
> stops being one-per-slot — which is what `staging()` and `view::Candidate` are built on. **A
> decision and not an implementation, and it is the only one left in this list.**

Everything under it was built. `karakuri_environment::watch::Built` carries `(layer, index, hash)`
for every node of a rebuilt slot; the channel is made, drained and matched to its swap; and
`Playing::at_launch` seeds each slot from the material the run compiled, so a first edit has
something to be compared against. What was missing was one thing rather than two: **nothing
compared the two lists**, and the place where both exist at once is one call — `Keeping::took_up`,
where the build that just landed replaces what the slot was on.

The lane's whole reason for existing sharpened while that sat open.
[ADR-0310](0310-a-source-can-say-it-refused-and-the-lane-draws-it.md) gave it the checker's
refusal and said what it still could not say: *"What is still not said is the node."*
[ADR-0316](0316-an-over-budget-candidate-stays-in-the-slot-and-the-slot-stops-updating.md) made an
over-budget candidate stay in the slot with the slot stopped, and named the way out — *"an earlier
version, landed from the Library bay's `history` scope"* — while `console.html` recorded that the
row reporting the fault could not offer it: *"What this row cannot yet offer is the second of those
as a press — put a node's previous version back names a node, and a node is what a swap event has
not got."*

And the operation that would take it existed with one producer.
[ADR-0308](0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)
split `RestoreProcedure`'s payload into `Revision::Picked` and `Revision::Previous` precisely
because two surfaces can ask and each says the half it holds; the Library bay filled the first arm
and the second had no producer in this binary at all.

## The maintainer's decision

The maintainer, on 2026-09-09:

> 変更ノードごとに 1 行

*One row per changed node.*

## Decision

**A candidate row is one node a build changed. Four points.**

### 1. One build that changes two nodes draws two rows, and the verdict is repeated on each

A rebuild restates the whole stack and a `swap::Event` is about the build, so a save that touched
two files is one build with one verdict. That verdict is written on **both** rows. Each row carries
the node's address — `L4:0`, the address the Inspector already writes on a node head — and what
that node's procedure calls itself, which is `Set::node_names`' entry rather than the build's label.

**Simplicity over a nested shape**, and the repetition is what it costs.

### 2. The changed set is a diff, and it is taken where both lists are in one hand

`Keeping::took_up` receives the build's per-node hashes and overwrites what the slot was playing.
For the length of that call both lists exist; afterwards one of them is gone. So the diff is taken
there, and `staging` calls it inside its own drain rather than leaving the caller to take the build
up afterwards. A node whose hash differs — or that the old list did not have — is a changed node; a
node the rebuild removed is not reported, because a lane row is a version somebody has still to rule
on and a node that is gone has none.

### 3. A verdict with no changed node is one row on the slot, with no address

Three states, one answer. `Event::Rejected` and `Event::SourceRefused` are builds that did not
happen, so there is no new list to compare. A rebuild that restated the stack unchanged — a
composite press or a residency re-aim, which rebuild from files nobody edited — has an empty diff.
And a build whose sources the store would not take has no list either, which the watcher says at the
time.

Such a row is still **drawn**: a re-aim that stops the slot for cost is exactly the thing this lane
exists to say. `view::Candidate::at` is `Option<NodeAt>`, the address column is empty, and the row
offers neither press.

### 4. The row is *Keep a candidate* and the capsule at its end is *Put a node's previous version back*

The Library bay's list one bay up, arranged the same way: a row that is itself a control, with a
smaller box inside it asked first — there the star, here a `back` capsule. What decides which act
goes on which target is what each costs. **Keeping moves nothing**: the version stays where it is,
the store holds every version it held, and what leaves is a line in a list. **A step back writes
over the operator's working copy and rebuilds the slot.** The free one takes the large target and
the one that writes takes the small one.

- `KeepCandidate { deck, node }`, silent — `Written::Silent(Silent::Surface)`, unchanged. Not
  offered on `Stage::Overloaded` (ADR-0316: a slot that has stopped is not a candidate anyone is
  choosing between) and not offered on a row that names no node.
- `RestoreProcedure { deck, revision: Revision::Previous(node) }`, which is the producer ADR-0308
  said was missing. Offered wherever the row names a node, the overloaded row included — landing an
  earlier version is one of the three ways out of a stopped slot, and this is the only one of the
  three this bay can offer.

**`Previous` resolves to the entry after the newest**, in `history::list`'s order, narrowed to that
node of the Set the deck is running. The newest is the version the slot is running — the history is
gated on **compiling** and not on landing
([ADR-0089](0089-history-is-gated-on-compiling-not-on-landing.md)), so a version stopped for cost is
filed too and is exactly the one an operator wants to step away from. A node whose only version is
the one it is playing is refused in a sentence saying so
([P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)). The write, the
address resolution and the refusals after that point are the same code `Picked` takes: **one
function and two ways of naming the file.**

**The capsule's word is `back` and not `keep`.** This console already spends `keep` on the Inspector
pane head's capsule, where it names *Keep what a deck is playing* and writes a Set into the library.
Two capsules reading `keep` and meaning two acts is `docs/contributing.md` §4's *a name meaning two
things*, and the expensive misreading is the one where a person presses this expecting a save. The
row's own act needs no word, because the control is the row.

## Alternatives rejected

**One row per slot, with the changed nodes as sub-rows under it.** The shape truest to where a
verdict comes from: the verdict is over a build, a build is a slot's, and the nodes are what the
build changed. It is what a nested reading would draw, and the maintainer named the alternative and
took the flat one — *simplicity over a nested shape*. What it costs is a second kind of thing to
read in a bay whose whole job is to be read at a glance mid-set, and a second set of rules for how
tall a lane is: `lib.rs` pins this bay at 125 because the mock draws three `.cand` rows and two
gaps, and a row that is sometimes a header and sometimes a row makes that arithmetic a function of
the content. **The repetition it avoids is one word per row**, and that word is the one thing in the
row that must stay readable anyway.

**One row per slot naming both nodes.** `L1:0 + L4:0`, in the space the address column has. It keeps
the row model and it is one line, so it looks like the cheap answer. It loses on what a row is
*for*: the row is what the two operations press on, and both are spelled `{ deck, node }` — so a row
naming two nodes could be drawn and could not be pressed, or would have to press on both at once,
which is a keep an operator did not ask for on a node they have not looked at. It also does not
scale past the width: a slot holds a geometry, a chain, a camera and several renderers, and a
rebuild can change any set of them.

**Leaving the row addressed by the slot and putting the node in a tooltip.** Cheaper than either,
and it would have closed item 4 without touching `staging()`. Refused because a tooltip is not
something a lane row has — nothing on this panel carries one — and because it answers the wrong
half: the node is not extra detail about the row, it is what the row's presses are addressed by.

**Drawing no row for a build that changed nothing.** The tidy reading of *empty is this lane's
ordinary state*: no changed node, no candidate, no row. It is refused by the case it silently drops.
A composite press or a library load rebuilds a slot from sources that may not have changed at all,
and a build that lands over budget **stops the slot** — so the one verdict this lane most needs to
draw would be the one it drew nothing for. The row exists because the verdict does, and the node is
what it may or may not have.

**Two capsules at the end of the row, `keep` and `back`.** The symmetric arrangement, and it makes
each press name itself. It loses on the width and on the words. A `.cand` at this bay's own size
holds a deck letter, an address, a name, a verdict and — on one verdict — a diagnostic; two capsules
between the name and the verdict leave the name nothing, and the diagnostic row would be the one
that suffers most. And a `keep` capsule would be this console's second, meaning something else.

**The whole row for `back` and a capsule for `keep`.** The same arrangement the other way round.
Refused outright: a press anywhere on a row would write over the operator's working copy and rebuild
a live slot. A destructive act belongs on a named target; the row is where a pointer lands by
accident.

**Ordering the two controls at the call site rather than refusing in the control.** `StagingBay::back`
is asked before `StagingBay::keep` on every route this program has, so the row could simply claim
its whole rectangle and rely on being asked second. It is refused because the correctness would then
live in a `match` arm's position: `keep` takes the `egui::Context` and asks `back_capsule` itself, so
a press on the capsule is not a keep whatever order a caller asks the two in
([P-0087](../principles/0087-name-the-property-never-the-shape.md) — the property is *the keep acts
on the row less its capsule*, not *the keep is asked second*).

**Carrying the changed nodes on `swap::Event`.** The engine already has the id and the label, and a
`nodes` field would put the address where every consumer could read it. Refused because the engine
does not have the fact: a `Request` restates the whole stack and `karakuri-engine` has never seen
the previous one. The diff is between two of the **watcher's** reports, which is
`karakuri-environment`'s and the host's, and ADR-0308 and ADR-0310 both said so — the changed set is
derivable where the old and new node lists are both in hand, and that is not inside `swap.rs`.
`swap.rs` is unchanged by this record.

## Consequences

- **`view::Candidate` has two more fields and the lane draws one more thing.** `at:
  Option<NodeAt>` is the payload half and `addr: String` is the drawn half, which is `view::Param`'s
  arrangement one bay over and `view::Node::addr`'s rule for who writes it — both are filled by
  `crates/karakuri/src/main.rs`, in one place, so they are filled together or not at all.
  `Candidate::name` changed meaning: it is the **node's** name on a row that names one, and the
  build's label on a row that does not.
- **`settle` replaces a slot's rows whole rather than rewriting one in place.** How many rows a slot
  has moves with every build — two on the save that touched two files, one on the next — so a
  find-and-overwrite would leave a row from a build that has been superseded. `Verdict::Settled`
  still retains by slot and takes all of them.
- **`staging` takes the build up itself**, so `Keeping::took_up` is called from inside it and the
  frame loop's second pass is gone. `took_up` answers the changed addresses and `staging` resolves
  them against the Set the swap installed. The event drain is collected per slot into a `Vec` first,
  because `Deck::events` borrows the deck and the names are read off `Deck::slot(slot).set()`; an
  empty drain collects into a `Vec` that allocates nothing.
- **`Overloaded` reuses its `Swapped`'s diff.** Both carry the id of one build and arrive in one
  drain, so the diff is taken once and looked up by id. A verdict whose build is not in the drain —
  which nothing produces today — falls to the no-node row rather than to a panic.
- **`node_addr` is one function and the Inspector calls it.** It was
  `format!("{}:{index}", layer_word(layer))` written inline in `inspector`; two bays draw the same
  address now, so it is written once (`docs/contributing.md` §4). `ir_layer` is `asked_layer`'s
  inverse, added for the same reason `karakuri_environment::mcp` has `kind_of` beside `layer_of`.
- **`put_back` takes a `Revision` rather than a name**, and the two arms differ in one `match` over
  the same listing. Its refusals gained one: a node whose only version is the one it is playing.
  `restored`'s `Previous` arm no longer says *nothing on this panel asks for it*, because something
  does.
- **`input::PROBES` has two more rows and `CONTROLS` rises by two.** Each claims one, for the
  Library bay's list's reason: a press lands on one of however many rows the bay drew, and a count
  that followed the arrangement would answer a different question. The legend
  `crates/karakuri/src/main.rs` prints rises with it, and `press_handler::ASKED` gained the two
  entries the array's length demands.
- **Two panel badges.** *Keep a candidate* moves from `plan staging` to `has staging row`, which is
  M5.7's exit condition met. *Put a node's previous version back* keeps `has` and its badge names
  both controls — `library history row, staging back` — which is the *load button, row menu → load
  to slot, or library → deck* spelling one bay up. **M5.7's known exit defect is closed with it**:
  that row's `has` came from another bay, and this lane draws it now.
- **`console.html`'s note lost three claims it could no longer make** — *the node is omitted rather
  than approximated*, *there is no control for it yet*, and *the version it puts back has to exist,
  which in this program it does not*. The last of those was already false: this program calls
  `Watch::storing_to`, `Watch::snapshotting_to` and `history::seed`, so it writes the history a
  restore reads and has since M5.7's item 6 landed.
- **A keep is still silent and still writes no record**, which is what makes the large target
  defensible. `karakuri-operation-record`'s two arms are unchanged and their reasons still read
  correctly.
- **What a row still does not say is who wrote it and when**, unchanged: `origin` has no producer
  anywhere, and the timestamp waits on the same spelling the Library bay's `.dim` column waits on.
  This record does not touch either.
- **The tests, each watched to fail against the shape it replaced.** In
  `crates/karakuri/src/main.rs`, `a_build_that_changed_two_nodes_draws_a_row_each` holds the
  decision itself — two rows, one verdict each, both addresses, and a later build replacing both;
  `a_step_back_lands_the_version_before_the_one_running` holds the resolver against three versions
  of one node and a newer version of another, so *the one before the one running* is a different
  answer from both *the oldest* and *the newest of the Set*, with the one-version and no-version
  refusals beside it; `a_keep_takes_its_own_row_off_the_lane` holds a keep against two rows of one
  slot, which is the case a retire-by-slot would fail; and
  `every_verdict_says_what_it_does_to_the_lane` gained the no-diff row. In
  `crates/karakuri-console/tests/staging.rs`, `a_row_is_a_well_and_four_things_in_it` counts the
  address and the capsule in and back out again, `a_press_on_a_candidate_row_asks_to_keep_it` and
  `the_back_capsule_is_the_smaller_box_inside_the_row` press both controls and carry the rows that
  offer neither, and `the_staging_bays_ground_is_not_a_control` is the old *nothing here is a
  control* narrowed to what is still true.
