---
id: 0074
title: A plugin boundary is drawn by the deterministic path, not by the platform
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0085, 0092]
tags: [process, engine]
---

# A plugin boundary is drawn by the deterministic path, not by the platform

## Context

Two remaining M2 items each wanted a second toolchain in a workspace that is cleanly closed to
Rust: Syphon needs Objective-C interop, Ableton Link needs cmake and a C++ compiler. The proposal
was to define an I/O plugin interface and hang both off it.

## Decision

The line is agreed, and **it is not the line it looked like.**

**Two interfaces, not one**, and both are legal for the same reason — **neither is on the
deterministic path.**

- An **output** plugin is downstream of everything. It consumes the composited frame and **writes
  no record**, so a session replays identically whether one was attached.
- An **input** plugin produces **records** — the same records a key press produces. Link's
  contribution is a tempo, and `tempo` already goes through a record every frame, so a session
  recorded with Link replays without it.

A generator, a blend mode or a tone map operator **could not be a plugin on these terms**, whatever
toolchain it wanted, because what they do reaches the pixels a replay has to reproduce.

**And the platform is not the test. Link is not platform-dependent** — it runs everywhere and is
outside for its toolchain alone. That distinction survives into what the host says when something
is missing: Syphon absent on Windows means **the feature does not exist there**; Link absent means
**nobody built that plugin**.

## Consequences

- One half already existed: `karakuri-midi` plus the CLI's `Router` and `Surface`. A MIDI action
  ends in the method a key press ends in, so the input interface is **extracted from something that
  runs** rather than invented — the only reason to specify it before a second instance exists.
- **Do not build the SDK abstractly.** An ABI decided by imagination is always wrong: get one
  plugin through, then extract the specification. But because the second and third are already
  known (Spout, then NDI), surface-kind negotiation goes in from the start.

## Evidence

Session 2026-08-11T08:50Z–09:03Z, `docs/plugins.md`. Standing rules:
[P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md),
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md).
