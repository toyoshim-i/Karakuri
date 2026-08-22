---
id: 0008
title: `blend` is declared now, so the second mode is an addition
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: []
tags: [ir, engine, format]
---

# `blend` is declared now, so the second mode is an addition

## Context

Sorting was open: what to do when additive blending is not the wanted look.

## Decision

The next mode is **weighted blended OIT, not depth sorting**. Sorting 262144 elements every
frame is not free even when only indices move; WBOIT is order independent, adds two targets
that the existing `Rgba16Float` pipeline already accommodates, and **cannot conflict with
compaction** because it does not care about order.

The action taken at decision time was not to implement it. It was to **declare `blend
additive` in the L4 header as the only legal value**, so that adding the second mode is a
format *addition* rather than a format *change*.

That prediction can now be checked: `blend weighted` landed on 2026-08-15 as one enum
variant and one parse arm, and nothing in the grammar moved.

## Alternatives rejected

- **Depth sorting.** The cost is real at this element count, and it interacts with compaction.
- **Add nothing until the second mode exists.** Would have made it a breaking change to every
  `.kir` in the library.

## Consequences

- `blend` becomes part of an artifact's identity, which is right: a procedure writes its
  colour and alpha knowing which blend it is for.
- Homework recorded at decision time for whoever implements `weighted`: the weight function
  depends on depth range, so a camera must supply one; a wide range degrades the approximation
  for distant bright elements; a revealage target and a resolve pass are added.
- Corrected in the same pass: stable draw order was being justified as "for non-commutative
  modes". It is not — floating-point addition is non-associative, so **additive depends on it
  too**. See [ADR-0002](0002-compaction-preserves-order-and-determinism-is-bit-exact.md).

## Evidence

Session 2026-07-25T12:03Z–12:24Z. Delivered 2026-08-15.
