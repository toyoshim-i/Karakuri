---
id: 0036
title: The first M2 slice is two Sets, mixed, through a tone mapper
status: accepted
date: 2026-07-30
supersedes: []
superseded_by: []
principles: []
tags: [process, engine]
---

# The first M2 slice is two Sets, mixed, through a tone mapper

## Context

M2's goal is surviving an hour-long set. Its list runs to twenty items. Building them together
is the opposite of how M1 was built.

## Decision

**Two Sets on screen at once, mixed, through a tone mapper.**

Chosen for being the smallest thing that is **not M1**: `VideoSource` becomes real, a deck
exists (at two slots), and L5 exists. And it needs **no audio, no MIDI, no Link, no governor** —
each of which is a milestone's worth of work that this slice can prove nothing about.

It is also the generation question in disguise. The examples all carry `exposure` well below
1.0 because the artifact's own parameter is standing in for the missing tone mapper, which is
why all three LLM-generated procedures looked wrong. Tone mapping stops the generator having to
guess a value it cannot know. See
[ADR-0019](0019-exposure-is-three-things-and-none-stands-in-for-another.md).

Rejected alternative: **one Set reacting to sound**. A legitimate first slice, and which a VJ
wants first is not a question the implementer can answer.

## Consequences

- Residency ships as **Live and Allocated only**. Priming — running hidden at reduced rate —
  means nothing without a governor to allocate the slack, so it waits. But **"Allocated keeps
  its state" is built now**, because hot-swap rollback already depends on parking a Set unstepped
  and having it resume where it stopped.
- Two debts were repaid inside the slice rather than beside it: the frame guard
  ([ADR-0034](0034-the-frame-guard-owns-the-encoder.md)), which the roadmap had scheduled for
  exactly this moment, and the per-frame allocations in `prepare`.
- The brief explicitly told the implementer that `swap.rs`'s existing comment claims more than
  it delivers and not to repeat it — the mistake found the previous round, carried forward as an
  instruction rather than a memory.

## Evidence

Session 2026-07-30T16:32Z–16:42Z.
