---
id: 0068
title: A timestamp is checked against a second measurement, not against a constant
status: accepted
date: 2026-08-03
supersedes: []
superseded_by: []
principles: [0095]
tags: [engine, process]
---

# A timestamp is checked against a second measurement, not against a constant

## Context

An intermittent probe failure had been left **diagnosed but unfixed on purpose**, with the diagnostic
strengthened so that the next occurrence would say something. It did:

```
measured via GpuTimestamp: light 0.103 ms, heavy 0.095 ms, ratio 0.9x
```

**It was not the host clock.** Every earlier guess had been about host timing and contention, and all
of them were wrong. Two million points at size 40 measured 0.095 ms — *lighter* than sixty-four
points. On the failing runs the adapter advertises `TIMESTAMP_QUERY`, the probe's calibration passes,
**and the timestamps are meaningless**.

Not a defect in the test. A defect in the probe, and exactly what the `README` had warned about in the
abstract: timestamps are advertised, enabled, and untrustworthy. The governor budgets from this
number, so a 60 ms Set can look nearly free.

**Waiting for the number instead of guessing at it was the decision that paid.**

## Decision

The calibration guard was a **constant floor of 0.1 ms**, against a workload of roughly five hundred
million transcendental operations that should take **tens of milliseconds**. The lying measurement,
0.095 ms, missed the floor by **five microseconds**. The documentation itself recorded that a `> 0`
check had let garbage through and a floor was added — the floor was simply raised from 0 to a number
the driver's lie also clears.

**There is no safe constant here.** Low lets garbage through; high rejects a genuinely fast GPU.

So: **compare against a second measurement of the same work, whose bias direction is known.** The
calibration workload can be timed by the GPU timestamp and the host clock **in the same submit**. The
host clock includes submit and synchronisation overhead, so it is **biased upward** — an upper bound
on the true GPU time. Requiring `gpu_ns >= host_ns / 4` catches 0.095 ms against 60 ms instantly, and
**the magic constant disappears**: the test is the same on a fast machine and a slow one.

Two further decisions:

- **Every measurement, not only calibration.** The evidence is precisely *calibration passed and a
  later measurement lied*, and the documentation already said the behaviour varies between attempts
  within one process. **One verdict is not a lifetime guarantee.**
- **On detection, that `Probe` is pinned to the host clock for its life**, and measurements already
  taken are redone. The probe's own documentation says numbers from the two methods must not be
  compared, and the governor sums them — a mid-life switch mixes scales in a budget with no way to
  tell which is which. This is why `run` became `&mut self`.
- **Below 5 ms of host time the check does not run.** For a light candidate the host figure is mostly
  its own overhead, so the ratio would reject honest measurements — and there is no danger there,
  since cheap reading as cheap is right on either clock. The check bites only on the dangerous side:
  **expensive material appearing nearly free.**

## Consequences

- The observed numbers, 0.095 ms against 60 ms, went straight into a test, together with an assertion
  that they clear the old floor by five microseconds.
- **The test's threshold was never relaxed.** Loosening it until the reports stopped would have kept
  this hidden — and it is the same instinct that made the previous failure worth leaving unfixed.

## Evidence

Session 2026-08-03T04:00Z–04:34Z, commit `328a80a`. Standing rules:
[P-0095](../principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md)
for what the probe does, and `docs/contributing.md` §1, *Working style* for how a
reader checks a number — P-0088 stated both and was split on 2026-09-05.
