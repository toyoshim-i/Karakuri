---
id: 0111
title: A name lives in the Set file and may be written on the command line
status: accepted
date: 2026-08-19
supersedes: []
superseded_by: []
principles: [0056, 0028]
tags: [format, ir]
---

# A name lives in the Set file and may be written on the command line

## Context

[ADR-0101](0101-a-sources-number-is-recorded-not-derived.md) settled that a source's value is
assigned and recorded, and that a name is an alias written on the **use** side. One clause of it had
been marked, in the specification itself, as **preference rather than force**:

> **A name is written in a Set file and not on the command line.** … Anyone who needs to point at a
> source is already writing a Set file.

## Decision

**That reasoning is falsified by the code, and the clause changes.**

A Set file cannot save a chain or a second geometry — `refuse_unsavable` rejects it by name. So
**the person with two geometries is exactly the person who cannot write a Set file.** You need a Set
file to get a name, and a name to write a Set file. **Circular.**

So: **names live in the Set file *and* may be written on the command line.** The file is where a
*use* is recorded, so it is the name's home and it is where M5's GUI will write. The command line
gets a spelling because **it is the only authoring surface that exists today**, and it overrides the
file by the rule `--param` beside `--load-set` already follows.

Names are optional; without one, derive from the procedure name and disambiguate with a suffix.
**The moment it is recorded it is the address, derived or not** — the specification's own *where a
value came from stops mattering once it is recorded* applies unchanged. The internal address stays
`(layer, index)` and **the name is an alias**, exactly the shape `Published` already has.

**Scope is split in two**, and only the first is taken now: **identity** (a name on a node,
recorded) closes the sharp cases — `--watch` rebuilding two geometries, saving a Set file, MCP, edit
history, and salt assignment. **Edges** (which two a `pairs` takes, which source a mask hits) is a
separate design decision, because a `.kir` cannot name them without binding the procedure to one Set.

## Consequences

- **The section was rewritten to keep the half that worked and drop the half that did not** — and to
  record that being **marked as preference is what let it be revisited without a redesign.** That is
  what the mark is for.
- Unavoidable under any option: **`Record::Capacity` and `Record::Seed` carry only a layer**, so two
  geometries of different capacity and a per-source salt are **not expressible in the format at all**.
  Adding names means touching both.
- One general rule was written down once, in one place: **the CLI is scaffolding and the destination
  is a GUI application.** An authored value lives in a record and a flag is a way to write into it —
  which is why every control ends in the record a key press ends in. **Not fastidiousness: it is what
  lets a GUI replace the CLI without replacing the engine.**
- Also found, unreachable but real: the Set file reader **discards an L1's `index`**, so two
  `slot L1` lines silently last-write-win, while the L4 side reports the duplicate.

## Evidence

Session 2026-08-19T14:15Z–14:21Z. Standing rule:
[P-0056](../principles/0056-mark-what-is-preference-so-it-can-be-revisited.md).
