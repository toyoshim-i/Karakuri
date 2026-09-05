---
id: 0084
title: A procedure change goes into the record stream
status: accepted
date: 2026-08-12
supersedes: []
superseded_by: []
principles: [0090]
tags: [format, determinism, mcp]
---

# A procedure change goes into the record stream

## Context

While `--record-session` runs, **a change of procedure is recorded nowhere.** Material is written
once by `session_head` before the first frame and never again.

```
10:00  recording starts — slot 0 is drift_shell
10:10  a model rewrites slot 0 through MCP; the picture changes
10:20  stop

--replay  →  the whole thing drawn with 10:00's drift_shell
```

**With no warning.** Quietly succeeding while saving something else is the failure mode `--load-set`
refuses on principle.

Two things made it heavier than the day before, and the second is the larger one.

**MCP turned the exception into the norm.** Hand-editing while recording is rare, but **Vibe Live
Coding *is* changing procedures while running** — the feature's main activity became the one thing
not recorded.

**And M6's safety rests on it.** The roadmap requires the record stream to be the sole mutation path
so that an agent is structurally incapable of doing anything a human could not. An agent's main
activity is **generating material**, so if material change does not go through a record, that
invariant **already fails for the part that matters most to an agent.** Discovering it inside M6
means an unobservable autonomous system, which is exactly what putting M5 first was meant to avoid.

## Decision

**Record it.** A record pointing at the source lands the moment a swap does. The store already keeps
`.kir` files content-addressed and immutable, and `Record::Src` already exists; the Set format has
precedent for both a hash reference and an embedded source.

The reason is not "replay becomes correct". It is that **M6 does not stand without it**, and adding
it later is retrofitting into decisions already made. It is also simply cheaper now: rebuilding one
Set today, four slots of agent-driven generation later.

## Alternatives rejected

- **Write it down as out of scope.** Zero work, and it returns at M6 — with the feature MCP mainly
  performs left unrecorded.
- **Refuse `--record-session` together with `--mcp`.** Cheap, and in this project's style of
  refusing rather than quietly doing the wrong thing. But **the set you most want to record becomes
  the one you cannot** — the session made with a model is precisely the one worth keeping. It points
  the opposite way from the goal.

The question that settled it was not technical: **do you actually want to replay a Vibe Live Coding
session** — to render it later, to see what happened, to audit what the model did? That is not a
judgement to guess at.

## Consequences

- **The store write happens on the worker thread**, where the watcher already reads and checks the
  source. The landing frame does a hash-map lookup and pushes two records; **no file I/O reaches the
  render thread.**
- Verified end to end: a model desaturated `soft_points.kir` mid-recording, and the replay's mean RGB
  goes chromatic → neutral across the frame the record lands on.

## Evidence

Session 2026-08-12T16:03Z–2026-08-14T05:08Z, commit `85f9b24`.
