---
id: 0138
title: A model names a Set, not a hash
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [mcp, metadata, store]
---

# A model names a Set, not a hash

## Context

The metadata card is addressed by content hash — `<hash>.meta.ndjson` — so the obvious tool
for reading one takes a hash. It is also the cheapest to write: `Store::read_meta` already
takes exactly that.

## Decision

`read_set` takes **the id of a saved Set**, resolves each of its `slot` records to the
artifact behind it, and renders that artifact's card.

## Alternatives

**Take a hash.** Rejected, and the reason is not taste: **nothing on this surface has ever
handed a model a hash.** The MCP tools address a slot, a layer and an index, and `save_set`
returns an id. A hash-addressed tool's *first call could never be made* — the model would
have no way to obtain the argument. Cheapest to implement and impossible to use.

A second reason arrived while it was being built: `Hash::from_str` requires a `sha256:`
prefix while the store's filenames are bare hex, so a hash over the wire would also have
owed a spelling decision nobody needed to make.

**Take `(slot, layer, index)`.** Rejected as answering an answerable question. That address
names material a run is holding, whose source `read_procedure` already returns — a model
can already compile it, or read it. **A saved-but-not-loaded Set has no slot at all**, and
that is the blind spot worth closing: material in the library that nothing in the run is
pointing at.

## Consequences

The handle a model holds comes from the surface that gave it: `save_set` returns an id, and
`read_set` takes one. It is validated by the same `checked_id` a save uses — paths never
cross this protocol, and a read is the direction that hands a file back.
