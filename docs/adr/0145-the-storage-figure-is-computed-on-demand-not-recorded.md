---
id: 0145
title: The storage figure is computed on demand, not recorded
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: []
tags: [store, metadata, engine]
---

# The storage figure is computed on demand, not recorded

## Context

`Set::element_storage` had an owner and no reader, and `docs/roadmap.md` M4 says what the
reader should be: *"a record written against a saved Set, which `--save-set` already puts in
the content-addressed store."*

That expectation was written when the figure could only be had from a built Set. Two things
have changed since: `Set::validate` runs the check pass without a device
([ADR-0142](0142-validation-runs-without-a-device-and-hands-build-a-plan.md)), and `read_set`
gives a saved Set a reader that a model can reach
([ADR-0138](0138-a-model-names-a-set-not-a-hash.md)).

## Decision

**Computed from the Set file plus a compile pass, at the moment somebody asks.** No record,
nothing written into the store.

A stored number is a second copy of an answer the code already decides. It is right on the
day it is written and drifts the first time a stride, a flag or an amplifier's factor
changes — and this is the *one* figure in this system with a documented history of exactly
that: a `bytes/element` was published, went 85% wrong against what the engine allocated, and
was withdrawn rather than corrected. Recording it again would recreate the failure whose
repair this is.

**One place decides a buffer's size.** A new private `storage` module holds it, and both the
allocating constructors and `Plan::element_storage` call it. Re-deriving the arithmetic in
the reader was the obvious shortcut and would have been a second copy of the rule — the
defect class this project has removed all week and twice recreated while believing it fixed.

`Buffer::size()` returns the descriptor's value verbatim, so the computed figure is an
equality with what is allocated rather than an estimate. That was checked before the shape
was chosen, not assumed.

## Alternatives

**Write a record against the saved Set, as the roadmap expected.** Rejected above. The
roadmap's shape was right for the world it was written in, where the figure needed a device.

**Have `Simulation::element_storage` and `Deform::element_storage` call the shared sizing
too.** Rejected deliberately, and it is the one place a second derivation is kept on purpose:
they read `Buffer::size()` off the buffers that exist, so the GPU test compares two
independent answers instead of asserting a function equals itself.

## Consequences

A Set file with no renderer gets a sentence saying the figure was not computed and why,
rather than a number: `setfile::load` and `Set::validate` both refuse a Set with no L4, so
an L4 is needed to cost a Set even though a renderer allocates no element storage.

The answer states what it excludes — render targets, uniform blocks, everything not indexed
by element. A number that reads as device memory would be the withdrawn figure's mistake in
a new costume.
