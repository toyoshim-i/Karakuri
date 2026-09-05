---
id: 0022
title: Revision goes param, then range, then regeneration
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: [0085]
tags: [process, ir]
---

# Revision goes param, then range, then regeneration

## Context

The expected way of working is a person watching a preview and saying "a bit slower, pull back,
hold at the top of the arc". Treating every such instruction as a generation request is the
obvious implementation and the wrong one.

## Decision

A revision tries three steps in order and stops at the first that works:

1. **Within an existing param's range** → a uniform write. No fork, no compile, no priming;
   it lands inside the frame.
2. **Beyond the range but structurally the same** → change the range. A fork, no compile.
3. **Structurally different** → regenerate, compile, prime.

**The mandatory-range rule is what pays for this.** A model told that `speed` is `[0.0, 2.0]`
and currently `0.15` can turn "slower" into `0.08`. Without a range it can judge nothing and
every instruction falls through to code generation. The specification listed three jobs for a
range — the fader's extent, an agent's search space, the normalisation basis for a signal
binding. This is the fourth.

**The camera is the place to build this loop first.** It is low-dimensional, it is directly
describable in human words, and the judgement being made is *against the music*, which is the
part the system cannot evaluate. And while L3 does not exist the camera is a record —
`{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15}` — so revising it needs **step 1 only**
and no code generation at all. When L3 arrives the same loop's interior changes from a record to
a procedure and nothing else moves.

## Alternatives rejected

- **Regenerate for every instruction.** Slower by orders of magnitude, and it discards the parts
  that were already liked.

## Evidence

Session 2026-07-26T02:23Z. Standing rule:
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md).
