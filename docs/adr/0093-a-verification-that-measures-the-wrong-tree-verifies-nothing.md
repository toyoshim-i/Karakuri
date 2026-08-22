---
id: 0093
title: A verification that measures the wrong tree verifies nothing
status: accepted
date: 2026-08-15
supersedes: []
superseded_by: []
principles: [0050]
tags: [process]
---

# A verification that measures the wrong tree verifies nothing

## Context

Asked to split a change into one commit per topic, I tried to stage it by thinning the hunks of a
`git diff` and applying them with `git apply --cached`.

**Removing a hunk invalidates the line numbers of the ones after it**, so `--unidiff-zero` inserted
lines at wrong positions — into an unrelated `match` — and a round trip through `stash` left
duplicated blocks and orphaned lines in `main.rs`. Recovering, I used `git cat-file … > file`, where
**the redirection creates the file before git can fail**, and zeroed five files.

Everything was recovered: the first stash's commit object was still in `git fsck --unreachable`, and
a stash made with `--include-untracked` keeps the untracked tree as its **third parent**.

## The part worth recording

**The first verification reported "140 tests pass" while measuring `HEAD`.** The patch application
had failed, so nothing under test was what I thought was under test — and I read only the last line
of the command block. **The tell was in the number: 140, where the suite has 147.**

This sits one layer below *watch a test fail before trusting it*
([P-0025](../principles/0025-a-test-meant-to-catch-something-is-run-against-the-defect.md)). The
tests were correct. **The thing they ran against was not.**

## Decision

**No patch surgery for splitting commits.** Build each intermediate state by removing known strings
from the complete version, **asserting each removal**, and **build the tree before `git add`** — so
what is verified is what is staged.

Each of the four commits was then checked out individually and its suite run green.

## Evidence

Session 2026-08-15T16:11Z, commits `6422f72`, `0b439ca`, `b12c696`, `3c5e166`. Standing rule:
[P-0050](../principles/0050-verify-the-thing-you-think-you-are-verifying.md).
