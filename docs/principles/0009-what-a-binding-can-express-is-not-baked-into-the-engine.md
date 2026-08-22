# What a binding can express is not baked into the engine

If a behaviour can be produced by binding a signal to a declared `param`, the engine does not
implement it as a fixed property. A binding stays adjustable — its depth is `range`, its shape
is `curve`, its period is `rate` in cycles per beat — and an engine constant does not.

**What it rules out.** A Poisson spawn process, which bakes in an irregularity nobody can turn,
when noise bound to `spawn_rate` gives the same look under control. By the same test it ruled
out the per-step white noise that replaced it: correct, deterministic, and periodless, which is
the same defect one layer down.

**The test has a boundary.** A correction the engine must apply *whether or not* a procedure
asks — the birth fraction that removes spawn banding — belongs to the engine precisely because
a binding would let it be forgotten. Adjustable belongs in a binding; obligatory does not.

**Where it holds.** [ir-spec.md](../ir-spec.md), "Spawn timing" and "Binding noise". Decided in
[ADR-0005](../adr/0005-spawn-is-an-accumulator-and-the-engine-owns-the-birth-fraction.md) and
[ADR-0011](../adr/0011-a-noise-binding-carries-a-kind-and-a-rate-in-beats.md).
