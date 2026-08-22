---
id: 0063
title: An invariant that is not yet true says so
status: accepted
date: 2026-08-02
supersedes: []
superseded_by: []
principles: [0036]
tags: [docs, determinism]
---

# An invariant that is not yet true says so

## Context

A review checked the headline invariant — *the record stream is the only path that mutates engine
state* — against the code rather than against the documentation, and confirmed audio genuinely goes
through records: values are reconstructed from a record and reach the bus, with no second path.

And then:

> **`tick` does not go through a record.** `Live::steps()` passes a `u8` straight to `deck.render`,
> and `Record::Tick` is never constructed anywhere in the CLI.

The session seed, the initial tempo from `--bpm`, and the `--bind` bindings were not in the stream
either. **Audio had got ahead of `tick`** — the newest subsystem obeyed the invariant the oldest one
did not.

## Decision

The `README` states the invariant **unconditionally**, so either the text or the implementation is
wrong, and until the implementation catches up **the text is corrected** to say what is actually
true.

An aspiration written in the present tense is indistinguishable from a description, and this project
had already been bitten by that from the other side — a comment describing a design in the function
that replaced it ([ADR-0031](0031-a-document-describing-replaced-behaviour-is-worse-than-none.md)).

## Consequences

- Closed on 2026-08-03: `tick` gained a writer with the session recorder, and `audio` followed the
  same day. *"The record stream is the only path"* became a description rather than a goal, and every
  record type has a writer.
- The general form: **an invariant is a claim, and a claim has a truth value at a date.** Where it is
  aspirational, saying so costs one clause and buys the reader the ability to trust the rest.

## Evidence

Session 2026-08-02T06:13Z; closed at 2026-08-03T04:00Z (`12c4d66`) and 2026-08-03T14:12Z (`768b4d3`).
Standing rule: [P-0036](../principles/0036-an-invariant-that-is-not-yet-true-says-so.md).
