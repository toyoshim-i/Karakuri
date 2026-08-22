---
id: 0018
title: Three documents, three jobs
status: accepted
date: 2026-07-26
supersedes: []
superseded_by: []
principles: []
tags: [docs, process]
---

# Three documents, three jobs

## Context

A long-term roadmap was added beside the `README` and the specification, and the three
overlapped.

## Decision

- **`README.md` — where the project is.** Usage, the rules in force, and a table of what
  works and what does not. Status lives here and nowhere else.
- **`docs/roadmap.md` — where it is going.** Milestones, and a reference of what has been
  settled. **Not history:** for completed items it keeps the decision, not the account of
  reaching it.
- **`docs/ir-spec.md` — the language.** With a boundary, `## Beyond v0.2`, above which
  everything is implemented and tested and below which things are **decided but built by
  nobody**, each tagged with the milestone it belongs to.

The `Beyond v0.2` line is the useful invention: it lets a decision be recorded at full
detail without implying it exists.

## Alternatives rejected

- **Keep status in the roadmap.** Then a milestone reads as done because its prose is
  confident. M1 already claimed to prove hot-swapping, which had not been built.

## Consequences

- **Not superseded, despite the note below.** The front matter briefly said otherwise; ADR-0000 did
  not replace this split, it added two homes beside it, and all three documents still do what this
  record says they do.
- Explicitly *not history*, which is correct for these three and is what leaves the project
  with nowhere to record a supersession. That gap is what
  [ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md) closes almost a
  month later; this record is superseded there only in the sense that the split gains a
  fourth and fifth home, not that the split was wrong.

## Evidence

Session 2026-07-26T02:08Z, 2026-07-26T07:50Z.
