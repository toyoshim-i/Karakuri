# Architecture Decision Records

How this directory and [../principles/](../principles/) are run is
[ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md). Read that first.

An ADR is not edited after it lands, except to set its status. A retired number is never
reused; when one is superseded it stays in the table with a pointer to what replaced it. This
index is maintained by hand — see ADR-0000 for when that stops being enough.

`principles/` has no index. `ls docs/principles/` is the index, because each filename is the
rule it states.

**Reconstruction is in progress.** Records are being recovered in date order from the session
history that runs from 2026-07-25. Everything through **2026-07-26** is written; the numbering
is chronological, so later dates take later numbers.

## Records

| | Decision | Date | Status |
| --- | --- | --- | --- |
| [ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md) | Record decisions here and standing rules in principles/ | 2026-08-22 | accepted |
| [ADR-0001](0001-identity-is-seed-an-ordinal-not-a-slot-index.md) | Identity is `seed`, an ordinal, and `id` is dropped | 2026-07-25 | accepted |
| [ADR-0002](0002-compaction-preserves-order-and-determinism-is-bit-exact.md) | Compaction preserves order, and determinism means bit-exact | 2026-07-25 | accepted |
| [ADR-0003](0003-hash-builtins-are-salted-from-the-seed-stream.md) | Hash builtins are salted from the layer's seed stream | 2026-07-25 | **superseded by ADR-0024** |
| [ADR-0004](0004-var-is-added-so-a-loop-can-carry-a-value.md) | `var` is added so a loop can carry a value | 2026-07-25 | accepted |
| [ADR-0005](0005-spawn-is-an-accumulator-and-the-engine-owns-the-birth-fraction.md) | Spawn is an accumulator, and the engine owns the birth fraction | 2026-07-25 | accepted |
| [ADR-0006](0006-the-step-count-is-a-record-not-a-measurement.md) | The step count is a record, not a measurement | 2026-07-25 | accepted |
| [ADR-0007](0007-a-set-file-is-a-projection-and-a-session-stream-is-the-timeline.md) | A Set file is a projection; a session stream is the timeline | 2026-07-25 | accepted |
| [ADR-0008](0008-blend-is-declared-now-so-the-second-mode-is-an-addition.md) | `blend` is declared now, so the second mode is an addition | 2026-07-25 | accepted |
| [ADR-0009](0009-capacity-is-a-dial-not-part-of-a-procedures-identity.md) | `capacity` is a dial, not part of a procedure's identity | 2026-07-25 | accepted |
| [ADR-0010](0010-one-t-value-is-one-record-shape.md) | One `t` value is one record shape, across every file | 2026-07-25 | accepted |
| [ADR-0011](0011-a-noise-binding-carries-a-kind-and-a-rate-in-beats.md) | A noise binding carries a kind and a rate in beats | 2026-07-25 | accepted |
| [ADR-0012](0012-one-severity-and-a-rejection-carries-numbers.md) | One severity, and a rejection carries the numbers | 2026-07-25 | accepted |
| [ADR-0013](0013-cost-has-three-axes-that-must-not-be-added.md) | Cost has three axes, and they must not be added | 2026-07-25 | accepted |
| [ADR-0014](0014-generated-code-cannot-be-captured-by-a-name-a-procedure-can-spell.md) | Generated code cannot be captured by a name a procedure can spell | 2026-07-25 | accepted |
| [ADR-0015](0015-a-measurement-carries-how-it-was-taken.md) | A measurement carries how it was taken | 2026-07-25 | accepted |
| [ADR-0016](0016-agents-leave-work-in-the-tree-and-the-reviewer-commits.md) | Agents leave work in the tree; the reviewer commits | 2026-07-25 | accepted |
| [ADR-0017](0017-an-invariant-that-can-be-tested-is-a-test.md) | An invariant that can be tested is a test, not a sentence | 2026-07-25 | accepted |
| [ADR-0018](0018-three-documents-three-jobs.md) | Three documents, three jobs | 2026-07-26 | **superseded by ADR-0000** |
| [ADR-0019](0019-exposure-is-three-things-and-none-stands-in-for-another.md) | Exposure is three things, and none of them stands in for another | 2026-07-26 | accepted |
| [ADR-0020](0020-a-corpus-expresses-taste-and-never-a-missing-feature.md) | A corpus expresses taste, and never a missing feature | 2026-07-26 | accepted |
| [ADR-0021](0021-a-palette-is-the-library-filtered-not-a-new-object.md) | A palette is the library filtered, not a new object | 2026-07-26 | accepted |
| [ADR-0022](0022-revision-goes-param-then-range-then-regeneration.md) | Revision goes param, then range, then regeneration | 2026-07-26 | accepted |
| [ADR-0023](0023-regeneration-is-destructive-in-a-slot-and-safe-in-the-library.md) | Regeneration is destructive in a slot and non-destructive in the library | 2026-07-26 | accepted |
| [ADR-0024](0024-each-source-counts-from-zero-and-carries-a-source-attribute.md) | Each source counts from zero and carries a `source` attribute | 2026-07-26 | accepted |
| [ADR-0025](0025-sources-are-distinguished-downstream-and-a-renderer-never-branches-on-one.md) | Sources are distinguished downstream, and a renderer never branches on one | 2026-07-26 | accepted |
| [ADR-0026](0026-the-deck-is-l5s-surface-and-its-size-is-two-budgets.md) | The deck is L5's surface, and its size is two budgets | 2026-07-26 | accepted |
| [ADR-0027](0027-a-set-value-is-immutable-and-its-compiled-instance-is-not.md) | A Set value is immutable; its compiled instance is not | 2026-07-26 | accepted |
| [ADR-0028](0028-closed-form-and-accumulating-is-a-static-classification.md) | Closed-form and accumulating is a static classification | 2026-07-26 | accepted |
| [ADR-0029](0029-amplification-is-a-second-kind-of-l2-not-a-new-layer.md) | Amplification is a second kind of L2, not a new layer | 2026-07-26 | accepted |

## Retired numbers

None yet. A superseded record keeps its row above; a **deleted principle** is recorded here as
`P-nnnn — retired <date> → P-mmmm`.

## Standing rules with no record yet

A principle with no ADR is one whose reasoning is still only in the code and the specification.
These predate the reconstruction reaching their decision date.

- [P-0001](../principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md) — Nothing allocates or compiles a shader on the render thread
- [P-0002](../principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md) — Simulation time comes from a record, never from a clock *(ADR-0006 covers the tick record; the clock-for-cost exception is still unrecorded)*
- [P-0005](../principles/0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md) — A swap happens on a frame boundary, and an over-budget Set rolls back on its own
- [P-0006](../principles/0006-the-workspace-stays-closed-to-rust.md) — The workspace stays closed to Rust
