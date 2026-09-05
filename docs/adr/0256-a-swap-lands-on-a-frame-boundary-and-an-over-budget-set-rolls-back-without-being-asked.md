---
id: 0256
title: A swap lands on a frame boundary, and an over-budget Set rolls back without being asked
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0094]
tags: [engine, live]
---

# A swap lands on a frame boundary, and an over-budget Set rolls back without being asked

## Context

P-0005 — *A swap happens on a frame boundary, and an over-budget Set rolls back on its own* — named
no ADR. It is retiring into
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md),
which carries the alternative it ruled out but not the mechanism or the numbers.

**Both directions were checked.** Records name P-0005 constantly and none of them decides it.
[ADR-0234](0234-carrying-a-show-through-is-a-principle-not-a-property-of-the-finished-instrument.md)
lists it as an instance of the rule it is arguing for — *recovery is automatic "because this runs in
front of an audience"*.
[ADR-0235](0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md)
leans on it to say why `write_procedure` is safe to expose.
[ADR-0041](0041-a-candidate-is-judged-only-on-frames-it-contributed-to.md) refines *which frames
count* and presupposes the trial;
[ADR-0071](0071-there-is-no-panic-key.md) refuses a panic key partly because `HotSwap::previous`
exists only inside the trial window. Every one of them stands on this decision. None states it.

## Decision

**A swap lands only at the top of `HotSwap::begin_frame`, and a candidate that does not hold up is
taken back out by the engine rather than by the operator.**

**Pipelines are double-buffered and exchanged between frames.** The expensive half — parse-checked
IR in, WGSL out, shader modules, pipelines, buffers, bind groups — happens on a worker thread, and
the render thread's side of the channel is `try_recv` and never `recv`: a frame that finds nothing
waiting does nothing about it and renders.

**The outgoing Set is parked rather than dropped, and it is not stepped.** `t` is simulation time
and does not advance for a Set nothing calls `prepare` on, so a rollback resumes it exactly where it
was — the same property `Residency::Allocated` has. A recovery that leaves a hole in the picture is
not one.

**The trial does not start at frame one.** A cold Set's first frames pay for the driver's first use
of each freshly created pipeline, first touch of freshly allocated buffers, and the whole-capacity
element upload `Set::build` leaves staged — megabytes at 262144 elements. `WARMUP_FRAMES` are
discarded before `JUDGE_FRAMES` are measured, and the worker flushes and waits for that upload on
its own thread first, which removes the cost rather than hiding it inside the warmup.

**The verdict is a median over a window rather than a worst case**, because on a host clock a single
sample carries whatever else the scheduler was doing. **One hitch is not a reason to throw away
generated material; thirty frames that all miss is.**

## Alternatives rejected

**Swap when the new pipeline becomes ready.** The moment that is easiest to detect, and it is inside
a frame: a caller can open one encoder and end up recording half of one Set's passes and half of
another's, so the frame that lands is of no generation at all. The frame boundary is the only point
where the question *which Set is this frame* has an answer.

**Leave a bad Set on air until somebody notices.** The recovery plan is then the operator's
attention, which during a set is on the room and is the scarcest thing in the building. It is also
the answer that is available *later* than the one taken: an automatic rollback costs a swap somebody
asked for, and waiting costs a performance.

**Ask the operator before rolling back.** A confirmation on the live path, which buys safety with
the operator's authority — and there is nothing to decide: the candidate is the thing they asked for
and it did not fit, so the only two states are the one they had and the one that misses frames.

**Keep a permanent rollback target so a candidate can be undone at any time.** `HotSwap::previous`
exists only inside the trial window and is retired the moment a build is accepted. Making it
permanent means holding a Set's worth of VRAM per slot for ever, which is refused for its own
reasons in [ADR-0071](0071-there-is-no-panic-key.md).

**Judge on a worst-case sample rather than a median.** It rejects healthy material on one scheduler
hiccup, which is the false-reject direction ADR-0041 records as the expensive failure in a system
whose whole point is generating material.

## Consequences

- **One Live slot's cost cannot be separated from its neighbours'** on this instrument, because a
  frame interval cannot be divided. ADR-0041 records the defect in place; the governor does not use
  the watchdog's number for exactly this reason and uses the per-Set measurement the worker took
  ([ADR-0054](0054-the-governor-budgets-from-the-probe-and-never-touches-a-live-slot.md)).
- **The budget has to sit between one refresh period and two**, because under vsync a frame interval
  quantises to multiples of the period. That is what `DEFAULT_BUDGET_MS` is.
- **The swap transfers no state**: a new procedure means new buffers, so the incoming Set starts
  cold. Warming one out of sight is Priming and belongs to the deck's residency model, and half of
  it built in `swap.rs` would be a second, worse answer the deck would then have to remove.
- **A frame cannot be built from two generations of Sets, structurally, only at the deck.** A
  `HotSwap` driven on its own keeps the weaker conventional property and says so
  ([ADR-0034](0034-the-frame-guard-owns-the-encoder.md)).
