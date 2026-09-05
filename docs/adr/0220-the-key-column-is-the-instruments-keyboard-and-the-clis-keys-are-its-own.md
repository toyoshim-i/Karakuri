---
id: 0220
title: The key column is the instrument's keyboard, and the CLI's keys are its own
status: accepted
date: 2026-08-29
supersedes: []
superseded_by: []
principles: [0087, 0093]
tags: [docs, process, ui]
---

# The key column is the instrument's keyboard, and the CLI's keys are its own

## Context

[ADR-0213](0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
fixed what the **panel** column of [every operation](../manual/operations.html) measures: an operator
running the instrument reaches it. Nothing said what the **key** column measures, and it had quietly
stopped being well-defined.

[ADR-0208](0208-resetting-is-the-default-case-of-restoring-an-arrangement.md) read that column off
`karakuri-cli`'s thirty-nine keys, which was unambiguous while there was one program with a
keyboard. [ADR-0214](0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md) made
a second, and nothing re-decided the column. **Two programs now have keyboards and a key badge does
not say whose.**

The measurement that settles it: **the two collide on eight letters — `f g n p r s u z` — and agree
on exactly one, `esc`.** Seven of those eight are specified on the page for what the *command line*
does with them (`f g` fade, `z n` transition settings, `u` scrub, `r` renderer, `p` latency offset),
while at the panel they fold, solo, unfold, reset and report.

## Decision

**The key column is the instrument's keyboard — the keys `cargo run -p karakuri` binds, and no other
program's.** It is written into the legend beside the badge glosses.

**The first rule is what checks it.** The manual's first rule pairs *the panel* and *the keyboard
alone* for one operator. If `key` meant the command line, the two columns would describe two
programs and no operator would ever have both, because `karakuri-cli` has no panel at all.

**And the clincher is specific to this column: a key badge is an instruction to press something.**
With eight colliding letters, *any program in this workspace* and *the command line* both tell a
reader standing at the panel to press a letter that does something else in front of them. That is
[P-0087](../principles/0087-name-the-property-never-the-shape.md) one column along — *which program
happens to bind the letter* is the shape, and *the operator presses it and the thing happens* is the
property.

**It is also the only reading that is checkable in one place**, which is not a coincidence: the keys
live in `crates/karakuri/src/main.rs`, that package has no library target on purpose, and
`karakuri-cli` has none either — so a check that had to see both keyboards could live nowhere.

## What it costs, and it is the visible half

**Twenty rows stop reading built** — every transport, deck, mixer, library and window-size row. They
are reached by a key today, in `karakuri-cli`, and
[ADR-0045](0045-the-cli-is-an-instrument-not-a-demo.md) calls that program an instrument. **The key
column goes from 21 built to 6, and the page's total from 57 of 216 to 42.**

That is ADR-0213's trade taken a second time: **a meter that reads honestly low beats one that reads
well against the wrong program.** The panel column read 0 of 46 for the same reason and was kept.

**The letters stay on the page**, and that distinction is the point of the change rather than a
softening of it. [The console page](../manual/console.html) says *"the key is owed and this page does
not choose the letter, because which keys exist is the operations page's to say"* — this page is the
specification for keys, so deleting twenty of them would **unspecify the keyboard**, which is a
larger act than reclassing a badge. They are `plan` with their text intact.

One gloss had to be restated with them: `plan` read *"designed — this surface is meant to reach it,
and no program a player runs does yet"*, and that clause is false the moment the CLI binds a key for
a `plan` row. It now reads *"designed — this surface is meant to reach it and does not yet."*

## Alternatives rejected

**Any program in this workspace binds a key to it.** The most generous reading and the one that
keeps the meter high. It fails the badge's own grammar: with eight colliding letters it prints an
instruction that is wrong at the panel, and it is uncheckable, because no crate may depend on both
key handlers.

**The command line's keys**, which is the status quo and ADR-0208's basis. It makes the panel column
and the key column describe two different programs, and the first rule is about one operator with
both.

**Split the column in two.** Honest, and it costs the page a fifth column for a state that is
temporary: `karakuri-cli` is scaffolding rather than the destination, and its keys are what the
instrument's keyboard is being built towards rather than a surface with a permanent claim.

## Consequences

- **The column is checked, in `crates/karakuri/src/main.rs`**, both ways round, beside the keys it
  measures — a page claiming a key the instrument does not bind fails, and a bound key whose badge
  says `gap` fails. A third assertion holds the file's own key table against the `match` that binds
  them, so an arm nobody wrote down cannot appear.
- **What that check cannot see is stated in it**: `egui` sees every key first, so a key it consumed
  would still read as bound. The sufficient half is one boundary further out than the two console
  checks stop at, and nothing reaches it.
- **The same question is now open for the MIDI and MCP columns**, and this record does not answer it.
  `crates/karakuri` has no MIDI, no MCP, no audio and no session, so those two columns measure
  `karakuri-cli` and `karakuri-environment` while panel and key measure the instrument. **Nothing
  checks the MIDI column at all.**
- **Three documents use *the instrument* for two different programs.** ADR-0045 is accepted and
  unsuperseded and calls `karakuri-cli` one; `README.md` and ADR-0214 call it scaffolding rather than
  the destination; ADR-0213 and `crates/karakuri`'s own manifest use the word for the new binary.
  This record takes the last, and **names the collision rather than resolving it** — retiring
  ADR-0045's word is a separate decision about a record that was true when written.
- **`karakuri-cli`'s own key handler documents the page as the specification for its keys**, which
  this makes false and which is corrected with this change.
