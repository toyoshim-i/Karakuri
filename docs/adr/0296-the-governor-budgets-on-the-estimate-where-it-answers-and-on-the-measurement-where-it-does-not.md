---
id: 0296
title: The governor budgets on the estimate where it answers, and on the measurement where it does not
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0095]
tags: [engine, governor, estimate, deck, measurement]
---

# The governor budgets on the estimate where it answers, and on the measurement where it does not

## Context

`crates/karakuri-engine/src/estimate.rs` was built on 2026-09-06, exported from `lib.rs`, and
called by nothing. It fits `a + b·area` through two small draws and evaluates it at the output's
size, which is a strictly better statement than the single draw at 1280x720 that `swap::measure`
takes — and it was a better statement nobody read.

It was not worth reading until this morning.
[ADR-0285](0285-a-renderers-floor-is-bounded-from-its-declared-ranges-or-refused.md) bounded a
renderer's sub-pixel floor from its declared ranges and `estimate` refused twelve of the fifteen
shipped L4 procedures on it;
[ADR-0293](0293-a-rung-may-sit-under-the-floor-because-what-it-hides-is-bounded-and-paid.md)
loosened how strictly that floor is read and the twelve refusals became none. **So there is now a
number for every piece of material this instrument ships, and this record is what reads it.**

The consumer is `Deck::govern`. What it does today, per
[ADR-0054](0054-the-governor-budgets-from-the-probe-and-never-touches-a-live-slot.md), is sum the
Live slots' measurements, subtract that from a compute budget, and admit priming requests against
what is left. Every millisecond in it is `swap::measure`'s: **one draw, at
`swap::PROBE_RESOLUTION`, whatever the deck is actually drawing into.**

That is the gap. [ADR-0246](0246-the-render-size-is-the-outputs.md) makes the render size the
output's, so a deck on a 640x360 window is refusing priming on fragment work nobody is doing, and a
deck on a 4K output is admitting against a number several times too small. **The measurement cannot
tell those apart**, because one draw gives `a + b·area` and no way to divide it — which is
`estimate`'s first line.

## Decision

**A slot carries two numbers, and the governor budgets on the estimate where it answers and on the
measurement where it does not. A refusal changes nothing.**

### 1. One rule, in one place

`SlotState::budgeted` returns the number and where it came from:

```text
estimate that answered  ->  (Basis::Estimated,   its ms at the output's size)
otherwise, a measurement ->  (Basis::Measured,    its ms at 1280x720)
neither                  ->  (Basis::Unbudgetable, no number)
```

`Report::committed_ms` and `Governor::admit` both call it, so a Live slot and a priming one can
never be judged on two different readings of the same Set. `Decision::budgeted_ms` is the number
that was spent and `Decision::basis` is which kind it is; `Decision::cost_ms` still means the
measurement and only the measurement, because a status line showing an estimate where it says
"measured" would be overstating what was taken.

**Why the estimate wins rather than, say, the larger of the two.** Taking the maximum would undo
[ADR-0266](0266-a-slots-cost-at-full-size-is-a-fit-through-two-small-draws.md)'s whole finding: the
area-ratio rule overshot by nearly the whole area ratio and put every per-element slot in the band
that stops a slot, and a `max` reintroduces that wherever the measurement is the larger. The
estimate already rounds toward refusing at every step — the floor is a lower bound on the rate, the
lower rung is measured first so drift lands in `b`, and a floored pair is corrected upward — so
there is nothing left for a second safety factor to buy.

### 2. A refusal is not a licence

`Unfit` is a refusal to answer, not a small answer. So a slot whose estimate refused falls back to
its measurement and is decided by arithmetic identical to what stood before this record. A slot with
**neither** number is `Reason::Unmeasured` — parked if it asked to prime, and
`Reason::CommittedUnknown` for the whole deck if it is on air.

That is *Why an unmeasured Set is not a free one* reached from a second direction, and it is the
half of this change most easily got wrong: an estimate is the more precise instrument, so the
temptation on a refusal is to treat the slot as unexceptional. **The instrument declining to answer
has not said the answer is small.** `Report::refused_estimates` is how a caller finds out, and the
refusal is kept on `Decision::estimate` even under `Basis::Measured` — a slot that fell back must
not read like one nothing ever asked.

**This is not hypothetical on the machine this was written on.** `docs/contributing.md` §1: the
probe demotes to a host clock here, so a 128-row deck's two rungs at 32x32 and 64x64 are two noise
figures, and the upper one reads *cheaper* than the lower — `Unfit::FragmentTermNegative`, a failed
measurement rather than a cheap Set. The fallback is the path this repository's own hardware takes.

### 3. P-0095: both halves of the record travel, and neither is spent twice

[P-0095](../principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md)
is why `Estimate` carries `floor_from` and `floored` at all: **`floor_from` says where the floor
came from and `floored` says how strictly it was read**, and an estimate taken with every primitive
at least a pixel across at both rungs is not the same statement as one taken with some of them
rounded up. A governor acting on `ms` alone would hand a status line a number nobody can check.

`Decision::estimate` is a `Copy` summary — `Estimated` — carrying the answer or the refusal, the
target, the instrument, the floor, `FloorRead` and `Floored`. It is a summary rather than the
`Estimate` because a governor pass is `Copy` of small things by construction and `Estimate` holds
two `Vec`s; **the whole record stays on the slot**, reachable through `HotSwap::estimated_cost`, and
`FloorRead` is the discriminant so a reader knows which of the three they have without fetching it.

**What the governor does with them is report them and nothing arithmetic.** `Fit::ms` already
carries `Floored::correction` — ADR-0293 §4 applies it to the answer rather than to either term —
so applying it again would inflate a number that is already sound, by a third, which is a whole band
of the badge it feeds. `Report::corrected` and `Report::floor_unknown` count them and the `Display`
line says them, beside the count of how many slots were estimated at all and the size they were
estimated for.

**It does not re-derive a floor and it does not refuse one.** A `FloorRead::Stated` floor is a
caller's assertion nothing checked and a `FloorRead::Contradicted` one is a bound a held value
falsified. ADR-0285 makes the first a record rather than a refusal, and ADR-0293 §6 makes the second
*the greatest floor there is* — placed and paid for. Both are numbers this module spends and both
are numbers it names.

### 4. Where the number comes from, and when it stops being true

`Deck::estimate_slots(device, queue)` is `Deck::measure_slots`'s counterpart: one `Probe` for the
whole deck, every slot that has none and has not stepped, at **`(width, height)` — this deck's own
output size**, which is the entire reason the number is worth having. It carries every one of
`measure_slots`' warnings twice over, because it submits and waits at two sizes rather than one.

**The estimate is dropped whenever it would stop being about this Set at this size**, in
`HotSwap`: a build landing, a replay's `install`, and — the one that is new in kind — a `resize`.
`Estimate::target` says which size the answer was for, so a kept one after a resize is a right
number about a frame nobody is drawing, and the governor would spend it. Dropping it puts the slot
back on its measurement, which is the fallback in §2 doing its job rather than an outage.

A rollback restores the parked Set's estimate with its measurement, for that field's reason: the
governor must not go on budgeting for a Set that is no longer there.

**A swap leaves the incoming Set unestimated**, because the build worker measures what it built and
cannot estimate it — an estimate is taken against the output's size and the worker does not know it.
That is stated rather than fixed; see *Consequences*.

## Alternatives rejected

**Hand the governor `&Estimate` rather than a summary.** Zero-copy and loses nothing. Rejected on
the borrow: `Deck::govern` collects a report and then writes effective residencies back through
`set_effective`, so a `Report<'a>` borrowing the slots cannot coexist with the `&mut self` the
write needs. `Arc<Estimate>` clears the lifetime and costs `Decision` its `Copy`. The summary keeps
both, and keeps the full record one accessor away.

**Budget on `max(estimate, measurement)`.** Argued above: it is ADR-0266's overshoot reintroduced
wherever the measurement is larger, which for a deck drawing smaller than 1280x720 is everywhere.

**Correct the estimate again in the governor, or refuse a corrected one.** The correction is already
in `Fit::ms`. Applying it twice inflates by 1.77 at the allowance, which is two bands. Refusing a
corrected estimate would be ADR-0293's strict reading smuggled back in one level up — the refusal
that record exists to remove — and would refuse exactly the shipped per-element corpus.

**Refuse a `FloorRead::Stated` or `FloorRead::Contradicted` estimate.** Tempting, because one is
unchecked and the other is falsified. Rejected because both are decided elsewhere and decided the
other way: ADR-0285 makes a stated floor a record, and ADR-0293 §6 makes a contradicted bound an
unknown floor, which is a placement. Inventing a third answer here would put a rule about `estimate`
inside `governor`, where nobody looking for it would find it.

**Estimate on the build worker, so a swapped-in Set arrives with one.** It is where `measure`
already runs and it would close the gap in §4. Rejected for now because the worker does not know the
output's size and giving it one makes a build depend on a window — and the size can change between
the build starting and the Set landing, which puts the staleness this record handles with a `resize`
hook back inside a channel where nothing can see it.

**Make `estimate_slots` part of `measure_slots`.** One call, one probe, one stall. Rejected because
the two answer different questions and cost differently by a factor of two and a half, and a caller
that wants only the cheap one — every `--render` and `--seq` run, which has no governor to feed —
would be paying for the other. They are separate calls and a startup that wants both makes both.

## Consequences

- **`crates/karakuri-engine/src/governor.rs` gains four public items** — `Estimated`, `FloorRead`,
  `Basis` and `SlotState::budgeted` — and `SlotState` gains an `estimate` field. `Decision` gains
  `budgeted_ms`, `basis` and `estimate`; `Decision::cost_ms` is unchanged in meaning and is no
  longer necessarily what the arithmetic was done on.
- **`Report` gains five reading methods** — `estimated`, `corrected`, `floor_unknown`,
  `refused_estimates`, `estimated_target` — and its `Display` line grows a clause naming the size
  the estimates answered for and how many carried a correction or an unknown floor.
- **`Report::committed_ms` can now be a sum of two kinds of number** on a part-estimated deck. That
  is the honest reading rather than a defect — each term is this crate's best statement about that
  slot — and `Report::estimated` says how many of them are the better kind. It still sums Live slots
  only, which is a separate open question about what `committed_ms` should be the total *of*.
- **`crates/karakuri-engine/src/swap.rs`**: `HotSwap` holds `estimate` and `previous_estimate`, with
  `estimated_cost`, `estimate_live` and `set_estimated_cost` beside the measurement's three.
  **`HotSwap::resize` now drops the estimate**, which is a behaviour change to a call that
  previously only forwarded a size.
- **`crates/karakuri-engine/src/deck.rs`**: `Deck::estimate_slots` is new. Nothing calls it yet
  outside tests — it is a startup step for a caller with a governor, exactly as
  `Deck::measure_slots` is, and `crates/karakuri` is where that call belongs.
- **A Set that swaps in arrives unestimated** and the slot governs on the worker's measurement until
  something estimates it. On a live run that is every swap. Closing it needs either an estimate on
  the worker (rejected above) or a re-estimate hook on the frame side, which is a call that submits
  and waits and therefore cannot go where the swap lands.
- **A param write does not invalidate the estimate.** ADR-0293 §8 states the rule — a write
  invalidates the estimate standing on it and the estimate is taken again — and the detection
  (`Set::rate_bound_contradicted`) is read at estimate time only. So an operator moving a fader
  after `estimate_slots` leaves a number that was right when it was taken. It is the same staleness
  `Deck::resize` now handles and it is **not** handled here.
- **The badge is producible and is not drawn.** `Decision::budgeted_ms` under `Basis::Estimated` is
  the number the console's five bands read, and
  `a_governed_slot_now_has_a_number_a_band_can_be_predicted_from` in
  `crates/karakuri-engine/tests/governor.rs` is the mechanised statement that it arrives — it
  predicts the band and fails when the number stops arriving, where the console's only badge
  assertion, that the badge is *absent*, fails only when somebody draws one. Drawing it is not this
  record's.
