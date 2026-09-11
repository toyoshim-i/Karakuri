---
id: 0345
title: A file that crosses 1000 lines gets a nudge, not a gate
status: accepted
date: 2026-09-11
supersedes: []
superseded_by: []
principles: []
tags: [docs, process]
---

# A file that crosses 1000 lines gets a nudge, not a gate

## Context

`docs/refactoring.md`'s smell #1 ("Giant Monolithic Files") named `crates/karakuri/src/main.rs`
at 33,165 lines and `crates/karakuri-console/src/view.rs` at 24,258 lines as the sharpest cost in
this codebase — "extreme maintainability bottleneck; poor IDE performance; inhibits modular
testing." Acting on it (this session) meant splitting `view.rs` into
`view/{mod,mixer,library,transport,inspector,program}.rs`, pulling `session.rs`/`keymap.rs` out of
`main.rs`, and splitting `karakuri-environment`'s `mcp.rs` into its own crate once it reached
10,678 lines — three separate multi-hour efforts, each undertaken only after the file had already
grown large enough to show up in an audit.

**Nothing caught any of these on the way up.** `mcp.rs` grew by 873 lines between the report's
snapshot and this session's read of it — nobody was told at the moment it happened, because
nothing was watching. A file crossing a size worth noticing is cheap information exactly once: the
commit that pushes it over is short, easy to read, and the person writing it has full context for
why the growth was needed right there. The same fact, noticed later by an auditor with none of
that context, costs a read-the-whole-file investigation and, as this session found three times
over, a multi-file refactor.

## Decision

**A pre-commit check warns, once, the commit a `.rs` file's staged content first crosses 1000
lines — and does not fail the commit.** `.githooks/pre-commit` compares each staged Rust file's
line count against its own `HEAD` version: if the staged version is over 1000 lines and the
version at `HEAD` was not (or the file is new), it prints the file and its new line count and
points at this record. Every other commit touching an already-large file — including the ones that
made `mcp.rs` grow — prints nothing, because the fact was already known the day it first crossed.

**1000 is a size to notice, not a limit to enforce.** Several files in this workspace are correctly
large *after* being split as far as their coupling allows — `karakuri-mcp/src/lib.rs` (10,700
lines, one crate's whole surface, ADR'd as a one-directional consumer with nowhere further to
carve it), `crates/karakuri-console/src/view/mod.rs` (7,493 lines, the shared dispatcher and the
four smaller bays `docs/refactoring.md`'s P2 didn't name). Warning on every commit to those would
train the same reflex `docs/contributing.md` §2 already warns about for a different gate: "a gate
costing minutes on every commit gets `--no-verify`'d and is then not a gate." Warning once, at the
crossing, costs nothing on every other commit and says something true exactly when it becomes
true.

**Never a gate.** This check cannot fail a commit and never will without a separate record
deciding that on purpose. The value here is a data point delivered at the one moment it's cheapest
to read, not a rule anybody is made to satisfy before landing work — `docs/contributing.md`'s own
"Working style" already says a change should ship as a vertical slice, not be held up reorganizing
a file nobody asked to reorganize today.

## Alternatives rejected

- **Fail the commit.** Rejected for the reason above: a same-day, one-file growth is not evidence
  a design is wrong, and blocking it would either force an unrelated, unreviewed refactor into an
  unrelated commit or get the hook disabled. `docs/contributing.md`'s own gates (`fmt` on commit,
  the full suite on a tag) are sized to what they cost and what they're sure of; this isn't sure of
  anything by itself.
- **Warn on every commit that touches a file already over 1000 lines**, not only the crossing.
  Rejected as the noisiest option and the one most likely to be trained out — three files in this
  workspace are already, correctly, over the line, and a warning on every future one-line fix to
  any of them teaches "ignore this printout" faster than it teaches anything about file size.
- **Apply only to `src/`, exempting `tests/`.** Considered and rejected: a test file is exactly as
  capable of becoming unreadable as production code, and this session found and split one — the
  `key_column`/`focus_keys`/`press_handler` cluster inside `main.rs`'s own test modules carried
  real, load-bearing complexity worth the same notice a production file gets. No exception is
  carved before a file in `tests/` demonstrates it needs one.
- **A principle instead of an ADR.** `docs/contributing.md` §4's own rule: a principle is
  something that "decides a question it does not itself mention." A line count is a number a
  script checks, not a question anything else turns on — it belongs here, the way the
  formatting/lint/test gates already do, not in `docs/principles/`.

## Consequences

- `.githooks/pre-commit` gains one more check, still fmt-cost cheap (`wc -l` on a `git show`
  output already being read for the fmt check), still non-blocking, and still judged by output
  the way the fmt check already is ("A gate is judged by its output, not by its exit code" —
  `docs/contributing.md` §3).
- `docs/contributing.md` §1 and §2 both point here, so the rule is visible from the entry document
  and from the hooks section describing what actually runs.
- Nothing here obligates anyone to act on the warning the day it prints. A file that crosses 1000
  lines and stays there because further splitting genuinely isn't warranted (`karakuri-mcp`'s own
  case) is not a violation of anything; the printout already happened once and nothing repeats it.

## Evidence

Session 2026-09-11, at the end of a session that split `view.rs`, `main.rs`, and `mcp.rs` after
each had already grown past the point a one-line warning would have made cheap to reconsider —
proposed by the user directly rather than found in an audit.
