---
id: 0102
title: A renderer's address is `(layer, index)`, and `layer` alone means nothing
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: [0090]
tags: [format, engine]
---

# A renderer's address is `(layer, index)`, and `layer` alone means nothing

## Context

With more than one renderer over one geometry, three surfaces needed the same thing — parameters,
edit history and MCP — and so did turning L2, L3 and L5 into nodes.

## Decision

An address is **`(layer, index)`**, and **`layer` only means anything when `index` is beside it.**

That qualification is the decision. For `slot` and `procedure`, absent means zero, because those
always name exactly one node. For `param` and `bind` it cannot, and the reason is specific:
**`param`'s `layer` was a placeholder.** Writers put `L1` on everything and said so in a comment;
readers ignored it. Beginning to honour it would have **silently retargeted every existing Set
file** — an `exposure` that had been reaching a renderer would arrive at L1 and do nothing.

Tested against **hand-written old-format files**, because a round trip through the new writer can be
wrong on both sides in the same way and still agree.

## Consequences

- **`--bind` gained no syntax at all** — one more field — because it was built on *field names
  belong to the record*. A design principle paying a dividend a month later. `--param` is positional,
  so it needed a prefix: `--param L4:1:exposure=2.0`.
- **Edit history turned out to be a bug rather than an inconvenience.** Every renderer in a stack
  shared one chain *and* shared the memory of what was last written, so two renderers took one
  snapshot per save and overwrote each other's record — making both look modified on the next save.
  The chain an operator walks back through oscillates between two procedures neither of which was
  edited. **It failed worst exactly where there is most to walk back through.**
- A methodology slip worth keeping: an injection reported **zero failures and looked like a
  surviving mutant**, when it had simply failed to compile. A loop that counts failing tests cannot
  tell those apart — **"the test did not go red" has two causes, one a finding and one a measurement
  error** — so the loop now checks.

## Evidence

Session 2026-08-16T11:43Z–14:02Z.
