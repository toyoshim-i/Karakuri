---
id: 0303
title: A frame's cost is the period, and a measurement names which resolution it is about
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0091, 0094, 0095]
tags: [performance, console, engine, probe, process]
---

# A frame's cost is the period, and a measurement names which resolution it is about

## Context

**Nothing in this repository has ever timed a frame.** `Probe` times a pass, offscreen, at a known
capacity and a known resolution; `estimate` fits two of those; the panel's startup reading prints
three medians — `engine`, `egui`, `upload+pass` — and `Cost::whole` sums them under *"the whole
frame is a median"*. Every one of those numbers stops at a submission, and `Cost::wait` — the block
in `get_current_texture`, which on a `Fifo` surface is most of the wall clock — is reported *beside*
the three rather than in them, on the reading that *"blocking in `get_current_texture` is the
display's pace and not a price"*.

**That reading holds only while the GPU is not the bottleneck, and it says nothing at all when it
is.** A panel drawing at four frames a second was diagnosed on 2026-09-08 by argument rather than by
reading a number: the three medians summed to a few milliseconds, so the loop was reported as
*idle 97.6% of the time*, and the loop was not idle — it was waiting for a shader, one field along
in `Cost::wait`, where nothing distinguishes a wait on the GPU from the vsync idle that field is
named for. The maintainer's correction was one sentence: *"the cost of a shader running is nearly
all GPU, which is why we were talking about the screen size being what limits it."*

P-0095 asks what a number carries about how it was taken. *The whole frame* had no number, so it
carried nothing.

**And the doc claimed a tiling that was not there.** `Cost::whole` said its three fields *"tile the
frame exactly and do not overlap"*. They do not overlap; they do not tile. Between them sit the
sinks being aimed, the Program bay rearranging itself, the panel being solved and a listing re-read
on the frame a save lands, and after the last of them sits `Queue::present`. Nothing measured that,
so nothing could have caught it.

## Decision

**The frame's cost is its period, measured between the same statement of two consecutive redraws,
and the wait is inside it.**

- **`Cost::period`** is the wall clock from the top of one `RedrawRequested` to the top of the next.
  Two consecutive anchors bracket a whole redraw — the block on the swapchain, every timed stretch,
  every untimed one, `Queue::present`, and whatever `winit` does between two frames — so the
  periods tile the run exactly: nothing falls between two and nothing is in two. `None` on the first
  frame of a run, which has no predecessor to be an interval from.
- **`Cost::elsewhere`** is the residue: the period, less the wait, less the three stretches
  `Cost::whole` sums. It is printed rather than argued, so *how much of the frame the timed fields
  do not see* is a number this window either produces or does not, instead of a sentence in a doc
  comment.
- **`Cost::drained`** is what the GPU still owed when the CPU had finished the frame:
  `Device::poll` to a drained queue, timed on the host clock, taken between the submission and
  `Queue::present`. It is the whole submission — four slots stepped and drawn, the composite, five
  presents and the panel's pass — and not one of them, and it is biased high: the poll's round trip
  is in it and so is anything of the previous frame still in flight.
- **One frame every 500 ms is audited**, and the rule is a stretch of wall clock rather than a frame
  count *because of the case the instrument exists for*. One frame in sixty sounds steadier and is
  backwards: a loop at four frames a second is the loop that most needs the GPU's own number, and
  sixty frames there is fifteen seconds — five times the `STILL` the reading is taken over, so the
  reading would carry no GPU figure exactly where it is the whole answer. It also costs least where
  it fires most often: a frame already waiting on the GPU pays almost nothing to be told how long,
  because that is a wait it was about to take in `get_current_texture` anyway.
- **The instrument prints its own price.** The reading prints the median period of the audited
  frames against the median period of the rest. A blocking poll is the one thing here that could
  become the cost it is measuring, and two medians beside each other are the only honest way to say
  it did not — on the machine reading them, rather than on the machine this was written on.
- **The number says which clock took it, and why that one.** `Costs::clock` is the verdict the
  deck's own startup probe earned — `Deck::clock`, new here — and not
  `Features::TIMESTAMP_QUERY`, which is what the platform says rather than what it does. The
  reading prints that verdict beside the figures, and prints *nothing probed this adapter* as
  itself rather than as a verdict.
- **A frame is a host-clock figure on any adapter.** GPU timestamps bracket work inside a command
  buffer, and most of a frame is not in one — the block in `get_current_texture`, the `egui` pass,
  `Queue::present`. So even where timestamps work the period is a host number; what timestamps
  would buy is a better `drained`, split by pass. On this machine they do not work: `Probe::new`'s
  calibration is failed by this adapter every time (ADR-0169, `docs/contributing.md` §1), which the
  reading now says out loud instead of leaving to be inferred.

**The three medians are kept and one of their sentences is corrected.** They answer a different
question — what this program's own code costs per frame, which is what a schedule of live regions
is built from and what `budget::PANEL_PASS` is checked against — and `Costs::say` prints them to an
operator. What changes is the line that summed them: *"the whole frame is a median X ms, so the
loop is spending Y% of a second drawing"* is now *"the CPU's three stretches are a median X ms …
that is not what the frame cost"*, with the block below it measuring the frame.

**The transport row is not changed here.** `view::Transport::frame_ms` is fed `Cost::whole` and is
documented as *"what one frame cost on the CPU"*, so it is honest about its own scope and wrong
about nothing — but the row draws it as `12.4/16.6 ms` against the refresh interval, and a
GPU-bound frame therefore draws as headroom. Which number that row should carry, and whether it
grows a second one, is the maintainer's and is not taken here.

## The second half: there is no third resolution

**This application has two resolutions and `swap::PROBE_RESOLUTION` was a third.** The maintainer's
statement, 2026-09-08:

1. **The final output size.** User-settable. The mix is composited **once** at it and every output —
   the Program bay's picture included — is that one render resized into whatever rectangle it is
   drawn in, *"otherwise somebody with several outputs pays for several renders"*. That is ADR-0247,
   and `Present::draw`'s letterboxing is the mechanism.
2. **The slot preview size.** Each deck cell's own render, sized from the cell — `karakuri`'s own
   `deck_a_preview_texture_is_its_cells_size_and_a_resize_frees_the_old_one` says a preview sized
   from anything but its cell is *"four to twenty times more texels than the audition needs, per
   frame, for as long as the deck runs"*.

`PROBE_RESOLUTION` was 1280x720, and its own doc argued *comparable matters more than absolute*.
The argument holds and the constant does not: a measurement taken at a size the application never
renders at is a number about a frame nobody draws, and it only looked harmless because `CANVAS`
happens to be the same constant. The maintainer's ruling is that it is
*leftover provisional code from the CLI era* and that it **becomes the slot preview size**.

**So the size is a parameter and the caller names it.**

- `swap::measure` takes an `at` and re-points the probe to it (`Probe::resize`, which keeps the
  calibration verdict — a second `Probe` could land on a different clock and make two slots'
  numbers incomparable).
- `HotSwap::set_measure_size` writes a size the **build worker** reads per build, packed in one
  `AtomicU64` so a worker cannot read a width from one frame and a height from the next. A worker
  measures what it builds, and the render thread is what knows the layout.
- **A resize does not write it.** The output size moving is not the same event as the size a
  measurement is *about* moving; whoever named the measurement size names it again.
- `Deck::set_measure_size` tells every slot the same size, so one probe answers for a whole deck and
  the slots stay comparable. `Deck::new` seeds it with the deck's **own output size**, so a caller
  that never names one — `karakuri-cli`, which has no previews — measures at a size it actually
  renders, and never at a third.
- `karakuri`'s window names its deck preview cell, once a frame, in `Engine::aim`, off the first
  aimed cell. The cell moves when a divider moves or a bay folds, so a size taken once at startup
  would measure every candidate of a session against whatever the window opened at.

**And this changes what the number means, which is not allowed to slide.** A slot measured at its
preview cell is a statement about an **audition**, not about the composited frame. The governor
prefers the estimate where it answers and falls back to the measurement where it does not
(ADR-0296), and it **sums** across slots — so a deck with one of each is adding an output-size
figure to an audition-size one and holding the total against one budget. That is what ADR-0015
exists to prevent, and it is forced by the code the moment the two sizes differ.

**What is forced, and what is not.** Forced: the two numbers are no longer about the same frame and
must not be compared or summed as though they were. Not forced, and therefore the maintainer's:
which repair. Three are available and none is obviously right — extrapolate the measurement to the
output size (which is exactly the area-ratio rule ADR-0266 measured at 2.0x–4.1x overshoot on the
shipped corpus and refused); stop budgeting on the measurement at all where an estimate is possible,
which makes `Unfit` mean unbudgetable rather than "fall back"; or state the budget per size.
**Nothing here rescales a measurement**: a rescaled number would be an inferred one handed out as
measured, which is the last thing P-0095 names.

**One thing had to move, and this is what and why.** `Engine::ask_to_prime` set the compute budget
from the two **measurements** — *"a number computed from the measurement rather than a constant"* —
and the governor spends the **estimate** wherever one answers. While both were 1280x720 that was one
currency by accident. It is not any more, and the test that proves it is this repository's own:
`the_budget_parks_a_deck_and_the_strip_carries_both_residencies` fails with the measurement narrowed
to a preview cell, reporting `governor: 45.07 / 18.26 ms live … OVER: live slots exceed the budget` —
an estimate of 45 ms at 1280x720 held against a budget built from a 12 ms measurement at 252x142, so
a single live slot is permanently over budget and priming is suspended for a reason that is
arithmetic rather than load.

So `ask_to_prime` now takes each slot's budget figure on **the same precedence the governor decides
on**: the estimate where it answers, the measurement where it refuses. That is not a new policy — it
is ADR-0296's, applied to the budget's own currency so that both sides of one comparison are about
one frame. **It is the smallest repair that makes the comparison mean anything, and it is offered as
that rather than as the answer**: the two alternatives above are still open and the choice is the
maintainer's.

**ADR-0296's precedence still reads correctly and its reason is now stronger.** *"Where the estimate
answers, it is the number"* was argued from ADR-0246 — a deck drawing into a 640x360 window budgeted
against 720p figures refuses priming on fragment work nobody is doing. At preview scale the
measurement is further from the frame than it was, so preferring the estimate matters more. What
ADR-0296 does not say, and now needs to, is what happens to a **sum** that contains one of each.

## Alternatives rejected

**Time the frame's passes with GPU timestamps.** It is the number this would most like to have and
it is not reachable: this adapter advertises `TIMESTAMP_QUERY`, fails `Probe::new`'s calibration
every time, and returned literal zeros for a load that cannot take zero time — ADR-0068, ADR-0169
and P-0095 are all the record of that. Building the path anyway would ship an arm nothing here can
exercise, and *falling back silently is the failure P-0095 exists to prevent*.

**Poll on every frame.** It is the accurate version of `drained` and it destroys the thing it
measures: the CPU stands still until the GPU catches up, on every frame, which is the one pattern
the frame path may not make a habit of. Auditing at an interval keeps the loop pipelined 99% of the
time and puts the cost of the other 1% in the reading.

**`Queue::on_submitted_work_done` instead of a blocking poll.** Non-blocking and therefore free, and
useless at this resolution: the callback fires when the device is next polled, so the number it
yields is quantised to a frame — which is the quantity being measured.

**Widen `Cost::whole` to include the wait.** One number would then answer two questions and neither
well. The three stretches are what a region's schedule is built from and the wait is not a region's;
`PANEL_PASS` is checked against a figure that must exclude it. A measurement widened to fit a second
caller stops meaning what its first caller reads it as. The wait goes into a *new* number instead,
and ADR-0015 is why the two are not summed anywhere.

**Keep `PROBE_RESOLUTION` and pass a size only where it matters.** It is the smaller change and it
keeps the defect: a default that is a third resolution is still a third resolution, and the callers
that did not opt in would go on producing numbers about a frame nobody draws. Seeding from the
deck's own output size gets the same convenience with no size the application does not render at.

**Rescale the measurement to the output size at the seam.** One line, and it is the area-ratio rule
under another name — ADR-0266 measured that at 2.0x to 4.1x overshoot for the shipped corpus and
`estimate` exists because of it. It would also hand a governor an inferred number wearing a
measurement's label.

## Evidence

Session 2026-09-08, Metal / Apple M4 Pro / macOS, `--release`, headless, through
`crates/karakuri-engine/examples/frame_cost.rs`, which is the harness these figures come from. The
material is the pair a bare `cargo run -p karakuri` opens on — `coil_vortex.kir` + `star_flares.kir`
at the L1's declared 10240 elements, four slots — and **not** the workspace's reference workload,
which is `examples/drift_cloud.kset` at 1280x720 (`docs/contributing.md` §1, ADR-0270).

**GPU timestamps are advertised here and are not usable.** `gpu.timestamps` reads `true` — the
adapter offers `TIMESTAMP_QUERY` — and `Probe::new`'s calibration demotes it: every `Measurement`
this run produced is `HostWallClock`. That is the same verdict ADR-0169 records for Metal (20
fallbacks in 20) taken again on the day.

**One frame of a four-slot deck at 1280x720**, 120 frames with the first 20 discarded, two runs:

| | run 1 | run 2 |
|---|---|---|
| CPU — recording the four slots, the composite and the submission | 1.295 ms | 1.519 ms |
| GPU — submit to a drained queue (`Device::poll`) | 3.204 ms | 3.240 ms |
| the frame's period | 4.503 ms | 4.785 ms |

**The GPU is two to two and a half times the CPU, and a reading that stops at the submission reports
29% to 32% of what the frame cost** — on material that is not GPU-heavy by any standard. That is the
whole of the item: the panel's three medians are the 1.3 ms, and nothing in this repository could
see the 3.2 ms.

**What `Deck::measure_slots` costs at each of the two sizes**, four cold slots, calibration
included, same two runs:

| taken at | startup | per slot |
|---|---|---|
| 1280x720 — the old `PROBE_RESOLUTION` | 235.0 / 180.8 ms | 1.35 – 1.49 ms |
| 252x142 — a deck preview cell | 84.1 / 91.5 ms | 0.95 – 1.18 ms |

**Startup falls to between a third and a half of what it was**, and the run-to-run spread on the
startup figure is wide enough that the ratio is what to read rather than either number. **The
per-slot number does not fall the way the area does**: a twenty-fifth of the texels buys about a
third off the millisecond, because most of a measurement here is the invariant term plus the
submit-and-poll floor — which is ADR-0266's `a + b·area` read from the other end, and is exactly why
an area ratio overshoots. A preview-scale measurement is not a scaled-down output-scale one and must
not be treated as one.

**And what the governor then does with it**, at each size, with the budget set the way
`Engine::ask_to_prime` sets it — `committed.ms + warming.ms / 2.0`, off the **measurements**:

```
at 1280x720: governor: 4.29 / 2.14 ms live [host clock] — 4 refused, budgeted on the measurement
at 252x142:  governor: 3.33 / 1.61 ms live [host clock] — 4 refused, budgeted on the measurement
```

Both sides moved together here because `estimate` refuses this material outright, so every slot fell
back to its measurement and the comparison stayed within one scale. **The mixed case is real and is
in this repository's own suite**: `karakuri`'s
`the_budget_parks_a_deck_and_the_strip_carries_both_residencies` produces
*"3 estimated at 1280x720, 3 corrected for a floored rung, 1 refused, budgeted on the measurement"*
— one report, both bases, and a budget derived from measurements taken at a size that is now not the
estimates'.

**The windowed reading could not be taken in this session, and this says so rather than reporting a
number.** `karakuri`'s surface answers `CurrentSurfaceTexture::Occluded` on every attempt — nobody
can see the window — so `Missed::Idle` is correct, the loop stops asking for frames, and zero frames
are ever drawn. Two runs, `Missed::Idle` twice each, `0 frames drawn`. The instrument in `main.rs`
is therefore verified by its unit tests and by compilation, and its numbers are owed by the next
session that can put a window on a screen.
