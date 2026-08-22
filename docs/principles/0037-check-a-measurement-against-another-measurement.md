# Check a measurement against another measurement, not against a constant

To tell whether an instrument is lying, measure the same work a second way whose **bias direction you
know**, and compare. The GPU timestamp is checked against the host clock over the same submit: the
host clock includes submit and synchronisation overhead, so it is biased upward and is an upper bound
on the true GPU time. `gpu_ns >= host_ns / 4` catches a driver reporting 0.095 ms for work that takes
60 ms, and it is the same test on a fast machine and a slow one.

**What it rules out.** A threshold. The guard this replaced was a constant floor of 0.1 ms against a
workload that should take tens of milliseconds, and the lying measurement missed it by **five
microseconds** — a floor that had already been raised once, from `> 0`, for exactly the same reason.
**There is no safe constant here:** low lets garbage through, high rejects a genuinely fast GPU.

**One verdict is not a lifetime guarantee.** Calibration passed and a later measurement lied, so the
check runs on every measurement — and once an instrument is caught, it is **pinned to the fallback for
its life** and earlier numbers are redone, because a budget that mixes two scales cannot say which is
which.

**Bite only on the dangerous side.** Below 5 ms of host time the check does not run: the host figure
is then mostly its own overhead, and cheap reading as cheap is right on either clock. What must never
pass is expensive material appearing nearly free.

**And never relax the threshold to stop the reports.** Loosening it would have hidden this.

**Where it holds.** [probe.rs](../../crates/karakuri-engine/src/probe.rs). Decided in
[ADR-0068](../adr/0068-a-timestamp-is-checked-against-a-second-measurement-not-a-constant.md).
