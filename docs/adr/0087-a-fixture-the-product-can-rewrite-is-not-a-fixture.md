---
id: 0087
title: A fixture the product can rewrite is not a fixture
status: accepted
date: 2026-08-15
supersedes: []
superseded_by: []
principles: [0089]
tags: [process]
---

# A fixture the product can rewrite is not a fixture

## Context

Two tests broke, and what broke them was **the product working as designed.**

They built "a different procedure" by substituting a known sentence in `examples/soft_points.kir` —
one of the very files the MCP surface exists to rewrite. When a live session rewrote it, the
substitution silently matched nothing, the "different" procedure was identical to the original, and
the test failed with a message **pointing at the write path**. The write path was fine. The fixture
was.

## Decision

A test owns `tests/fixtures/flat.kir`, which nothing in the product can reach. And the write test
**prepends a line** rather than substituting one, because a prepend changes the content whatever the
content is.

**The criterion is not "it probably will not change" but "it can change."**

## Consequences

- **It happened twice in the same file.** A later pass fixed the writing side and left the reading
  side, which is its own lesson about fixing a class of defect: the fix has to be carried to every
  site the argument covers, not to the one that failed.
- Related, from the same day: **a model's write reached tracked files with no backup**, one day after
  the manual gained the sentence saying a write replaces your file with no backup. It was recoverable
  **because of git, which is not a property of the feature** — and it is the direct cause of the
  three-place separation in [ADR-0088](0088-what-ships-what-you-saved-and-what-you-are-editing.md).

## Evidence

Session 2026-08-15T07:06Z, commit `105997a`. Standing rule:
[P-0089](../principles/0089-a-check-you-have-not-watched-fail-is-guessing.md).
