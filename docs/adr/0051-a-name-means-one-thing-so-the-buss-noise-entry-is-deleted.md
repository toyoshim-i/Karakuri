---
id: 0051
title: A name means one thing, so the bus's `noise` entry is deleted
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: [0031]
tags: [signal, format]
---

# A name means one thing, so the bus's `noise` entry is deleted

## Context

After [ADR-0050](0050-a-declared-generator-is-certain.md), `"noise"` meant **two different things at
two different confidences**. `SynthesizedBus::sample` answered it with a default generator — signed,
`[-1,1)`, confidence 0.1 — while `Binding::sample` intercepted the same name and returned the
declared generator mapped to `[0,1]` at confidence 1.0. **Which you got depended only on which door
you came in by.**

The specification already carried the rule, written for record tags and argued on general grounds:
*it is not enough to be disjoint in practice; they have to be disjoint by name.*

## Decision

**The bus's `"noise"` entry is removed.** The name belongs to the `bind` record alone.

The entry was genuinely dead — the only consumers intercept it — and keeping a dead entry was not
defensible on completeness grounds either, because bus completeness is already supplied by the
unknown-name arm. It bought nothing and cost a second meaning.

Deleting it made `SynthesizedBus`'s `seed` field dead, and that was removed too: **a seed carried
and read by nothing reads as randomness that has been accounted for.** Every remaining signal is a
pure function of `t` and `bpm`.

Related, and refused rather than accommodated: `signal: "noise"` with no `noise` object silently
became a default generator. In a codebase that makes `param` ranges and `capacity` mandatory
specifically so nothing is guessed, one place quietly filling in a default is out of pattern.

## Evidence

Session 2026-08-01T03:17Z–05:31Z. Standing rule:
[P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md).
