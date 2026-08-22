---
id: 0030
title: Simulation time comes from an integer step count, and advances per substep
status: accepted
date: 2026-07-27
supersedes: []
superseded_by: []
principles: [0002, 0025]
tags: [engine, determinism]
---

# Simulation time comes from an integer step count, and advances per substep

## Context

Wiring compaction into the L1 dispatch, the implementing agent reported that substepping was
not bit-exact and had never been: `Set::t` was an **f32 running sum**, so twenty steps taken
singly and ten taken in pairs land 2 ULP apart. It recorded the fact in the `README`'s list of
fragile things and moved on.

That is the wrong disposal. It is the roadmap's own settled invariant — *two tick histories
reaching the same elapsed time are the same point in the session* — and an invariant is not
something a note discharges.

Digging found a larger fault underneath. **`t` was advancing once per frame.** A frame of two
steps ran both element passes **at the same instant**. Not accumulated drift: a whole step of
error.

**Why no test caught it.** Every example in the repository computed `position` as a closed-form
function of `t`, and a closed-form procedure overwrites its intermediate values. That entire
class of procedure **cannot detect this defect in principle**, and the whole corpus was of that
class.

## Decision

Split the per-frame state on one criterion: **is it an input, or is it the simulation's own
state?**

- **Inputs** — `param` values, signal bindings, the camera. Genuinely sampled once per frame,
  and they stay in the uniform.
- **The simulation's clock** — `t` and the spawn count. Moved to a **per-substep** buffer.

And `t` **stops accumulating**: it is computed as `steps_taken * dt` from an integer count.

The spawn accumulator moves with it, and for its own reason: advanced once per frame as a single
batch, one frame of two steps inserts both batches before the second element pass, where two
frames of one step interleave them. Advanced per substep it sums to the identical per-frame
quantity and substepping survives. (The specification's `carry += spawn_rate * dt * float(steps)`
was written for the frame reading and was corrected — a specification an LLM generates against
must not be knowingly wrong.)

## Consequences

- **The static render's golden hash changed, deliberately.** The old value carried the
  accumulation drift; the new one is exactly `240 * dt`. The baseline is gone and **an invariant
  test replaces it**, which is the stronger instrument: it states the property rather than
  freezing an output.
- The new test is a procedure that **accumulates** a `t`-dependent term —
  `position = position + vec3(t, 0.5, -t) * dt * 0.05` — walked to 21 steps as 1×21 and as 3×7,
  compared per pixel. It was **run against the old implementation and observed to fail**, without
  which it guards nothing.
- Writing `spawn_count` and `seed_base` per substep forced them out of the single uniform —
  `queue.write_buffer` cannot be interleaved into an encoder's existing commands — so they live
  in a `MAX_STEPS`-entry buffer with one prebuilt bind group per substep. This incidentally
  removed the last fields that exceeded the specification's literal uniform list.

## Evidence

Session 2026-07-27T03:58Z. Standing rules:
[P-0002](../principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md),
[P-0025](../principles/0025-a-test-meant-to-catch-something-is-run-against-the-defect.md).
