# A check you have not watched fail is guessing

A test is run against the broken implementation and **observed to fail** before it is kept, and a
green result is read by asking what it was green against. Assert the property, not a consequence of
it; a rejection test carries a negative control; a `compile_fail` test has a compiling twin differing
by exactly the thing under test, with the expected error verified to be the only error.

**What it rules out.** Trusting the suite. Changing `Poll` to `Wait` — a full render-thread stall,
the exact thing the module documentation calls a bug — left all seven meter tests passing, because a
`Wait` paces the loop and a paced reading is exactly one frame old, so `frames_behind >= 1` and
`mean > 0.0` are satisfied by a stalled loop. A verification reporting *140 tests pass* while
measuring `HEAD`, where the tell was the number: the suite has 147. Fifteen commits in a row losing a
`Co-Authored-By` trailer, each verified with `git log --oneline -1`, a view in which a trailer is
structurally invisible. Assuming a validation stage does what its name says: five defects checked
clean and came up short at runtime, and **not one was found by an existing test** — among them
`spawn` with no `spawn_rate`, where `Proc::spawn_rate()` existed with no caller anywhere and the
engine defaulted the missing value to `0.0`, so a procedure parsed, type-checked, cost-checked, built
a Set, and created zero elements every step forever: a black frame, no diagnostic. A fixture the
product can rewrite, where the criterion is not *it probably will not change* but **it can change** —
tests derived a second procedure by substituting a sentence in `examples/soft_points.kir`, one of the
files the MCP surface exists to rewrite, and a live session rewrote it until the two procedures were
identical; prepend a line instead, which changes the content whatever the content is. A hand-built
fixture, which dodges by accident: the codegen's naga tests passed while the generator was broken
because the fixtures named their locals `uu` and `vv` rather than the `u` the specification's own
example uses — and `u` was the uniform block, so a `param` named `array` passed all four validation
stages and produced WGSL that would not compile. The blocklist that lost there states why a class is
*unlikely*; mangling states why it is *closed*. A green suite as evidence that a gate works — a
gate's test is something that must not pass, and this one first failed by passing, because
`rustfmt --check` reading stdin prints its diff and exits 0, so judge it by output. And a gate that
takes minutes on every commit, which gets `--no-verify`d and then is not a gate: one second on every
commit, everything on a tag push, because a tag is the deploy.

**Where it holds.** [check.rs](../../crates/karakuri-ir/src/check.rs) is the pass whose stated purpose
is that nothing checks clean and comes up short at runtime, and `tests/check.rs` beside it is where
each refusal carries a negative control — a `spawn` block *with* a rate, a range whose minimum is
exactly 1 — so a checker that refused everything would fail them.
[lower.rs](../../crates/karakuri-codegen/src/lower.rs)'s `mangle_local` is `usr_{name}`
unconditionally, which states why the class is closed rather than why a collision is unlikely, and
its regression fixtures name their locals after the generated identifiers on purpose rather than
around them. [deck.rs](../../crates/karakuri-engine/src/deck.rs)'s `Deck::begin_frame` carries a
`compile_fail` doctest and a `no_run` twin differing from it by exactly the second borrow, with the
reason written between the two. `karakuri-engine/tests/meter.rs` asserts the property rather than a
consequence of it: over 240 unpaced frames either a reading is more than one frame behind or a
measurement was skipped, and a `Wait` anywhere in the frame path makes both impossible.
`karakuri-cli/tests/fixtures/flat.kir` is a fixture the product cannot reach, and the write test
prepends a line rather than substituting one. And [.githooks/pre-commit](../../.githooks/pre-commit)
judges `rustfmt --check` by its output rather than by its exit code, with the reason at the line —
reading stdin it prints the diff and exits 0, which is how the gate first failed by passing. Decided
in [ADR-0014](../adr/0014-generated-code-cannot-be-captured-by-a-name-a-procedure-can-spell.md),
[ADR-0030](../adr/0030-simulation-time-comes-from-an-integer-step-count.md),
[ADR-0032](../adr/0032-nothing-checks-clean-and-comes-up-short-at-runtime.md),
[ADR-0044](../adr/0044-a-test-that-survives-mutation-is-not-a-test.md),
[ADR-0087](../adr/0087-a-fixture-the-product-can-rewrite-is-not-a-fixture.md),
[ADR-0093](../adr/0093-a-verification-that-measures-the-wrong-tree-verifies-nothing.md),
[ADR-0102](../adr/0102-a-renderers-address-is-layer-and-index.md),
[ADR-0103](../adr/0103-a-trailer-missed-fifteen-times.md),
[ADR-0109](../adr/0109-format-the-workspace-and-split-the-gate.md) and
[ADR-0114](../adr/0114-tests-run-when-somebody-asks-not-when-git-does.md).
