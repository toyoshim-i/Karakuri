---
id: 0104
title: The slot contract is the element layout
status: accepted
date: 2026-08-17
supersedes: []
superseded_by: []
principles: [0024]
tags: [ir, engine]
---

# The slot contract is the element layout

## Context

A motion-streak renderer consumes `velocity` and `age`. An L1 that emits only `position` could not be
paired with it — **and that pairing is the entire reason slot contracts exist.**

## Decision

**The contract *is* the element layout**, absorbed rather than placed beside it, which is what the
roadmap's note had asked for. `generate_element_layout` no longer takes `emit`; it takes **what the
Set decided**, so a slot can exist that no procedure named.

**The rule is applied at the Set** — the first point holding every procedure at once, and the only
place that can tell **"nobody emits it" from "nobody *yet* emits it."**

Three paragraphs of the design were wrong, all leaning the same way:

- **"`velocity` needs a third buffer."** It needs a slot. The obstacle was never the buffer count —
  it was **compaction's reordering**: index `i` is not the same element across two frames, so
  subtracting buffers subtracts strangers. A value carried on the element travels with it, exactly
  as `seed` and `birth_frac` always have.
- **"Storage is always on."** It is conditional — allocated only when something consumes it.
- **"Adapters are nodes."** Both are pure functions of the element and the clock, so nothing is
  inserted into the chain and there is no position to explain.

## Consequences

- **Three fixtures were about to start passing for no reason**, because they used `velocity` as an
  attribute that could not be satisfied. Repointed at `normal`/`uv`, which have no rule.
- **A `spawn` block reading a derived attribute took the process down** — the read fell through to
  the generic path and named a field the `Element` struct does not have, so wgpu's handler panicked
  the building thread. **The third time this session closing that same class.** It is now refused
  rather than substituted, because there is nothing to substitute: every derivation reads state an
  element being allocated does not yet have.
- **`closed_form` could not see a derived read.** `velocity` is a stored delta, so reading it is
  reading where the element was — and all three consumers took accumulating material for closed
  form: beat sync allowed it, the governor put it on air unwarmed, `seek` would have jumped without
  stepping.
- **Velocity was up to twenty times its true value on an element's first update** (measured). A
  spawned element's first pass runs *part* of a step with `dt` scaled by the fraction — right for a
  body that integrates, wrong for one computing position from `t`, and the quotient inflates by
  `1/birth_frac`. Both directions are a difference against a state the element was never in, so it
  returns zero until the element has lived a whole step.
- **A claim of mine was retracted.** I had written that `ElementLayout` answers `offers` so every
  reader asks rather than being told — **and it had zero callers**, with the same untruth in the
  roadmap. Deleted, and the prose narrowed to what is true.

## Evidence

Session 2026-08-17T14:34Z–15:22Z, commit `209b9e2`.
