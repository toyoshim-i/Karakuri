---
id: 0109
title: Format the workspace, and split the gate
status: superseded
date: 2026-08-19
supersedes: []
superseded_by: [0114]
principles: [0089]
tags: [process]
---

# Format the workspace, and split the gate

## Context

`cargo fmt --check` produced a diff in nearly every file, with no `rustfmt.toml` and no CI. I had
read that as a deliberate choice not to follow rustfmt and left it alone — then was told to format
it and enforce it.

## Decision

**Format everything, and add no `rustfmt.toml`.** Measured: **no configuration makes it a no-op** —
6493 lines move by default, 5397 with the closest setting, and `use_small_heuristics = "Max"`
**increases** it by rejoining line breaks that were put there on purpose. Between one arbitrary style
and another, choose **what everyone's `cargo fmt` emits with no argument**. Hand-wrapped prose
survives, because `wrap_comments` is off by default.

**A retraction:** I had implied a configuration could make it a no-op. That was a **measurement
error** — `cargo fmt --check -- --config` printed nothing and I read the silence as zero diff.

**The gate is split in two:**

| | Contents | Why there |
| --- | --- | --- |
| `pre-commit` | `rustfmt --check` on the staged `.rs` | One second, so it can sit on every commit |
| `pre-push` | fmt, clippy `-D warnings`, the whole suite | Minutes. **A gate that takes minutes gets `--no-verify`d and stops being a gate** |

`pre-commit` reads the **staged content**, not the working tree: otherwise an unfinished edit on disk
blocks a good commit, or a bad commit fixed after staging goes through.

## The hook first failed in the worst possible way — it passed

`rustfmt --check` **reading from stdin prints its diff and still exits 0** — different from what it
does with a filename. The hook read that success and let unformatted files straight through. It now
judges by **output rather than exit code**.

**A gate's test is something that must not pass**, and this only surfaced by actually committing an
unformatted file. Both directions are now checked.

## Consequences

- One line cannot live in the repository: `git config core.hooksPath .githooks`, once per clone.
- A slip worth recording: a temporary verification commit was cleaned up with `git reset --hard`,
  **taking the formatting result in the working tree with it**. `cargo fmt` is deterministic so
  nothing was lost, but `--soft` was what the situation needed.

## Superseded

The suite came off `pre-push` the next day in
[ADR-0114](0114-tests-run-when-somebody-asks-not-when-git-does.md). The split survives and the
formatting half is unchanged; what moved is that a branch push runs nothing and a **tag** push runs
everything. This record's own rule — *a gate that takes minutes stops being a gate* — was right and
pointed one step further than it went.

## Evidence

Session 2026-08-19T10:31Z–11:01Z. Standing rule: none in
`docs/principles/`. P-0089 stated it and was retired on 2026-09-05 — it is a testing discipline
rather than a property of the instrument, so it is stated in `docs/contributing.md` §3, *A test is watched to fail before it is kept*.
