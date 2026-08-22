---
id: 0059
title: A record's pointer into the principles registry is metadata
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: []
tags: [process, docs]
---

# A record's pointer into the principles registry is metadata

## Context

Discovered by operating the system rather than by designing it.
[ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md) says an ADR is never
edited after it lands, except to set `status` and `superseded_by`. It also says a principle is
**deleted and re-recorded under a new number** when it stops being true.

The two rules collide the first time a principle is retired. P-0015 stated one-tag-one-shape for
record tags; the same rule turned out to be the narrow case of a broader one — a name means one thing
anywhere — so P-0015 was retired and P-0031 written. **Every ADR that pointed at P-0015 now points at
a file that does not exist**, and no permitted edit can fix it.

## Decision

An ADR's `principles:` front matter and its closing *"Standing rule:"* line are **metadata, not
content**. They are pointers into a registry that is expressly mutable, and they may be updated when a
principle is renumbered, exactly as `status` may be.

**The prose of a record is still never edited.** If a decision's reasoning would have to change, that
is a new record, not a correction.

## Alternatives rejected

- **Never retire a principle, only sharpen it in place.** Discards the mechanism the whole two-tier
  arrangement rests on, so that a rule which genuinely broadened would keep the narrow name forever.
- **Leave the dangling links and rely on the retirement table.** A reader following a link into
  nothing has to go and find out whether the record is stale or the registry is; that is precisely the
  cost the index exists to remove.
- **Reuse the retired number for its successor.** Rejected in ADR-0000 for its own reasons, and it
  would silently redirect every existing citation to different text.

## Consequences

- Retiring a principle now has three steps rather than two: delete the file, write the successor, and
  **re-point the records that cited it**, then record the tombstone in `INDEX.md`.
- This refines ADR-0000 without replacing it. It adds a permission the original did not anticipate;
  everything ADR-0000 decided still stands.

## Evidence

The first two retirements, on 2026-08-22: P-0015 → P-0031, and P-0022 → P-0032.
