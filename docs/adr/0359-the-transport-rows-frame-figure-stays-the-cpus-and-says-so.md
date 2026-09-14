---
id: 0359
title: The transport row's frame figure stays the CPU's, and says so
status: accepted
date: 2026-09-14
supersedes: []
superseded_by: []
principles: [0095]
tags: [ui, console, transport, measurement, m6]
---

# The transport row's frame figure stays the CPU's, and says so

## Context

The transport row draws `58 fps · 12.4/16.6 ms + 0.98 chain`. The `12.4` is `Cost::whole`, the
sum of the three CPU stretches this program times inside a frame — the engine's half of
`compose`, the panel's immediate-mode pass, and the upload and the one submission — and it stops
at the submission. What the GPU then takes is not on that clock, so a frame the GPU is holding up
reads as a short one, and the page's tip said *"12.4 ms of 16.6 — there is headroom"*.

[ADR-0303](0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md)
defined the frame's cost as its period, wall clock from one redraw to the next, and left the row
to the maintainer: *"which number that row should carry, and whether it grows a second one, is
the maintainer's and is not taken here."*

The GPU's own time is not available per frame. GPU timestamp queries do not work on the two
backends this program runs on
([ADR-0169](0169-the-timestamp-verdict-is-the-backends-not-the-machines.md)), and the one
measurement that does reach the GPU — `Cost::drained`, `Device::poll` to a drained queue on the
host clock — stalls the CPU until the GPU catches up, so it is taken once every `Costs::AUDIT`
(500 ms) and printed in the readout rather than drawn every frame.

## Decision

The maintainer's, on 2026-09-14.

**The figure stays the CPU's, and the row says so.** It draws `cpu 12.4/16.6 ms`, and the tip
says what the three stretches are and that the GPU is not in them. The rate beside it is what
says whether the display is being kept; the CPU figure is what this program's own code costs, which
is the number a schedule of live regions is built from.

**The period is not put in its place.** On a `Fifo` surface that is keeping up the period is the
refresh interval by construction, so `16.6/16.6` would be drawn on every frame that is fine and
carry nothing the rate does not; on a frame that is late it exceeds the interval, which the rate
already shows as a drop. A figure that reads full whenever nothing is wrong is not a reading.

**And the promise of headroom leaves the tip.** A reading that stops at the submission cannot
say there is headroom ([P-0095](../principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md)).

## Alternatives rejected

**The period in place of the CPU figure.** Above: it follows ADR-0303's definition and says
nothing the rate does not.

**Both side by side.** `cpu 12.4 · frame 16.7 / 16.6` — the second number is the rate restated in
milliseconds, and the row is a readout that has to stay short.

**The audited `drained` figure as a GPU column.** It is an upper bound biased high, taken once in
tens of frames, and it includes whatever of the previous frame was still in flight; drawing it as
*the GPU's time* beside a per-frame CPU figure would put a sampled bound next to a measurement
with nothing on the row saying which is which. It stays in the readout, where the print says what
it is. If a per-frame GPU number is ever wanted, it is the timestamp path on a backend that
supports it (ADR-0169), at that backend's price.

## Consequences

- `view::transport::frame_job` draws `cpu ` before the figure; the mock and the tip on the
  console page follow.
- The roadmap's M6 item on the transport figure is settled by this record.
- `Report::headroom_ms` — the governor's budget-side prediction — is a different number from
  anything on this row and stays undrawn until the deck-total question is decided.
