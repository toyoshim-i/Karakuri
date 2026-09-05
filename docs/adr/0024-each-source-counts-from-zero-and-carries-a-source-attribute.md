---
id: 0024
title: Each source counts from zero and carries a `source` attribute
status: accepted
date: 2026-07-26
supersedes: [0003]
superseded_by: []
principles: [0008, 0092]
tags: [ir, engine]
---

# Each source counts from zero and carries a `source` attribute

## Context

The roadmap promised "per-source **ID namespaces**" for merging several geometry sources into one
Set. `id` had been deleted from the language a day earlier. A roadmap audit found it, and the
important part of the finding was that **correcting the wording would have deleted the question**:
the phrase named a mechanism as though it existed, and the real problem — how two sources avoid
colliding in `seed` — had never been asked.

It was therefore left in the document **as an open question**, not silently renamed.

Concretely: with two sources both counting from zero, two elements share `seed 5`. `hash1(seed)`
gives them the same colour, and an attribute mask — the roadmap's headline feature for L2, *only
elements where `seed % 3 == 0`* — hits both sources indiscriminately. **There is no way to say
"only source B".**

## Decision

**Each source keeps its own counter from zero, and elements carry an implicit `source` attribute.**

The mechanism was already there. `seed`, `alive` and `birth_frac` exist as implicit attributes
that the IR cannot name and the engine always allocates; `source` is a fourth, and compaction and
double buffering carry it with no new machinery. Masks become `source == 1`.

Randomness separates for free: the hash salt moves from **per layer** to **per source**, so
`hash1(seed)` differs across sources while `seed % 512u` does not — **the same lattice twice, in
two colours**, which is exactly the wanted behaviour. This supersedes the per-layer salt of
[ADR-0003](0003-hash-builtins-are-salted-from-the-seed-stream.md).

## Alternatives rejected

- **One shared counter**, source B continuing from where A stopped. No collision, and lattice
  generation breaks: `seed % side` is offset for B, and the guarantee that `seed` is the initial
  slot index for a procedure with no `spawn` block is gone.
- **Pack the source into the high bits**, `seed = (source << 24) | ordinal`. No extra storage,
  and an author using the raw `seed` as an index gets the same breakage in disguise.

Both alternatives also still need a source identifier for masking, so neither avoids `source`.

## Consequences

- Because each source counts from zero, **A's `seed 5` and B's `seed 5` are natural
  counterparts** — the basis for interpolating between sources. The identity decision turned out
  to be the foundation for blending.

## Evidence

Session 2026-07-26T02:47Z–07:22Z; the audit at 2026-07-26T02:53Z.
