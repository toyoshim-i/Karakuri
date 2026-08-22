---
id: 0105
title: `Field` is a kind with no node
status: accepted
date: 2026-08-18
supersedes: []
superseded_by: []
principles: []
tags: [ir, engine]
---

# `Field` is a kind with no node

## Context

`examples/field_march.kir`'s distance function is three lines inside a loop, and **the only way to
reuse it is to copy it.** The file says what that costs: a correct normal needs the same field
evaluated at six more points — eighteen lines of copy — so it is not written, and the example ships
with a cheap normal that is wrong on the box's faces.

## Decision

**A `.kir` file with `kind Field`, spliced into whoever calls it at Set build. A file, and no node.**

The argument that settled it was **running the roadmap's L5 reasoning backwards**:

> A `.kir`'s `kind` says what a procedure **lowers to**. L5 has no code to lower, so there are no
> `kind L5` files and there should not be.

**`Field` is the exact mirror: it has only code to lower**, so there is a file and no node — no
buffer, no pass, no position in the chain.

And in that shape it adds **zero new syntax categories**: one `kind`, one block, one ambient
(`point`), one output (`distance`) — the pattern L2 and L3 already established. **No user-defined
functions anywhere**, so the v0.2 non-goal never had to be opened; a caller evaluates `field(p)`,
the same relation a `camera { }` block has to reading `camera`. (And the non-goal's `(not in v0.2)`
is a schedule rather than a prohibition — the roadmap puts `Field` in M3.)

## Alternatives rejected

- **A `field` definition inside a procedure.** The smallest change, and it **compromises the
  purpose**: one `proc` per file is a hard parser rule, so there is no cross-file reuse — and reuse
  is what `Field` exists for. Cheap, not uncompromised.
- **Real nodes and edges.** Consuming a Field edge means the consumer inlines the field's code, so
  it needs cross-procedure lowering *plus* **fan-in and the edge notation fan-in brings** — designing
  that notation for one shape is what the roadmap calls *one shape's worth of machinery pretending
  to be a system*. C is not more principled than B; it is out of order.

## The honest limit

**`Field` does not make anything cheaper.** Every form inlines, so six calls cost six times, and
"six more evaluations will not fit under 4096 ops" survives unchanged. What is gained is being able
to **express** a correct normal. Making it affordable is fusion, which is a different piece of work.

## Consequences

Splicing needed three things, and each was a discovery:

- **A field's `param`s fold into the caller's uniform under a dedicated prefix**, because a spliced
  body has no uniform of its own. A renderer and a field may both declare `exposure`, so the
  separator is one that **cannot appear in a `.kir` identifier — unforgeable rather than
  improbable.**
- **The clock is a function argument, not a uniform read.** A field cannot know how its caller
  spells `t`: an L1 reads `step_args` per substep and everything else reads `u`, so one body cannot
  say both. Found when an L1 evaluating a field emitted `u.t` into a module with no such field.
- **Caller and field are costed together at the Set**, because `field(p)` weighs nothing in a single
  file, so each had been passing a ceiling against a figure that was missing the other. **Cost
  became a fourth quantity** — a field scales with how many times it is called, which is the
  caller's property, so `blob — 48 ops/evaluation` and a 48-step march is 2304.
- **The check rejected its own example.** A 40-step `field_lens` inlining `melt_blob` is 4378
  ops/fragment against a 4096 ceiling, while neither file exceeds it alone. The example ships at 34
  steps because that is the number that fits.
- **Two checks were silently dead the day they were written**: `Checked::cost` is filled by nothing,
  so both the cost check and the "this Set has no field" refusal read `None` and concluded they had
  nothing to say — the latter meaning a marcher with no field reached wgpu and killed the process.

## Evidence

Session 2026-08-17T15:27Z–2026-08-18T08:20Z.
