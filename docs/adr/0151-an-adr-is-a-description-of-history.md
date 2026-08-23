---
id: 0151
title: An ADR is a description of history
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: [0066]
tags: [process, docs]
---

# An ADR is a description of history

## Context

[ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md) says the directory
is append-only: *"an ADR is not edited after it lands, except to set `status` and
`superseded_by`."* `INDEX.md` compressed that to *"except to set its status"* and put it above
the table, where it is the first thing anybody reads.

That compression was enough to mislead. Working in this repository on 2026-08-23 I took the
directory to be closed, passed "do not touch `docs/adr/`" to every sub-agent I briefed,
declined to fix a pointer I could see was broken, and wrote a sentence into
`docs/contributing.md` saying a record *"must not be corrected when the present moves"* —
which would have told the next reader the same thing.

The counter-evidence was in every record the whole time. `status`, `superseded_by` and
`principles` are front-matter fields whose only purpose is to be written after the record
lands. If landing made a file untouchable those fields could not work. And `contributing.md`
already instructed the opposite in one specific case: when a principle is retired, **re-point
the ADRs that cited it**.

Two concrete failures were sitting in the tree while the misreading held. `docs/principles/`
had two files numbered `0061`, reached for by two writers three hours apart, with ADR-0146
citing one and `INDEX.md` the other — visible, and left alone as somebody else's territory.
And a citation in the code pointed at a document that had been deleted hours earlier.

## Decision

**An ADR is a description of history**, and that settles all three cases.

- **Revision — editing a landed record so it agrees with the present — is history revision**
  and is refused. It destroys the only account of how things came to be, silently, because
  the rewritten record reads as plausibly as the original. An argument that would have to
  change is a new record.
- **A description that was wrong is corrected.** A misstated fact, or a pointer that has gone
  bad, is not history; it is a bad account of history, and append-only is not a reason to keep
  it.
- **Annotation is welcome.** Recording what a record later became adds information about the
  past without changing what it says happened, which is what the front matter is for.

The operational test: *does the edit change what the record says happened, or what a reader
can find out about it?* Revision, or maintenance.

## Alternatives

**Treat the file as immutable, front matter included.** The simplest rule, and it does not
work: supersession could never be recorded, and retiring a principle — which requires
re-pointing the records that cited it — would be impossible. The mechanism the directory
already depends on refutes it.

**Allow editing whenever the present moves.** This is what "keeping the documentation current"
would ordinarily mean, and it is exactly what must not happen here. It costs the thing the
directory exists for.

**Leave it as prose in `INDEX.md` and `contributing.md`.** Both were corrected earlier today,
which produced two statements of one rule in two documents — the drift `ADR-0000` opens by
counting five instances of. It is a principle instead, and both documents point at it.

## Consequences

`P-0066` is the rule. `INDEX.md` and `contributing.md` §4 cite it rather than restating it.

The wider cost is worth recording: this misreading suppressed work for a day. Repairs I could
see and did not make, and the sub-agent briefs I wrote saying a directory was off-limits, were
all downstream of one compressed sentence at the top of an index — which is its own argument
for keeping the entrance documents exact.
