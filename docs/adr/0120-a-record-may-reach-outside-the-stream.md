---
id: 0120
title: A record may reach outside the stream, and a replay is a sandbox
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: [0059, 0028]
tags: [format, determinism, store]
---

# A record may reach outside the stream, and a replay is a sandbox

## Context

Saving a Set from a running session — the missing half of the three-place design
([ADR-0088](0088-what-ships-what-you-saved-and-what-you-are-editing.md)) — raised whether the save
should be a session record.

**I first recommended it should not**, on the ground that provenance belongs to the artifact's
`origin` in its metadata file, which M4 already owes.

## The reconsideration

Asked for the benefits and costs, **two of them turned out to be undercounted**, and one decides it:

- **For M5's timeline, a save is the only landmark.** The history surface is meant to be walked, and
  a screen showing every fader move and no saves is one with **the only navigational mark missing.**
- **Provenance gains context.** `origin` can say *session S, beat 128* — a pointer into a stream where
  nothing marks the spot. Only the stream can order two saves in one session against the fader moves
  around them.

I said so plainly: **I recommended not recording because I had not counted M5's timeline as a
reader.**

The remaining objections — a first record a replay must ignore, a third reason in `is_set_state`'s
classification — were dissolved by the operator's framing rather than by argument:

> The question is what to do with a record whose effect lands outside the stream. If a later record
> depends on that effect, the effect must be recorded and replayed. For a save, drawing that line is
> the operator's responsibility. In general, record the side-effecting act too — and if a replay is a
> kind of sandbox, then skipping some effects is a natural thing for it to do.

## Decision

**Record it. A replay does not perform it, and says what it skipped.**

> **Some records have an effect outside the stream. A replay is a sandbox: it does not perform those
> effects, and it says what it skipped. Bringing outside state into a replay is the operator's
> responsibility.**

**And it does not break `is_set_state`'s classification.** That function asks *whose state is this*;
this adds an **orthogonal second question** — *does this reach outside the stream*. Not a third
reason; another question.

**The test was actually run rather than assumed.** No later record can depend on it today:
`setfile::load` has exactly one caller, and it runs once at startup. There is no path to load a Set
mid-session — no MCP tool, no key. **And the one thing that would break it is named**: M5's live
"load a Set into a slot". The escape is already there, because the system is content-addressed — a
live-load record naming a **hash** needs only the kind of artifact a replay already needs for its
head, so the dependency never has to exist.

## Consequences

- The one real cost: **a replay must say what it skipped.** Consistent with the session writer
  counting dropped batches and `--load-set` printing every note it could not honour — silent success
  being the failure this system dislikes most.
- A design change made while writing the brief: **`setfile::save` stops reading files.** A live save
  holds the hash of the version that **landed**, and must not read the path, because immediately after
  a rollback the scratch holds **a version that is not on screen**. So a `Node` carries a hash and
  `put_artifact` moves out to the caller — one address for one arithmetic — and as a side effect **the
  caller holding a hash gets the landed version while the caller holding a path gets the path, both
  automatically right.**

## Evidence

Session 2026-08-22T00:51Z–01:39Z. Standing rule:
[P-0059](../principles/0059-a-replay-is-a-sandbox.md).
