---
id: 0147
title: Geometry is not shared across Sets
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: [0062]
tags: [engine, deck, design]
---

# Geometry is not shared across Sets

## Context

M4's variant-pool passage claims that "three alternatives that differ only in L4 share one
simulation" while also describing the pool's members as *deck slots*. Those cannot both be
true here: a Set owns its `sources` and the element buffers inside them, and nothing lets two
Sets share a `Source`. The roadmap had already noticed the gap from the other end, in M2 —
"the same L1 in two slots is still two simulations… a different want, and nothing has needed
it yet". A pool is the thing that would need it.

So the question was live: build cross-Set geometry sharing, or accept that a deck-slot pool
costs one simulation per alternative.

## Decision

**The drawing pipeline is closed within a Set. Geometry is not shared across Sets.** A slot
that wants the same geometry as another holds its own copy and simulates it.

## Alternatives

**Share a `Source` between Sets**, so that a pool of L4-only alternatives is backed by one
simulation. It is real savings and it was the roadmap's implied direction.

Rejected because a Set is the unit every stateful rule in the engine is defined against.
Compaction order, the parity flip, `rewind`, `seek` and the per-slot probe measurement are
all written as *this Set's sources*; a shared source turns each into a question about two
Sets whose clocks need not agree, and two slots can be scrubbed, primed, parked and rolled
back independently — that independence is what a deck slot *is*. The saving would be bought
by making the Set stop being a boundary, which is the thing that makes the rest of the engine
statable.

## Consequences

**The passage's cost claim is false for a deck-slot pool, permanently — not pending.** Three
alternatives differing only at L4, held as three deck slots, are three simulations. The
roadmap says so now instead of implying a prerequisite that was never going to be built.

The claim is true *inside* one Set, and that is the shape the built half takes
([ADR-0146](0146-a-selection-is-its-own-record-and-lands-once.md)): renderers of one Set,
sharing its simulation because they are its renderers.

A pool across deck slots is still worth having — for alternatives that differ at L1 or L2,
where separate simulations are not an inefficiency but the point. What it costs is now a
statement rather than a surprise, and it lands on the deck's four-member cap and on the VRAM
budget the governor says outright it cannot keep.
