---
id: 0129
title: A vendor file is ignored, and removed from history
status: accepted
date: 2026-08-22
supersedes: [0127]
superseded_by: []
principles: [0043, 0006]
tags: [process]
---

# A vendor file is ignored, and removed from history

## Context

[ADR-0127](0127-the-repository-names-no-tool.md) settled that the repository names no tool, and
rejected a `.gitignore` entry on the ground that it would write a vendor name into the repository
anyway. **The operator overruled that clause**: ignoring one is fine, committing one is not — and the
file that had been committed should come out of the history.

## Decision

**The standing decision is unchanged: the repository names no tool**, and the rule about recording
decisions stays in `docs/contributing.md` §4, which names none.

Two clauses change.

**A `.gitignore` entry is right, and my objection was the wrong shape.** Ignoring a name is not the
same act as depending on it — `/.claude/` has been there since 2026-07-30 for exactly this reason, put
there by the same decision that erased it from the history. An ignore rule is how a repository says
*this does not belong here*, which is the position, stated in the one place git will act on. Three
names are listed rather than one, because the argument is about the class and not about which vendor
happened to arrive first.

**And it comes out of the history, not merely off the tip.** The precedent is the same record:
`.claude/` was removed from all nineteen commits rather than untracked, because a file that is in the
history is in the repository.

## How, given the constraints of the moment

Both conditions that made ADR-0035's rewrite cheap held again, and one did not.

- **Nothing had been pushed.** `origin/main` was twenty-two commits behind, so no other copy of the
  history existed and no force-push was needed. The cheapest moment is still before it is shared.
- **But the working tree was not clean** — another session was mid-change in six files — so
  `filter-branch` was unusable, since it refuses to run and would have needed their work stashed.

So the four affected commits were rebuilt with plumbing — `read-tree` into a **temporary index**,
`git rm --cached`, `write-tree`, `commit-tree` — which never touches the working tree or the real
index. The resulting tip's tree was verified **identical** to the old one before the branch was moved,
so nothing in the other session's working state could shift underneath it.

**One commit disappeared and one message changed.** The commit whose only content was deleting the
file became empty and was dropped. The commit that added it described it in its message, and a message
describing a file that no longer exists is the same defect as a comment describing replaced behaviour
([P-0023](../principles/0023-a-document-that-describes-replaced-behaviour-is-worse-than-none.md)) — so
that paragraph was rewritten rather than left.

## What is not done, and why

**The old objects are unreachable but not yet pruned.** The backup ref is deleted and nothing points at
them, so they are gone from every history git will show. They leave the object store at the next `gc`.

`gc --prune=now` was **not** run, deliberately: the other session recovered five zeroed files from
`git fsck --unreachable` earlier today, and pruning now would remove that safety net while they are
still working. It costs nothing to wait, and nothing has been pushed, so no unwanted copy can escape in
the meantime.

## Evidence

`a8ca033` (added), `f4248f9` (removed), rewritten to `b857388` / `b65ad5c` / `619f396` on 2026-08-22.
