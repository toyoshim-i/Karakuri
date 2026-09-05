---
id: 0222
title: A sequencer lane is a fifth route and not a binding
status: accepted
date: 2026-08-29
supersedes: []
superseded_by: []
principles: [0090, 0092, 0094]
tags: [architecture, signal, ui]
---

# A sequencer lane is a fifth route and not a binding

## Context

The Sequencer bay is the last of the console's nine regions with nothing drawn, and
`docs/roadmap.md` carried its design under **Settled decisions**:

> A sequencer is **one more name on that bus**, stepping on the oscillator that already drives
> `beat`. **A lane is a binding**, so a lane drives a Set parameter as readily as a deck fader.

Surveying the bay before drawing it found that both halves are false, and that the repository
already held the reasons — written down about something else, before this was proposed.

**A bus name cannot be a lane.** `karakuri-signal`'s `SynthesizedBus` holds one field, `oscillator:
&Oscillator`, and its own header states the contract: *every value on it is a pure function of the
local oscillator's `t` and `bpm` — nothing else*, and *nothing seeded lives here*. **A pattern is
authored state**, and the bus is stateless by construction. Worse, it is keyed by **name alone** and
`Signals` is one per session, so two lanes sourced from `seq 1` with different targets would sample
the same name in the same frame and get the same value — which is not a sequencer. Carrying the lane
in the name (`seq1:A`) is the four-fields-in-one-`&str` problem that file already refused for
`noise`, in its own words.

**And three of the mock's four lanes cannot be bindings at all.** A `Binding`'s target is a `param`
of a procedure inside a Set — nothing else. The lanes the mock draws first are deck A, B and C's
opacity, which is `Deck::set_opacity` and `Record::Opacity`, classified `Vocabulary::Session`, while
`Record::Bind` is `Vocabulary::Set`. **There is no binding on a deck fader anywhere in the engine.**

**The repository already held three accounts of a lane and nobody had noticed.** The console page
says *a lane is a binding* in its Sequencer note and *the sequencer is a fifth route* in its rules —
drawn as `sequencer → command` beside the pointer, the keys, MIDI and MCP — and
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) names *a
follower, **a sequencer lane** and **a signal binding*** as separate entries.

## Decision

**A sequencer lane is a fifth route into `karakuri-operation`, not a signal binding.** It emits
operations on the beat, the way the pointer, the keys, a MIDI map and MCP emit them.

### What decides it is P-0094, and the binding reading fails it structurally

P-0094: *moving a control by hand stops whatever automatic thing was moving it… a control that had
to be held against a writer that reasserts itself sixty times a second is not a control at all — it
is a display that moves when you push it.*

**A binding does not cancel.** It blends against the manual value every frame, and a lane's source
would carry confidence 1.0 the way `control:` does — *it is the operator's hand and not a guess* —
so the blend writes the mapped value bit for bit and the hand has **no effect at all**. The
asymmetry is visible in one grep: `Deck::set_gain`, `set_opacity` and `set_mask_position` each call
`cancel` explicitly, and **`cancel` does not appear in `binding.rs` even once**. There is no site for
it, because a binding is a per-frame projection and not a writer competing with a hand.

**So a lane built as a binding is precisely the control P-0094 rules out**, and it would have been
found only after it was built. `TakeParamBack` is the vocabulary's escape hatch and is not an
answer: the principle is about the hand, and that is a deliberate operation.

### What the route reading buys, beside being the only one that works

- **The three fader lanes exist.** They emit `SetOpacity`, which is what a hand on the strip emits,
  so a hand and a lane meet at `Live::operate` where every other conflict is already resolved.
- **P-0090 is satisfied by construction** — a lane's write ends in the same record every other
  control ends in, rather than in a second path that produces values without records.
- **The console page's own rule stands**: a step sequencer is a fifth route and works for the same
  reason the other four do, which is the sentence that turned out to be right.

### The clock is not a blocker, and the roadmap implies it is

A step grid is **the beat clock subdivided**, not a fourth clock. A step index is
`floor(beats × steps_per_beat) mod length` — a pure function of `Oscillator::beats`, which is
already anchored so a tempo correction changes the rate from now on without moving beats that
already happened. No new state, no scheduler, no event queue.
P-0069 permits exactly this
on the beat clock — *selection among options already prepared* — and credits a transition with the
same property. **Two caveats are recorded rather than waved away**: P-0069's stated period band for
the beat clock is 0.5–4 s and a 1/8 step at 128 BPM is 234 ms, so the principle as written does not
say a subdivision is still that clock; and a step onset is observed at the next frame, because
bindings and their kin resolve once per frame. The second is fine for a sequencer that writes values
and would not be for one that fires events.

## Alternatives rejected

**A bus name** — the roadmap's settled answer. It loses on the bus's own two stated properties, and
it loses before any of this: the bus refused to carry `noise`'s four fields for the same reason.

**A lane is a binding** — the roadmap's other settled half, and the console page's Sequencer note. It
loses on P-0094 as above, and it cannot express three of the four lanes the mock draws.

**A source kind on the binding, shaped like `noise`.** The near miss, and worth recording because it
is the right shape for the wrong question: `Record::Bind` already carries `noise: Option<BindNoise>`
*because a noise generator is the one signal with parameters of its own*, and a pattern is the
second such thing. It would give per-lane distinctness for free and need no bus name. It still loses
to P-0094 — the cancel problem is the binding's, not the source's — but if a lane ever needs to
drive a Set parameter continuously rather than on a step, this is the shape.

## Consequences

- **The roadmap's Settled decisions entry is wrong and is corrected with this record.** It also
  already carried the doubt: the estimate section says this half *is an argument rather than a
  measurement*, and says to widen the top of the range if it lands here. **This is the measurement,
  and the argument failed.**
- **What the bay can draw today is nothing beyond its head**, which is already drawn. Under
  [ADR-0200](0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md) every part of
  it — the ruler, the playhead, `step 6`, the lanes, the banks — reads a value nothing holds. **This
  is the one bay where that rule yields nothing**, and that is the finding rather than a delay.
- **`TakeParamBack` is the lane mute, already.** Its own documentation says *stop a signal driving a
  knob without losing the binding — nothing does this today*, and the mock's muted lane says *the
  pattern is kept and drives nothing*. It is the third time in this milestone that a bay's missing
  operation turned out to exist under a name nobody had connected to it.
- **The `step` curve is not a `Curve` and cannot become one cheaply.** `Curve`'s doc argues four is
  complete because *a fifth would be a re-parameterisation of one of these*, and a quantiser is not
  one — so that argument does not cover it, and `Curve::parse` refuses `step` today with a
  diagnostic rather than a silent `lin`. Under this decision the question disappears: a route does
  not carry a curve.
- **The largest thing left is not the sequencer's.** Three of four mock lanes need a way to drive a
  deck fader from something that is not a hand, and P-0094 already names that hole — *none of them
  writes a mix control yet* — for the follower and for agents as much as for this bay.
- **`--seq` is taken** by the CLI's frame-sequence render, so a command-line spelling has to be
  something else.
