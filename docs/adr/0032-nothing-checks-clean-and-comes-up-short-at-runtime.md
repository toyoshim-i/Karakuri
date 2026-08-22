---
id: 0032
title: Nothing checks clean and comes up short at runtime
status: accepted
date: 2026-07-30
supersedes: []
superseded_by: []
principles: [0024]
tags: [ir, process]
---

# Nothing checks clean and comes up short at runtime

## Context

Closing M1 turned up **five defects of one shape**: the validator passes, and the thing is
missing at runtime.

1. **Attribute derivation** — `check` resolved and recorded derived attributes; `codegen` never
   emitted them. A `consumes` satisfied only by derivation passed every stage cleanly and was
   absent when it ran.
2. **Set composition checking** — specified, implemented nowhere.
3. **`spawn` with no `spawn_rate`** — the specification says one sentence in prose; `Proc::spawn_rate()`
   existed **with no caller anywhere**, and the engine defaulted the missing value to `0.0`. A
   procedure with a `spawn` block and no rate parsed, type-checked, cost-checked, built a `Set`,
   and then created **zero elements every step forever**: a black frame, no diagnostic.
4. **`capacity [0, …]`** — an assert firing on the worker thread, silent from the render thread.
5. **`t` frozen across substeps** — the reason substepping exists, not working.

**Not one was found by an existing test.** Three came from reading the specification against the
code, one from doubting a comment, one as a by-product of unrelated work.

## Decision

The stated purpose of the IR pass is that **nothing can check clean and then come up short at
runtime**, and that purpose is treated as a claim to be actively hunted rather than a property to
be assumed. Each of the five got a contract diagnostic and regression tests.

The `spawn_rate` case is the template. It was harmless while `spawn` was wired to nothing;
**connecting compaction is what made it load-bearing**, so a rule that had been decorative became
a live failure without anything about it changing. It is also the single likeliest mistake for a
model writing against the specification, because the rule appears once, in prose, and nothing
else references it.

## Consequences

- Rejection tests carry a **negative control**, so they cannot pass against an implementation
  that rejects every `spawn` block, and each was verified to fail with its check disabled.
- A rule stated only in prose, with no code path referencing it, is a rule that is not enforced.
  `Proc::spawn_rate()` existing with no caller is the visible form of that.

## Evidence

Session 2026-07-27T09:22Z, 2026-07-30T12:52Z. Standing rule:
[P-0024](../principles/0024-nothing-checks-clean-and-comes-up-short-at-runtime.md).
