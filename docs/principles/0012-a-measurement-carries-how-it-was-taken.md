# A measurement carries how it was taken

A `Measurement` records its own provenance, the way a signal carries confidence. A consumer can
see that a timing came from the host clock rather than a GPU timestamp; it never receives a
number whose meaning it cannot check.

**What it rules out.** Trusting a feature flag — this platform advertises `TIMESTAMP_QUERY` and
returns zeros, negatives and plausible values for the same load run to run. And silently falling
back, which produces the specific failure this exists to prevent: `0.0 ms` reads as a very fast
shader, and stage 8 promotes it. Degeneracy is detected by calibration against a load that
cannot be zero, not by asking the driver what it supports.

**Where it holds.** [probe.rs](../../crates/karakuri-engine/src/probe.rs). Decided in
[ADR-0015](../adr/0015-a-measurement-carries-how-it-was-taken.md).
