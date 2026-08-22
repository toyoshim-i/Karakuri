---
id: 0096
title: Camera is an edge into L4, and an existing field is not a fact
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: [0051]
tags: [ir, process]
---

# Camera is an edge into L4, and an existing field is not a fact

## Context

I had put a question to the operator as a design fork: **is the camera per Set, or one per deck?**
Both answers were defensible, so it looked like a real branch.

## Decision

**Neither. The algebra had answered it from the beginning:**

```
L4 : (Geometry, Camera) -> Texture
```

Camera is an **input edge of L4**. I had been importing today's `Set.camera` field into the node
model and reasoning outward from an implementation shape.

The accurate statement is not "one per L4" but **"an L4 takes a Camera edge, and sharing is edge
fan-out."** One L3 read by two L4s draws one viewpoint two ways; separate L3s composite two
viewpoints — which is the *blend two scenes together* case. **Both fall out with no rule added.**

## The general failure, which is the point of the record

**An existing field is not a fact about the design.** Reasoning outward from one produced a
question that was **answerable and wrong**, and it reached a document as a request for the
operator's judgement.

What makes it hard to catch is precisely that it is answerable: **two plausible answers look like a
genuine fork.**

The rule that follows: **a scope question — per what, owned by what, how shared — is checked first
against the declared type or the algebra.** If the thing already appears there as an argument or an
edge, the scope is settled, and the question is a leak from the current code.

## Evidence

Session 2026-08-16T04:50Z, 2026-08-16T05:30Z. Standing rule:
[P-0051](../principles/0051-an-existing-field-is-not-a-fact-about-the-design.md).
