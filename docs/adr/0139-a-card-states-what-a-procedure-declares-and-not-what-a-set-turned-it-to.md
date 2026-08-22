---
id: 0139
title: A card states what a procedure declares, and not what a Set turned it to
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [mcp, metadata]
---

# A card states what a procedure declares, and not what a Set turned it to

## Context

`read_set` reads a Set file to find its nodes, so the `param` and `capacity` records saying
what *this Set* has each knob turned to are already in hand, a few lines from the
declarations being rendered. Printing both is one line of work and reads, at a glance, like
a more complete answer.

## Decision

**Only the declarations are rendered** — the range a value would be refused outside of, and
the default that holds until something turns it. The answer says so in its own first
sentence.

## Alternatives

**Render the Set's values beside the ranges.** Rejected. The format keeps two names for
these on purpose — `param_decl` beside `param`, `capacity_decl` beside `capacity` — because
a declaration and a value are different facts, and the reason for two names does not survive
putting both in one block under one heading. *What is this knob allowed to be* and *where is
it now* are two questions, and the second one is about a Set rather than about the artifact
this card describes.

A test asserts a fixture's Set value does not appear in the answer, so the two cannot quietly
merge later.

## Consequences

A model choosing among saved material gets what it needs to choose — the ranges. Asking
where a Set left a knob is a second call, and it is owed a tool of its own if something
wants it; nothing does yet, and inventing the surface before then is what the withdrawn
`bytes_per_element` figure was.
