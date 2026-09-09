---
id: 0313
title: A candidate is judged on its own cost, and the deck's period is a deck-level alarm
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0084, 0091, 0094, 0095]
tags: [engine, live, governor, m5]
---

# A candidate is judged on its own cost, and the deck's period is a deck-level alarm

> **Annotated 2026-09-09: the gate is unchanged and what it does about a candidate it turns down is
> not.** [ADR-0316](0316-an-over-budget-candidate-stays-in-the-slot-and-the-slot-stops-updating.md)
> is the freeze this record calls *"the maintainer's option (b), which he deferred"*, and it landed
> the same day on the measurement below. **`Event::RolledBack` is `Event::Overloaded`**, with the
> same fields and the same asymmetry on `cost_ms`; there is no rollback, so the `Parked` local this
> record introduced is gone with the last of `HotSwap::previous`, and the displaced Set is retired at
> the install. Every sentence here about *rolling a candidate back* names the verdict correctly and
> the action wrongly: what a verdict against does now is leave the candidate live and stop the slot.
>
> The objection this record raises against the freeze — *"a freeze under this gate would freeze slots
> for their neighbours' cost, and a frozen slot is a hole in the picture where a rollback at least
> leaves the previous material running"* — is what the gate fix answered, and the second half of it
> turned out not to be true of the freeze that was built: a stopped slot holds the frame it last drew
> and goes on being mixed, so it is not a hole.
>
> **One test was still written for the trial and was repaired on 2026-09-09**, beside ADR-0316's
> work rather than by it: `karakuri`'s `the_swap_report_says_what_the_lane_says` asserted a
> `landed` row that the swap's own drain had already settled, which had been failing since this
> record landed. It now reads the verdict off the deck and holds the lane, the health capsule, the
> cells and the server against it on either outcome.
>
> **And this record's last Consequence is one pass behind.** It says P-0094's *Undo* example is false
> and owes a delete-and-re-record pass, and offers a replacement that still ends *"leaves the outgoing
> Set unstepped so a rollback resumes it exactly where it was"*. ADR-0316 moves `swap.rs` out of
> *Undo* altogether and writes the replacement example there.

## Context

The maintainer, on 2026-09-09: *"今現在でほとんどの素材がrolled backされちゃう。ちょっとゲートが厳
しすぎる？"* — nearly everything gets rolled back; is the gate too strict? The answer is no. The gate
is not strict; **it is measuring the wrong quantity**, and making it looser would have hidden that.

`HotSwap::record` compared the interval between two `Deck::begin_frame` calls — the whole deck's
frame period — against `DEFAULT_BUDGET_MS`, as a median of thirty frames after eight warmup ones.
Since [ADR-0269](0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md) that
period contains **every** slot's step and draw whatever its residency, plus the composite, the cell
presents, the picture, the `egui` pass and the vsync wait. So one slot's candidate was judged on the
other three slots' cost plus the console's. `swap.rs` said so in its own module documentation —
*"every Live slot in a deck is judged against the whole deck's frame interval, so a budget that fits
one Set rolls back every candidate in a deck of four"* — and
[`governor.rs`](../../crates/karakuri-engine/src/governor.rs) quoted that sentence back as the reason
the governor does not use the watchdog's number. The defect was recorded in two places and repaired
in neither.

Under `PresentMode::Fifo` the interval is also quantised to the display period, because a frame that
misses one vsync lands at the next and there is nothing in between. One dropped vsync inside the
median is therefore a whole period, and it rolls the candidate back.

**Measured headless at 1280x720 on an M4 Pro, host clock, biased high** (four-slot deck, no panel
passes, no vsync):

| what is on the deck | frame period, median |
|---|---|
| the panel's four default slots (`coil_vortex` @10240) | 4.3 ms |
| the reference Set in one slot of four | 11.3 ms |
| the reference Set in all four, one Live | 23.2 ms |
| the reference Set in all four, all Live | 69.9 ms |

On the panel each cell's present adds about 1.5 ms on top of those, and there is an `egui` pass over
the lot. So loading the reference Set into **one** slot of four is about 18 ms of work against a
16.7 ms vsync, lands at 33 ms under Fifo, and was rolled back against a 20 ms budget. What that Set's
*own* frame costs, measured by the probe the worker already runs on it, is **9.58 ms**.

That is the whole of it: a 9.58 ms Set thrown out on a 33 ms number that is mostly other slots and
the console. It is
[P-0084](../principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)'s
standing case — the judgement was confident, automatic, and about something else — and
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)'s
*Where it loses* clause had already named the same failure from the other end: *"auditioning adds a
draw that is not in the budget, inside the window the swap watchdog judges candidates in, so watching
a heavy slot can roll back an unrelated slot's build."*

**The maintainer chose to fix the gate first, and only the gate**: *"ゲート (a) だけ先に"*. Turning a
rollback into a freeze is a separate record and is deliberately not decided here.

**And he narrowed what the fix may read:** *"判定は他のスロットのロードとは独立にあるべきだね"* — the
verdict on a candidate must be independent of what the other slots are carrying.

## Decision

**A candidate's verdict is the candidate's own cost against one frame's budget. Nothing about the
other slots appears on either side. The deck's frame period stays measured and becomes a deck-level
alarm that warns and never acts.**

**The left-hand side is one number, through one rule.** `governor::budgeted` — the estimate at the
output's size where one answers, the probe measurement the worker took where it does not, with
`Basis` saying which — is
[ADR-0296](0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)'s
rule, and it now has three callers rather than two: the committed sum, the admission test, and this
verdict. It was extracted from `SlotState::budgeted` into a free function taking the two numbers,
because the watchdog holds a candidate's measurement and estimate and has no residency or closed-form
flag to invent (`P-0085` — take the mechanism that exists and pay the bill now). A Set can no longer
be rolled back on one reading and admitted on another.

**The right-hand side is one frame of the display, and not a share of anything.** A budget divided by
the live slot count, or a share taken beside what the neighbours are committed to, would both put the
neighbours' load back on the left-hand side by another route: the same candidate would be kept on an
empty deck and thrown out on a busy one, which is what this repairs. Where the platform will name a
refresh rate the caller passes it — `karakuri`'s window reads `refresh_rate_millihertz` when it opens
and calls `Deck::set_frame_budget_ms` — and where it will not, `DEFAULT_BUDGET_MS` stands
(`P-0095`: not knowing is not a licence to invent).

**A candidate with no number is kept and reported as not judged.** The probe run on the worker is
caught rather than propagated, so a build can arrive unmeasured; nothing has estimated an incoming Set
either. Rolling such a candidate back would be deciding it is over a budget on a number it does not
have. `Event::Accepted::cost_ms` is an `Option` and is `None` there, with `Basis::Unbudgetable`
beside it and a `Display` line that says the verdict was not reached; `Event::RolledBack::cost_ms` is
a plain `f32`, so **a rollback cannot be reached without a number, by construction**. This is the
opposite direction from the governor's `Reason::Unmeasured`, and the two are not in conflict: the
governor is deciding whether to spend budget nobody asked it to spend, where refusing costs a
warm-up, and this is deciding whether to take away material the operator asked for, where refusing
costs the thing they asked for.

**The verdict lands in the call the swap lands in, and the trial is gone.** The number was taken on
the worker before the Set was ever handed over, so `WARMUP_FRAMES` and `JUDGE_FRAMES` had nothing
left to settle: `WARMUP_FRAMES` is **deleted**, and `JUDGE_FRAMES` is renamed `PERIOD_FRAMES` and
survives only as the alarm's window. `HotSwap::previous`, `previous_cost`, `previous_estimate`,
`Trial`, `samples` and `on_trial` are gone with it; the rollback target lives for one call as a local
`Parked`. `HotSwap::previous` existing *only* inside the trial window is what
[ADR-0071](0071-there-is-no-panic-key.md) and P-0084 rest on when they refuse a panic key, and there
is now no window at all — the same argument with nothing left to qualify.

**The frame period becomes `Deck::frame_period_ms` and `Report::deck_over_period`.** It is still a
rolling median on a host clock, it is still one number for four slots, and **nothing may divide it
among them**. What it is entitled to say is that the deck as a whole is not keeping up. It warns and
does not act, on `governor.rs`'s own *What it does not touch* terms and on the terms this repository
already takes with the level meter: show the number first, decide later whether anything should move
by itself. It is deliberately **not** `Report::over_budget`, which is the Live slots' summed per-Set
cost against the compute budget; what `over_budget` should say once the deck's total is computed is
`roadmap.md` M5.14 item 3 and is the maintainer's. **This record supplies the measurement that item
needs and takes none of its decision.**

**`begin_frame_parked` stops differing from `begin_frame`.** An off-air slot's trial used to be
frozen, because judging a candidate nobody was drawing against the deck's interval accepted it on a
budget it never spent. The candidate's own cost is the same number whatever the slot's residency, so
there is nothing to freeze — and a candidate on a parked slot no longer waits an unbounded time for a
verdict it could have had at the install.

## Alternatives rejected

**The period delta across the swap** — judge the candidate on how much the deck's frame period moved
when it landed. It reads the right quantity in the easy case and nothing else. The baseline is broken
by auditioning, which adds a draw that is not in the budget inside the very window the delta would be
taken over (P-0094's *Where it loses*); it is broken again by any other slot's build landing in the
same window, or by a residency change, or by the operator dragging the window. And under Fifo it is
worst where it matters most: a heavy candidate arriving in a deck that is *already* over its period
adds nothing the display can show, so the delta reads small and the candidate is kept.

**The display's interval in place of the constant, and nothing else.** This was the tempting minimal
fix — the constant is 20 ms and the machine is 120 Hz, so the budget is nearly two and a half frames
of the display. It tightens: 8.3 ms at 120 Hz against a deck period that is mostly the other slots
would roll back *more* material, not less, and it would do so more confidently. Moving the
right-hand side of a comparison whose left-hand side is the wrong quantity is the shape of repair
this repository refuses. It is taken here **beside** the gate fix and not instead of it.

**Leave the gate and turn the rollback into a freeze** — the maintainer's option (b), which he
deferred. Keeping the wrong quantity and changing what it does about it would make the failure worse
rather than better: a freeze under this gate would freeze slots for their neighbours' cost, and a
frozen slot is a hole in the picture where a rollback at least leaves the previous material running.
The freeze is a real decision about what to do with a slot that genuinely is over budget, and it
needs a gate that can tell which slot that is. That is now available and the record is still owed.

**Divide the budget by the slot count.** Rejected on the maintainer's sentence above and on its own
terms: a slot's share would move whenever another slot's residency changed, so the same candidate
would be kept and then thrown out with nothing about it having changed. What a *sum* of slots costs
is the deck alarm's question and M5.14 item 3's.

**Keep the judging window and only change the number.** A window is what a frame interval needed
because it had to settle; a probe reading taken on the worker before the Set was handed over is
finished before the first frame. Thirty-eight frames of latency with nothing to settle is half a
second of material on screen that was already going to be thrown out — and, on a parked slot, an
unbounded wait. Keeping the window for a reason it no longer has is the drift this repository's
records exist to catch.

## Consequences

- **The verdict on the finding's own case reverses.** Measured on 2026-09-09, panel-like deck at
  1280x720, host clock, biased high: the reference Set loaded into one slot of four was judged on a
  deck period that reads 11.2 ms headless and about 33 ms on the panel under Fifo — **rolled back**.
  Its own frame is **9.58 ms**, `HostWallClock`, and it is now **accepted** against 20 ms and against
  a 60 Hz display's 16.67 ms alike. In the same run the swap landed on frame 96 and the verdict came
  on frame 96; the old path could not have answered before frame 134, because eight warmup and thirty
  judged frames were required by construction.
- **The engine's harness cannot reproduce the rollback and says so.** Headless there is no vsync and
  no panel pass, so the same load reads 11.2 ms and the old gate would have kept it. The rollback is
  a *panel* fact — four cell presents, an `egui` pass and Fifo quantisation on top — and the numbers
  for those halves come from the reference measurements of 2026-09-09 rather than from a run of the
  panel taken here. That is a gap in the evidence and not in the argument: the quantity being
  compared is wrong at 11.2 ms as much as at 33 ms.
- **A refusal no longer waits behind a verdict.** ADR-0310's last consequence — *"a refusal that
  arrives while a candidate is on trial waits for the verdict… on a parked slot, whose trial is
  frozen, it is until the slot goes on air"* — is now false in both halves. There is no trial to wait
  behind, and `install_if_ready` drains on the first boundary after the worker sends.
- **`Event::Accepted` and `Event::RolledBack` changed shape**, and every consumer had to say what it
  does about it: `median_ms` is `cost_ms`, `basis` is new, and the accepted side's number is an
  `Option`. The lane reads which verdict it was and not the number, so `karakuri`'s `verdict` mapping
  is unchanged; its test now names the right quantity.
- **`karakuri-cli --budget-ms` keeps its flag and changes its meaning**, from *the frame interval a
  candidate's median must stay under* to *what one frame of one candidate Set may cost*. The default
  is still `DEFAULT_BUDGET_MS`. `--budget-ms 0` still rejects everything, which is what its own
  documentation offers it for.
- **`DEFAULT_BUDGET_MS` keeps its value and loses its argument.** 20 ms was one 60 Hz frame plus
  slack for vsync quantisation, and a probe reading is not quantised by anything. The slack is
  re-argued from the other half of the same instrument: the probe falls back to `HostWallClock` on
  this crate's machines, which brackets a submit-and-wait the GPU never spent and reads biased high
  (`docs/contributing.md` §1), and a biased-high number against an exact 16.7 ms line rolls back
  candidates that would have fitted.
- **The two numbers are still taken at two sizes, and this does not fix that.** The candidate's
  measurement is taken at whatever `set_measure_size` last named — the deck's own size in the engine,
  a 252x142 preview cell in `karakuri` — while the budget is about the whole frame. That is
  [ADR-0303](0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md)'s
  open finding reaching a second consumer. It errs **loose**: a preview-size measurement understates,
  so the gate is permissive, which is the direction P-0084 wants when the alternative is throwing out
  material on a number that is not about it.
- **What is left over is the deck.** A deck can now be over its period with every one of its
  candidates having been kept on its own merits, which is exactly what the panel's four-slot case
  is. Nothing acts on that: `Report::deck_over_period` warns, and the two records that would act are
  the freeze the maintainer deferred and M5.14 item 3's ruling on what `over_budget` says as a deck
  total. Both now have a measurement to stand on that they did not have.
- **Two clauses of P-0094 describe mechanisms that are gone, and the principle is not edited for
  it.** Its *Where it loses* paragraph reads *"auditioning adds a draw that is not in the budget,
  **inside the window the swap watchdog judges candidates in**, so watching a heavy slot can roll
  back an unrelated slot's build"* — there is no window, and an audition's draw is not on either side
  of the comparison any more, so the risk that paragraph takes knowingly is not taken. And its *Undo*
  clause reads *"discards `WARMUP_FRAMES`
  before measuring `JUDGE_FRAMES` and takes a median — one hitch is not a reason to throw away
  generated material and thirty frames that all miss is"*, and that example is now false: there is no
  warmup, there is no judged window, and the median is the alarm's rather than the verdict's. **The
  principle is not edited**, on ADR-0000's rule that a rule which stops being true is deleted and
  re-recorded rather than amended. The replacement example is here: `swap.rs` builds on a worker,
  polls with `try_recv`, replaces `live` only at the top of `begin_frame`, **judges the candidate on
  the candidate's own measured cost in the call the swap lands in**, and leaves the outgoing Set
  unstepped so a rollback resumes it exactly where it was. P-0094's delete-and-re-record pass is the
  maintainer's and is owed.
- **Five tests hold it, each watched to fail against the shape it replaced.** In
  `karakuri-engine/tests/deck.rs`:
  `a_light_candidate_is_kept_in_a_deck_whose_frames_are_over_the_budget` is the finding's own case;
  `a_candidates_verdict_does_not_move_with_what_the_other_slots_carry` is the maintainer's sentence
  as an assertion, with the deck alarm differing between the two runs so that it cannot pass
  vacuously; and `an_off_air_slots_candidate_is_judged_at_the_install` is what the frozen trial used
  to forbid. In `tests/hot_swap.rs`,
  `a_candidate_is_rolled_back_on_a_budget_derived_from_its_own_measurement` holds a candidate against
  half its **own** measured cost rather than against a constant, which is `docs/contributing.md` §1's
  rule and is the assertion a watchdog still reading frame intervals cannot pass; and
  `a_candidate_that_holds_the_budget_is_kept` compares the verdict's number with the slot's own
  measurement bit for bit.
