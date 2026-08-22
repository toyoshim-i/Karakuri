---
id: 0066
title: A flag becomes a record writer, and a Set file matches it byte for byte
status: accepted
date: 2026-08-03
supersedes: []
superseded_by: []
principles: [0028]
tags: [store, format]
---

# A flag becomes a record writer, and a Set file matches it byte for byte

## Context

The record vocabulary was largely complete and **material still arrived only through flags**. The
roadmap also carried a debt: two diagnostics — refusing a `bpm` binding, checking `noise.octaves` —
existed only in the flag parser, and *"the decoder owes them too."*

## Decision

**Set files load, and the acceptance test is byte equality.** The same material via a file and via
flags renders **byte-identical PNGs**. Only then is it true that the engine follows the format rather
than resembling it.

And the debt was paid by **deleting a copy rather than writing a second one**: `--bind` now assembles
a `Record::Bind` and hands it to the decoder. One rule, one place, so the command line and a Set file
cannot come to mean different things by the same field. This is
[ADR-0046](0046-a-flag-writes-into-the-record-it-does-not-invent-one.md) discharged as designed —
what was written in the record's shape became a record.

**One check stays on the flag, for a stated reason.** `BindNoise::octaves` has a serde default,
because a generator omitted field by field is *unspecified* rather than *refused* — so the record
**cannot express whether `octaves` was named**. The flag knows, so that check is the flag's alone.

## Consequences

- **Two defects that reading did not find, running did.** A loaded Set was *added* to the deck rather
  than replacing the default pair, so the deck held two slots and the file's bindings landed on
  unrelated material and reported no such parameter. And `--load-set` together with a `.kir` path is
  two answers to what should play; it is now refused.
- **What is dropped is announced.** The format is per layer and the engine is per Set, and they differ
  in three places: the layer of `seed` / `capacity` / `param`, vector `param`s, and a `camera` record
  carrying two of `Orbit`'s six fields. Every one prints on load, because **half-applied and silent is
  the failure this repository keeps refusing**.

## Evidence

Session 2026-08-03T03:45Z, commit `a11c4c1`.
