---
id: 0281
title: Every route reaches every write, and a read is the route's own interface design
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: []
tags: [manual, operations, surfaces, vocabulary, rules, m5]
---

# Every route reaches every write, and a read is the route's own interface design

## Context

Rule 01 of [the seven](../manual/index.html) read, until this record:

> Every operation is reachable from the panel, from the keyboard alone, from a mapped MIDI control,
> and from a model over MCP. There is no operation only the mouse can reach, and none the map cannot
> address.

It is one sentence over sixty-three rows and two kinds of operation. `Operation::SetGain` moves a
fader and `Operation::ReadSet` answers a question, and the rule asks the same four routes of both.

**The maintainer's words, and they are the decision:**

> 操作の原理を少し変えよう。書き込み動作については今まで通り、全ての操作経路から到達出来なければ
> ならない。読み込みについては任意。それが必要になるか否かは経路ごとのインタフェースデザインに
> 依存する事であり、強制するものではない。

and, on where the line falls:

> 副作用があれば書き込み

**It is a loosening.** Everything that satisfied the old rule satisfies the new one; no route
acquires an obligation it did not have. The whole of the change is on the read side, and the whole
of what the page has to say that it does not say today is *this empty column is not a debt*.

### The boundary already exists in the code, under a different name

`karakuri_operation_record::written` answers `Written::Silent` for four different reasons and
`Silent::Question` is one of them —
*"**It asks rather than changes.** Reading a Set, listing the store, reading a procedure, finding
out what a write did. A record is what a replay reconstructs a performance from and a question
changes no performance"* (`crates/karakuri-operation-record/src/lib.rs:515`). Four operations are in
that arm and no others: `ListSets`, `ReadSet`, `ReadProcedure`, `SwapOutcome`
(`crates/karakuri-operation-record/src/lib.rs:1247`).

**So the boundary is not the record; it is one of the record's four reasons for silence.** That is
why `SelectDeck` was the right counter-example and why it settles nothing: it is
`Silent::Surface` — *"a surface's own state"* — beside `SelectScope`, `SetTransition`, the four
folds, `SizeWindow` and `KeepCandidate`. Thirty-one operations write no record; four of them are
reads.

**`gate.rs`'s `Standing` is about authority and not about reading**, confirmed:
`Operation::TransferSet { .. } => Standing::Open` sits in the same list as `WriteProcedure`,
`SaveSet` and `WireInput`, all `Open` (`crates/karakuri-operation/src/gate.rs:523`). An open class
is one a model may ask for, which is a different question from whether the act changes anything.

**`op-when` is orthogonal**: `immediate`, `grid`, `launch`, `on a worker` say *when a thing lands*.
All four reads are `immediate`, and so are most of the writes.

### Where the definition is sharp and where it is not

Sharp, and the four are the ones `Silent::Question` already names. Not sharp, in three places:

1. **A surface's own state is an effect.** Under *副作用があれば書き込み*, `SelectDeck` changing
   which deck the next `l` addresses is a side effect and the operation is a **write**. So are
   `SelectScope`, `SetTransition`, the four *Arranging the console* folds, `SizeWindow`,
   `KeepCandidate`, `ResetArrangement`, `SaveArrangement`, `RestoreArrangement`. **Nothing moves for
   any of them** — they were writes under the old rule too and their columns are what they were.
   The narrower sentence that would move them — *a write is what a replay has to reconstruct* — is
   a second, larger loosening that would make the console's own arrangement optional per route, and
   it is not taken here.
2. **A row can hold a read and a write.** *Send a Set to somebody, and take one in* is
   `Operation::TransferSet` with two arms:
   [ADR-0260](0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)
   decides `Send { id }` is a read, and `Take { file }` stores a Set and refuses an id already
   taken, which is a write by any reading. The badge is on the row and the rule is on the operation.
3. **A read may cost, and cost is not an effect.** `read_set`'s fourth figure runs `compile::check`
   over every source in the Set (`crates/karakuri-environment/src/mcp.rs:2509`) and changes nothing;
   the page already says which figure costs what. And `swap_outcome` moves events out of a channel
   into a kept buffer on the way to answering (`crates/karakuri-environment/src/mcp.rs:784`), which
   is a cache fill the asker cannot observe — asked twice it answers the same and more.

## Decision

**An operation that has a side effect is a write, and rule 01 stands for writes exactly as it
did: every write is reachable from the panel, from the keyboard alone, from a mapped MIDI control,
and from a model over MCP. An operation that only asks is a read, and whether a route offers it is
that route's own interface design. Nothing compels it.**

**The page marks the row, not the cell.** A read carries one `read` mark in its `op-head`, beside
its title. The three badge states are untouched: `has` still means built, `plan` still means
designed and not yet, `gap` still means this surface does not reach it. What the mark changes is
what an empty column *means* on that row — on a write row it is owed, on a read row it is that
route's answer.

**Four rows carry the mark today**, which is `Silent::Question` read off the page: *List what the
store holds*, *Read what one Set holds and declares*, *Read one node's source*, *Find out what a
write did*.

## Alternatives rejected

### a. A fourth badge state — *not offered here, by design*

**What it had going for it.** It says the thing in the cell, where the meter greps; it keeps the
cannot/declined distinction the page currently spends tips on (*"The MIDI badge is empty on purpose
rather than pending"*); and the four files that read this page compare only against `"has"`
(`panel_column.rs:605`, `vocabulary.rs:1133`, `mcp.rs:6457`, `main.rs:17407`), so a fourth class
would be invisible to every one of them and cost no test.

**It loses because on a read row the distinction it draws is not there.** `gap` means *cannot reach
it at all*, and the reason MIDI cannot reach `ReadSet` — *"a read's answer has nowhere to land on a
controller"* — is a fact about that controller's interface, which is the same sentence the new rule
uses for a route declining a read. For a read, *cannot* and *chose not to* are one answer given
twice. It also costs four cells per read row against one mark per read row, and every one of those
cells is a place the two can drift apart.

### b. Widen `gap`, and add nothing

**What it had going for it.** The smallest possible edit: one legend clause, no markup, no CSS, and
every cell that would carry the new meaning is already `gap`.

**It loses because the same badge would mean two things and nothing on the page would say which.**
A `gap` on a write row is a hole rule 01 still requires somebody to fill; a `gap` on a read row is
finished. Without a mark on the row, telling those apart means knowing the vocabulary — which is
the failure this repository has been bitten by repeatedly: a claim only a person is keeping true.
The mark is what makes the difference greppable, and it is the reason the row and not the cell is
where it goes.

### c. A sentence in the row's tip

**What it had going for it.** The page's own idiom — every row that has something to explain about
an empty cell explains it in `data-tip`, and there are a dozen of them.

**It loses on the meter.** `docs/roadmap.md`'s *The instrumentation* counts badges with `grep`, and
the M5.x exits are *no `plan` badge in the panel column of this bay's rows*. A tip is invisible to
both, so the new rule would be held true by a reader and not by a command. That is exactly what the
meter exists to stop.

### d. Split *Send a Set to somebody, and take one in* into two rows

ADR-0260 §f considered and refused this — *"the split buys nothing: the destination question is
identical after it"*. That was true of the destination question and is not true of this one: split,
the read half's empty columns stop being debts and the write half's do not. **It is still not taken
here**, because a row is the manual's unit and cutting one is a change to the specification that
this record does not need to make: `TransferSet`'s panel badge is `plan` because
[ADR-0267](0267-the-panel-sends-into-the-folder-the-library-bay-is-pointed-at-and-the-destination-is-drawn-before-the-press.md)
decided the panel *does* send, which is an interface design already taken and one this rule
permits. The split is available the day somebody wants to un-take it.

## Consequences

- **No obligation is added anywhere.** Every badge on the page is as honest after this record as
  before it; nothing that was `has` becomes a lie and nothing that was owed becomes owed twice.

- **No mechanical check changes, and none is lost — because none of them ever held rule 01.**
  `karakuri-console/tests/panel_column.rs` asserts two things: every operation a console control
  emits has a `has` panel badge, and every `has` panel badge is emitted by a control and names a
  home. Neither says *no `plan`* or *no `gap`*. The same is true of `tests/vocabulary.rs`,
  `karakuri-environment/src/mcp.rs`'s pair and `crates/karakuri/src/main.rs`'s key-column pair. All
  four check **badge honesty**, which is untouched by a rule about coverage. Rule 01's *must be
  reachable* was held by the M5.x exit sentences and the meter's `grep`, and it still is.

- **What a machine cannot hold is the new sentence, which is why the row is marked.** *This empty
  cell is by design* is not derivable from anything in `crates/`: `written` knows which operations
  are `Silent::Question` and `karakuri-console` may not depend on that crate (`Cargo.toml`,
  ADR-0156), and no crate can depend on `crates/karakuri`. The mark is the page saying it once per
  row where a `grep` can find it. **Holding the mark against `written` is available and not taken
  here**: `karakuri-operation`'s own test reads this page already, and a test there asserting that
  the marked rows are exactly `Silent::Question`'s would need the record crate, which is the
  dependency `karakuri-operation` has by charter refused (ADR-0180). If it is wanted, the place is
  `karakuri-operation-record`'s own tests reading the page, and that is a new file rather than an
  edit.

- **A readout's `has` badge is not settled by this record and the question survives it.**
  `docs/roadmap.md` under M5.4: *"`panel_column.rs` requires that a control **emit** the operation
  before the page may mark the panel column `has`, and a readout emits nothing."* This rule frees
  an **empty** cell; it says nothing about a full one. The precedent that already works is
  [ADR-0264](0264-a-reading-is-one-question-and-only-the-opening-asks-it.md) — *Read what one Set
  holds and declares* is `has` on the panel because **opening** the reading emits `ReadSet`, and
  closing it and moving the cursor emit nothing. A readout with no gesture at all has nothing for
  the scan to see, and under this rule the alternative it now has is to be honestly empty.

- **Three rows' `plan` panel badges become elective rather than owed**, and no exit sentence becomes
  false, because every exit is phrased *no `plan` badge* and a badge dropped to nothing satisfies
  it. *Read one node's source* (M5.5), *Find out what a write did* (M5.4) and the Send half of
  *Send a Set to somebody, and take one in* (M5.3). Each stays `plan` until somebody decides the
  bay does not draw it; this record decides none of the three.

- **M5.3's blocker is not dissolved by this, and that is worth saying because it was expected to
  be.** The folder-with-a-directory that *Send* waits on is ADR-0267's, and ADR-0267 is an interface
  design that chose to offer the read. This rule permits that choice; it does not overturn it.

- **The two rows in *Rows the manual has not given a home* are unaffected.** *Wire a procedure's
  input to a node* writes an edge and rebuilds, and *Narrow the published interface* changes what
  the console shows and what a MIDI knob counts. Both are writes; both still need a home.

- ***Walk the edit history* is a write.** `Operation::WalkHistory`'s doc names *a revision to land
  on* as the answer among the three shapes a walk could be addressed by
  (`crates/karakuri-operation/src/lib.rs`, at the variant) and `docs/roadmap.md` M5.3 says
  *"landing on a row is a **load**"*. The walking is a listing and the row is not.

  *(**Corrected 2026-09-08: *settled* ran one step ahead of the code.** This read
  *"`Operation::WalkHistory`'s payload is settled as a revision to land on
  (`crates/karakuri-operation/src/lib.rs:1374`)"*, and line 1374 was the variant itself —
  `WalkHistory { step: Undecided }`. What the doc settles is **which of the three** a walk needs — a
  cursor and a direction, a revision to land on, or a count of steps — *"because a row is what an
  operator picks"*. It then declines to write it into the payload, in the next sentence: *"It is not
  written into the payload here, because a payload is what a surface can say
  ([ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)) and
  no surface can say a revision yet."* So the payload is deliberately open, not settled. **The
  conclusion is untouched**: whichever of the three the payload ends up spelling, a walk lands on a
  revision and landing is a load, so the row is a write and rule 01 asks all four routes of it. The
  line number has drifted since and the citation names the item instead.)*

- **`index.html`'s cap is not touched.** This is rule 01 saying less, not an eighth rule.

- **This is a decision and not a principle.** Two plausible alternatives lost — a fourth badge state
  and a widened `gap` — and what the rule decides is one column's worth of obligation on one page.
  Under [ADR-0249](0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)'s gate, the
  question it settles outside the manual is `written`'s `Silent::Question` arm acquiring a second
  reader, which is a consequence rather than a question it answers unmentioned.

## What this does not decide

- **Whether any particular route drops a read it currently plans.** Every `plan` on a read row
  stands until the bay that owns it says otherwise; this record makes the choice available and takes
  none of it.
- **Whether a readout may carry a `has` panel badge.** Named above; it was open before this record
  and is open after it.
- **Whether the *Send*/*Take* row is cut in two.** Alternative (d).
- **Whether the mark is held against `written` by a test.** Named in the consequences, with the
  crate that could hold it and the dependency that stops the two obvious homes.
- **What a read row's tip should say about a column it declines.** The rows that decline one today
  already say so in prose; whether that prose is now redundant against the mark is the page's, one
  row at a time.
