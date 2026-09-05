---
id: 0010
title: One `t` value is one record shape, across every file
status: accepted
date: 2026-07-25
supersedes: []
superseded_by: []
principles: [0093]
tags: [format, store]
---

# One `t` value is one record shape, across every file

## Context

Found by the agent implementing the artifact store, not by reading. The metadata file and the
Set file both used `{"t":"param",...}` for **differently shaped records** — the Set's carried
`layer`/`key`/`value`, the metadata's carried `key`/`type`/`min`/`max`/`default`. A decoder
that dispatches on `t` cannot read both, and every ndjson decoder dispatches on `t`.

The collision was introduced by an earlier edit in this same session, which is the useful part:
it was created and shipped within hours by someone holding the whole format in mind.

## Decision

**One `t` value has one shape, everywhere.** The metadata records are renamed `param_decl` and
`capacity_decl`, and the rule is written into the specification rather than left as a property
the current files happen to have.

## Alternatives rejected

- **Dispatch on `t` plus the containing file.** Every decoder then needs to know which file it
  is reading, and a record can no longer be moved or quoted between streams.

## Consequences

- Forward compatibility is round-trip, not just read: an unknown record is preserved
  **verbatim**, because re-serialising a payload-free `Unknown` would fabricate
  `{"t":"unknown"}` and destroy the line — worse than dropping it, because it looks faithful.

## Evidence

Session 2026-07-25T12:59Z. Standing rule:
`docs/contributing.md` §4.
