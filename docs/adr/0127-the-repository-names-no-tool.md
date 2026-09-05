---
id: 0127
title: The repository names no tool
status: superseded
date: 2026-08-22
supersedes: []
superseded_by: [0129]
principles: [0043, 0085]
tags: [process]
---

# The repository names no tool

## Context

Asked whether anything told a new session to keep records, the answer was no — the registry was
readable and nothing said to add to it. The rule went into `docs/contributing.md` §4, which is right
and which nothing here changes.

**A `CLAUDE.md` went in beside it**, three paragraphs of pointers, on the argument that a session sees
it without being told to look. It was committed in `a8ca033` and lived for twenty minutes.

## Decision

**The repository names no tool.** No `CLAUDE.md`, no vendor directory, no file that exists because one
particular program reads it.

**The answer was already in this registry.**
[ADR-0035](0035-claude-is-removed-from-history-while-there-is-no-remote.md) did not merely untrack
`.claude/` — it removed it from **all nineteen commits**, dropped the backup ref and the reflog, and
ran `gc`, making the rewrite deliberately irreversible. That is not tidiness. It is a position, and
adding a vendor file twenty minutes after writing *check whether it lost an argument already* is that
check not being run.

## Alternatives rejected

- **A pointer-only `CLAUDE.md`.** It buys **reach** — a session sees the rule without being told — and
  reach is a real benefit. It is not enough. It makes one vendor's convention a fact about a
  repository whose stated position is that the workspace closes over `cargo`
  ([P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)) and that nothing external
  enters ([P-0043](../principles/0043-nothing-external-enters-the-render-process.md)). And it decays
  in an obvious direction: the second tool wants its own file, and the argument that admitted the
  first admits every one after it.
- **A `.gitignore` entry so a local one cannot be committed.** Cheap, and it still writes a vendor
  name into the repository. Left undone deliberately; this record is the guard.

## Consequences

- **Acting on the rule unprompted is not something a repository can provide.** The repository states
  the rule; making a particular tool read it is that tool's configuration, and it belongs outside —
  in the operator's own config, or in an untracked local file. That boundary is the same one
  [ADR-0074](0074-a-plugin-boundary-is-drawn-by-the-deterministic-path.md) draws for plugins: the
  thing that is specific to one environment lives outside the thing that is not.
- **This is the §4 trigger's first real case, and it caught it late** — after the file was committed
  rather than before. The rule is written; running it on one's own change is the part that has to
  become habit.

## Evidence

`CLAUDE.md` created in `a8ca033` and removed in this record's commit. Precedent: ADR-0035, 2026-07-30.
