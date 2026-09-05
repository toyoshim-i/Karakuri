---
id: 0107
title: A chain is materialised per source
status: accepted
date: 2026-08-18
supersedes: []
superseded_by: []
principles: [0087]
tags: [engine, ir]
---

# A chain is materialised per source

## Context

Merging several geometries into one Set had been carried as a design since before the compiler.
Implementing it decided three things the specification had assumed differently.

## Decision

**N simulations, with the chain materialised per source. Geometries are not concatenated into one
buffer.**

Two forces fix that shape:

- **Two sources kill independently**, so compaction is per source and there is no shared live range
  to concatenate.
- **With a chain instance per source, two sources need not agree on `emit`** — each instance is
  compiled against its own source's layout. One buffer holding both would turn that into a question
  with no answer.

**And `source` need not be an element slot — it is a uniform.** A chain instance statically knows
which source it belongs to, so what varies by source is a uniform, not sixteen bytes on every
element of a merged Set.

**That is the third time this milestone** the specification asserted a cost the implementation shape
removed: `velocity`'s third buffer, derived storage always being on, and this.

## Consequences

- **Each source counts `seed` from zero and carries its own salt.** `Simulation::seed_base` was
  already per node, so this was free — and using one file twice produces two colours with nothing
  arranged.
- `build_many` takes `&[(&Checked, u32)]` rather than a list and a number, because each L1 declares
  its own range and **one number cannot serve two**, and a pair can express a length mismatch.
- The three tests **deliberately measure differently** — total light for *both drew*, lit **area**
  for *the lattices overlap*, per-channel sums for *the colours differ* — because **a count cannot
  see position and a position cannot see colour.** Three injections each reddened a different one.
- **`is_static` was wrong and had zero callers.** It asked only whether a `spawn` block exists, while
  its own doc claims every element is live from frame zero — false for a procedure that kills without
  spawning. Fixed **because the next thing builds on it**: interpolation pairs elements by index, and
  they are the same element only while `seed` is a slot index, so a predicate saying yes to a
  compacting procedure **silently breaks the correspondence the restriction exists for.**
  `contains_kill` moved from codegen into the IR — one function and a weaker copy of the same
  question became one function.

## Evidence

Session 2026-08-18T08:30Z–12:29Z.
