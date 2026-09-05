---
id: 0263
title: The library bay lists most recent first, because the listing is the operation's and not the surface's
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0090]
tags: [console, operations, library, docs]
---

# The library bay lists most recent first, because the listing is the operation's and not the surface's

## Context

*List what the store holds* has three surfaces and its row on
[every operation](../manual/operations.html) opens with the order: *"Most recent first, narrowed by
what a node is called or by which layer a Set uses, and it says how many it did not show."*

Two of the three did that. `karakuri-environment`'s MCP `list_sets` and `karakuri-cli`'s
`--list-sets` both sort with the same comparator —
`b.written.cmp(&a.written).then_with(|| a.id.cmp(&b.id))` — and `listed_sets` in
`karakuri-cli/src/main.rs` carries the reason for the tie-break in a comment.

The panel did not. The Library bay listed whatever `Store::list_sets` returned, which is ascending
by id. So one operation had two answers, and the row on the page — the specification — agreed with
the two that were not the one an operator looks at.

Nothing made this visible. The bay drew names off a directory read and never constructed the
operation at all, so there was no place where the panel's order and the row's could be compared;
the disagreement only became reachable when the filter fields gave the bay the operation
([ADR-0262](0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)).

## Decision

**The Library bay lists most recent first, breaking ties by id**, which is the row's own sentence
and the comparator the other two surfaces already used.

The tie-break is not decoration. `--save-set` twice inside one tick of a coarse filesystem clock
gives two files one `written` stamp, and a sort whose keys tie leaves the order to whatever
`read_dir` said — so two frames over an untouched store could draw two different lists. The id is
unique by construction, which makes the order total.

**What settles it is the row, not the panel.** The order an operation answers in is part of what the
operation *is*, and [operations.html](../manual/operations.html) is where that is specified —
`docs/contributing.md` §5 step 2, *the page is the specification, so it moves first*. Where a
surface and the page disagree, the surface is what changes. That the two other surfaces already
agreed with the page is corroboration and not the argument: three surfaces agreeing on a wrong order
would still have been wrong.

## Alternatives rejected

**Leave the bay ascending by id and change the row to say so.** The reading that treats the panel as
the specification. It costs the manual's claim about every other surface — the MCP tool and the
command line would both have to change with it, and both have written-down reasons for the order
they have — and it exchanges *what an operator most recently kept is at the top* for *what an
operator named earliest in the alphabet is at the top*, which is a property of the name and not of
the work.

**Give the panel its own order and say the row's order is the tool's.** One operation, two
documented answers. `docs/contributing.md` §4's *A statement is held true by the thing it
describes* is against it, and so is the failure this actually was: the row and the bay had
disagreed for as long as both existed and nobody had noticed, because nothing put the two side by
side.

**Sort in `Store::list_sets` so every caller gets it.** Tempting, and it puts a presentation order
into the store's directory read. The store answers *what files are there*; the recency order and its
tie-break belong to the operation that is being answered, which is why all three surfaces sort at
the point they answer it. It would also not have been enough — the bay was not calling the
summarising path at all.

**Break ties the other way, or not at all.** Not breaking them leaves the order to `read_dir` and is
the defect described above. Breaking them by descending id would put the tie-break in the opposite
direction from the two surfaces that already had one, which is a second answer bought for nothing.

## Consequences

- **An operator sees this change.** The Library bay's rows are in a different order than they were,
  and the most recently kept Set is at the top where the oldest name alphabetically used to be.
  There is no migration and nothing is stored: it is a sort at the point the listing is read.
- `crates/karakuri/src/main.rs`'s `library` reads `karakuri_environment::setfile::summarise` rather
  than `Store::list_sets` and sorts with the comparator above. It reads one Set file per row on top
  of the directory read, which is what the filter fields needed anyway.
- **Three surfaces now carry the same comparator in three places**, which is a copy and is named as
  one: `mcp.rs`, `karakuri-cli/src/main.rs`'s `listed_sets`, and `karakuri/src/main.rs`'s `library`.
  Nothing holds them together but this record and the row they all answer. A fourth surface, or a
  change to the order, has to find all three — `grep -rn 'written.cmp' crates/` is the sweep.
- The row on [operations.html](../manual/operations.html) is unchanged, because it was already
  right. This record is why it stayed.
