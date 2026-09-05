---
id: 0014
title: Generated code cannot be captured by a name a procedure can spell
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0089]
tags: [codegen]
---

# Generated code cannot be captured by a name a procedure can spell

## Context

The first end-to-end run of a real `.kir` to the GPU was rejected by naga. Generated modules
bind the uniform block as `u`; the procedure wrote `let u = hash1(seed);`, and `u.radius`
resolved to a local `f32`.

**The specification's own canonical example was the one that stepped on it.** The code
generator's naga tests passed because the hand-built fixtures happened to name their locals
`uu` and `vv` — a general lesson about hand-built fixtures: they dodge by accident.

## Decision

**Mangle unconditionally.** Every IR-derived identifier is prefixed (`radius` → `usr_radius`,
params likewise), so the generated namespace is **structurally disjoint** from anything a
procedure can spell. No fixed name the generator emits starts with the prefix, so no local can
equal one, whatever a procedure calls it.

The brief given for the fix was the durable part: **state why the class is closed, not why it
is unlikely.** "A realistic procedure would not use that name" is the reasoning that produced
the bug.

## Alternatives rejected

- **A blocklist of generated names.** Requires tracking WGSL's reserved words forever, and it
  was already failing: a `param` named `array` passed all four validation stages and generated
  WGSL that would not compile.
- **Rename generated identifiers to something unlikely.** Same class of reasoning, later
  failure.

## Consequences

- Readability survives: `usr_radius` still shows which IR name it came from in a debugger.
- Regression fixtures were rewritten to use the exact names that collide — an L1 whose locals
  are named `u`, `hash1`, `seed`, `prev_position`, `slot`, `gid`, `capacity`, `mod_f32` and so
  on, then calling those builtins for real afterwards.
- A second, unrelated bug surfaced while building that fixture: uniform trailing padding was
  emitted as `array<f32, N>`, which fails WGSL validation for N ≥ 2 because array elements in
  the uniform address space need a 16-byte stride. The only prior test needed exactly one pad
  float.

## Evidence

Session 2026-07-25T14:37Z–15:00Z. Standing rule: none in
`docs/principles/`. P-0089 stated it and was retired on 2026-09-05 — it is a testing discipline
rather than a property of the instrument, so it is stated in `docs/contributing.md` §3, *A test is watched to fail before it is kept*.
