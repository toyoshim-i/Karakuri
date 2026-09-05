---
id: 0046
title: A flag writes into the record; it does not invent one
status: accepted
date: 2026-07-31
supersedes: []
superseded_by: []
principles: [0090]
tags: [process, format]
---

# A flag writes into the record; it does not invent one

## Context

With several Sets on the deck, `--param` applies to all of them, which is almost never wanted. The
obvious fix is a syntax like `--param 1:turbulence=2.6`.

The agent wiring the CLI **did not add it**, and said why: there is nowhere in `--set l1,l4` to put
it, and inventing the syntax would be **accidentally designing the record stream**.

## Decision

That reticence is the rule. **A flag is a way to write into the record, not a second place the
truth lives.** How a parameter is addressed per slot must match how a Set file will express it once
the store reaches the engine, and that is not a question the CLI's convenience gets to answer
first.

The same reasoning shaped `--bind`: one comma-separated `field=value` per JSON field of
`Record::Bind`, so replacing it with a real Set file is *deleting the parser and calling the same
method from the decoder*. One deviation was forced and is documented — `range` is `LOW..HIGH`
rather than `[low,high]`, because a comma inside a value cannot be told from the separator.

Loading Set files into the engine was deliberately **kept out of this slice**. It is where bindings
belong, and it is its own slice ("actually use the store"). Reaching them by flag now, in the
record's shape, makes the later replacement mechanical.

## Consequences

- `--bpm` was added with the same discipline and one honest gap noted: **it has no record to map
  onto**, because the v0.2 vocabulary has no tempo record. A `beat` binding is meaningless at a
  tempo nobody can set, so the flag exists and the hole is recorded rather than papered over.
- The destination is a GUI application and the CLI is scaffolding. Every decision about where a
  value lives follows from that, and a design that puts a value **only** on the command line is
  building on the part that gets replaced.

## Evidence

Session 2026-07-31T13:27Z, 2026-07-31T23:48Z. Standing rule:
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md).
