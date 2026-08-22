# An invariant that can be tested is tested, not asserted

Where an invariant can be checked mechanically, a test checks it. `karakuri-signal` scans its own
source for `Instant::now`, so "rendering reads only the local oscillator" is a property of the
crate rather than a claim about it. The vertical slice is verified by offscreen render and
readback — elements appear, additive blending accumulates, the same seed reproduces bit for bit —
rather than by a screenshot. Generated WGSL is compiled by naga in a test, because a generator
that has never been through a compiler emits plausible invalid code.

**What it rules out.** Invariants that live only in a `README` and are enforced by review.
Everything above was found by a test that a reviewer had already read past.

**And a caveat with teeth.** A **hand-built fixture dodges bugs by accident**: the naga tests
passed while the generator was broken, because the fixtures named their locals `uu` and `vv`
instead of the `u` and `v` the specification's own example uses. A fixture is the real text, or
it is adversarial on purpose.

**Where it holds.** `no_clock_access.rs` in [karakuri-signal](../../crates/karakuri-signal);
`naga_test.rs` in [karakuri-codegen](../../crates/karakuri-codegen). Decided in
[ADR-0017](../adr/0017-an-invariant-that-can-be-tested-is-a-test.md).
