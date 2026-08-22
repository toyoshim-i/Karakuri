---
id: 0001
title: Identity is `seed`, an ordinal, and `id` is dropped
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0007]
tags: [ir, determinism, engine]
---

# Identity is `seed`, an ordinal, and `id` is dropped

## Context

Draft v0.2 gave every element an `id`, and the specification's own examples wrote
`hash1(id)` to pick a colour. Reading the two documents together turned up an
inconsistency — the ambient table said `id` was L1-only while an L4 example used it — and
following that inconsistency reached a deeper one.

`id` is a buffer slot number. The moment the element count is dynamic, compaction moves
elements between slots, and a particle whose colour came from `hash1(id)` **changes colour
while it is alive**. The invariant was not merely unenforced; the specification's canonical
example demonstrated the bug.

## Decision

`id` is removed from the language. Identity is **`seed`**: a `uint` assigned at spawn from a
monotone counter, carried per element, moved with the element by compaction.

`seed` is an **ordinal, not a random number**, and that is the load-bearing part. `id` had
two jobs — a randomness source via `hash1(id)`, and a *structural layout index* via
`id % 512u`, which is how a procedure with no `spawn` block builds a lattice. Defining
`seed` as a monotone counter keeps both: `hash1(seed)` is the idiom for randomness, and with
no `spawn` block `seed` equals the initial slot index, so lattices still work — and do not
move under compaction.

`seed` is a **carried attribute, not an ambient**: it lives in the buffer and travels with
the element. It is implicit — never declared in `emit` or `consumes` — because requiring the
declaration would produce a stream of contract errors for forgetting it. Four bytes buys the
removal of a failure class.

`id` stays **reserved** rather than merely undefined, so the check is lexical and the
compiler can name the replacement: an LLM writes `id`, and the diagnostic says the identity
is `seed`.

## Alternatives rejected

- **Keep `id` and forbid compaction from moving elements.** Would require an atomic free
  list, which is rejected for its own reasons in [ADR-0002](0002-compaction-preserves-order-and-determinism-is-bit-exact.md).
- **Expose a pre-hashed random value instead of an ordinal.** Kills `seed % 512u`, and with
  it every static lattice procedure. The expressive loss is visible.
- **Mix the Set's seed into the element value internally**, so one word means both.
  Rejected in favour of salting the hash builtins — see
  [ADR-0003](0003-hash-builtins-are-salted-from-the-seed-stream.md).

## Consequences

- Open questions about spawn determinism closed as a consequence rather than on their own.
- Documented as a name the type checker special-cases, because the model will write `id`.
- Every `hash1(id)` in the specification's examples changed to `hash1(seed)`.

## Evidence

Session 2026-07-25T11:43Z–11:50Z. First commit `003a74d`. The rule that survives is
[P-0007](../principles/0007-an-element-is-never-identified-by-its-buffer-slot.md).
