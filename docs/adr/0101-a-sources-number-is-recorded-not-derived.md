---
id: 0101
title: A source's number is recorded, not derived
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: [0053]
tags: [ir, format, determinism]
---

# A source's number is recorded, not derived

## Context

The last open question the project had carried since before there was a compiler: **how two merged
geometries keep their identities apart.** Half of it had been settled — `seed` counts from zero per
source, a fourth implicit attribute `source` makes the identifier a pair, the hash salt moves per
source, and downstream masks on `source` while a renderer never branches on it
([ADR-0024](0024-each-source-counts-from-zero-and-carries-a-source-attribute.md)).

What remained was **what the number actually is**, and it matters because the roadmap calls
attribute masks the system's largest source of expressive power: a `mask source == 1` that quietly
points at different material sends a whole modulator at the wrong thing — **and it comes out as a
picture, not as an error.**

## Decision

**The value is assigned once and recorded. Nothing derives it.**

Every derivation was tried and written into the specification with its failure, because otherwise
the next person will think of position again:

| Derived from | Why it fails |
| --- | --- |
| Position among a merge node's inputs | Inserting an upstream merge silently retargets `source == 1` |
| Position in the Set's source list | Reordering retargets every mask |
| The `.kir`'s content hash | Moves on every character, and `--watch` is this project's central loop |
| The procedure's declared name | Two uses of one lattice collapse — `proc drift_shell` is a **type name** |

**And this is not new machinery**, which was the finding that settled it. The language's randomness
is already *deterministic given the record stream* rather than pure: the hash builtins are salted
from the recorded `{"t":"seed"}`. **Recording is this system's default move**, and the specification
had even noted that that seed is per layer rather than per source. One subscript was missing.

**A consequence that simplifies rather than constrains: the generator stops mattering.** A creation
timestamp, a counter, a hash of anything — once the value is recorded, where it came from means
nothing.

## The `id` metaphor settled two things at once

Treating it like an HTML `id`:

- **An id attaches to an element, not to a tag.** So the name belongs on the **use** side, in the
  Set — and two uses of one procedure are two names rather than a collision.
- **An element with no id cannot be referenced, and that is fine.** Most sources are never masked,
  so **you pay for a name only when you want to point at something.**

A salt is needed by *every* source, named or not, so an assigned value is required regardless — and
once it exists, **a name can only ever be an alias for it.** That is why deriving the value from the
name does not work.

The same structure the specification already uses for parameters: **internally a stable address,
externally a name someone chose.** Twice is probably not a coincidence.

## Marked as preference rather than as forced

So a later reader can tell them apart: the attribute carries **the assigned value itself** rather
than a dense number plus a separate salt, with the readable ordinal used only where a human looks;
and names live **in the Set file and not on the command line**, so `--set` gains no syntax and a
source named there is simply unreferenceable.

## Evidence

Session 2026-08-16T10:54Z–11:18Z. Standing rule:
[P-0053](../principles/0053-a-value-that-must-be-stable-is-recorded-not-derived.md).
