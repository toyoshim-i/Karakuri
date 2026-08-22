---
id: 0125
title: A client-named id is an allow-list
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: [0043]
tags: [mcp, store, security]
---

# A client-named id is an allow-list

## Context

Found by the implementing agent, outside its brief. `Store::set_path` embedded a Set id into a
filename **with no validation**. From the operator's own shell that is unremarkable. **From a model it
is not**, and `'../../../etc/passwd'` was accepted as the name of a file under `<store>/sets/`.

## Decision

**An allow-list — alphanumerics, `-`, `_` — not a deny-list.** Neither `.` nor `/` survives, so there
is nothing to enumerate and nothing to miss.

## Consequences

- **The rule was already written down; the vigilance had not reached this value.** `mcp.rs` says: *a
  path does not cross the protocol — a client may be on another machine behind `ssh -L`, and a tool
  that takes a path is an invitation to write anywhere on the render machine's disk.* The hand-rolled
  HTTP was already hardened with an `Origin` check, a body cap and a read timeout, with the comment
  that **loopback is not a boundary** — it is reachable from any process on the machine and from a
  `fetch()` on a page the operator has open. The id was the same class of value and had been left out.
- The review was told to check, before anything else, that `checked_id` covers **every** path — because
  *a validation that only exists in one place* was met twice on the same day.
- The review then found a further gap and it is recorded rather than fixed: a client-named id **bypasses
  the overwrite guard**, which exists because *the second Set file overwrote the first, and the operator
  was told both had been kept.*

## Evidence

Session 2026-08-22T05:34Z.
