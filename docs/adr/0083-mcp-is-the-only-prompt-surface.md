---
id: 0083
title: MCP is the only prompt surface
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0043, 0093]
tags: [process, mcp]
---

# MCP is the only prompt surface

## Context

The reason a refined GUI is wanted is a close human–AI loop — *the next track is hard techno, so
generate a shader to match and stage it.* The original intent was **a prompt box in the GUI calling a
remote model**; MCP came up as the simpler route.

MCP appears nowhere in the roadmap, and yet M6's first requirement is already about it: *the record
stream must be the sole mutation path, so that an agent is structurally incapable of doing anything a
human could not do through the same interface.* **MCP is that requirement arriving from outside the
process**, and the invariant is already standing — three holes in it were closed this same week.

## Decision

**MCP is the prompt surface, and the GUI does not get its own.**

- **Karakuri stops being an AI application.** A prompt box makes API keys, model selection, prompt
  assembly, streaming, retries and rate limits Karakuri's problem, and every model change becomes a
  Karakuri change. As an MCP server it holds none of them: **no network and no secrets enter the
  render process.** The same shape as putting the GPL in another process.
- **It becomes a conversation with state rather than one shot.** The model reads the current set, sees
  what is loaded, proposes, is told it is too loud, and fixes it. **A prompt box cannot do this
  structurally**, because there is no read-back.
- **M6 shrinks.** The LLM call leaves the process; what remains is receive IR text, validate, compile,
  stage. And MCP is the staging lane's *first producer*, which M5 had already predicted would be the
  operator's own regeneration rather than an agent. Autonomous agents still need an in-process path —
  MCP replaces **operator-driven** generation only.

**Three tools: `read`, `write`, `outcome`.** `revert` was proposed and then dropped: the client reads
before it writes, so **the previous source is already in the conversation** and "put it back" works.
Undo becomes context rather than a tool — a property only a conversational client has, and one a GUI
prompt box could not have had.

**The return value that decides whether this works at all is the diagnostic.** `Err("failed")` versus
the rendered `karakuri-ir` diagnostic is the whole difference, because this IR was designed so a model
can write it from the specification alone; without the compiler's own words, half of that is thrown
away.

**Tools speak in slots and content, never in paths**, because a remote client cannot see the server's
filesystem — and that is cleaner locally too. **The listener binds to localhost**; a venue network is
shared, and a render machine with an open port that rewrites the visuals would sit on it. Exposure is
a separate explicit act, so that it is a decision rather than an accident. (SSH covers the remote case,
which also removes the objection that a second chat window is awkward at a show: the render machine is
loaded and probably out of sight, and the chat client is on the machine you are sitting at.)

## Consequences

- **The safety device already exists.** Hot swap compiles on a worker, swaps at a frame boundary,
  measures thirty frames and discards over budget, restoring the parked `t`. Built for hand editing, it
  works unchanged against a model.
- Two tiers should be visible through MCP, because the model can then choose the cheap one: a `param`
  write lands immediately with no recompile, while a rewrite needs a swap and a trial.
- **Resources beat links.** A resource flows over the connection that is already open; a link needs the
  client to fetch and the URL to be reachable, which an SSH tunnel makes awkward. Two of them: the
  **prose specification**, and a **generated vocabulary** built from `Builtin::ALL` and `signature()` —
  the checker's own table, so it **cannot go stale**, because the moment it does, compilation fails.
  Prose drifts from code; this project was cut by that three times in one week.
- Should the GUI ever grow a prompt surface, it **calls the same tools** — the rule already imposed on
  keys and on MIDI.
- **It reorders the plan.** M5 exists because *an autonomous system that cannot be observed and
  overridden is not usable on stage*. MCP hands a chat model the whole system **before that observation
  surface exists**, which is precisely what M5's rationale warns about. And the read-back MCP needs is
  the same structured current state the overlay needs — so one investment serves both.

## Evidence

Session 2026-08-11T17:12Z–17:25Z. Standing rules:
[P-0043](../principles/0043-nothing-external-enters-the-render-process.md),
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md).
