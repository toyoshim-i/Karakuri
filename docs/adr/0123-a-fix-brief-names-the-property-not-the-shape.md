---
id: 0123
title: A fix brief names the property, not the shape
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: [0087]
tags: [process]
---

# A fix brief names the property, not the shape

## Context

The defect in [ADR-0122](0122-a-save-writes-the-bytes-that-are-on-screen.md) is the class this
milestone has closed four times: **one fact with two derivations.** It took three rounds to actually
close, and the reason is in how I wrote the brief.

## What happened

**Round one.** I wrote: *delete the `Sources::Startup` arm.* **I named the shape and did not require
the property.** The arm went. The duplication did not — it **moved into the launch seeding**, and
into a *less visible* form: re-reading the same path a few seconds after the compile that had already
read it.

**Round two.** The review found the same class in the new place.

**Round three.** The brief finally said **carry the bytes from the read that produced the compile** —
naming **which read is canonical** — and `compile::load` came back returning `(Checked, String)`.
That closed it.

## Decision

**A shape is one instance of a property.** Delete the shape and the duplicate **relocates to wherever
the thing is still being recomputed** — and it is *harder* to find afterwards, because the obvious
branch that used to advertise it is gone.

Acceptance criteria are written as properties: **"there is exactly one derivation of X"**, and where
possible, **which one is canonical**.

## Evidence

Session 2026-08-22T04:57Z. Standing rule:
[P-0087](../principles/0087-name-the-property-never-the-shape.md).
