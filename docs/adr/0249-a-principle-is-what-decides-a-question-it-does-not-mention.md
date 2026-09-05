---
id: 0249
title: A principle is what decides a question it does not mention
status: accepted
date: 2026-09-04
supersedes: []
superseded_by: []
principles: []
tags: [process, docs]
---

# A principle is what decides a question it does not mention

## Context

`docs/principles/` was meant to hold the design's **meta-principles** — the maintainer's thinking
written down, so that somebody stuck on an undecided question could infer what he would decide. In
his words: *a company creed you hold a problem up against.*

It holds 78 files. That is not a creed anyone consults; it is a catalogue nobody reads, and the
symptom is contradictions accumulating between files that were never obliged to agree with each
other. One-off decisions had been written in beside the meta-rules, and once they are side by side
nobody can tell, at the moment of being stuck, which kind they are reading.

**The gate let them in, and the gate is [ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md)'s.**
It reads:

> A rejection earns a principle only if **a future proposal could violate it**. `An element is never
> identified by its buffer slot` is a rule somebody will otherwise re-propose.

That test admits any specific prohibition, because a specific prohibition is exactly the kind of
thing somebody re-proposes. Violability is a property of a *rule's subject*; being a creed is a
property of its *reach*. The two come apart, and the directory is what they look like when they do.

**ADR-0000's own worked example is the proof.** *An element is never identified by its buffer slot*
(P-0007) passes the old gate — indexing by slot is the natural thing to reach for, so somebody will
re-propose it — and fails the one below: ADR-0102 settled a *renderer's* address as layer-and-index
without needing it. It is a rule about one identity in one buffer. The gate's showcase is a file the
gate should not have admitted.

## Decision

**A principle is a file that decides a question it does not itself mention.** That is the gate, and
it replaces *could a future proposal violate it*.

**It has to be demonstrated, not asserted.** To add one, name a concrete undecided question — from a
domain the file does not discuss — that reading the file answers. If no such question can be named,
the rule is a decision and belongs in an ADR, however true, important or load-bearing it is.

**Judge the body, never the filename.** A narrow title over a general argument is the shape that
defeats a sort: *Solving a layout never mutates it* (P-0071) reads like one crate's rule and its
third paragraph names a failure shape — a read that quietly corrects what it read, invisible at the
moment and only visible across time — that decides things about governors, replays and previews.
Where a title undersells the body, widening it is a delete-and-re-record rather than an edit.

**Being covered by another principle is a third answer, and it is the one most often got wrong.**
Two files can state one rule (which is the duplication ADR-0000 already forbids), or one can be an
instance of the other, or they can agree while deciding different things. Only the first two are a
defect. The test is drift: **would changing one leave the other false?** Matching by topic, or by the
shape of the rule, finds pairs that are neither.

**ADR-0000's other clauses stand**: one rule per file, `ls docs/principles/` as the index, numbers
never reused, and a rule that stops being true deleted and re-recorded rather than edited.

## Alternatives rejected

**Keep the old gate and apply it more strictly.** It is not a matter of strictness. *Could somebody
re-propose this* is answered *yes* for every specific prohibition worth writing down, so applying it
harder admits the same files with more argument attached.

**Group the 78 into thematic chapters.** Proposed, and it reduces nothing: the same rules sit in six
drawers instead of one, and a chapter cannot be sorted by altitude because two rules on one topic sit
at different heights — *a test meant to catch something is run against the defect* and *do not use
rewritten product data as a fixture* are both "testing" and only one is a creed. It also costs every
principle its address, which [P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md)
requires source to cite, and it breaks retirement: one rule cannot be deleted out of a six-rule
chapter without editing the chapter.

**Cap the count.** A number is not a test, and a cap decided in advance is answered by merging files
rather than by judging them.

## Consequences

- **`docs/contributing.md` §1 gains a step**: when a judgement is genuinely open, hold it against the
  principles before deciding it, and record where none of them reached. That is what the directory is
  for and it was written nowhere.
- **`docs/contributing.md` §4's gate sentence is replaced**, and the demonstration requirement is
  stated with it.
- **ADR-0000 is annotated** with the clause that moved. Its example survives as this record's
  evidence rather than as the gate's.
- **The existing 78 are audited against this gate rather than grandfathered.** The first pass, by two
  subagents, put the tally at 41 creeds; reviewing its *absorb* column by the drift test above
  overturned ten of twelve, which is this record's *judge the body, never the filename* clause
  failing in practice before it was written down. The sort is evidence for the maintainer, not a
  decision.
- **Nothing is deleted by this record.**
