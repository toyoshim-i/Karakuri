---
id: 0080
title: The GPL boundary is a process, and the protocol is generic
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0092]
tags: [process, legal]
---

# The GPL boundary is a process, and the protocol is generic

## Context

Ableton Link is **GPL-2.0-or-later**; this workspace is MIT. The question posed was whether keeping
them apart means a separate repository, a separate process, and a separate distribution channel.

## Decision

Three separations, and **they do not weigh the same.**

**A separate process is the only essential one.** The GPL's unit is the work, and two programs
conversing over a pipe or a socket are ordinarily separate works — the FSF's own FAQ draws exactly
that line between pipes, sockets and command-line arguments on one side and dynamic linking in one
address space on the other. If this fails, neither of the others rescues it.

**A separate repository is not legally decisive but is needed in practice**, and for a specific
reason: **the helper itself becomes GPL-2.0-or-later**, because it links Link. So the separate
repository is not tidiness — it is *where the GPL code lives*, and it keeps GPL sources out of an MIT
repository.

**A separate distribution channel is not required.** Mere aggregation is explicitly permitted, and
shipping a GPL work alongside, with its conditions met, imposes nothing on the other side. It is
still recommended, because the manifest-and-release design in
[ADR-0076](0076-commit-a-manifest-not-a-binary.md) already answers it and removes the source-offer
bookkeeping.

## The constraint that makes the position strongest

**Make the protocol a tempo-source protocol, not a Link protocol.**

The FSF's criterion is how *intimate* the communication is. A general interface defined by Karakuri,
which a GPL program happens to implement, is the strongest possible shape — and here it is true
rather than arranged:

- Karakuri runs **completely without the helper** (there is a tracker and `--bpm`). The dependency is
  one-way and optional.
- **A second implementation already exists** — the beat tracker is another source in the same role.
- Anyone can write another: MIDI clock, LTC, a hand tap.

So this is **not a device for evading the GPL; it is plainly the better design**, and that is the
part that matters.

## Consequences

- The same boundary now has **two independent justifications**: `docs/plugins.md` already required an
  out-of-process boundary for stability. Two reasons for one constraint is worth recording, because
  losing one does not lose the constraint.
- The record vocabulary was found short: `Record::Tempo` carries `{ bpm, shift, confidence }` and
  **`shift` is relative**, while Link's whole point is an **absolute beat number** — which beat is
  the downbeat. A relative shift cannot carry a shared bar line.
- What is passed is an **anchor, not a sample**: `beat_at_time` is a function of time, so "the beat
  is 42.7 now" is stale on arrival. `Oscillator` was already built from anchors.

## Evidence

Session 2026-08-11T13:48Z, 2026-08-11T14:32Z.
