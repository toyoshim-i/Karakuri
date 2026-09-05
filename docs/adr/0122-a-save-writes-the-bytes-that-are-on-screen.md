---
id: 0122
title: A save writes the bytes that are on screen
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: [0087]
tags: [store, engine]
---

# A save writes the bytes that are on screen

## Context

The live save control — `k`, a `save` record, a replay that skips it out loud
([ADR-0120](0120-a-record-may-reach-outside-the-stream.md)) — landed, and the first review returned a
**critical**.

**A save could write down a version that never reached the screen.** When `playing[slot]` is `None`,
the code fell back to re-reading the startup paths. But `None` does not only mean *nothing has been
built*: it is also **a slot's first rollback**, because `previous[slot]` stays `None` until a second
build swaps in. Run starts on A, an edit to B lands, the governor refuses B, the rollback restores A
— the screen shows A, the disk holds B, and `k` saves **B**.

Two more ways in, both ordinary: an edit that never compiled sits on disk indefinitely; and `--mcp`
without `--watch` creates no watcher at all, so the screen and the disk diverge permanently while the
run still counts as editable.

**And it fails in the worst available direction.** If the edit also changed the `kind` line, the
startup layer and index are paired with the new file's bytes; `refuse_unwritable` only inspects
`Node.layer`, so the save returns `Ok` and prints *saved as set X — load it with `--load-set X`* —
and `--load-set X` then dies with `WrongKind`. **A success message for a file that cannot be loaded.**

**An existing test had codified the bug**, because it hand-built `playing` and therefore only ever
covered a rollback onto a previous *rebuild*.

## Decision

**Remove the second answer.** `Sources` stops being an enum: the `Startup` arm and the path-reading
path are gone. A new `Running` type owns `playing` and `previous` for the deck and is seeded by
`Running::at_launch`, which reads every launch `.kir` and puts it in the store **before the first
frame** — so *what bytes is this node running* has exactly one derivation.

## Consequences

- **The evidence arrived in its best possible form.** The two new tests were run against the
  **unmodified pre-fix code** and both failed — the defect demonstrated rather than injected:
  *the save wrote down the version the governor refused — the one left on disk, which never reached
  the screen.*
- **A side effect repaired an existing replay divergence**, and it was not allowed through on that
  basis. Rollback onto the launch version had emitted **no `procedure` record**, so a replay kept
  drawing the withdrawn build — performance and replay silently disagreeing, which is this system's
  quietest failure. It was caught by **reading the agent's report rather than trusting it**: a
  behaviour change with no test beside it, so a test was written and watched to fail first.
- A by-product: **a run that only opens a window now writes nothing.** An intermediate state had been
  creating `.karakuri/` in the working directory while the manual said it did not; made lazy, so the
  declaration and the implementation agree again.

## Evidence

Session 2026-08-22T02:25Z–04:57Z, commit `f496e65`.
