# Architecture Decision Records

How this directory and [../principles/](../principles/) are run is
[ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md). Read that first.

An ADR is not edited after it lands, except to set its status. A retired number is never
reused; when one is superseded it stays in the table below with a pointer to what replaced
it. This index is maintained by hand — see ADR-0000 for when that stops being enough.

`principles/` has no index. `ls docs/principles/` is the index, because each filename is
the rule it states.

## Process

| | Decision | Status | |
| --- | --- | --- | --- |
| [ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md) | Record decisions here and standing rules in `principles/` | accepted | |

## Standing rules with no ADR yet

Reconstructed from the source documents ahead of the records that decided them. Backfilling
these from the session history is outstanding work; a principle with no ADR is one whose
reasoning is still only in the code and the specification.

- [P-0001](../principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md) — Nothing allocates or compiles a shader on the render thread
- [P-0002](../principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md) — Simulation time comes from a record, never from a clock
- [P-0003](../principles/0003-compaction-preserves-order.md) — Compaction preserves order
- [P-0004](../principles/0004-a-live-set-is-never-mutated-in-place.md) — A live Set is never mutated in place
- [P-0005](../principles/0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md) — A swap happens on a frame boundary, and an over-budget Set rolls back on its own
- [P-0006](../principles/0006-the-workspace-stays-closed-to-rust.md) — The workspace stays closed to Rust
