---
id: 0208
title: Resetting is the default case of restoring an arrangement, and that is what gives it a row
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: []
tags: [console, vocabulary, surfaces, docs]
---

# Resetting is the default case of restoring an arrangement, and that is what gives it a row

## Context

`crates/karakuri-console/tests/vocabulary.rs` has held a two-name list called `NO_ROW` since it was
written: the variants of `karakuri_console::panel::Op` that `docs/manual/operations.html` does not
name. It carried the sentence that separated them — *"`Reset` is a change and `Report` is a
question"* — and
[ADR-0205](0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md) took the second of
the two the other way, for a reason that does not reach the first: a question whose reply is a list
of pixel rectangles keyed by a private handle cannot be promised to three of the four surfaces, so
`Report` keeps no row **for good**. That record said in the same breath that `Reset` *"is not this
and is not decided here … it is getting a row in a later change"*. This is that change.

**What was still missing was not a badge but a framing.** *Reset* on its own reads as *start
again*, and a row written from that reads as a convenience — which is why it sat unnamed while
every other operation on the panel's own shape was specified. `docs/roadmap.md`'s M5 states the
framing the maintainer took, and it is the whole of this decision:

> **An arrangement is saved and restored, and resetting is the special case of restoring the
> default.** The panel's own state — every fold, every boundary, what is soloed — lives only in a
> running process today, and the one operation that changes it wholesale is `Reset`. That is the
> wrong way round: an operator who has spent a set arranging the console wants that arrangement
> back tomorrow, and *the default* is one arrangement among the ones they could name.

Read that way the operation is not one thing but the only built member of a family of three — name
an arrangement, keep it, put one back — and the row has to say so, because a reader who meets it as
*start again* will not ask where the other two are.

## Decision

### 1. *Reset the arrangement* is a row in *Arranging the console*, and `Operation::ResetArrangement` is the variant

It takes **no payload**, and that is decided rather than `Undecided`. `Op::Reset` names no target
either, and has not since
[ADR-0175](0175-an-operation-carries-what-it-acts-on.md) sorted the panel's operations into the four
that name a `NodeId` and the four that act on the arrangement as a whole — *"`UnfoldAll`, `Unsolo`,
`Reset`, `Report`"*. So the vocabulary can say this one outright, where it says `FoldBay { bay:
String }` and `Solo { region: Option<String> }` with a name it can only sometimes resolve.

**That is also what separates it from the two unnamed splits.** `vocabulary.rs` holds the root
column and the body row as the standing price of `Op` staying `Op` (ADR-0204) — a `String` cannot
say either, so a vocabulary built on names loses them. A reset addresses nothing at all, so nothing
about those two blocks it, and the row lands without reopening ADR-0204.

### 2. The row's prose is written from the family, and it answers *Move a boundary*

The page already promises, on the row above this one, that a window dragged too small *"gives
everything less and forgets nothing: widen it again and the arrangement comes back exactly"*, and on
*Fold a bay away* that a folded bay's *"size is remembered, so bringing it back puts it where it
was"*. Both promises are kept in the arrangement a reset **replaces**: `Panel::op`'s `Op::Reset` arm
builds `karakuri_console::layout()` fresh, carries only the viewport across, rebuilds and drops any
drag in hand. So the row says it is the one operation in the section that forgets, and names what:
every fold, every divider a hand has moved, the solo, and both sets of remembered sizes.

A row that did not say this would leave two sentences on the same page promising an operator that
nothing is lost while a third quietly loses it.

### 3. Every badge is derived rather than eyeballed, and all four are empty

- **`panel —`.** The page's rule is that *"every panel route is marked designed rather than built,
  because the panel itself is not built — where it says a region, that is where the control lives in
  the console"*, so `plan` is a claim that the mock has a home for it. It does not: `docs/manual/console.html`
  has no arrangement control anywhere, and its `presets` is a Library **scope over
  Sets**, which is the shape one level over rather than this control's home. ADR-0205 §3 named
  exactly this failure as the sharpest of the five gaps a status line was concealing — *"the page
  names a home that does not exist"* — so this row does not name one. The badge stays empty until
  the family it belongs to has somewhere to live.
- **`key —`.** `crates/karakuri-console/examples/panel.rs` binds `Key::Character("r") =>
  Op::Reset`, and that is the only caller in the workspace. It is the example's harness, not a
  surface the page specifies: `karakuri-cli`'s thirty-nine keys are what the `key` column is read
  off, and `r` there is `Live::cycle_renderer`. A `has` badge here would be the specification
  gaining a route from a development affordance.
- **`MIDI —`, and this one is a route nobody built rather than one the grammar refuses.** The other
  four rows in the section are unreachable because a map line cannot spell a region name or a
  viewport pixel. This operation addresses nothing, so `note N -> …` is exactly the shape `tap`
  already has — a press naming an operation with no payload. Nothing is in the way but the work, and
  the row and *What the gaps say* both record which kind of gap this is.
- **`MCP —`.** `mcp.rs` publishes six tools — `read_procedure`, `write_procedure`, `swap_outcome`,
  `save_set`, `read_set`, `list_sets` — and none of them is this. The test in that file reads the
  page's MCP column and fails both ways round, so the badge is checked rather than asserted here.
- **No `CLI` badge.** There is no flag, and sixteen rows carry a fifth badge only because one
  exists.

### 4. `written` answers `Silent(Surface)`

`karakuri-operation-record`'s survey (ADR-0194) puts `FoldBay`, `FoldPane`, `Unfold` and `Solo`
there already, and `Silent::Surface` names *"what is folded"* among its own examples. A reset is
every fold at once, so it is the same arm for the same reason. It is not `Silent::NoRecord`, which
is for state a **performance** has that the record vocabulary cannot carry; an arrangement is not
something a replay reconstructs anything from. It is not `Owed`, because nothing is missing — there
is nothing to write. A saved arrangement would not move it: the record such a family needs is the
panel's own, which is why `SizeWindow` sits in this arm beside it.

### 5. The console is not routed through the vocabulary

ADR-0204 settled that `panel::Op` stays `Op`, and nothing here reopens it. What changes is that
`vocabulary.rs` now pins a **mapping** where it pinned an absence: `rows_of(Op::Reset)` is *Reset
the arrangement*, byte for byte, and `NO_ROW` is one name long.

## Alternatives rejected

- **No row, and `r` is a development affordance of the example.** The real alternative, and the one
  the `NO_ROW` list encoded for as long as it existed. It says a reset is a thing the harness needs
  for testing rather than a thing an operator asks for, and it is tidy: nothing on the page gains a
  row with four empty badges, and the example goes on binding `r` for its own convenience. It loses
  because **ADR-0175 already gave `Reset` an operation's shape** — it is in the group that acts on
  the arrangement as a whole, named beside `UnfoldAll` and `Unsolo`, both of which have rows — and
  because the maintainer's framing makes it the default case of something an operator plainly does
  ask for. An affordance does not get sorted into a vocabulary's target groups two records ago and
  then turn out to be a test hook.
- **Give it a row and name a panel home anyway** — the Library's `presets` scope, or a control on
  the transport row. It would make the row look finished and it is the fault ADR-0205 measured: a
  page that names a home the mock does not have sends whoever builds the panel looking for a
  control nobody drew, and the badge stops being readable as work owed.
- **Give it a row titled *Start again*, or *Reset the console*.** Both read as a convenience and
  neither carries the family. The title is what a MIDI map's target words are taken from verbatim
  (ADR-0196), so a title that names the wrong thing costs more than a heading later.
- **Wait for the whole family and land three rows together.** Symmetrical, and it is what
  ADR-0201 did for the mask's two halves. It loses for the reason ADR-0202 gave when it declined the
  same move: the member that exists works, and holding it back trades a specified operation for a
  symmetry nobody performs with. It would also leave `NO_ROW` carrying `Reset` with a reason that
  had stopped being true.
- **Route the console through `Operation` while the row is being added.** Out of scope and already
  decided: ADR-0204 holds `Op` as `Op`, and this change gives the page and the vocabulary a row
  while the console goes on performing its own.
- **A new principle.** Nothing here is a rule a future proposal would violate. It is one row on one
  page and the operation behind it.

## Consequences

- **The page recounts to 49 operations and 212 ways in, of which 51 exist** — the new row adds four
  badges and no route. Counted from the page itself. *Arranging the console* is six rows and
  twenty-four routes, four of them the pointer's.
- **Two rows on the page now have no route at all**, where there was one. *Bring back what is
  folded* is unreachable because a folded region has no rectangle; this one is unreachable because
  the family it is the default case of is not built. The intro paragraph says both.
- **Three floors move from 48 to 49**: `karakuri-operation`'s `the_vocabulary_is_not_empty`,
  `tests/the_manual_and_the_vocabulary_agree.rs`'s row scan, and `karakuri-cli`'s `mcp.rs` route
  scan. A fourth was looked for and there is none — `vocabulary.rs`'s floor of eight is on
  `panel::Op`, which does not change here.
- **`NO_ROW` is one name long and `Report`'s reason is the permanent one.** The list's own prose no
  longer separates a change from a question; it says why the remaining entry cannot be given a row
  at all.
- **`Silent::Surface` answers for eight operations rather than seven**, and `written` is one match
  over forty-nine.
- **The roadmap's M5 entry is hooked from here and says what stays owed**: naming an arrangement,
  a record carrying one, and the two operations beside this one.
