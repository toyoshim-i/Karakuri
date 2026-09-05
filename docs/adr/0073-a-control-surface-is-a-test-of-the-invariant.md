---
id: 0073
title: A control surface is a test of the invariant, not a feature
status: accepted
date: 2026-08-10
supersedes: []
superseded_by: []
principles: [0090]
tags: [process, format, determinism]
---

# A control surface is a test of the invariant, not a feature

## Context

MIDI arrived as `--midi-in` and `--midi-map`, split the way `karakuri-audio` is: everything that
decides is a pure function over bytes and testable with no hardware, and the device side has nothing
in it that can be wrong.

But the reason to build it now was not the feature.

## Decision

**A control surface is the first real load on the invariant.** *The record stream is the only path
that mutates engine state*, and the reason M6's agents can be allowed to run is *an agent can do
nothing a human cannot do through the same interface*. A control surface is the **first non-keyboard
thing** to test that claim.

## What it found

**The claim was already false in one place.** Tapping the beat built a `Record::Tempo`, applied it, and
**threw it away**. The session's grid was moved by hand with nothing in the timeline saying so, so on
replay every parameter bound to `beats` runs at a different phase. **Two keys did the same thing.**
`push_tempo` is now one place, so a third cannot forget.

It became visible the moment a habit was turned into **a claim**. Nothing about the code changed; the
statement did, and then the exception was obvious.

## Consequences

- Review reported that `midi.rs` and `run_surface` had **zero tests and all six injected defects
  survived**. The seam between a knob and an action was pulled off the port into a `Router`, which made
  it testable.
- A masking test I wrote found **my own parser bug**: the mask was applied *after* the pattern match, so
  velocity `0x80` read as a press at velocity 0 rather than a release. A place where order matters.
- Out-of-range slot reports were written to stderr **per message** — one fader sweep is hundreds of
  messages, and that is a lock-taking write inside the frame.
- **A format hole this raised the price of, recorded rather than patched:** the session stream has no
  way to say **what the deck held**. `--replay` therefore builds a one-slot deck and reports and skips
  records naming other slots, so a four-slot surface map loses three quarters of its movement on
  replay. Written into the records section of the specification, noting that MIDI is what made it
  expensive.

## Evidence

Session 2026-08-10T23:22Z, commit `8ac6fa4`.
