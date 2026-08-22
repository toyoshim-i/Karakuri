---
id: 0134
title: The metadata `preview` becomes `thumbnail`
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [store, records, metadata]
---

# The metadata `preview` becomes `thumbnail`

## Context

The format's rule is **one `t` means one shape, across every file** — stated in
`docs/ir-spec.md` with its reason: every ndjson reader dispatches on `t` alone, so it is
not enough for two vocabularies to be disjoint in practice, they have to be disjoint by
name.

`preview` broke it. `Record::Preview { slot: Option<u8> }` is a deck record meaning *which
slot the operator is auditioning*; the metadata section specified
`{"t":"preview","path":"…"}` for a stored asset.

**The failure was silent, which is why it matters.** Because `slot` is an `Option`, a
spec-conformant metadata `preview` line decodes as `Preview { slot: None }` with `path`
dropped as an unknown key. So the metadata decoder's promised *"an unknown `t` is ignored"*
was violated for exactly one record: it was not passed over, it was misread. The design
was also already blocked — `is_metadata`'s doc said the remaining records "are added here
when one arrives", and `preview` could never be, both the variant name and the `t` string
being taken.

## Decision

The **metadata** record is renamed `thumbnail`, and the store's asset directory follows it
from `previews/` to `thumbnails/`.

`docs/roadmap.md` already used that word and already drew the distinction the collision was
hiding: a library thumbnail is *"Distinct from the live slot preview built in M2 — that one
renders a running instance, this one is a stored asset."* The rename says out loud what the
roadmap knew.

## Alternatives

**Rename the deck record instead.** Rejected on cost, and the asymmetry is total: the deck
record is written into session streams that exist on disk, and renaming it would stop those
replaying. Nothing has ever written the metadata record, and no file anywhere contains that
line.

**`preview_path`, keeping the stem.** Rejected. A suffix says *another version of one
concept*, which is what `param_decl` beside `param` legitimately is — a declaration beside
a value. A stored asset beside a live audition is not that; they are two things, and a name
that hides it invites the next collision.

**Leave the directory as `previews/`.** Rejected: a `thumbnail` record carrying a
`previews/…` path re-creates the split the rename removed. Nothing writes or reads the
directory, so the cost was one assertion.

## Consequences

`thumbnail` joins `origin`, `parent`, `perf` and `tag` as records the specification
describes and nothing yet writes. The names are now disjoint across all three vocabularies,
checked rather than asserted.
