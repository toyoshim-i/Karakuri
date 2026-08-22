# Nothing checks clean and comes up short at runtime

That is what the validation pass is for, so it is treated as a claim to be hunted rather than a
property to assume. A rule stated only in prose, with no code path referencing it, is not enforced —
`Proc::spawn_rate()` existed with no caller anywhere while the engine quietly defaulted the missing
rate to zero, so a `spawn` block with no rate passed every stage and created **no elements, ever,
with no diagnostic**.

**What it rules out.** Trusting the suite. Five defects of exactly this shape were found closing
M1 — derived attributes resolved but never generated, Set composition specified and implemented
nowhere, the missing `spawn_rate`, `capacity [0, …]` asserting on a worker thread and silent from
the render thread, `t` frozen across substeps — and **not one was found by an existing test.** Three
came from reading the specification against the code, one from doubting a comment, one as a
by-product of unrelated work.

**A rule can become load-bearing without changing.** The `spawn_rate` requirement was harmless while
`spawn` was wired to nothing; connecting compaction is what turned a decorative rule into a live
failure.

**Where it holds.** `check.rs` in [karakuri-ir](../../crates/karakuri-ir). Decided in
[ADR-0032](../adr/0032-nothing-checks-clean-and-comes-up-short-at-runtime.md).
