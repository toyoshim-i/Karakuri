# A test meant to catch something is run against the defect

A regression test is run against the broken implementation and **observed to fail** before it is
kept. A rejection test carries a negative control, so it cannot pass against an implementation that
rejects everything. A `compile_fail` test has a compiling twin differing by exactly the thing under
test, and the expected error is checked to be the *only* error.

**What it rules out.** A test that passes for a reason other than the one it claims. The generated-code
fixtures passed while the generator was broken, because they named their locals `uu` and `vv` instead
of the `u` and `v` the specification's own example uses.

**Assert the property, not a consequence of it.** The meter's headline claim is that it never blocks
the frame path, and a test carried that name. Changing `Poll` to `Wait` — a full render-thread
stall, the exact thing the documentation calls a bug — left **all seven tests passing**, because a
`Wait` paces the loop and a paced reading is exactly one frame old, which satisfies every assertion.
Four such tests were found in a single review round. The class had moved up a level: from
specification versus implementation to **the test versus the property it believes it protects**.

**And an invariant test beats a golden output.** When `t` stopped accumulating drift, the static
render's baseline hash changed — correctly, since the old value *contained* the drift. Losing a
baseline costs nothing when what replaces it states the property: two tick histories reaching the
same elapsed time render the same pixels.

**Where it holds.** `tests/lifecycle.rs` and `tests/check.rs`; the `compile_fail` twin on
`Deck::begin_frame`. Decided in
[ADR-0030](../adr/0030-simulation-time-comes-from-an-integer-step-count.md),
[ADR-0032](../adr/0032-nothing-checks-clean-and-comes-up-short-at-runtime.md) and
[ADR-0044](../adr/0044-a-test-that-survives-mutation-is-not-a-test.md).
