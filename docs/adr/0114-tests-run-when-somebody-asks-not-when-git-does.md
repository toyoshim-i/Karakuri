---
id: 0114
title: Tests run when somebody asks, not when git does
status: accepted
date: 2026-08-20
supersedes: [0109]
superseded_by: []
principles: [0057, 0054]
tags: [process]
---

# Tests run when somebody asks, not when git does

## Context

A push sat for five and a half minutes running the full suite — **on a tree that had produced
`pre-push: ok` at the end of the previous session, with a clean working tree.** The same checks, on
the same bytes.

Two defects underneath: the hook **reads neither its arguments nor stdin**, so it does not know what
is being pushed and cannot tell "the tip that just passed" from anything else; and it checks the
**working tree** rather than the commits being pushed, so a dirty tree means refusing a push on the
strength of code that is not being pushed.

I proposed making the gate cheaper. Three reframings followed, each larger than the last.

## The reframings

**"A push is not a debugging step. It is closer to a deploy."** My cheaper-gate options missed the
target: however cheap, **the structure still settles quality at the moment of pushing**, and if that
is when you learn, it is already too late.

**"It is not overhead — there are simply a lot of tests. Run what is needed, when it is needed, and
never twice. `pre-*` may be the wrong place entirely."** That is the decision:
**a git lifecycle event does not know when "needed" is.**

## Decision

**The tests come off the hooks.**

- `pre-commit` — `rustfmt --check` on staged Rust, and nothing else. One second.
- `pre-push` — reads the refs on stdin; **the whole suite on a tag push and on nothing else**. A tag
  is the deploy; a branch push is part of working.
- In between, **whoever makes a change names the smallest suite that answers it** and runs that; the
  workspace runs once at a boundary.

**And I was manufacturing the redundant runs.** Three agent briefs each ordered
`fmt` + `clippy --workspace` + `test --workspace` before reporting: five and a half minutes, three
times, plus my own run, plus the push — **over twenty-five minutes of the same 896 tests, of which
one was meaningful.** An agent's question is *did my change break something*, and the minimal answer
is the named suite. Briefs now say so.

## Consequences

- The commit message records **the whole in-and-out arc** — a `TESTED=1` stamp had been added an hour
  earlier and was itself a sticking plaster over the same problem, and "a rough check beats none" won
  once and then lost. Hidden, someone reinvents it.
- Supersedes [ADR-0109](0109-format-the-workspace-and-split-the-gate.md), which put the suite on every
  push. Its own rule — *a gate that takes minutes stops being a gate* — was right and pointed one step
  further than it went.

## Evidence

Session 2026-08-20T16:27Z–18:07Z, commit `867973b`. Standing rule:
[P-0057](../principles/0057-run-what-the-question-needs-when-it-is-asked.md).
