---
id: 0113
title: The slot narrowing beat its estimate, and why is the finding
status: accepted
date: 2026-08-19
supersedes: []
superseded_by: []
principles: [0025]
tags: [engine, codegen]
---

# The slot narrowing beat its estimate, and why is the finding

## Context

The roadmap predicted a fifth to two fifths recoverable from the element slot, bounded by `vec3`
alignment.

## Decision

Measured: **39% off the shipping geometry.** `drift_shell` 80 B → 48 B; `lattice_shell` and
`sphere_shell` 64 B → 48 B; the shipping L1 buffers together 80.2 → 48.8 MiB.

**The estimate was low because it counted only the scalars before the attributes.** A `vec3` is
16-byte aligned and 12 long, so **four addressable bytes remain behind it** — `position, size` share
one block and `velocity, age` another. **No reordering was needed: declaration order stands, and an
`emit` list ending in a scalar packs itself.** The `velocity` flag for *has this element lived a
whole step* had been living in a padding `.w`; it now has its own field, and is still free.

## The price

**WGSL's layout rules now have to be known, where previously nobody needed them.** The host writes
bytes at published offsets and the shader reads through a struct, and **a mismatch produces no
compile error anywhere — it reads the middle of the previous element.** The align/size table lives in
one place, and a naga test validates the **module actually emitted** rather than the arithmetic.

## And a test changed its question rather than its number

`a_derivations_slot_exists_only_where_something_consumes_it` measured *a slot exists* **by stride** —
the same sentence only while every slot was 16 bytes. `birth_t` became an `f32` fitting in the four
bytes left over from `seed` and `birth_frac`: **free.** Measured by stride, the test would now report
an existing slot as absent.

## Evidence

Session 2026-08-19T14:08Z.
