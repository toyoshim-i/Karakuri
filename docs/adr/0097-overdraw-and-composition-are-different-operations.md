---
id: 0097
title: Overdraw and composition are different operations, and the graph says which
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: []
tags: [engine, render]
---

# Overdraw and composition are different operations, and the graph says which

## Context

I had recommended that several Texture nodes become successive passes onto one attachment — the
first clearing, the rest loading — and had added a hedge:

> **this is also the answer that will be wrong first**: a real graph has a node that eats textures,
> and compositing becomes that node.

Then the operator described exactly that graph: an L5 inside a Set, with several L1–L4 pipelines
branching and rejoining. **I had written down my own failure in advance and had not noticed that
"later" was "today".**

## Decision

My answer was not wrong; it was **missing a case**, and the two cases are different operations.

| Graph shape | What happens | Cost |
| --- | --- | --- |
| Several L4s straight to the Set's output | **Overdraw**, in declaration order, one target | one target |
| Several L4s into an L5 node | **Composition**, each L4 owning a target | 7.03 MB per L4 |

Drawing the same cloud as sprites *and* strokes is the first (no extra memory). Crossfading two
scenes is the second (worth paying for). **The graph says which**, so an explicit L5 node and not
wasting memory are no longer in tension.

## Evidence

Session 2026-08-16T04:50Z–04:57Z.
