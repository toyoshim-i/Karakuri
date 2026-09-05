---
id: 0086
title: A hint says why
status: accepted
date: 2026-08-15
supersedes: []
superseded_by: []
principles: [0083]
tags: [ir, mcp]
---

# A hint says why

## Context

The first real MCP session: a model was asked to turn the particles into line art. It reached for
the obvious method — integrate a streamline per sample — and hit the loop-bound rule. The diagnostic
said:

> hint: loop bounds cannot reference a param, an ambient, or any other expression —
> **cost estimation needs them fixed at parse time**

## Decision

**That clause is why it worked.** A hint that only corrects syntax would have had the model fix its
`for` statement and produce a procedure that **compiles and cannot be afforded**. Because the reason
was there, it concluded that *streamline integration is incompatible with the cost model itself* and
switched to analytic curves — splitting `seed` into a strand id and a position along the strand.

So the rule: **all fifty-eight hints should say why.** A diagnostic written for a human turned out to
be a specification for a model, and the clause that made the difference was the one explaining the
constraint rather than the syntax.

## Consequences

- **A second thing worked by accident.** `write_procedure` validates before it writes, which was
  designed to show the compiler's answer to the model. The model used it deliberately as an
  exploration safety net — *if it does not compile, the file is not touched, so probing is safe.* A
  free probe, as a side effect.
- Its session began by looking for a worked example in another slot. **Publishing a library of
  procedures as a resource would have replaced four failed compiles with one call** — recorded as
  the next improvement, and later deferred with a resumption condition
  ([ADR-0092](0092-a-resource-listing-is-a-curriculum-not-an-index.md)).
- The example was kept with a header explaining *why it is written that way*: that `topology` accepts
  only `points`, that hue and width come from the **strand** rather than the element (per element it
  reads as rainbow noise rather than a line), and why the direct method is impossible. So the next
  reader does not conclude that points mean particles.

## Evidence

Session 2026-08-15T06:27Z–07:06Z, commit `105997a`. Standing rule:
[P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md).
