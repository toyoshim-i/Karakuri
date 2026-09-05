---
id: 0092
title: A resource listing is a curriculum, not an index
status: accepted
date: 2026-08-15
supersedes: []
superseded_by: []
principles: [0093]
tags: [mcp, process]
---

# A resource listing is a curriculum, not an index

## Context

The model's first session began by looking for a worked example in another slot. Publishing saved
shaders, presets and samples as MCP resources would have answered that in one call instead of four
failed compiles.

## Decision

Placed in M4 as a **thin layer that comes before search**, in two steps of rising cost: the bundled
examples as resources (files that already exist and already work), then saved Sets and their
artifacts (already written by `--save-set`, already content-addressed).

**And one design decision fixed before building it: a resource listing is a curriculum, not an
index.**

Handed two thousand procedures, a model learns nothing beyond what it could already have guessed.
**Four chosen to span the language** change what it writes immediately. Searching a large library is
therefore a **tool call** — which is also the only escape from `resources/list` being something a
client reads in its entirety.

Embeddings, thumbnails and genealogy stay in M4. Those answer *which of these two thousand*; this
answers *how is this written*. Different questions.

The general form went into `docs/plugins.md`, since it is about surfaces rather than this
implementation:

> a surface for a model is half tools and half things to read, and the reading half is the cheaper
> one to get wrong. A keyboard needs no curriculum.

## Deferred, with a condition rather than a mood

The feature was then **deferred** — it was a problem from before the API was in place, and whether
it is still needed should be judged by watching more sessions. Recorded with an **observable
resumption condition** rather than as a change of mind: *the model reaches for something the
specification and the vocabulary already explain and gets it wrong, or asks for examples twice.* If
that happens it comes back; if it does not, it was not needed — and either way it can be decided
after the fact.

## Evidence

Session 2026-08-15T07:11Z, commit `55dd600`; deferred 2026-08-15T16:31Z.
