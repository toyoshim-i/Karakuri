# An element is never identified by its buffer slot

Identity is `seed`: a `uint` assigned at spawn from a monotone counter, carried per element,
moved with the element by compaction. It is an **ordinal, not a random number** — write
`hash1(seed)` for randomness and `seed % 512u` for structure. In a procedure with no `spawn`
block `seed` equals the initial slot index, so a lattice is expressible and does not move.

**What it rules out.** Exposing a slot index under any name. A slot number stops being an
identity the moment compaction moves elements, and the failure is silent and beautiful: a
particle whose colour came from its slot changes colour while it is alive. `id` is kept as a
**reserved word** rather than merely removed, so the compiler can name the replacement when a
generator writes it.

**Where it holds.** [ir-spec.md](../ir-spec.md), "Element identity". Decided in
[ADR-0001](../adr/0001-identity-is-seed-an-ordinal-not-a-slot-index.md).
