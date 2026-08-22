# A value that must be stable is recorded, not derived

`source` is assigned once and written into the record stream. Nothing computes it. Every derivation
was tried and each one moves when something unrelated moves: a merge node's input position breaks when
an upstream merge is inserted, a Set list position breaks on reordering, a content hash moves on every
character — and `--watch` is the central loop — and a procedure's declared name collapses when the
same lattice is used twice, because `proc drift_shell` is a **type name**.

**What it rules out.** Deriving an identity from where a thing currently sits. It matters because a
`mask source == 1` that quietly points at different material sends a whole modulator at the wrong
thing, **and it comes out as a picture rather than as an error.**

**This is not new machinery.** The language's randomness is already *deterministic given the record
stream* rather than pure — the hash builtins are salted from a recorded `{"t":"seed"}`. Recording is
this system's default move.

**So the generator stops mattering** — a clock, a counter, a hash of anything. Once recorded, where a
value came from means nothing. A simplification, not a constraint.

**And a name is an alias, never the value.** Every source needs a salt whether or not it is named, so
an assigned value is required regardless; a name can then only be an alias for it. Names live on the
**use** side, so two uses of one procedure are two names rather than a collision — and you pay for a
name only when you want to point at something.

**Where it holds.** [ir-spec.md](../ir-spec.md), "Multiple L1 sources, and `source`". Decided in
[ADR-0101](../adr/0101-a-sources-number-is-recorded-not-derived.md).
