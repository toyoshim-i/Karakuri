---
id: 0103
title: A trailer missed fifteen times, because I never read my own commits
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: [0089]
tags: [process]
---

# A trailer missed fifteen times, because I never read my own commits

## Context

Asked whether the `Co-Authored-By` trailer had been dropped lately: **fifteen commits in a row.**

## Two causes, and the second is the useful one

**It is mine to write.** The trailer goes in the heredoc of `git commit -F -`; nothing in the
harness adds it. The previous session wrote it and this one did not, and **once dropped it stays
dropped**, because the last commit becomes the template for the next.

**And I never read my own commits back.** After each one I ran `git log --oneline -1` and looked at
a hash and a subject. **That is a view in which a trailer is structurally invisible** — fifteen
verifications, none of which could have caught it.

This is the same shape as
[ADR-0093](0093-a-verification-that-measures-the-wrong-tree-verifies-nothing.md): the check ran, and
it could not see the thing it was supposedly checking. There, the wrong tree; here, the wrong view.

## Decision

All fifteen rewritten, and **verified to be content-identical** before reporting it: tree hashes
compared before and after, each message compared body-for-body modulo the trailer, the suite green.
A backup ref was cut first, and the force-push was left to the operator, with
`--force-with-lease` recommended and the reason stated — no ssh key here, so `origin/main` could not
be fetched and its age was unknown.

## Consequences

- **Commits were being verified by content and not by message.** The content check was thorough; the
  message check did not exist. Both are part of the commit.

## Evidence

Session 2026-08-16T05:19Z, backup ref `backup-before-trailer-fix` at `15f090a`.
