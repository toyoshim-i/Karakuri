---
id: 0133
title: What was fed to a generator is a field, not an object
status: accepted
date: 2026-08-22
supersedes: [0021]
superseded_by: []
principles: [0085]
tags: [store, process, docs]
---

# What was fed to a generator is a field, not an object

## Context

Two unbuilt plans contradicted each other, and moving the roadmap's settled decisions into this
registry is what surfaced it — a summary hides not only whether the thing it duplicates has drifted
but whether it agrees.

**`docs/ir-spec.md`, on the metadata file:**

> Anything else fed to the generator belongs here too, for the same reason: two artifacts from the
> same prompt that differ because different reference material was supplied are otherwise
> unexplainable.

**[ADR-0021](0021-a-palette-is-the-library-filtered-not-a-new-object.md), rejecting:**

> The first-class corpus object, with versions and hashes in `origin`. Justified by **reproducibility
> — which the material does not want.** … **Machinery for explaining why two outputs differ solves a
> problem nobody has.**

The same justification, load-bearing in one and refuted in the other.

## Decision

**ADR-0021's rejection stands for the object and is withdrawn for the field.**

The test that separates them: **does recording it need a subsystem, or is it a field over things that
already have addresses?**

- **The object** — named, versioned, selected, combined, with a lifecycle of its own — is what
  ADR-0021 killed, and killing it was right. Nothing here revives it.
- **The field** is a list of hashes of artifacts the store already holds, in a record that already
  exists, using content addressing that is already there. That is
  [P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) rather than a violation of
  it, and ADR-0021's own accepted half kept `origin` and `parent` for exactly that reason.

ADR-0021's argument was written against a subsystem and reads as though it also disposes of the
field. It does not, and the sentence in the specification stands unchanged.

## Why the field earns its keep even though the material is sampled

ADR-0021 is right that the workflow is sampling — generate twenty, keep three — and that nobody asks
why two of the twenty differ. Two things sit outside that:

- **M4 already carries the same debt in the same shape.** `parent` *"has to start being recorded with
  the first generated artifact or the genealogy has a hole at its root"*, and the roadmap calls it the
  requirement most easily missed because it arrives late. Reference material is that debt again: it
  costs nothing at the first artifact and cannot be recovered at the thousandth.
- **[ADR-0092](0092-a-resource-listing-is-a-curriculum-not-an-index.md) is deferred with a resumption
  condition**, and if it resumes, supplied material stops being exceptional and becomes a routine
  input. A field that is free now is a migration then.

## Consequences

- **Nothing is built by this record**, in either direction. `origin` and `<hash>.meta.ndjson` do not
  exist yet — the store has no record type for them and the specification says so. This settles which
  of two plans is the one to build.
- The roadmap's "Settled decisions" section loses the entry it was keeping open, so it is now a
  pointer and nothing else.
- **The general shape, which is why this is a record rather than an edit:** an argument written
  against one design reads as though it disposes of everything nearby. ADR-0021 aimed at a
  subsystem and the sentence it produced covered a field it was not thinking about. That is not a
  reason to distrust the argument — it is a reason to check, when reusing one, what it was aimed at.

## Evidence

Surfaced 2026-08-22 while emptying `docs/roadmap.md`'s "Settled decisions", which had carried the
specification's claim as a settled rule since before this registry existed.
