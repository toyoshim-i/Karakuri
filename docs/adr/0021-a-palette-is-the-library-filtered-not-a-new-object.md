---
id: 0021
title: A palette is the library filtered, not a new object
status: superseded
date: 2026-07-26
supersedes: []
superseded_by: [0133]
principles: [0019]
tags: [store, process]
---

# A palette is the library filtered, not a new object

## Context

Saved prompts were designed up into a first-class object: named, versioned, selectable,
combinable, content-addressed, with its hashes recorded in an artifact's `origin` so that two
artifacts from one prompt could be told apart. The user's reframing was smaller — keep the
prompts that worked and start the next one from them.

## Decision

**The heavier design is dropped.** A palette is the library filtered by a tag:

- `origin.prompt` already records the prompt that produced an artifact.
- `tag` already exists to mark the ones worth keeping.
- `parent` already links a revision to what it revised, so following the genealogy reaches the
  original prompt, with each step's added instruction in its own `origin.prompt`.
- Previews already give the thumbnails, without which a list of strings cannot be judged.

**Nothing is built.** The feature is the library browser, filtered.

Prompt assembly collapses from four layers to two: **what the system always prepends** (the
language, the default camera, the world scale) and **what the user typed** (a saved prompt plus
this time's instruction). The first is not the user's to manage, so it never reaches their
attention.

## Alternatives rejected

- **The first-class corpus object**, with versions and hashes in `origin`. Justified by
  reproducibility — which the material does not want. These are sampling materials: generating
  from one starting point repeatedly and keeping the few that are liked *is the workflow*.
  Machinery for explaining why two outputs differ solves a problem nobody has.

## Consequences

- Selection becomes the main activity: generate twenty, keep three. **Previews stop being a
  convenience for a large library and become a precondition** for the basic loop, which moves
  them earlier than planned.
- Re-seeding gives a second, cheaper source of variation: changing the seed stream re-rolls the
  hash salt, so the same procedure scatters differently with **no compile and no fork**. Five
  procedures at four seeds may beat twenty generations.

## Evidence

Session 2026-07-26T02:29Z–02:39Z. Standing rule:
[P-0019](../principles/0019-prefer-the-mechanism-that-already-exists.md).
