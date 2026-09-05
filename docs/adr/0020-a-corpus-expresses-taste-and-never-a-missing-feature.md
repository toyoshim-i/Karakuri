---
id: 0020
title: A corpus expresses taste, and never a missing feature
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: [0093]
tags: [process, docs]
---

# A corpus expresses taste, and never a missing feature

## Context

The generation experiment showed that a prompt needs more than the language specification —
craft knowledge about writing good generative art. Agreed. The question is what may go in it.

## Decision

Three kinds of thing came out of that experiment, and they are not alike.

**Facts about the engine go in the specification.** Where the default camera is and what field
of view it has, the world scale a procedure should assume, that `uint / uint` truncates, that
an L4 must not declare `topology`. These are **not taste**. A generator guessing at them is not
short of craft knowledge; the specification has a hole.

**Taste goes in the corpus.** Use `sphere_point` with `curl` at 0.3–0.5 for a shell; scatter hue
by `seed` to read as a volume rather than a surface; derive a lattice's `side` from `capacity` so
density follows the element count. No single right answer, context-dependent, genuinely craft.

**And a workaround goes in neither.** Writing *"set exposure to 0.05"* into a corpus is writing
a tone mapper in English and distributing it in every prompt forever — and since the right value
depends on the element count, it ends as a table: 0.05 at 262144, 0.5 at 16384. That is the tone
mapper's job. **A corpus that compensates for a missing feature rots.** When you notice you are
writing one, fix the feature instead.

## Alternatives rejected

- **Let the corpus absorb whatever the generator got wrong.** It is the path of least
  resistance and it makes the corpus the place engine defects go to live, outliving the fix and
  getting blamed on the model.

## Consequences

- The corpus is honestly non-deterministic — closer to *"masterpiece, anime style"* than to a
  manual. Its value is **bias, not reproducibility**: fewer misses, aimed in a direction. That
  it produces slightly different material each time is the feature, since the output is
  sampling material rather than a deliverable.
- Engine facts are attached by the system on every generation regardless of which corpus is
  selected, so a corpus cannot suppress one.

## Evidence

Session 2026-07-26T02:22Z–02:39Z. Standing rule:
`docs/contributing.md` §4.
