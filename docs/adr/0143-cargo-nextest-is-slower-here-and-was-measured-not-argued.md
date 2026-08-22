---
id: 0143
title: cargo-nextest is slower here, and was measured rather than argued
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [testing, workflow]
---

# cargo-nextest is slower here, and was measured rather than argued

## Context

`cargo test` runs a crate's test binaries **one after another**. `karakuri-engine` has 30 of
them, each taking between 3.5 and 11.5 seconds, so a large share of the crate's wall clock
looked like binaries queueing rather than tests running. `cargo-nextest` runs every test in
its own process and parallelises across binaries, which is the obvious remedy for exactly
that shape.

## Decision

**Rejected.** Measured on this machine (Apple M4 Pro, Metal), warm, same tree:

| | wall clock |
|---|---|
| `cargo test -p karakuri-engine` | **373 s** |
| `cargo nextest run -p karakuri-engine` | **572 s** |

53% slower. All 361 tests passed, so it never even reached the flakiness question.

**The criteria were fixed before the measurement**, deliberately: reject on any flake without
investigating, reject if stability needs configuration, reject if the speed-up is under 2×.
Deciding afterwards is how a number gets argued with.

## Why it loses

Every test here builds its own `wgpu` instance, adapter and device, and the per-test cost is
pipeline construction rather than device creation
([ADR-0140](0140-a-gpu-test-lives-under-mod-gpu-and-the-rule-is-enforced-both-ways.md)).
Splitting tests into separate processes gives up whatever the driver and the process were
amortising across a binary's tests, and that loss is larger than the serial-binary queueing
it removes. The premise — that the queueing was the cost — was wrong.

## What this is really a record of

**Two hypotheses about GPU cost in this workspace were tested today and both came back
inverted.** A shared device made a target's serial run *slower* (48.7 s → 53.3 s) and broke
a test in parallel; nextest made a crate's run 53% slower. Intuition about where this
suite's time goes has been wrong every time it has been checked. The measurement is cheap —
one install and one run — and it is the only thing that has been right.

## What would change this

A machine where process startup is cheap relative to device creation, or a workspace where
tests share a device within a binary. Neither is true here, and the second was itself
measured and rejected. If the suite ever moves to a runner that reuses one process per
binary while parallelising across them, this is worth re-measuring — not re-arguing.
