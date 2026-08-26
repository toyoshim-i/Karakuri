---
id: 0191
title: The panel's parked deck is parked by the governor, or it is a drawing of one
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: [0025, 0047, 0072]
tags: [console, decks, testing]
---

# The panel's parked deck is parked by the governor, or it is a drawing of one

## Context

[ADR-0190](0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md) drew
the mixer strip's residency chip rolling toward the word it was asked for and falling back, and
`tests/parked.rs` asserts every pixel of it — the width, the clip, the blank band, the curve at
phases the file chose, and the deadline the panel declares while it lasts. All of that is asserted
from `view::Strip`s the test writes by hand, which is right: those are claims about a presentation,
and a presentation is a function of the values it is handed.

**None of it could be seen by running the window.** `crates/karakuri-console/examples/panel.rs` is
the one place a person looks at this console, and its own module doc said what was missing: *"no
audio, no MIDI, no store, no argument parsing, no governor, no records"*. A `Deck` grants every
residency it is asked for — `Deck::set_residency` writes the request **and** writes the effective
level beside it — and only a [`Deck::govern`] pass can hold a slot below what was asked for. With no
governor, `Deck::requested_residency` and `Deck::residency` were equal on every slot of that deck on
every frame it ever drew, `Deck::is_parked` was never true, and the animation with the most tests
behind it on this panel was unreachable in the one place it would be looked at.

The example also had one slot, and the slot it had is the picture. So the state needed a second
deck as well as a governor.

## Decision

**The example builds a second slot, asks for it to be primed, and lets `Deck::govern` refuse it.**
The parked strip on that panel is the governor's verdict read back off the deck, not a state this
file wrote.

`Engine::ask_to_prime` is the whole of it and it is `karakuri-cli`'s startup order rather than a
second one:

1. `Deck::measure_slots`, before the first frame, where the stall is free — the same call and the
   same reason the CLI has at the same point.
2. `Deck::set_residency(slot, Priming)` — the request, which is the operator's half.
3. The compute budget, **set from what step 1 just measured**.
4. `Deck::govern`, which is the only thing in the engine that writes an effective residency, and
   whose `Report` comes back whole so the legend can print it and a test can assert on it.

`mixer` then reads both residencies off the deck the way it already read one, and
`view::Strip::pending` derives the rest. Nothing in the example writes a residency, computes
`is_parked`, or constructs a `Strip`.

### The budget is computed from the measurement, and that is the load-bearing part

A deck starts on `governor::DEFAULT_COMPUTE_BUDGET_MS` — 16.7 ms of measured per-Set cost — and what
one of these Sets costs is the machine's business. On the machine this was written on it measures
between 3.9 and 9.8 ms across runs on a host clock, so against the default budget the request is
**granted**, at full rate or at one step in two depending on which run you took. A demo that parks
on this machine and primes on the next one is not a demo of a park.

So the budget is set to **what deck A is already committed to, plus half of the least the slowest
priming rate could ask for**: `committed + cost / (2 × SLOWEST_PRIME_ONE_IN)`. The governor will
slow a candidate to one step in `SLOWEST_PRIME_ONE_IN` and no slower, and a slowed slot costs
`cost / n` amortised, so a headroom under `cost / SLOWEST_PRIME_ONE_IN` cannot take the Set at any
rate the governor is willing to call priming. The refusal is `Reason::NoHeadroom` arithmetically,
on every machine, at whatever the probe says. Two runs, verbatim from the window:

```
governor: 9.63 / 10.22 ms live [host clock]
  slot 1 (deck B) parked, request held: NoHeadroom — warming it was measured at 9.403 ms, against 0.588 ms of headroom
governor: 9.78 / 10.02 ms live [host clock]
  slot 1 (deck B) parked, request held: NoHeadroom — warming it was measured at 3.893 ms, against 0.243 ms of headroom
```

The two measurements differ by a factor of two and the verdict does not, which is the property being
bought.

### What it costs, measured

**Per frame: 69 allocations and 137.4 kB more in the `egui` pass**, and that is the second strip
rather than the roll — 456 allocations and 556.9 kB before, 525 and 694.3 kB after, on the same
window at 1440x900, taken from the example's own reading and stable to the allocation across runs.
The whole-frame median moved from 6.3 ms to 3.3 ms between those two runs, which is this machine's
power management rather than the change and is why the allocation count is quoted instead: the
readout's own last paragraph says to compare ratios, and the allocation count is the one figure here
that does not move under it.

**Per frame, from the parked slot itself: nothing.** A parked slot is `Residency::Allocated`, which
neither steps nor draws, so the engine half of the reading is still one Set of 262144 elements
stepping once a frame. That is not incidental — see the alternative below.

**At startup: one `Deck::measure_slots`**, which is a `Probe` calibration and two measurements. It
is 0.66 s in the test that takes it, against 0.15 s for the next-slowest test in the file, and the
whole example test binary went from 0.24 s to 1.20 s — the rest being a second `Set::build` in the
six `mod gpu` tests that construct an `Engine`. Which is why the pass is a call of its own rather
than part of `Engine::new`: the five tests that have nothing to do with residency pay the build and
not the probe.

### The test asserts the reason, not only the park

`the_budget_parks_a_deck_and_the_strip_carries_both_residencies`, under `mod gpu` because a `Deck`
takes a device, asserts the pre-state (both residencies agree on both slots, nothing pending), then
the park, then the reason, then that both residencies cross the seam into the strips, then that
`View::animating` declares the roll's staleness off strips the engine produced.

**The reason is asserted because three of the four ways to satisfy `Deck::is_parked` mean the
example forgot to do something.** `Unmeasured` and `CommittedUnknown` are a probe that never ran;
`NoPrimingNeeded` is a closed-form Set that never wanted warming. Only `NoHeadroom` is the budget
refusing, which is the thing being demonstrated. Run against its defects (P-0025), on its own:

- Delete the `set_compute_budget_ms` call: *"the request was granted rather than parked — governor:
  9.75 / 16.70 ms live + 5.01 priming [host clock]"*.
- Delete the `measure_slots` call: *"deck B is parked for a reason that is not the budget —
  governor: 0.00 / 16.70 ms live (+1 unmeasured) … left: Unmeasured, right: NoHeadroom"*. The park
  still happens, and it is the wrong park.

## The alternatives

**Staging the state: hand the view a `Strip` with two residencies in it, or hand the deck a
measurement.** Cheapest by a distance — no second Set, no probe, no governor, and `tests/parked.rs`
already does exactly this and is right to. It loses because the example is not a test of a
presentation, it is the program: a panel showing a parked chip that no `Deck` ever parked is a
drawing of a state the engine never entered, and the next person to change `Deck::govern` would
break the real path with the picture still looking correct.
[P-0047](../principles/0047-a-fixture-the-product-can-rewrite-is-not-a-fixture.md) is the same
argument from the other side — a test owns its inputs, and this window's input is an engine. The
tempting version is not the `Strip`, which nobody would write here, but
`HotSwap::set_measured_cost`: it is public, it is documented for *"a caller that has a number from
somewhere else"*, and one call makes the budget arithmetic come out exactly. It would be this file
writing down the number the probe exists to take, and the run above — a factor of two between two
runs on one machine — is what that number would be pretending to know.

**Parking for free, by never measuring.** Ask for priming and skip `measure_slots`: the committed
cost is unknown, priming is suspended wholesale, `is_parked` is true, and it costs no probe, no
budget arithmetic and 0.66 s less in the suite. Rejected because that park says *this harness has
not measured its deck*, which is a defect in every other program in this repository — the CLI calls
`measure_slots` at startup with a comment saying that without it the governor is an elaborate way of
saying no. A demo whose state is reached by omitting a startup call teaches the omission.

**Leaving the budget alone and adding slots until 16.7 ms overflows.** The honest-looking version:
no budget knob, just more requests than the machine can hold. Rejected because how many slots that
takes is the machine's answer, not this file's — at 3.9 ms a slot it is five and a deck holds four,
and at 9.8 ms it is two. It would be a demo that parks on slow machines.

**A third slot, admitted, so the panel shows a grant beside a refusal.** Genuinely better
pedagogy — the same request on two slots, one primed and one parked, with the budget as the only
difference — and it was rejected on what it costs rather than on what it says. A slot that is
*actually* priming steps every frame on the render thread, so the panel would carry a second
simulation into exactly the numbers the example exists to print. The reading would then have to
explain how much of the frame is a demonstration. A parked slot costs nothing per frame, which is
what keeps the reading about the console.

**Putting the pass in `Engine::new`.** One call site instead of two, and every `mod gpu` test would
have a governed deck without asking. It costs a `Probe` calibration in five tests that never look at
a residency, and it would make `Engine::new` a constructor that measures — the thing `measure_slots`
documents as a startup call precisely because it must not happen anywhere else.

## Consequences

- **The roll is reachable by running the window**, which is what ADR-0190 assumed and did not check.
  `cargo run -p karakuri-console --example panel` now draws a deck whose chip rolls once a second
  and never lands, and the legend says which slot, what it was asked for, what it is doing, what the
  budget is, and what warming it was measured at.
- **The example runs a governor**, and its module doc no longer says it does not. That sentence was
  a present-tense document about the file it sits in, and it was the whole of the gap.
- **The panel is never still while the example runs**, which changes nothing: the picture is a live
  engine frame and the window was already drawing every vsync (P-0072's first clause has been
  reported as *stopped holding, and the reason is the engine* since the picture landed). The roll's
  30 Hz declaration is under that, so `View::animating` is never the thing keeping this window
  awake. On a console with no live picture it would be, and `tests/parked.rs` is where that is
  asserted.
- **`ADR-0164`'s figure is further behind, and it was already behind.** The reading prints *"ADR-0164
  measured 184 allocations and 226.2 kB a frame here with every bay empty, and the `egui` pass above
  is still that"* over a measured 525 and 694.3 kB. It read 456 and 556.9 kB before this change, so
  the sentence was untrue at the mixer bay's landing rather than at this one; it is left alone here
  because correcting it is a measurement of the mixer, not of a residency.
- **The second slot is a Set nobody looks at**, built from the same pair at a different salt. It
  holds VRAM for the length of the run and is the price of having a deck with something to refuse.
