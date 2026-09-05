---
id: 0031
title: A document describing replaced behaviour is worse than none
status: accepted
date: 2026-07-27
supersedes: []
superseded_by: []
principles: [0093]
tags: [docs, process]
---

# A document describing replaced behaviour is worse than none

## Context

After [ADR-0030](0030-simulation-time-comes-from-an-integer-step-count.md) inverted where `t`
lives, a review looked for text that still described the old design. It found four places, and
one of them is the reason this is a record rather than a chore:

> `t` is constant across a frame's substeps, the same way parameters and signal bindings are.

That comment is **inside `VideoSource::render`** — the function implementing the inversion. The
sentence describing the discarded design sat in the code that discarded it.

The others: the `README` listing "deriving `t` from an integer step count fixes it, **which is
why it has not been done yet**" as a live fragility, when it had just been done; `layout.rs`
saying the counts buffer is per-frame state "exactly like `t` and `dt`"; and a test's rationale
asserting that a comparison it now passes is impossible.

Two more comments were not stale but **wrong**, and dangerously: *"the tail of the draw range
holds the dead"*. It is the reverse — a killed element is compacted in among the survivors and
the tail holds the newest live ones. The implementation is right because the vertex stage reads
a per-element flag. **Someone who believes the comment optimises by truncating the range and
drops living elements.**

## Decision

Stale documentation is treated as a defect of the change that made it stale, not as
housekeeping. A change that inverts a design **searches for the text describing the old one**,
including its own comments, and rewrites it as history or deletes it.

## Consequences

- The cost is asymmetric, which is the argument. Absent documentation makes a reader go and look
  at the code. Confident, wrong documentation makes them act.
- This is the same failure the project keeps meeting from the other side — see
  [ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md), where
  `contributing.md` described the test hooks as they had been the day before it was written.

## Evidence

Session 2026-07-27T09:22Z. Standing rule:
`docs/contributing.md` §4.
