---
id: 0119
title: `Source` binds a uniform and is read as a value
status: accepted
date: 2026-08-21
supersedes: []
superseded_by: []
principles: []
tags: [ir]
---

# `Source` binds a uniform and is read as a value

## Context

The last piece of multiple geometries: a mask naming which source it applies to.

```
proc dissolve {
  kind L2
  uses only : Source
  mask { strength = select(0.0, 1.0, source == only); }
}
```

The design had proposed `uses only : Geometry` with `only.source` read as a member, for consistency
with `far.position`, and called the alternative a wart.

## Decision

**A fourth slot type, `Source`, read directly as a value — and it is not a wart.**

**Binding `Geometry` binds an element buffer**: one bind-group entry per node. A mask wants **the
identity only** and reads no element at all. And a second geometry slot is already refused — *a node
takes one second geometry* — so an L2 that already has a `far` **could not structurally express**
naming a source to mask on.

`Source` binds **one `u32` in a uniform**. The two types are not two spellings of one thing; **they
bind different things**, which is why their arity rules may differ — `Geometry` at most one, `Source`
any number, so `source == a || source == b` is expressible.

| Type | Binds | Limit | Read as |
| --- | --- | --- | --- |
| `Geometry` | an element buffer | one | `far.position` |
| `Field` | a spliced body plus uniform params | any | `shape(p)` |
| `Camera` | a bind group | one | `view.clip` |
| **`Source`** | **one `u32` in a uniform** | **any** | **`only`, as a value** |

`Source` is the only one readable **alone**, and the brief required that to be written **beside** the
specification's existing sentence — *a geometry is not a value; `far` alone is refused, the language
has no type for a whole source* — because otherwise a reader takes one of the two for a mistake. That
sentence is about `Geometry` and remains true; `Source` is a `uint`, and the language has that type.

Two refusals came with it, each closing a silent path: `source` is refused in an L3 and a `Field` on
`Ambient::Seed`'s existing grounds; and it is refused **in any procedure declaring a geometry slot**,
because in a pairing Set the far simulation lives inside the near `Source` and **shares its uniform**,
so a read there would quietly mean the near one.

## Consequences

- **`source == only` compares two uniforms, so it is constant across the whole chain instance** — this
  is strictly an instance being on or off rather than a mask. It is also the cheapest possible branch
  on a GPU, since every lane agrees.
- The honest optimisation — do not instantiate the node in chains it excludes — is **recorded rather
  than taken**, because it changes that source's element layout by dropping a skipped node's `emit`,
  and would collide head-on with the placement work that landed the same week.

## Evidence

Session 2026-08-21T08:56Z–08:57Z.
