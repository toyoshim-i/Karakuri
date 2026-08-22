---
id: 0106
title: An error scope, not a second process
status: accepted
date: 2026-08-18
supersedes: []
superseded_by: []
principles: []
tags: [engine, process]
---

# An error scope, not a second process

## Context

Things kept taking the process down, and the main application must not fall over — so the question
was whether an out-of-process design would save those cases.

## Decision

**Worth checking, and the answer is that the cheapest fix was already in the dependency tree.**

wgpu 26 has `push_error_scope` / `pop_error_scope`, which return a validation error **as a `Result`**
— and by the WebGPU specification an error a scope captures **never reaches the uncaptured handler**,
so it does not panic.

**All five crashes of the session came back as messages**, each reproduced by disabling its own
refusal. The fifth is the one that decides it: a buffer-size limit is **not a shader**, so
pre-validating the generated WGSL with naga — the other cheap option — catches four of five. **The
scope is the better net, measured rather than argued.** It also removes the dependence on
`catch_unwind` in `swap.rs`, which unwinds across a panic and was never something to want.

**Out-of-process saves device loss, a GPU hang and a driver crash — and none of the five was one.**
Nor would compiling out of process save them, because they happen while *rendering*; saving them
means the frame path crossing a process boundary, which a real-time renderer cannot pay. `wgpu` has
`set_device_lost_callback`, and the deck can already rebuild a Set from records, so in-process
recovery is the live option if it ever matters.

## Consequences

- **The message says it is a bug report, not a diagnostic.** Everything the net catches is something
  the **check pass should have refused with a sentence about the `.kir`**, so whoever sees it should
  know they found a hole rather than made a mistake, and the text says so and asks for the procedure.
  **The net is a net, not a plan.**
- The body moved into `build_inner`, because a scope must be popped on **every** path and the outer
  function returns early in a dozen places. **A scope left on the stack catches the next build's
  error and reports it as this one's.**
- A build that errors is discarded rather than returned: the resources it named do not exist, and a
  handle to them is something wgpu would refuse again at the first draw, by which time nobody is
  watching.
- Recorded as something whose value can now be judged from experience rather than argued.

## Evidence

Session 2026-08-18T08:37Z–08:51Z.
