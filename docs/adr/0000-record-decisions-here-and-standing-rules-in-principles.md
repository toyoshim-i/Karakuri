---
id: 0000
title: Record decisions here and standing rules in principles/
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [process, docs]
---

# Record decisions here and standing rules in `principles/`

## Context

The reasoning behind this project's design is written down, and written down well.
What it is not is *addressable*. The same set of foundational invariants is currently
stated in five places:

- [docs/architecture.md](../architecture.md) §1 — "Core Principles & Design Goals"
- [docs/contributing.md](../contributing.md) §1 — "Project Invariants & Core Rules"
- [docs/roadmap.md](../roadmap.md) — "Settled decisions", which opens `Reference, not history`
- [docs/ir-spec.md](../ir-spec.md) — "Resolved"
- [crates/karakuri-engine/src/lib.rs](../../crates/karakuri-engine/src/lib.rs) — the crate header

They have already drifted. `architecture.md` and `contributing.md` were committed on the
same day, each presenting *the* four core invariants, and they are not the same four:
one lists double-buffered hot swap where the other lists order-preserving compaction.
The engine's crate header — the best-written of the five — says `Nothing in this crate
reads a clock`, which `swap.rs` and `probe.rs` both falsify by grep, though the rule it
means to state is true.

Two failures follow from having no single home:

- **A decision can be lost in a day.** `867973b` (2026-08-21) moved the full test suite off
  every push and onto tag pushes only, with the reasoning written into the hook. `161b9f0`,
  one day later, added `contributing.md` saying `pre-push` runs all test suites.
- **A settled question gets re-opened.** The identity of merged geometries was argued,
  settled, and asked about again as `I feel like we discussed this before — did we ever
  answer it?`, because the answer lived in a conversation rather than at an address.

## Decision

Two directories, with different rules, because they answer different questions.

**`docs/principles/` — what is in force.** One standing rule per file, current only.
It is a working set, not a log: when a rule stops being true it is **deleted**, and its
replacement is recorded under a **new number**. The question it answers is *what constrains
the decision I am about to make*.

- The **filename is the rule**, written as a sentence. `ls docs/principles/` is therefore
  the index, and being generated from nothing it cannot drift.
- **Every rule states what it rules out.** This is the load-bearing part. A conclusion on
  its own does not stop anyone re-proposing the alternative; the sentence that does is the
  one naming the alternative and saying why it loses — as `Settled decisions` already does
  with *"Atomic allocation is cheaper and forfeits it."* Because the rejected alternative
  travels inside the rule, deleting the rule's history costs nothing.
- Numbers are **never reused**. A retired number stays retired.

**`docs/adr/` — how it was decided.** Every decision, including the ones that were reversed
and the ones that were rejected. Append-only: an ADR is not edited after it lands, except
to set `status` and `superseded_by`. The question it answers is *why is it this way, and
what did we already try*.

**Every principle has an ADR. Not every ADR yields a principle.** A rejection earns a
principle only if a future proposal could violate it. `An element is never identified by
its buffer slot` is a rule somebody will otherwise re-propose; `rekordbox never publishes
its deck BPM` is a fact about the world and lives here only.

> **Annotated 2026-09-04.** **That gate is replaced by
> [ADR-0249](0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)**: a principle is
> what decides a question it does not itself mention, demonstrated by naming one rather than
> asserted. *Could a future proposal violate it* admits every specific prohibition, which is how
> `docs/principles/` reached seventy-eight files and stopped being something anybody consults. The
> example above is the proof rather than the showcase: `An element is never identified by its buffer
> slot` passes the old test and fails the new one. Everything else on this page stands — one rule per
> file, `ls` as the index, numbers never reused, deletion rather than editing.

**Citation.** `ADR-0007` and `P-0007` are different documents. Always write the prefix.

## What moves, and where

Other documents keep the present tense. `ir-spec.md` in particular should read as the
current language and nothing else. Each entry in its `Resolved` section already splits
cleanly, because every one of them ends by pointing at the normative text:

| Part of the entry | Goes to |
| --- | --- |
| The normative statement | stays in `ir-spec.md`, where it already is |
| The constraint on future work | `principles/` |
| The deliberation and the rejected alternatives | `adr/` |

`Resolved` can then be deleted outright. `Open questions` is about the present and stays.
`Deferred by decision` in the roadmap is about *when* rather than *what*, so it stays too,
with its argument moved here and linked.

`architecture.md` §1 and `contributing.md` §1 stop carrying copies and link to
`principles/` instead. That is what removes the drift structurally rather than by
proofreading.

## The index

`INDEX.md` in this directory is **maintained by hand**, and that is a decision rather than
an oversight. At five ADRs a generated index costs more than it returns, and the documents
are read by agents often enough that a contradiction surfaces on its own. When the count
passes roughly twenty and a missing entry stops being obvious, the check to add is a test
asserting `INDEX.md` matches the front matter of the files beside it — a test rather than a
script, because the workspace is closed to Rust and an index generator is not a reason to
open it. Where such a test lives is undecided; a documentation crate may earn its place by
then.

`principles/` has no index and should not grow one until it passes roughly forty files.

## Consequences

- A decision now has an address, so a commit message, a code comment, or a prompt can cite
  it, and a reader who follows the citation gets the reasoning rather than a restatement.
- The history layer is git's. Deleting a principle loses nothing —
  `git log --diff-filter=D -- docs/principles/` recovers every rule this project ever held.
- Served over MCP, `principles/` gives the model writing `.kir` the designs that are already
  ruled out, which is worth more than the same text buried in a 200 KB specification.
- This ADR is itself a decision and can be superseded like any other.
