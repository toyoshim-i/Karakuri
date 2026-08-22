---
id: 0035
title: `.claude/` is removed from history while there is no remote
status: accepted
date: 2026-07-30
supersedes: []
superseded_by: []
principles: []
tags: [process]
---

# `.claude/` is removed from history while there is no remote

## Context

`.claude/settings.json` had been tracked since the second commit. It records tool permissions
granted during a session — local environment state, not a project artifact — and some entries
were broad (`Bash(python3 -)`, an `xargs sed -i` rename).

## Decision

Remove it from **history**, not just from the tip: `git filter-branch` over all nineteen
commits, and `/.claude/` added to `.gitignore`. The file stays on disk, untracked, so the tool
configuration keeps working.

**The timing is the decision.** Every commit hash changed. With no remote configured this
inconveniences nobody; once the history has been shared it collides with everyone else's.
**The cheapest moment to rewrite history is before there is a remote**, and that moment is
identifiable in advance rather than in hindsight.

## Consequences

- Stated plainly at the time and worth repeating: the backup ref and the reflog were dropped and
  `gc` run, so **the rewrite cannot be undone**. That was the intent, and saying so is part of
  doing it.
- Verified before the backup was discarded, not after: 201 tests passing, 81 tracked files, clean
  tree.
- A related habit that this reinforces: permission grants accumulated during a session are not
  ridden into a correctness commit. They go separately or not at all.

## Evidence

Session 2026-07-30T12:57Z–14:48Z.
