---
id: 0137
title: An unfoldable default writes the key absent, not the record absent
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [metadata, ir]
---

# An unfoldable default writes the key absent, not the record absent

## Context

A `param_decl` states a declared default as a number, and `Param::default` is an
expression. The fold that turns one into the other handles a literal and a leading
negation — deliberately not general constant folding — so some legal declarations have no
number to write.

**One fold, and it moved.** The engine had the only one, private, and its own doc had
already said where it belonged: *"anything past that wants folding in `karakuri-ir` where
the checker could also use it, rather than a second evaluator here that agrees with the
shader by coincidence."* A card that read defaults for itself would have been exactly that
second evaluator, so the fold is now `karakuri_ir::Param::default_scalar` and the engine and
the card are its two callers. A negative default was a real bug here once; both sides now
get it right or wrong together, which is the property worth having.

## Decision

A parameter whose default does not fold gets its `param_decl` **with the `default` key
absent**.

## Alternatives

**Write no record for that parameter.** Rejected: it is a false statement. A missing record
says *no such parameter*, where a missing key says *this parameter's default is not known
here* — and the second is true. A model reading the card would otherwise be told a knob does
not exist.

**Write a placeholder — zero, or the range's minimum.** Rejected on this milestone's own
experience: a number published under a name that reads as a measurement fails in the
dangerous direction, which is why `bytes_per_element` was withdrawn rather than corrected.

## Consequences

The choice survives the format's forward-compatibility rule symmetrically: an unknown key
inside a known `t` is ignored, so an absent key and a key a later build adds behave the
same way. A record that is not there cannot be skipped into existence.
