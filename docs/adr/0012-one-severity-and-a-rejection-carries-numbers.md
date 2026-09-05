---
id: 0012
title: One severity, and a rejection carries the numbers
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0083]
tags: [ir, process]
---

# One severity, and a rejection carries the numbers

## Context

Diagnostics had one severity because `IrResult<T> = Result<T, Vec<IrError>>` has no room for
another. Cost estimation was about to acquire a stage that rejects conservatively, and the
question was whether to add warnings before that landed.

## Decision

**One severity.** An error means there is no artifact; there are no warnings. The response to
a rejection is **regeneration**, not proceeding with annotations. This is a system whose author
is a model, and the loop for a bad procedure is "lower the cost and generate again" rather than
"read the advisory and decide".

The consequence is the operative half: **if a rejection is the input to a regeneration loop, it
has to carry the numbers.** "Over budget" gives a model nothing to aim at. A cost rejection
states the estimate, the ceiling, and which call dominates:

```
6344 ops/element exceeds the 4096 ops/element ceiling
(dominated by `curl` in `element`: ~6144 of 6344 ops, 96%)
```

## Alternatives rejected

- **Add severity now, while it is cheap.** Adds a concept with no user, and invites diagnostics
  that neither stop a build nor get read.

## Consequences

- Every diagnostic in a file is reported **at once**, so the repair prompt is one round trip.
- Contract diagnostics carry a hint that names the fix — reading `energy` says the signal bus is
  unreachable from IR and to declare a `param` with a `bind`.

## Evidence

Session 2026-07-25T13:15Z–13:22Z; output shown at 2026-07-25T15:42Z. Standing rule:
[P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md).
