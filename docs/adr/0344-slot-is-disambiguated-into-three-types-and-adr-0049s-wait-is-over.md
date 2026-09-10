---
id: 0344
title: "`slot` is disambiguated into three types, and ADR-0049's wait is over"
status: accepted
date: 2026-09-11
supersedes: [0049]
superseded_by: []
principles: []
tags: [store, engine, console, docs]
---

# `slot` is disambiguated into three types, and ADR-0049's wait is over

## Context

[ADR-0049](0049-slot-means-two-things-and-the-clash-is-recorded.md) named two senses of `slot` — a
layer's position inside a Set, and a Set's position in the deck — and declined to rename either,
on the ground that *"choosing which to rename requires knowing which meaning the eventual GUI and
record format lean on — which is not known. Deciding later, when something actually needs the
distinction, is cheaper and better informed."* It was a deferral with a named trigger, not a
closed question.

The trigger has been pulled, twice over, without anyone writing it down:

- `karakuri_operation::NodeAt { layer, index }` and its mirror `karakuri_store::NodeAt` already
  exist and are used at ~57 call sites across `karakuri-operation`, `karakuri-store`,
  `karakuri-environment`, `karakuri-engine::swap`, `karakuri-console`, and `karakuri`. Something
  *did* need the Set-node distinction — the code answered it ad hoc, under a name ADR-0049 never
  authorized, while `Record::Slot` and six sibling `Record` variants still spell the same address
  as bare `layer`+`index` siblings. Two spellings of the address that ADR-0049 said would wait for
  one now coexist.
- A third sense — a declared input on a node (`uses far : Geometry`) — was never part of ADR-0049
  at all, and the MCP surface already disambiguates it from the deck sense in its wire vocabulary
  (`docs/manual`'s tools spell it `input`, not `slot`, precisely so *"one word means one thing
  across this surface"* — `crates/karakuri-environment/src/mcp.rs`'s own schema text). The
  vocabulary already agrees the third sense is a different thing; only the Rust identifiers still
  call it `slot`.
- `docs/refactoring.md`'s architectural audit (2026-09-10) names this collision, still
  undocumented as a live cost rather than a settled one, as contributing to *"high cognitive
  overhead"* across every crate that touches a Set or a deck — the condition ADR-0049 asked to be
  weighed against the cost of a rename has now been observed, not merely anticipated.

ADR-0049's own alternative-considered list is unchanged in its reasoning — a rename bought nothing
*in July*, before a GUI existed to lean on either meaning and before the record format was
settled. M5 has since closed and the record specs it names are finalized (per
`docs/contributing.md`'s own working-style notes); the console's arrangement has since been split
bay-by-bay into `crates/karakuri-console/src/view/{mixer,library,transport,inspector,program}.rs`,
which is what makes a mechanical rename inside the GUI reviewable module-by-module rather than as
one diff against a single 25,000-line file. Both of ADR-0049's stated unknowns are now known.

## Decision

**All three senses get their own type, per `docs/refactoring.md`'s proposal, and ADR-0049 is
superseded rather than left standing beside a rename it said to wait for:**

- `InputPort(String)` for a declared node input (`Record::Edge.slot`, `karakuri_ir::typed::Slot`,
  `karakuri_engine::set::Edge.slot`, the `slot: String` field on the wire-input operation). This
  sense was never covered by ADR-0049 and carries no deferred question — it is done first, and
  independently of the other two, because nothing about it needed this ADR to proceed.
- `NodeAddress { layer: Layer, index: u32 }` replacing `karakuri_operation::NodeAt` /
  `karakuri_store::NodeAt` under one name, and extended to the `Record` variants that still spell
  the address as bare sibling fields (`Slot`, `Capacity`, `Param`, `Bind`, `Procedure`,
  `Authority`, and their `karakuri-operation`/`karakuri-operation-record` mirrors).
- `DeckSlot(u8)` replacing the bare `u8`/`usize` used for a deck position across `Record`'s
  thirteen deck-addressed variants, `karakuri-engine::deck::Deck`'s methods, `karakuri-midi`'s
  `Target`, and the GUI/CLI call sites that currently range-check it ad hoc
  (`karakuri-cli::slot_in_range`, `karakuri-environment::mix::in_range`) rather than by
  construction.

**Both renamed types keep their current wire shape.** `Record`'s JSON keys (`layer`/`index` for a
node address, `slot` for a deck position) and every MCP tool schema's `"slot"` parameter stay
exactly as they are — a client, a `.kbset` file, or a session stream on disk sees no difference.
`record.rs`'s own precedent for this (`#[serde(rename = "proc")]` on `Record::Slot::proc_hash`) is
the pattern: rename the Rust identifier, pin the wire name.

**Named `NodeAddress` and `DeckSlot` rather than a bare `Slot`, on purpose.** A type plainly called
`Slot` already exists — `karakuri_engine::master::{Slot, SlotSpec, SlotError}` and
`karakuri_store::record::ChainSlot`, naming a master-chain position, a fourth sense this ADR does
not touch. `karakuri-pattern::SLOTS`/`StepMode::slot_of` (a sequencer's sixteen steps) is a fifth,
unrelated sense sharing only the word. Neither is disambiguated here; both are left exactly as
named, and the new types are named to not collide with either.

**Sequencing**: `DeckSlot` lands after `crates/karakuri/src/main.rs` gets the same per-module
decomposition `view.rs` already received (docs/refactoring.md's other P2 item, still open) — its
~1,300 occurrences in one 35,000-line file are the majority of the diff, and reviewing that
mechanical a change is materially safer file-by-file than as one pass against an undivided file.
`InputPort` and `NodeAddress` do not wait on this.

## Alternatives rejected

- **Leave ADR-0049 standing and rename anyway.** Two records disagreeing about whether this is
  settled is exactly the state ADR-0049 itself refused to leave undocumented — the fix is to
  supersede it, not to make a third text nobody can tell is the current one.
- **Rename only where the GUI already forced a name (`NodeAt`) and leave the rest.** Produces a
  third spelling — `NodeAt` in some places, bare `layer`+`index` in others, and unrenamed `slot`
  everywhere for the deck sense — which is a worse state than either ADR-0049's two-sense wait or
  a completed three-way split.
- **Wait for `main.rs`'s own decomposition before touching any of the three.** Rejected for
  `InputPort` and `NodeAddress`, which do not depend on it and have no reason to wait; accepted
  only for `DeckSlot`, where the dependency is real (see Sequencing above).

## Consequences

- `docs/architecture.md`'s "words that carry more than one sense" section loses its `slot` entry
  once the three types land; `layer`'s three senses and `authority`'s two are unaffected and stay
  recorded there.
- Every construction site of `Record::Slot`/`Record::Capacity`/`Record::Param`/`Record::Bind`/
  `Record::Procedure`/`Record::Authority` and every `Deck` method taking a bare slot index changes
  signature, mechanically, in the commits that follow this one.
- `NodeAt` is retired as a name once `NodeAddress` replaces it everywhere it is used; nothing keeps
  both names live past the migration that does it.

## Evidence

Session 2026-09-11. Prompted by a read-only investigation into `docs/refactoring.md`'s P3 item
finding `NodeAt`'s ad hoc existence, the MCP surface's already-disambiguated `input` vocabulary,
and confirming with the user that ADR-0049's deferral condition — *"when something actually needs
the distinction"* — has been met rather than still open.
