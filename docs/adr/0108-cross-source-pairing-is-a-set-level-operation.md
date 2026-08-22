---
id: 0108
title: Cross-source pairing is a Set-level operation
status: accepted
date: 2026-08-18
supersedes: []
superseded_by: []
principles: []
tags: [ir, engine]
---

# Cross-source pairing is a Set-level operation

## Context

Interpolating between two geometries had been settled as to *what* it is — a direct indexed read of
the corresponding element, restricted to static sources because `seed` is then the slot index — and
left open on two points. The second is the substantive one: **is the source that gets read also
drawn?** For morphing it must not be, or at k=1 A arrives at B's position with B drawn on top of it:
two of everything, at twice the brightness.

## Decision

**A Set-level pairing, in the shape `--merge` already established.** The Set declares that two
sources are one geometry mixed by `k`.

- **Zero change to `.kir`** — no new kind, block, ambient or expression syntax.
- **"Is B drawn?" is answered structurally**, because the pair *is* the source and there is no B
  beneath it.
- `k` lives where `--merge`'s per-input controls already live. It is an operator's dial.
- The precedent is exact: a Set asking for a fixed operation over nodes it already holds, with no
  change to the language.

**And A does not close B.** A fan-in L2 can absorb `--pair` when edge notation arrives, exactly as
`--merge` relates to a future graph — incremental, closing nothing.

## Alternatives rejected

- **A two-input L2 — fan-in in the language.** It buys a describable mix and generalises to any
  binary geometry operation, and it costs **the edge notation the roadmap says fan-in brings**, plus
  the Set's graph shape and the authoring half of the graph compiler arriving early. Designing that
  notation for one shape is the failure the roadmap names.
- **An ambient inside an ordinary L2** — `position = mix(position, paired(position), k)`.
  Structurally the cheapest **and the worst in principle**: it cannot say *which* B, and B is still
  drawn, so the Set would have to infer *do not draw the interpolation target* from an L2's body —
  an implicit coupling across files. Topology inference is allowed **because it stays inside one
  file**; this does not.
- **Defer.** The naming problem is shared with masking a source, and both want a Set file that
  carries sources. Legitimate, and it is what *decided, not built* already meant.

## Evidence

Session 2026-08-18T13:01Z–13:50Z.
