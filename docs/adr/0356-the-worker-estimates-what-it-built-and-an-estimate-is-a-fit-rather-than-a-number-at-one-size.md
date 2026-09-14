---
id: 0356
title: The worker estimates what it built, and an estimate is a fit rather than a number at one size
status: accepted
date: 2026-09-14
supersedes: []
superseded_by: []
principles: [0091, 0095]
tags: [engine, governor, estimate, swap, deck]
---

# The worker estimates what it built, and an estimate is a fit rather than a number at one size

## Context

[ADR-0296](0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)
made the governor budget on the estimate where it answers and on the measurement where it does not,
and left two holes in its own *Consequences*, both stated rather than fixed.

**A swapped-in Set arrived unestimated.** `HotSwap::install_if_ready` set `self.estimate = None` on
every install, so from the moment an operator loaded anything, `HotSwap::judge` and `Deck::govern`
were both deciding on `swap::measure`'s single draw at `HotSwap::measure_size`. On the desktop
application that size is the preview cell — `crates/karakuri/src/bridge/engine.rs` sets it to the
cell a slot is auditioned in, 252x142 — while the budget is a whole frame at the output's size.
`crates/karakuri/src/bridge/engine.rs` had already written down what that costs on this machine: the
reference workload estimates 45 ms at 1280x720 and measures 12 ms at the cell. **The gate and the
budget were about two different frames**, and `Deck::estimate_slots` could not close it because it
estimates only a slot that has never stepped — cold slots at startup, and nothing an operator loads
afterwards.

**A resize dropped the estimate.** ADR-0296 §4 made that the rule, on the argument that
`Estimate::target` says which size the answer was for and a kept one after a resize is a right
number about a frame nobody is drawing. That is true of the *number*. It is not true of the
`Estimate`: `Fit` carries `invariant_ms` and `ms_per_pixel`, and `Estimate::rungs` carries the two
`Measurement`s the fit came from, so the answer at another target is arithmetic over data already
taken. Dropping it put every slot back on its measurement for every frame of a window drag.

ADR-0296 rejected estimating on the worker for two reasons and
[ADR-0349](0349-the-chain-is-the-frames-so-it-reads-the-session-clock-and-is-charged-against-the-frames-budget.md)'s
alternatives repeat them: the worker does not know the output's size, and the size can change
between the build starting and the Set landing. **Both are answered by the same observation.** The
first is a missing channel and nothing more — `HotSwap::resize` is called with the deck's own size
and is the one call that knows it. The second stops being a hazard once an estimate is a fit: a
target that moved in flight is re-read at the install rather than being stale in a channel where
nothing can see it.

## Decision

**The build worker estimates what it built, at the output's size, and an estimate is re-read at a
new target rather than dropped.**

### 1. Two sizes, because they are two questions

`swap::Sizes` holds both atomics and is what the worker is handed. `measure_at` is what a caller
named and is where one draw is taken; `estimate_at` is the frame the slot draws and is where two
rungs are placed. `HotSwap::resize` writes `estimate_at` and `HotSwap::estimate_size` reads it back.
A `HotSwap` nobody has resized answers for 1x1, which holds no rungs and refuses without spending a
draw — the honest answer for a slot that has not been told what it draws into.

### 2. The estimate travels on the build, and the install keeps it

`Built` carries `estimate: Option<Estimate>` beside `cost`. The worker runs `estimate` after
`measure` and before the flush, so
[ADR-0033](0033-freeing-on-the-render-thread-is-the-same-invariant-as-allocating.md)'s
flush-before-handover covers the rungs' rewind uploads as it already covered the measurement's.
`install_if_ready` sets `self.estimate = built.estimate.map(|e| e.at(self.viewport))` where it set
`None`. Nothing else changes: `judge` and `govern` already read the slot through
`governor::budgeted`, so a swapped-in Set is now decided by the same rule and the same precedence a
cold one is.

**A refusal is a value and travels.** `Option::None` here means the estimate itself panicked;
`Some(Err(Unfit))` is the instrument declining to answer, which leaves the measurement deciding —
ADR-0296 §2 unchanged, and ADR-0313's *a candidate with no number is kept and reported as not
judged* unchanged with it, because there is still a number.

### 3. `Estimate::at` — an estimate is a fit

`Estimate::at(target)` re-runs `estimate::fit` over the rungs already on the record and carries the
floor and its provenance across, because neither depends on the target. It draws nothing. Two calls
use it: `HotSwap::resize`, which re-reads the live slot's estimate at the new viewport, and
`install_if_ready`, which re-reads the incoming one at the viewport it is landing in.

**The re-read can refuse, and that is the correction doing its job rather than an outage.** The
sub-pixel share is recomputed against the new target, so rungs that hid nothing at 128 rows may hide
more than `FLOORED_SHARE_ALLOWED` at four times the height, which is `Unfit::FlooringHidesTooMuch`.
A refusal falls back to the measurement, which is §2 above.

### 4. What is reported is the number that was spent

`Event::Accepted` and `Event::Overloaded` carry `cost_ms` and `basis` exactly as before, and `basis`
is now `Basis::Estimated` on any build whose fit answered. `Decision::cost_ms` still means the
measurement and only the measurement.

## Alternatives rejected

**Estimate on the render thread at install.** It is where the output's size is known for certain and
it needs no channel. Rejected on what an estimate *is*: two `queue.submit`s and two waits for the
GPU to finish, at a quarter and a sixteenth of the frame's area. A swap lands inside
`HotSwap::begin_frame`, so this is a submit-and-wait on the render thread at the exact moment a new
Set is going on air — the stall the whole background-build arrangement exists to avoid, and the one
`docs/architecture.md`'s render thread model forbids outright. ADR-0296 said the same thing about a
re-estimate hook and it is still true.

**Re-run `Deck::estimate_slots` after an install.** One call, already written, and it is a startup
step that a caller with a governor already makes. It loses twice. It is the same submit-and-wait on
the render thread, moved one frame later and made periodic. And its own guard rules it out: it
estimates a slot only where `Set::time() == 0.0`, so the first frame after the swap disqualifies the
Set, and loosening the guard would re-estimate a slot mid-performance — measuring means stepping,
and a `rewind` on a live Set is a picture that jumps.

**Have the worker estimate at `measure_at` and re-target at the install.** No new channel at all,
and `Estimate::at` is already the re-targeting. Rejected because the rungs are *placed* against the
target: a 252x142 cell puts them at 63 and 71 rows, and re-reading that pair at 1280x720 is a fit
across a spread of nothing extrapolated seventeen times, which `floored_share` correctly refuses as
`FlooringHidesTooMuch`. The re-target is sound across a resize because the rungs were placed for a
size of the same order; it is not a way to avoid knowing the size at all.

**Keep ADR-0296's rule and drop the estimate on resize.** It is one line and it is never wrong about
which size an answer was for. Rejected because it is wrong about what it is dropping: the rungs are
a measurement that was paid for, and throwing them away to avoid re-reading them costs the slot its
better number for the whole of a drag — and a window drag is exactly when a governor's verdict
matters. `Estimate::target` still says which size the answer is for; it is now kept true by moving
it rather than by discarding the record.

**Make the worker's estimate conditional on the measurement having succeeded.** Tidier on the face
of it, since a panicked `measure` leaves the Set at a probe's viewport. Rejected because
`install_if_ready` resizes the candidate to the slot's viewport regardless, so the tidiness buys
nothing and the cost is a Set with no estimate for a reason unrelated to estimating.

## Consequences

- **A build costs three draws instead of one.** The measurement's full-size draw at `measure_at`,
  plus two rungs at a quarter and a sixteenth of the output's area — ADR-0266's 5/16 of a full-size
  draw in fragment work, and the invariant term paid twice. It is paid on the worker thread, which
  is what that thread is for, and it is paid once per build rather than once per frame.
- **`crates/karakuri-engine/src/swap.rs`**: `Sizes` is new and private; `Built` gains `estimate`;
  `HotSwap` gains `estimate_at` and `HotSwap::estimate_size`; `HotSwap::resize` now writes the
  worker's target and re-targets the held estimate instead of clearing it; `run_worker` takes
  `Sizes` in place of one atomic.
- **`crates/karakuri-engine/src/estimate.rs`**: `Estimate::at` is new and is pure.
- **`HotSwap::install`** — the replay path — still clears the estimate, because a Set installed
  without a worker was never estimated and there is nothing to carry.
- **Which of a slot's two numbers is spent is now load-dependent, not only machine-dependent.** The
  worker's rungs are a few hundred microseconds each on a host clock and are taken while the render
  thread is drawing, so a busy deck can produce `Unfit::FragmentTermNegative` where an idle one
  fits. The number spent is the candidate's own frame either way; the *basis* is not a property of
  the Set. `crates/karakuri-engine/tests/deck.rs`'
  `a_candidates_verdict_does_not_move_with_what_the_other_slots_carry` asserts the verdict and no
  longer asserts the basis, and says why.
- **A budget derived from a measurement is not a budget on the quantity under test.**
  `an_over_budget_candidate_is_stopped_on_a_budget_derived_from_its_own_measurement` derived its
  threshold from `HotSwap::measured_cost` and had to move to `governor::budgeted`; its margin
  widened from a half to an eighth, because two `Gpu::headless` adapters on this machine read the
  same Set three times apart.
- **`Unfit::FloorUnknown` is not reachable through `estimate`**, and a brief that expects it for
  `Points` and `Lines` is reading ADR-0266 rather than
  [ADR-0293](0293-a-rung-may-sit-under-the-floor-because-what-it-hides-is-bounded-and-paid.md): an
  unbounded rate is a floor of `u32::MAX`, the accurate pair is placed against it, and the estimate
  answers with `floor: None`. The refusal that *is* deterministic is a target with no room under it,
  which is what `a_candidate_the_estimator_refuses_is_judged_on_its_measurement` forces with a 2x2
  slot.
- **`Deck::estimate_slots` is unchanged and is still a startup step.** Its guard now also means it
  never re-estimates a swapped-in slot, because that slot arrives with one.
- **A param write still does not invalidate the estimate.** ADR-0296's last consequence stands
  unchanged: the contradiction detection is read at estimate time only, so a fader moved after the
  build leaves a number that was right when it was taken. A resize moves the number now; a param
  write does not.
