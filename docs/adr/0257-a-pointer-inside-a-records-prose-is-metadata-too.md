---
id: 0257
title: A pointer inside a record's prose is metadata too
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0093]
tags: [process, docs]
---

# A pointer inside a record's prose is metadata too

## Context

[ADR-0059](0059-a-records-pointer-into-the-principles-registry-is-metadata.md) made an ADR's
`principles:` front matter and its closing *Standing rule:* line updatable when a principle is
renumbered, and drew the line there: **the prose of a record is still never edited.**

Retiring 58 principles into thirteen moved several hundred pointers, and most of them are inside
prose. Following ADR-0059's letter would have left a record whose front matter names the live rule
and whose sentences link a file that is gone.

## Decision

**A link is metadata wherever it sits, including inside a sentence, and it may be re-pointed under
one test: does the sentence stay true of what it now points at?**

- **True after the swap** — re-point it. The record's argument is unchanged; only where the reader
  lands moves.
- **False after the swap** — the sentence is about the retired *file* rather than about its rule.
  *P-0074 cited `SetOnAir` as its worked example*, or *its third paragraph*, or a verbatim quotation
  of a clause the new rule does not contain. Remove the link and leave the number and the title:
  `INDEX.md`'s *Retired numbers* table is where it resolves. Nineteen sites were left that way when
  the thirteen landed, twelve of them in
  [ADR-0234](0234-carrying-a-show-through-is-a-principle-not-a-property-of-the-finished-instrument.md),
  which is annotated to say so.
- **A claim that is now a record's rather than a rule's** — point it at that record. *P-0072's first
  clause* has no referent in a rule with no numbered clauses; it belongs to ADR-0164.

**The words are still never edited to change what a record argued.** Where a swap would need the
sentence reworded to stay true, that is the second case, not licence to rewrite.

## Alternatives rejected

**Keep ADR-0059's letter and leave every prose link dangling.** It is the reading that touches
nothing, and it produces records that contradict their own front matter and hundreds of dead links
in a directory whose value is that it can be followed.

**Re-point every link mechanically.** Cheaper and it makes records false — ADR-0234's lists would
have eight principles naming the rule they are cited for not stating.

**Rewrite the sentences to fit the new pointers.** This is the one ADR-0059 exists to forbid, and it
is worse here than usual: the sentences being rewritten are the evidence for the retirement.

## Consequences

- **ADR-0059 stands.** This widens what counts as a pointer and adds the test; it does not license
  editing an argument.
- **The practice is written down rather than inferred from six commits.** It was applied across the
  thirteen retirements before it was recorded, which is the wrong order and is noted here rather
  than tidied away.
