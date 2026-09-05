---
id: 0161
title: Solo remembers which region, because it cannot be derived
status: accepted
date: 2026-08-24
supersedes: []
superseded_by: []
principles: []
tags: [ui, format]
---

# Solo remembers which region, because it cannot be derived

## Context

`Layout::is_soloed()` answered *whether* a solo is in force and never *which* region holds it, so a
status line could not name the thing filling the window and a caller that wanted to had to keep its
own field — the failure this whole pass was fixing.

The obvious fix is to derive it. A solo collapses everything that is not the soloed node, on the
path to it, or inside it, so the surviving arrangement looks like it should say who it is for.

**It does not, and the case that breaks it is small enough to be ordinary.** Solo a split that has
exactly one child, and solo that child instead: the two leave *the same collapsed flags on every
node*. There is nothing left to tell them apart.

## Decision

**The soloed node is stored.** `Arrangement.soloed` is `Option<NodeId>` rather than `bool`, and
`Layout::soloed()` returns it; `is_soloed()` stays as the yes/no, because
`Outcome::Unsoloed { was }` asks exactly that and `soloed().is_some()` at every call site is worse.

`crates/karakuri-layout/tests/queries.rs::a_solo_on_a_single_child_split_cannot_be_told_from_one_on_its_child`
is the reason, written as a test rather than as a sentence: it builds the two arrangements, solos
one node in each, and asserts the flag sets are **equal**. A future change that makes derivation
look possible again has to make that test fail first.

**The saved arrangement changes with it.** `"soloed": true` becomes `"soloed": 7`. An arrangement
that names a node outside the arena is refused at load, as invariant 6 of `check_structure` with an
error of its own, because `soloed()` hands that id to a caller that will immediately ask for its
rectangle — this is
[P-0089](../principles/0089-a-check-you-have-not-watched-fail-is-guessing.md) in the same
place [ADR-0158](0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md)
put the rest of the structural checks.

**A file written before this does not load.** That is stated rather than mitigated: nothing outside
this repository has ever saved one, the panel it is for does not exist yet, and a compatibility
path for files that do not exist is a permanent cost paid against a hypothetical.

## Alternatives

**Derive it from the collapsed flags.** No stored state, no wire change, nothing to keep consistent.
Rejected because it is not possible, and the test above is the proof rather than the claim.

**Keep `is_soloed()` alone and let the caller remember.** What was happening, and it is the shape of
the problem rather than a solution: the caller ends up with a model of the arrangement, and its copy
and the arrangement drift the first time one of them is changed.

**Accept both `true` and a node in the wire format.** A day's work and a permanent branch in the
loader, to read files nobody has. Rejected for the reason above; the moment to do this is after
something has been saved that matters, not before.
