---
id: 0210
title: A declared cost is one panel pass, written down and held against the run
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0091]
tags: [ui, perf]
---

# A declared cost is one panel pass, written down and held against the run

## Context

[ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md) has
two halves. The first — *a panel with nothing changing on it is paid for once and not again* — is
built and asserted ([ADR-0165](0165-the-repaint-decision-is-one-closed-list.md),
[ADR-0193](0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)).
The second is *what must be live is named, and each named thing declares **two** numbers: what its
update costs, and how stale it may get* — and only one of the two existed.

`View::animating` answered a staleness and nothing else: `ROLL_STALENESS`, 33.33 ms, while anything
in the mixer bay is pending and the bay is laid out
([ADR-0190](0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md),
[ADR-0206](0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)). **No region declared
a cost.** ADR-0190 *measured* what the roll costs — roughly 5500 allocations and 6.8 MB a second —
and wrote the number into a record rather than having the region announce it. And **neither
schedulability condition was checked anywhere**, though P-0072 states both and states what is done
with them: *"They are sums over the named regions, so a test asserts them rather than a stage
discovering them."*

Neither gap was a decision waiting to be taken. Both were work. What needed deciding was smaller and
is this record: **where a cost comes from, and how it is kept honest.**

### What a region is here, and how many of them there are

A **region** is a node of the console's arrangement, by the name every surface addresses it by
(ADR-0156, ADR-0159) — which is what makes ADR-0193's *is this laid out* a question `Layout::visible`
can answer, and what a scheduler would need in order to know which rectangle it was spending on.
`view::REGIONS` names thirteen.

**One of the thirteen is live, and that is fewer than the plan says.** `docs/roadmap.md` has read
*four* — the picture, the beat grid, the mixer's readouts and whatever is pending — and counted from
the source three of those four declare nothing and two are not this principle's business at all.
The picture is the engine's output, *"already accounted for by the governor, and counting it here
would count it twice"*; the beat grid moves because the panel is being redrawn for something else
rather than because anything decided it must, which is precisely what
[P-0077](../principles/0077-continuous-motion-is-how-a-stopped-panel-announces-itself.md) is still
waiting for; the mixer's readouts change when a hand changes them, and *what the operator does costs
what it costs*. What is left is the mixer bay, with three presentations inside it at one rate.

## Decision

**A live region declares `Declared { region, cost, staleness }`; the cost is `PANEL_PASS`, one
constant every region declares; it is a number somebody wrote down and nothing reads at runtime; and
it is kept honest twice — asserted structurally without a clock, and reported against a measurement
by the one program that has a window.**

`crates/karakuri-console/src/budget.rs` holds the type and the four numbers. `View::declares`
answers with the declarations and `View::animating` becomes the soonest staleness out of them, so
every existing caller is unchanged and there is one derivation rather than two.
`crates/karakuri-console/tests/schedulable.rs` is the arithmetic.

### The cost is a whole panel pass, and every region declares the same one

`egui` is immediate mode. There is no retained tree, so *redraw the mixer bay* is not an operation
this console has — **what repaints is the panel and not the chip**, which ADR-0188 and ADR-0190 both
already state as the honest cost of the roll. So the price of servicing any one region's deadline is
the price of the whole immediate-mode pass, and a per-region figure would be a number describing work
this console cannot do.

That makes it a **deliberate over-declaration**, which is the direction ADR-0190 already chose for
the staleness beside it: *"A presentation declares what it needs; what the panel can afford is
decided elsewhere … a deliberate over-declaration that a scheduler can refine downwards."* It stops
being one on the day a region can be drawn once into a texture and composited after — P-0072's own
remedy for a region that cannot meet the second condition.

**1.26 ms**, taken on 2026-08-28 over five runs of `examples/panel.rs` on an Apple M4 Pro at
1440x900 logical, host clock, debug profile with dependencies at opt-level 3, nothing touching the
window. It is `ui + textures + buffers + record` — the immediate-mode pass, the panel's own texture
and geometry uploads, and the recording of its render pass. The five read per-frame medians of
1.047, 1.224, 1.259, 1.281 and 1.340 ms; this is the middle.

**Three numbers on that frame are deliberately not in it.** The engine pass, because the picture is
the governor's. The vsync wait, because a frame blocked in `get_current_texture` is not paying for
anything. And the submission — about 2.4 ms, larger than the whole declared cost — because it
carries the engine's half of the frame as well and is paid whenever anything is submitted, so
charging it to a region would charge a region for a frame it did not ask for.

### It is a constant, and this is where that was already settled

[ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md) decided it before there
was anything to declare: *"a measured schedule reorders itself with the machine's noise, so the same
state behaves differently frame to frame for reasons the operator cannot see, and when a declaration
is wrong it is wrong somewhere a person can read."* Nothing reads `PANEL_PASS` on a frame. The only
readers are a test and a printed sentence.

### How it is kept honest, which is the part that needed deciding

**Two guards, because the number has two ways of stopping being true and no one mechanism catches
both.**

**The shape is asserted, with no window and no clock.** `tests/schedulable.rs` holds every
declaration's cost to `PANEL_PASS`, so a per-region figure cannot be invented while the panel is
still redrawn whole — which is the failure mode that arrives by looking *more* precise. Run against
its defect: a region declaring 200 µs of its own fails it.

**The magnitude is reported, by the one program with a window.** `examples/panel.rs` already
measures the pass and already holds `WRITTEN_ALLOCS` against the run it has just taken, for a reason
written into that file: a number in prose reads as current forever, and this repository has watched
exactly that happen for two commits. `PANEL_PASS` gets the same treatment and the same 2x band, and
the reading says either *still one this window produces* or ***the declared cost is stale***, with
what to do about it.

**It reports rather than fails, and that is a decision.** An assertion would be a gate that needs a
window, a device and three seconds of nobody touching it — none of which `cargo test` can reach — on
a machine whose own readings move by a factor of two under load and whose five runs above spread
1.05 to 1.34. *Flaky is worse than broken* (`docs/contributing.md` §1), and a gate nobody can keep
passing is a gate that gets deleted.

### The two conditions are a test, and the numbers they are asserted with

`tests/schedulable.rs` sums over what the view declares, at the most the console can ever declare —
every strip pending in all three ways, every region laid out.

| | Asserted | Read | Against |
| --- | --- | --- | --- |
| capacity | `Σ (cost / staleness) ≤ budget / frame interval` | **0.0378** | 1.0 |
| indivisibility | `max(cost) ≤ a small part of the budget` | **1.26 ms** | 4.17 ms |

**`BUDGET` is the console's only written-down budget and it is the whole frame's.**
`view::Transport::budget_ms` is *"what a frame has to fit in"* — the `/16.6` the transport row draws
— and `examples/panel.rs` fills it from the display's refresh interval, because the surface is
`PresentMode::Fifo`. Nothing anywhere writes down a **panel's share** of that. So the conditions are
asserted in their most permissive form, and what that costs is that they cannot catch a panel which
fits the frame while leaving the engine nothing. It is fixed at the 60 Hz value rather than read from
the display, which is ADR-0164 in one number: *"the panel's absolute budget does not grow when the
frame lengthens. The extra time belongs to the engine."* `FRAME_INTERVAL` is the one that moves.

**`SMALL_PART` is a quarter and the fraction is a preference**
([P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md)). Forced: well under
a half, because *most of the budget* is the failure the clause names. Chosen: a quarter. A tenth is
1.67 ms against a pass measured at 1.26 whose runs already spread by 0.29 — a rule that passes by
less than the measurement moves by is a rule about this laptop. A quarter leaves a factor of 3.3.

### The alternatives

**Measure the cost at runtime and let the schedule read it.** ADR-0164 rejected this before there
was a schedule, on the argument that survives having one: the same state would behave differently
frame to frame for reasons nobody can see. It is also not available — GPU timestamps do not work on
either backend this project is developed on (ADR-0169), and a host clock cannot resolve a region
whose update is microseconds.

**A per-region cost.** The tidier factoring, and it is what P-0072's arithmetic assumes: a scheduler
choosing between two regions wants to know that one is dearer than the other. It lost on being
**false today** rather than on being expensive. There is no way to redraw one region of this panel,
so a smaller per-region number would be a more precise-looking figure for work that does not happen,
and the sum over two such regions would understate a frame that redraws both by redrawing
everything. It arrives with the texture cache, and `every_declared_cost_is_one_whole_panel_pass` is
what has to be changed on purpose that day.

**Declare the cost in allocations rather than milliseconds** — the one unit this repository can
actually count steadily. It is a real argument: the allocation count read **524 in every one of the
ten runs behind this record**, taken in two batches half an hour apart, while the panel-draw median
across the same ten moved from about 1.36 to 1.26 ms and the whole-frame median by a fifth. It lost on the arithmetic. `cost / staleness ≤
budget / frame interval` needs a time on both sides; a budget in allocations is a budget nothing else
in this repository is stated in, and P-0072 is explicit that a tolerance is *in milliseconds* for a
reason. The stability is kept where it belongs: `WRITTEN_ALLOCS` is still the allocation guard, and
the two verdicts sit beside each other in the reading.

**Cite the cost the way `room::size` cites the mock.**
`tests/transcribed_constants_cite_the_mock.rs` is this crate's other pattern for a number that must
stay true, and it is a strong one — 72 constants, each held against the stylesheet declaration its
own doc comment names. It does not transfer. That guard works because the stylesheet is a *document*
that can be resolved and that moves independently; a cost has no document to cite. The only source is
a measurement, so a citation guard would check that a sentence exists rather than that a number is
true — which is the failure it was written to prevent, one level up.

**Assert the measurement instead of reporting it.** Above: a gate that needs a window and a device,
on a machine that swings by a factor of two.

**Put the two sums in `src/`.** P-0072 says a test asserts them *rather than a stage discovering
them*, and the reason is not tidiness: a panel that discovered it was over budget would discover it
during a performance. A `fn schedulable()` in the crate would also be the first half of a scheduler,
written with nothing to schedule.

**Name a panel share of the frame budget and assert against that.** It would make the first condition
say something sharper than *the panel may have the whole frame*. Not taken here: nothing in the
repository writes such a share down, and inventing one inside the test that noticed it was missing is
how a number nobody decided becomes a number everybody cites. It is named as owed instead.

## Consequences

- **P-0072's *Where it holds* changes on both counts and was re-measured rather than reworded.** One
  region now declares a cost as well as a staleness, and both conditions are checked. The panel's own
  reading was re-taken on 2026-08-28 over five runs: 56.7 to 58.7 frames a second in four of them and
  37.0 in one, and 19.6 to 24.9% of a second drawing in those four against 14.3% in the odd one — at
  the same **524** allocations a frame the nine runs of 2026-08-26 read. The allocation count did not move and the milliseconds did, which is the
  machine rather than the panel and is the swing that paragraph's own last sentences warn about.
- **`View::animating` is now derived and every caller is unchanged.** It is `declares(…).map(…).min()`
  — the soonest staleness — so `tests/parked.rs`, `tests/armed.rs`, `repaint::Change::Animating` and
  the example's two call sites carry on saying what they said.
- **Nothing arbitrates, and nothing needs to.** No scheduler exists; with one region declaring there
  is nothing to choose between, and choosing between one region and nothing is an abstraction with
  one call site. What P-0072 still has unbuilt is the policy, not the inputs.
- **P-0077 is untouched.** Its forced clause — something is moving continuously while the console is
  live, and a scheduler may not stop it — has no holder still: the mixer's declaration runs only
  while something is pending, and the beat grid moves without declaring. The scheduler that would
  have to refuse the economy is what it is waiting for, and this record does not bring one.
- **The console's budget has no panel share, and that is now written down somewhere.**
  `budget::BUDGET` says so at the constant, which is where the next person to want the first
  condition to bite will be standing.
- **`DRIFT` in `examples/panel.rs` has a second reader.** Its doc said outright that *"two verdicts
  about one pass is one more thing to keep passing for nothing"* — true of the bytes, which move with
  the allocation count, and not of a cost, which is a different claim and the one a schedulability
  condition is asserted against.
