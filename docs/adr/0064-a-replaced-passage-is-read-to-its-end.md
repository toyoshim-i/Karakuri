---
id: 0064
title: A replaced passage is read to its end
status: accepted
date: 2026-08-02
supersedes: []
superseded_by: []
principles: [0093]
tags: [docs, process]
---

# A replaced passage is read to its end

## Context

A pass over the roadmap found nine entries out of step with the code. **Three of them I had broken
myself**, and all three the same way: new text spliced on top of old, without deleting what it
replaced.

- The residency entry **described three levels and then described two**.
- The governor entry said *Built* and then described **VRAM budgeting that does not exist**.
- The noise-rate entry still carried an `Original text:` marker, with the whole undecided original
  following the settled sentence.

The cause is exact and worth naming: **I did not read to the end of the passage I was replacing.**
The opening matched what I meant to say, so I stopped looking.

## Decision

Replacing a passage means reading it to its end before writing over it. In a long document the tail
of a paragraph is where the contradiction survives, because that is where nobody looks.

## Consequences

- The other six were content rather than splices, and worth recording for what they say about a plan
  written before an implementation: VRAM budgeting was expected by the original wording and **was not
  built** (only the compute half); the lifecycle named five states and three were built
  ([ADR-0062](0062-warming-and-cooling-are-transitions-not-states.md)); closed form takes three
  conditions rather than one, and matters more for scrubbing than for priming — **the reverse of why
  it was scheduled**.
- Two lessons were added to the milestone at the same time, both since promoted:
  *a confident wrong automatic judgement is worse than not judging*
  ([P-0084](../principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)),
  and *a test can pass against the exact defect it is named for* — eight found in this milestone
  ([P-0089](../principles/0089-a-check-you-have-not-watched-fail-is-guessing.md)).

## Evidence

Session 2026-08-02T08:59Z, commit `5ad1d1b`. Standing rule:
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md).
