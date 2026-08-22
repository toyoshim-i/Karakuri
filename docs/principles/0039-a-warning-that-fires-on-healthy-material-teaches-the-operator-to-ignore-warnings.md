# A warning that fires on healthy material teaches the operator to ignore warnings

`bad_texels` is displayed and is **not** a fault indicator — no threshold, no flag, nothing anywhere
saying "broken". Non-finite texels are commonplace and harmless, and that is exactly what makes them a
poor proxy for *the material is wrong*.

**What it rules out.** An indicator that is right most of the time, which is the same objection as
[P-0035](0035-arithmetic-and-an-operator-key-beat-a-rule-that-is-usually-right.md) wearing different
clothes. It also rules out the panic key: an anomaly is usually one frame and usually harmless, so a
control that resets the performance in response does more damage than what it responded to — and one
learned to be sometimes wrong becomes a control the operator hesitates over, which is the single
property a panic key must not have.

**Show the number, claim nothing.** The operator can see the count and decide; the system does not tell
them what it means.

**Where it holds.** [meter.rs](../../crates/karakuri-engine/src/meter.rs); the status line in
`karakuri-cli`. Decided in [ADR-0071](../adr/0071-there-is-no-panic-key.md), reached three separate
times in one milestone — the tempo octave, semi-automatic gain, and this.
