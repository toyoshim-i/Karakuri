---
id: 0152
title: A `.kir` names a slot, and the Set names the nodes
status: accepted
date: 2026-08-21
supersedes: []
superseded_by: []
principles: [0068]
tags: [ir, format, engine]
---

# A `.kir` names a slot, and the Set names the nodes

## Context

[ADR-0111](0111-a-name-lives-in-the-set-file-and-may-be-written-on-the-command-line.md) split
naming in two and took only the first half. **Identity** — a name on a node, recorded, with the
internal address staying `(layer, index)`
([ADR-0102](0102-a-renderers-address-is-layer-and-index.md)) — was taken then, and landed as
`Record::Slot`'s `name` together with an `index` on `capacity` and `seed`. **Edges** — which two
geometries a `pairs` takes, which source a mask hits — was deferred with its difficulty already
stated: *"a `.kir` cannot name them without binding the procedure to one Set."*

The deferral came due because a Set had begun holding more than one of everything a procedure
could take only one of. Where the second input came from was **`--set` position**, written
nowhere: reordering the command line silently changed what a morph morphed into. The same gap wore
other clothes elsewhere — one `kind Field` per Set, one L3 per Set, and a `mask` that could vary a
deformation over anything an element carried except which geometry it came from. Each of those was
the same missing thing: an input a procedure could not ask for by name.

## Decision

**A procedure declares a named input of its own, and the Set says what fills it.**

```
proc morph {
  uses far : Geometry
  position = mix(position, far.position, w);
}

--edge morph.far=sphere_shell
```

recorded as `Record::Edge { node, slot, to }`. The two halves of that record are the whole
decision: `slot` is **the procedure's own word**, standing in the same relation to whatever
supplies it as `consumes position` does to whichever L1 wrote `position`; `node` and `to` are
**nodes' names**, which every node has whether or not a caller wrote one. Nothing in the `.kir`
refers to a node.

**An unbound slot is refused**, and that is the second half rather than an oversight. *"If there is
exactly one, use it"* is the implicit rule the whole item exists to remove, and the refusal names
the slot and lists what the Set holds.

## Alternatives

**Name the node in the `.kir`** — write `far = sphere_shell` in the procedure, which is the
shortest spelling and the one that needs no record. Rejected because it couples the procedure to
one Set: a part that names its neighbours is not a library part, and a corpus of them cannot be
recombined, which is the property the whole slot-contract design exists to have.

**Keep "if there is exactly one, use it"** as the binding rule, and add a spelling only for the
ambiguous case. Rejected because that rule *is* the cap being removed — it is what held a Set to
one field and one camera, and reinstating it under a new spelling would cap the next fan-in the
same way and be discovered the same way.

**Leave it positional**, with `--set` order deciding. Rejected by the defect that opened the item:
the order is written nowhere, so the same two geometries listed either way gave different pictures
and nothing recorded which had been meant.

## Consequences

- **One shape, four times.** Geometry (`uses far : Geometry`, read `far.position`), field
  (`uses shape : Field`, called `shape(p)`), camera (`uses view : Camera`, read `view.clip` —
  [ADR-0118](0118-the-built-in-camera-is-a-node-unconditionally-and-last.md)) and source
  (`uses only : Source`, read as a value —
  [ADR-0119](0119-source-binds-a-uniform-and-is-read-as-a-value.md)) are the same declaration and
  the same `edge`, and each cost one `SlotTy` variant and a `Vec` where an `Option` was. The caps
  on fields and cameras per Set went with them.
- **`--set` order decides nothing.** The near source is whichever geometry no edge bound, rather
  than `l1s[0]`, which is what makes the two orderings the same picture. It also broke two places
  that had assumed simulations are walked in list order.
- **A rebuild had to learn the names**, because an edge resolved against spellings a rebuild had
  regenerated pointed at nothing. `Watch` restates the names the slot was spelled with.
- **A node instance still has no address, and that is what is left undecided.** A name is per
  *procedure*; a chain is materialised per source
  ([ADR-0107](0107-a-chain-is-materialised-per-source.md)), so a Set over two sources instantiates
  one chain of deforms twice and has more instances than it has names. `Set::element_storage`
  returns a row per instance and nothing can label them. Nothing in the engine needs such an
  address today, which is why none was invented here.

## Evidence

Commits `fe0389a` (2026-08-20), `9765f6b`, `3c7556e`, `356fd33`, `6859853` (2026-08-21). Standing
rule: [P-0068](../principles/0068-a-kir-never-names-a-node-of-a-set.md).
