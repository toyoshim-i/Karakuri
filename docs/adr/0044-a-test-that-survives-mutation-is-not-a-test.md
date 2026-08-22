---
id: 0044
title: A test that survives mutation is not a test
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: [0025]
tags: [process]
---

# A test that survives mutation is not a test

## Context

Four reviews in one round found **four tests that pass against the defect they are named for.**

The clearest: the meter's headline claim is that it never blocks the frame path, and a test
carried that name. Changing `PollType::Poll` to `PollType::Wait` — a full render-thread stall, the
exact thing the module documentation calls a bug — left **all seven tests passing**. Its
assertions (`frames_behind >= 1`, `mean > 0.0`, a level eventually arrives) are all satisfied by a
stalled loop, because **a `Wait` paces the loop and a paced reading is exactly one frame old.**

The test was not weak by accident. It asserted the observable consequence of the property, and the
mutation produces the same observable consequence by a different route.

## Decision

A test is **verified by mutating the thing it protects and watching it fail.** The meter's was
rewritten using the repository's own phrasing of the claim — *a blocking receive would make that
number zero* — into: over 240 unpaced frames, at least one reading must be more than one frame
behind, or at least one measurement must have been skipped. A `Wait` anywhere in the frame path
makes both impossible. Verified to fail under the mutation, and run three times for flakiness.

Carried forward into every subsequent brief, verbatim:

> Four reviews have found tests that pass against the very defect they are named for. **A test you
> have not watched fail is a test that is guessing.**

## Consequences

- **The class of defect has moved up a level.** Closing M1 the five defects were *specification
  versus implementation* ([ADR-0032](0032-nothing-checks-clean-and-comes-up-short-at-runtime.md)).
  These four are *the test versus the property it believes it is protecting*. The roadmap's note
  that implementation is not the expensive half held for M2 as well.
- It also caught a self-inflicted case in the other direction. A careless bulk edit while adding a
  size argument broke the **twin** of a `compile_fail` doctest — the positive control placed there
  precisely so the negative could not pass for an unrelated reason. Breaking both the same way made
  the twin fail and say so. Working as designed.

## Evidence

Session 2026-07-31T12:18Z, 15:37Z, 16:00Z. Sharpens
[P-0025](../principles/0025-a-test-meant-to-catch-something-is-run-against-the-defect.md).
