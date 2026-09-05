---
id: 0056
title: Beat lock is feed-forward, and the unmeasurable part is an offset
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: [0088]
tags: [signal, engine]
---

# Beat lock is feed-forward, and the unmeasurable part is an offset

## Context

The requirement, stated directly: following the tempo is not enough, the **beats have to land**, and
some overshoot mechanism is needed to make them.

Restating it as a control problem is what produced the design. There are **two latencies and they do
not cancel**:

- **`A`, analysis.** A beat sounding at time `T` cannot be detected until the input buffer
  containing it arrives and the analysis window covering it completes — roughly one buffer plus half
  a window for a centred estimate.
- **`D`, output.** A frame prepared at `P` becomes photons at `P + D`: rendering, queue depth
  (`desired_maximum_frame_latency` is 2), present, and the display's own pipeline.

A naive loop aligns the oscillator's **current** phase to music that is already `A` old, and shows
the result `D` later. The visible beat is late by **`A + D`, consistently** — which is worse than a
random error, because a consistent offset reads unmistakably as *not matching*.

## Decision

The oscillator runs **`A + D` ahead** of the estimate. This is the requested overshoot, and it is a
**feed-forward term, not a tuning constant**:

> At time `P`, the oscillator's phase should be the phase the music will have at `P + D`. The
> estimate in hand describes the music at `P − A`.

`A` and `D` are exposed as numbers with their derivations. Buffer length, window length and queue
depth are known — **the display's own latency is not.** That part is an **offset the operator tunes
by ear**, presented as an offset rather than as a measurement, so nothing pretends to have measured
what it cannot.

**The invariant does not move.** Rendering reads only the local oscillator. The analyser never
becomes a clock; it **corrects** one.

## Consequences

- Rate and phase are damped **separately**. Tempo slowly, phase quickly, because **a tempo error
  accumulates and a phase error costs one bar.**
- The half-beat boundary is decided rather than left: at exactly half a beat "slightly early" and
  "slightly late" are equivalent, and left alone the correction flips forever.
- **Gated on confidence.** With a weak or absent estimate the oscillator free-runs. Unplug the
  interface mid-set and the picture keeps its tempo rather than stopping or lurching.
- Prediction is the working assumption: the estimate is right while the tempo holds, tempo changes
  are brief and mostly at a track change, and error during the follow is tolerated.

## Evidence

Session 2026-08-01T06:49Z–06:57Z. Sharpens
[P-0088](../principles/0088-no-number-is-trusted-further-than-its-instrument-has-been-checked.md).
