---
id: 0149
title: Source cites what is in force, not a plan
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: [0093]
tags: [process, docs]
---

# Source cites what is in force, not a plan

## Context

Comments in this repository reach for `docs/roadmap.md` constantly, and by milestone and
section name: *"`docs/roadmap.md`, M4, \"Naming what a Set holds\""*, *"on M2's L5 mixer"*,
*"`docs/roadmap.md`'s pool is deck slots"*. Counted on 2026-08-23: **71 references to the
roadmap across 26 source files, against one reference to an ADR or a principle in the whole
of `crates/`.**

The code cites the plan seventy-one times and cites what is in force once.

Three things are wrong with a comment that points at a milestone.

**A plan is not an explanation.** A reader who follows the pointer arrives at a description
of work that was intended, in a document organised by when things were going to happen. What
they wanted was why this code is the way it is. The two coincide only while the milestone is
current.

**It goes stale in a way nothing detects.** A milestone closes, its section is rewritten to
say it closed, and the sentence the comment was pointing at is gone or now means something
else. Nothing fails. The comment still reads plausibly, which is worse than a broken link —
this repository already has [P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md)
for that failure in prose, and a comment citing a moved plan is the same failure with a
compiler in the room that cannot see it.

**It survives the reorganisation that proves the point.** Splitting closed milestones out of
the roadmap — the change that prompted this record — would have meant rewriting all
seventy-one, and the rewrite would have pointed them at a *history* file: a plan that is now
explicitly past. That is the moment the citations were revealed as decoration rather than
reference.

Most of them already are decoration. The claim is stated in the comment and the pointer adds
nothing:

> `Ln` is a node — `docs/roadmap.md`, "a Set stops owning everything" — and the unit that
> owns GPU state is the node rather than the grouping.

## Decision

**Source comments cite what is in force. They do not cite a plan.**

- Where the comment already states the claim, the pointer is deleted. Nothing is lost,
  because the sentence was carrying the argument and the pointer was carrying provenance
  nobody needs at that moment.
- Where the reason is genuinely elsewhere and load-bearing, the citation goes to an
  **ADR** or a **principle**. Both are addressable and neither is a schedule: a principle
  states what constrains the change you are about to make, and an ADR states what was
  decided and what lost. An ADR is the durable ticket a comment can point at for as long as
  the code exists.
- `docs/ir-spec.md`, `docs/manual.md` and the other present-tense documents remain fair to
  cite. The rule is about *plans*, not about documents.

**What is ruled out** is the citation that reads as authority and is a schedule: `docs/roadmap.md`,
and any `docs/history/` file that a closed milestone is moved into. If a comment needs
`history/`, what it actually needs is an ADR that does not exist yet.

## Consequences

Seventy-one sites are wrong today and are not fixed by this record; it is what makes them
findable. `grep -rn roadmap crates/` is the whole list, and each is one of three moves —
delete the pointer, inline the reason, or write the ADR the comment wanted.

The imbalance is the more interesting number. One ADR citation in `crates/` against a
hundred and forty-nine records means the catalogue is not yet where anybody reaches when
they are writing code, which is the thing worth changing rather than the pointers
themselves.
