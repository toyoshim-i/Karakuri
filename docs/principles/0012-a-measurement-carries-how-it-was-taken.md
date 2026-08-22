# A measurement carries how it was taken

A `Measurement` records its own provenance, the way a signal carries confidence. A consumer can
see that a timing came from the host clock rather than a GPU timestamp; it never receives a
number whose meaning it cannot check.

**What it rules out.** Trusting a feature flag — this platform advertises `TIMESTAMP_QUERY` and
returns zeros, negatives and plausible values for the same load run to run. And silently falling
back, which produces the specific failure this exists to prevent: `0.0 ms` reads as a very fast
shader, and stage 8 promotes it. Degeneracy is detected by calibration against a load that
cannot be zero, not by asking the driver what it supports.

**What cannot be measured is offered as an offset, not as a number.** Beat lock needs the analysis
delay and the output delay; buffer length, window length and queue depth are known, and **the
display's own latency is not**. That part is a dial the operator tunes by ear, presented as an offset,
so nothing pretends to have measured what it cannot. The same rule covers a figure that was inferred
rather than taken: the meter's lag on a vsync'd window is stated as inferred from queue depth,
because measuring it needs a surface.

**Where it holds.** [probe.rs](../../crates/karakuri-engine/src/probe.rs);
[meter.rs](../../crates/karakuri-engine/src/meter.rs). Decided in
[ADR-0015](../adr/0015-a-measurement-carries-how-it-was-taken.md) and
[ADR-0056](../adr/0056-beat-lock-is-feed-forward-and-the-unmeasurable-part-is-an-offset.md).
