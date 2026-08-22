---
id: 0058
title: Closed form is worth more for scrubbing than for priming
status: accepted
date: 2026-08-01
supersedes: []
superseded_by: []
principles: [0032]
tags: [ir, engine, determinism]
---

# Closed form is worth more for scrubbing than for priming

## Context

`closed_form` had been justified by priming: closed-form material needs no warming
([ADR-0028](0028-closed-form-and-accumulating-is-a-static-classification.md)). Transport showed that
was the smaller half — and the roadmap had in fact said so all along, *"Cold to Live with no warm-up,
**and seekable**"*, while my brief emphasised only the first clause.

## Decision

**The valuable half is scrubbable.** Closed form means the picture at any `t` is a pure function of
`t`, so forward at any rate, held, and **backwards** are all free.

And the first framing of accumulating material as *"rewind is impossible"* was wrong — corrected in
the same conversation. It is **priced, not impossible**:

| | Forward | Fast-forward | Backwards | Jump |
| --- | --- | --- | --- | --- |
| Closed form | free | free | free | free |
| Accumulating | free | proportional to extra steps | proportional to steps back to the target, **exact** | as above |

**Backwards means re-running from zero, and because this engine is bit-exact, the result is the same
state rather than an approximation.** Order-preserving compaction, `t` from an integer step count,
`tick` records, seed streams — everything paid for determinism buys a second thing here. The
roadmap's list of what determinism returns (undo, replay, A/B, session recording) **did not include
scrub**, and now does.

**And forward jump is priming.** The mechanism being built to advance a slot without drawing it *is*
the fast-forward implementation: shown it is an effect, hidden it is catching up.

So the operator's question changes from *can this slot be scrubbed* to **will this scrub arrive in
time** — which is the budget machinery already under construction. Re-running ten minutes is 36000
steps: affordable for light material, not for heavy.

**One condition, and it is the same requirement audio already had.** Re-running reproduces the state
only if the *inputs* over that span reproduce too — parameter history, binding history, tick history.
Re-running ten minutes during which audio moved a parameter needs **the recorded measurements**, not
the raw audio re-analysed. [ADR-0055](0055-a-measurement-enters-the-record-stream-raw-audio-does-not.md)
was argued from determinism in general; scrub is its concrete use.

## Consequences

- **The classifier is conservative and produces no diagnostic**, so it can only be wrong silently.
  Strict-wrong costs a warm-up that was not needed. **Permissive-wrong puts a slot on air unwarmed
  while telling the governor it needed no warming** — and once transport rests on it, a wrongly
  claimed closed form is a scrub producing garbage rather than a poor first second. Three
  disqualifiers: reading an emitted attribute anywhere (including laundered through a `let`), a
  `spawn` block, and `kill()`.
- **Spawning and closed form cannot coexist**, and not because of attributes — the `element` block may
  be perfectly pure. *Whether an element exists* is engine state accumulated over every frame, so
  jumping to `t = 30` yields one frame of newborns rather than thirty seconds of population.
- **Necessary for a seek, not sufficient.** A seek also needs everything else that is a function of
  time to be evaluable there, and an oscillator under tempo correction has a phase at a past `t` that
  depends on the correction history.
- Fast-forwarding accumulating material **runs extra steps rather than stretching `dt`**. Stretching
  is cheaper and coarsens the integration, so some material breaks. **Material that cannot be
  fast-forwarded should say so rather than quietly show something different.**

## Evidence

Session 2026-08-01T07:01Z–07:22Z. Standing rule:
[P-0032](../principles/0032-closed-form-means-scrubbable.md).
