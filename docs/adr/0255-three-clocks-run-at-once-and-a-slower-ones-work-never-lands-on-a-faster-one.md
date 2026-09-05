---
id: 0255
title: Three clocks run at once, and a slower one's work never lands on a faster one
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0092]
tags: [engine, architecture, determinism]
---

# Three clocks run at once, and a slower one's work never lands on a faster one

## Context

P-0069 — *The three clocks never collapse into each other* — said outright that it had no record:
*"part of the design skeleton rather than a decision anyone took against an alternative … written
down as the single most load-bearing idea in the design before there was an engine to test it
against."* It is retiring into
[P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md), which carries one clause of
it — a beat-clock event chooses among material already built rather than computing it — and not the
table or what the table is for.

**Records name it in the other direction, and none of them decides it.**
[ADR-0222](0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md) asks it whether a step grid
is a fourth clock and answers *no, it is the beat clock subdivided*.
[ADR-0232](0232-a-control-is-thrown-because-the-request-is-asynchronous-and-a-curve-is-a-helper-on-the-control-map.md)
heads a section with it — *the frame that makes the helper obviously right* — and quotes the table
twice. Both are records that **apply** the rule to a case. Neither states it, and neither
carries what it was chosen against.

## Decision

**Three timescales run at once, and what may happen on each is fixed.**

| Clock | Period | What happens on it |
| --- | --- | --- |
| Frame | 8–16 ms | GPU execution and parameter evaluation. Nothing else |
| Beat / bar | 0.5–4 s | Variant switching, parameter morphs, transitions — **selection** among options already prepared |
| Generation | seconds to minutes | Writing a procedure, checking it, compiling its shaders. A background worker |

**What it rules out is doing a slower clock's work on a faster one.** Allocating a buffer or
compiling a shader on the frame path, which is
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) from the other end. A beat-clock
event that *computes* the material it is switching to rather than choosing among material already
built — which is why a hot swap arrives from a worker already compiled. And, when a generator
arrives, any call to it on a path a frame waits for: an agent reacting to music chooses from a pool
it prepared earlier and queues generation for what it expects to need next.

**It is what makes the middle clock statable at all**, and that is the half worth keeping when the
rest sounds like a diagram. A transition schedules a move at a named instant on the beat grid and
produces its value from `beats` alone, so the same records give the same fade frame for frame on a
machine running at a different rate, and a tempo correction mid-fade is correct rather than a
glitch. **That property survives only while nothing on the beat clock waits on the generation
clock**: one call into a generator from inside a transition and the fade is a function of how long a
compile took.

## Alternatives rejected

**Two clocks — a frame and everything else.** The tempting simplification, and it is the one the
design would drift into on its own, because the generation clock has nothing on it yet. It loses on
what the middle clock is *for*: selection among prepared options is a statable, replayable operation
and generation is not, so collapsing them makes a transition's value depend on a compile. The third
row is written before there is anything on it precisely so that the first thing put there does not
get put on the beat clock instead.

**One scheduler that runs everything and prioritises.** Turns *what may happen here* into *what
happened to fit*, which is a property of the machine rather than of the design, and it puts the
frame path on the critical path of every decision. The engine's arrangement is the answer instead —
a build worker that hands over a finished Set, a swap installed at a frame boundary, a transition
that is a pure function of `beats`.

**Guarantee timing on the render path so a slower clock's work can land there safely.** Argued and
rejected in full in ADR-0232: guaranteeing timing on the frame path means becoming synchronous, and
the frame clock's charter is *GPU execution and parameter evaluation. Nothing else.*

## Consequences

- **The period bands are descriptive, and one case has already tested that.** ADR-0222 records that
  a 1/8 step at 128 BPM is 234 ms against a stated beat band of 0.5–4 s, so the table as written
  does not say a subdivision is still the same clock. It is: what fixes a clock is *what may happen
  on it*, not its period, and the band is there to say what the rows mean rather than to be the
  test.
- **Two of the three are built and the third is half built.** The build worker compiles off the
  render thread and hands over a finished Set
  ([ADR-0033](0033-freeing-on-the-render-thread-is-the-same-invariant-as-allocating.md)); nothing
  generates a procedure yet. This record is most of what the third one will be held to.
- **`transition.rs`, `swap.rs` and `deck.rs` are where it is enforced today**, and none of them
  states it — which is the reason the table was worth writing down before there was an engine and
  is the reason it is worth a record now that there is one.
