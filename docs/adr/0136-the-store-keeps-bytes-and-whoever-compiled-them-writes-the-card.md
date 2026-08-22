---
id: 0136
title: The store keeps bytes, and whoever compiled them writes the card
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [store, metadata, cli]
---

# The store keeps bytes, and whoever compiled them writes the card

## Context

`docs/ir-spec.md` says the store *"regenerates metadata from the `.kir` plus a compile
pass"*, which reads as though `Store::put_artifact` should write the card itself. It cannot:
`karakuri-store` depends on `serde`, `sha2` and `thiserror`, and deliberately not on
`karakuri-ir`. It has no compiler and should not grow one.

## Decision

The store gains the file operations — `write_meta` and `read_meta`, addressed by hash,
beside `put_artifact`/`get_artifact` — and the **content is produced where a `Checked` is
already in hand**, in `karakuri-cli`, the only crate depending on both.

The card rides on `Placed`/`SavedNode`, built at compile time, so no path re-compiles to
write one and no card is built from bytes other than the ones put.

**A card that will not write does not fail the save.** The artifact is the thing; the card
beside it is derived and regenerates on the next compile. The operator is told, and
`put_meta` returns the sentence it printed so the policy is testable rather than asserted.

**`write_meta` overwrites where `put_artifact` refuses to.** An artifact at a hash is
immutable by construction; a card is a derived answer that a later build may state better.

## Alternatives

**Give `karakuri-store` a dependency on `karakuri-ir`.** Rejected: it makes the store's job
"keep bytes and compile them", and the store is the one component with no opinion about what
the bytes mean.

**Build the card at write time from the source.** Rejected: it means re-compiling on the
save path, on the render thread's side of a save.

**Fail the save when the card fails.** Rejected: it trades a complete store for an empty
one over a file that regenerates.

## Consequences

`Store::put_artifact` is public and writes no card, so *"every path that stores an artifact
writes one"* is false as stated and the documents now say what is true: both paths that
store an artifact **from a compile** write one. An artifact without a card is not a damaged
store.
