---
id: 0288
title: A read row's built badge is met by an emission or by a drawing this file can find
status: accepted
date: 2026-09-08
supersedes: [0284]
superseded_by: []
principles: []
tags: [console, manual, operations, m5]
---

# A read row's built badge is met by an emission or by a drawing this file can find

## Context

[ADR-0284](0284-a-readout-is-drawn-and-the-panel-badge-does-not-move-because-the-meter-counts-a-gesture.md)
left *Find out what a write did* drawn on the panel and its panel badge at `plan`, and named the
correction to start from — its own alternative (a), *`has`, with `panel_column.rs` relaxed for rows
the page marks `read`*. It turned that shape down for one reason, and the reason was not what it
would allow:

> **It loses on what it would stop checking, not on what it would allow.** The assertion it relaxes
> is the only mechanical thing standing behind a `has` badge in that column, and relaxing it for a
> class of rows removes it for *every* member of that class, including the two that are `has` today
> and genuinely emit.

It also said what the shape needed beside it:

> what it needs beside it is a check that a drawn readout is *drawn* — which is not
> `panel_column.rs`'s to make.

**That second sentence is the one this record disagrees with**, and the disagreement is about where
the drawing is rather than about what the meter should say. A readout has two halves: the console
draws it, and the window fills it. `karakuri-console` cannot see `crates/karakuri`, so the *seam* is
beyond that file — but the *drawing* is the console's own source, which is the text
`panel_column.rs` already reads.

## Decision

**A `has` badge in the panel column is met by an emission, or — on a row the page marks `read`
— by a drawing named in a table this file holds and can find.** The check is not relaxed; it is
given a second way to be met, and the second way is as mechanical as the first.

- `DRAWN` pairs a row's title on [the operations page](../manual/operations.html) with a marker in
  `crates/karakuri-console/src` that draws it. One entry today:
  *Find out what a write did* against `pub health: Option<Stage>`.
- `every_drawn_entry_names_a_read_row_the_page_marks_built_and_a_drawing_that_is_there` holds the
  table against both sides. An entry naming a row the page does not mark `read` fails, so the table
  can never be used on a **write** row, where emitting is the only honest evidence. An entry naming
  a row the page does not mark `has` fails, so the table cannot outlive the badge it stands under.
  A marker no file contains fails, so a readout that stopped being drawn takes its badge with it.
- A row that is neither emitted nor drawn still fails, exactly as before.

**What ADR-0284 objected to does not happen.** *List what the store holds* and *Read what one Set
holds and declares* are `read` rows and are `has`, and neither is in `DRAWN`: both are still held by
their emission, because both genuinely emit — ADR-0264's gesture. The day one of those controls is
deleted, the first assertion fails naming it, as it did before this record.

**The precedent for a hand-written table is `press_handler::ASKED`** in `crates/karakuri/src/main.rs`
— the same shape, the same blind spot, and the same answer to it: an entry that names nothing fails,
and a thing with no entry fails.

## Alternatives rejected

### a. ADR-0284's decision, unchanged

The badge stays `plan` with the capsule drawn, and `plan` goes on carrying *the meter cannot see this
yet* rather than what ADR-0213 defined it as. That record priced this itself: *"`plan` is now
imprecise on this row, and that is the price."* The price is worth paying only while the correction
is unavailable, and the objection that made it unavailable was answerable.

### b. Alternative (a) of ADR-0284, taken as written

Relax the emission check for every row the page marks `read`. Rejected for the reason ADR-0284
gives, unchanged: it would remove the only mechanical guard behind three `has` badges to place a
fourth.

### c. A check on the seam rather than on the drawing

The stronger evidence, and it is out of reach here: `karakuri-console` cannot see `crates/karakuri`,
and a check that the value reaching the readout is the one the deck said needs both. That check
exists and is `crates/karakuri`'s own — `the_swap_report_says_what_the_lane_says`, which takes a
device. Naming it from here would be this file asserting something it cannot verify, so what
`DRAWN` names is the half this file can see, and the seam's half is named in the comment beside the
entry rather than claimed by it.

## Consequences

- ***Find out what a write did* carries a `has` panel badge**, and M5.4's exit condition — no `plan`
  badge in the panel column of this bay's rows — is met by this row rather than excused by it.
- **`plan` means what ADR-0213 defined it as again**, on every row of the column.
- **The table is one row wide and is written to grow.** Every readout a bay draws hereafter is an
  entry, and every one of them has to name a marker a reader can find, which is the cost of the
  badge rather than a formality.
- **What this still cannot see is a false positive of its own shape**: a field that exists and is
  never painted. That is the same blind spot `input::PROBES` and `press_handler::ASKED` both
  document on their own sides, and the answer to it is the same — the drawing test in
  `tests/transport.rs`, which reads the paint pass.

Co-authored with the maintainer, who chose this shape over ADR-0284's standing decision.
