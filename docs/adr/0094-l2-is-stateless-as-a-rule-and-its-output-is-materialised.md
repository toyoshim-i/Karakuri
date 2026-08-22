---
id: 0094
title: L2 is stateless as a rule, and its output is materialised
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: [0032]
tags: [ir, engine]
---

# L2 is stateless as a rule, and its output is materialised

## Context

Asked whether the layers other than L1 and L4 had anything undecided, the answer was **a great
deal** — starting with the specification's own "Open questions: None", which was true of the
questions anyone had written down and false of the specification.

## Decision

**L2 holds no state, written as a rule rather than observed as a fact.** Four things rest on it:

- **Stackability.** The only thing justifying the algebra's endomorphism is that stages compose;
  two stateful stages each want their own double buffer and their own place in the compaction
  order.
- **`closed_form` stays decidable.** A stateless L2 is vacuously seekable, so a Set's seekability
  remains L1's property.
- **Priming stays L1's question.** Warming a Set means warming what accumulates, and nothing else
  accumulates.
- **Fusion becomes legal later.** A stateless stage can be folded into its consumer at codegen —
  what the roadmap's graph compiler calls fusing a `Field` chain. Written as a rule, it is provable;
  left as an observation, fusion would be an optimisation leaning on an interpretation of the
  language. The same distinction as a fullscreen renderer's empty `consumes`
  ([ADR-0091](0091-declaration-by-absence-and-an-empty-consumes-is-a-rule.md)).

**An L2 cannot `kill()`.** It follows from statelessness, and it earns its own sentence because
what it buys is concrete: liveness is decided upstream, so **compaction runs once after L1** and
nothing reconsiders it afterwards. A killing L2 would put a scan between every pair of stages.

**Output is materialised, not fused**, and this was the implementation fork. Fusion costs no memory
and **pays the stage's cost once per reader** — a heavy deformation read by three renderers costs
three times. Materialising pays once however many read it, which is the shape the other half of this
milestone (several renderers over one geometry) needs to survive. Fusion goes back to being the
graph compiler's optimisation, where the roadmap had already put it.

## Consequences

- The block is named `deform`: `BlockKind::kind()` recovers a layer from a block, so a name
  appearing in two layers would take that away.
- An L2 declares both `emit` and `consumes`, and its `emit` **widens what is available downstream of
  that node** rather than restating what L1 wrote.
- Amplification's `copy` is implicit and **`seed` stays the parent's** — `hash1(seed)` giving eight
  mirror images the same colour is why they read as one object, and breaking that is a deliberate
  `hash1(seed ^ copy)`.
- A comment in `set.rs` explained the logical AND in `closed_form` as being *for when an L2 brings
  its own state*. The rule makes that false; the AND still belongs there, for a different reason.

## Evidence

Session 2026-08-16T04:42Z.
