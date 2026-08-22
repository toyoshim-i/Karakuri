---
id: 0004
title: `var` is added so a loop can carry a value
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: []
tags: [ir]
---

# `var` is added so a loop can carry a value

## Context

Writing up the identity change surfaced that **`for` was unusable**. `let` is immutable and
an attribute read always returns the previous frame's value, so a loop body had nothing that
could carry a value from one iteration to the next. `for` was in the grammar and could
express nothing but repeated attribute writes.

## Decision

Add `var`: a block-scoped mutable local with a required initialiser, plus compound
assignment `+= -= *= /=`.

Five rules came with it, each closing a specific trap:

- **Initialiser required**, type fixed at declaration. No uninitialised locals.
- **Block scope; never crosses a frame.** Element state exists only in emitted attributes.
- **Compound assignment is `var`-only and never applies to an attribute.** `position += v * dt`
  looks like accumulation and re-reads the previous frame every iteration, so inside a loop
  it is always a bug. Attributes take plain `=` only.
- Assigning an undeclared name is an error, not a declaration.
- `let` / `var` / loop variables may not shadow a param, attribute or ambient.

Two dependencies were added in the same pass, because `var` alone still left `for` unusable:
scalar constructors (`float(i)`, since the loop variable is `int` and could not otherwise
enter float arithmetic) and integer `%` (`seed % 512u` had no defined meaning).

## Alternatives rejected

- **Drop `for` from v0.2** and cover the cases with `fbm`. Rejected: hand-built octave sums
  and any accumulation over a fixed count become inexpressible.

## Consequences

- No effect on cost estimation: `var` lowers to a plain WGSL function-local, and the estimate
  stays instruction count × loop bound. Any real cost is register pressure, which is for the
  probe.
- Guidance in the specification: prefer `let`; reach for `var` for accumulation and for a
  value that differs between the arms of an `if`.

## Evidence

Session 2026-07-25T11:55Z–11:58Z.
