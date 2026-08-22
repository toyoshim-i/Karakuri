# Architecture Decision Records

How this directory and [../principles/](../principles/) are run is
[ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md). Read that first.

An ADR is not edited after it lands, except to set its status. A retired number is never
reused; when one is superseded it stays in the table with a pointer to what replaced it. This
index is maintained by hand — see ADR-0000 for when that stops being enough.

`principles/` has no index. `ls docs/principles/` is the index, because each filename is the
rule it states.

**Reconstruction is in progress.** Records are being recovered in date order from the session
history that runs from 2026-07-25. Everything through **2026-07-25** is written; the numbering
is chronological, so later dates take later numbers.

## Records

| | Decision | Date | Status |
| --- | --- | --- | --- |
| [ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md) | Record decisions here and standing rules in `principles/` | 2026-08-22 | accepted |
| [ADR-0001](0001-identity-is-seed-an-ordinal-not-a-slot-index.md) | Identity is `seed`, an ordinal, and `id` is dropped | 2026-07-25 | accepted |
| [ADR-0002](0002-compaction-preserves-order-and-determinism-is-bit-exact.md) | Compaction preserves order, and determinism means bit-exact | 2026-07-25 | accepted |
| [ADR-0003](0003-hash-builtins-are-salted-from-the-seed-stream.md) | Hash builtins are salted from the layer's seed stream | 2026-07-25 | accepted — narrowed to per source on 2026-08-16 |
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

## Retired numbers

None yet. A superseded record keeps its row above; a **deleted principle** is recorded here as
`P-nnnn — retired <date> → P-mmmm`.

## Standing rules with no record yet

A principle with no ADR is one whose reasoning is still only in the code and the specification.
These predate the reconstruction reaching their decision date.

- [P-0001](../principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md) — Nothing allocates or compiles a shader on the render thread
- [P-0002](../principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md) — Simulation time comes from a record, never from a clock *(ADR-0006 covers the tick record; the clock exception is still unrecorded)*
- [P-0004](../principles/0004-a-live-set-is-never-mutated-in-place.md) — A live Set is never mutated in place
- [P-0005](../principles/0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md) — A swap happens on a frame boundary, and an over-budget Set rolls back on its own
- [P-0006](../principles/0006-the-workspace-stays-closed-to-rust.md) — The workspace stays closed to Rust
