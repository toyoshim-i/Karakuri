---
id: 0266
title: Two rungs and a fit, because a frame is an invariant part plus a fragment part
status: accepted
date: 2026-09-06
supersedes: []
superseded_by: []
principles: []
tags: [engine, probe, estimate, performance]
---

# Two rungs and a fit, because a frame is an invariant part plus a fragment part

## Context

`f8f6f45` built the preparation slot's estimate and measured that it does not work yet.
`crates/karakuri-engine/src/estimate.rs` drew a Set once at 640x360, rewound it, restored the
viewport, and returned the measurement **scaled by the ratio of the target's area to the probe's,
clamped at 1.0, one rule for every topology**. `Estimate::area_ratio` was that factor and
`Estimate::ms` was the product.

The rule was chosen knowing it was wrong in one direction and shipped anyway, on the argument that a
single draw gives `a + b·A` at one area and no way to divide it, and that of the two extrapolations
available for the sum the area-proportional one is always the larger — so it is the one that
satisfies *round the estimate toward refusing*.

**The price was measured and it is not a rounding.** `examples/small_draw.rs` walked eight target
sizes with one `Probe`, interleaved, first pass discarded, on a `HostWallClock` — which
`docs/contributing.md` §1 says is what this machine's GPU timestamps demote to. `measured_small ×
area_ratio` against the same material at 1280x720, same probe, same run:

| material | topology | from 905x509 (2x) | from 640x360 (4x) | from 320x180 (16x) |
|---|---|---|---|---|
| `field_march` | `Fullscreen` | 1.13x | 1.48x | 3.11x |
| `speed_lines` @ max width | `Lines` | — | 1.46x | 4.96x |
| `soft_points` @ `point_scale` 0.0222 | `Points` | 1.54x | 2.58x | 9.00x |
| `soft_points`, shipped | `Points` | 2.01x | 4.06x | 15.3x |
| `speed_lines`, shipped | `Lines` | 2.05x | 3.90x | 16.6x |

The last two rows are the **shipped corpus**, and their overshoot is very nearly the area ratio
itself, which is what a flat curve produces. Against the console's five bands — green under 4 ms,
red about 12, purple over 16, and *purple is a slot that gets stopped* — `soft_points` measured
8.87 ms at 1280x720 and 9.00 ms at 640x360 and estimated as 36 ms. **Every `Points` and `Lines` slot
in `examples/` was stopped by this rule at any small size**, which is why the module was exported
from `lib.rs` and wired to nothing.

**The mechanism is that the whole sum was scaled.** A frame is three parts: the simulation over
`capacity`, the vertex stage per primitive, and the fragment stage over covered pixels. The first
two do not depend on the target's size. Scaling the sum scales them anyway, and for per-element
material at the reference capacity they are most of the frame — `soft_points` at `point_scale`
0.0222 is 9.0 ms of invariant term inside an 18.1 ms frame, and is above its sub-pixel floor at
every rung down to 320x180, so this is the `a` term rather than the floor.

**`Topology` looked like the fix and is not.** `docs/roadmap.md` said so under this very heading —
*a `Fullscreen` procedure's cost is the pixel count, a `Points` procedure is dominated by primitive
work, `Lines` is unmeasured* — and `node/renderer.rs` carried a comment claiming `estimate` asked a
three-way question of the field, which nothing ever did.
The third stage tracks area for **every** topology, because `point_rate` is a *fraction of the
render target's height* rather than a pixel count: a fullscreen pass covers the target's area, a
sprite covers `capacity × (rate × height)²`, a stroke its length times its width. What decides
whether a procedure *looks* invariant or *looks* area-proportional is its coverage,
`capacity × rate²`, and a param moves that at any time. The same `soft_points` sits on either side
of any rule keyed on the enum at `point_scale` 0.00556 and at 0.0222, and the measured `b` above is
real and positive for `Points`, `Lines` and `Fullscreen` alike.

## Decision

**The preparation draw is two draws, and the estimate is the affine fit through them.**

- **Two rungs, at half and at a quarter of the target's height** — a quarter and a sixteenth of its
  area, both at the target's own aspect ratio, because a Set's camera derives its aspect from its
  viewport and a rung at another one draws a different picture. `estimate::rungs` places them;
  `PREPARATION_RESOLUTION` is now what the rule *yields* at 1280x720 rather than a constant anything
  picks.
- **`b = (m_hi − m_lo) / (A_hi − A_lo)`, `a = m_lo − b·A_lo`, and the answer is `a + b·A_target`.**
  `estimate::fit` is pure arithmetic — no device, no queue — and is unit-tested against synthetic
  rungs built from a known `a` and `b`.
- **A negative term is a failed measurement and is reported, never clamped.** `b < 0` is
  `Unfit::FragmentTermNegative`; `a < 0` is `Unfit::InvariantTermNegative`. Neither is a quantity
  that can be below zero, so a fit that produces one measured something other than what it thinks —
  a tile-based renderer binning a quarter of a million primitives into too few tiles, or thermal
  drift between the two draws.
- **Both rungs must sit above ADR-0245's sub-pixel floor**, `1 / rate` rows. Below it a primitive is
  drawn at one pixel and dimmed rather than dropped, so its coverage stops falling: a rung there
  reads *high* against the model, drives `b` down, and makes the answer **undershoot**, which is the
  one direction *round the estimate toward refusing* forbids. `Unfit::RungBelowFloor` and
  `Unfit::NoRoomBelowTheTarget` are the refusals.
- **Where the floor is not knowable, the estimate says so and draws nothing.** `point_rate` is an
  expression evaluated per element in the vertex stage over attributes the CPU side never reads
  back, so `estimate` cannot compute `1 / rate` for a Set that draws `Points` or `Lines`, and
  answers `Unfit::FloorUnknown` without spending a draw. A caller that knows the smallest rate its
  material emits turns it into rows with `sub_pixel_floor_rows` and calls `estimate_above_floor`.
- **`Topology` decides one thing here and it is not the split.** A fullscreen procedure has no
  `vertex` block — which is how `karakuri_ir::check` infers the topology in the first place — so it
  writes no `point_rate` and has no primitive that can fall under a pixel. Its floor is one row.
  That is the whole of `estimate::floor_rows`.
- **`Estimate` carries the refusal rather than a number.** `Estimate::ms` is an `Option<f32>` behind
  `fit: Result<Fit, Unfit>`; `Estimate::method` and `Estimate::biased_high` say which clock answered
  and which way it is wrong. On this machine that is always `HostWallClock`, which brackets
  `queue.submit` and a `poll` and so carries a round trip the GPU never spent — roughly constant in
  the target's size, so it lands in `a` and is added to the answer once instead of being multiplied
  by the area ratio.

**No constant here comes from a number measured on this machine.** The form `a + b·area` is the
language's three stages; the floor is ADR-0245's arithmetic on a rate that is a fraction of the
height; the non-negativity of both terms is what the terms are. The rung placement is *chosen* and
is marked as chosen in the module doc, with what it costs and what it amplifies written beside it.

**This is still not wired to `Deck::govern`.** That decision is the maintainer's and this record
does not take it.

## Alternatives rejected

**Keep the area ratio.** It is one draw instead of two, it needs no floor, and it never undershoots.
It loses on the table above: for the shipped corpus the overshoot is very nearly the whole area
ratio, which puts every `Points` and `Lines` slot in the band that stops a slot. A governor acting
on it would refuse the material the instrument ships with, and an estimate that refuses everything
is not conservative, it is uninformative — the badge would say the same thing about a 3 ms slot and
a 30 ms one. *Round toward refusing* is a tie-breaker between defensible numbers, not a licence for
a number that is wrong by 4x in a known direction.

**Discriminate on `Topology`.** The obvious fix, written into `docs/roadmap.md` and into
`node/renderer.rs`'s comment, and the one somebody will re-propose because the enum is right there
and known at compile time. It loses because the enum does not carry the quantity that decides:
coverage is `capacity × rate²` and `rate` is a param. `soft_points` at 0.00556 is nearly flat in the
target's area and at 0.0222 is half fragment work — the same procedure, the same topology, opposite
sides of any rule keyed on it. A `Points` branch that assumed invariance would have *undershot*
`soft_points` at the higher setting by the same factor the area rule overshot it at the lower one,
and undershooting is the direction that drops a frame in front of an audience.

**One rung plus a fixed invariant fraction** — measure once, assume `a` is some share of the frame,
scale only the rest. Cheapest of all the fixes and it needs no second draw. It loses twice. The
share is not a property of the language: `field_march` measured 0.52 ms of `a` against 3.05 of
`b·A₇₂₀`, and `soft_points` at 0.0222 measured 9.0 against 9.2 — 15% and 49% of the same reference
frame. Any single number is therefore a constant taken from this machine's corpus, which
`docs/roadmap.md`'s *Numbers taken here are not a basis for a decision* rules out, and it would be
read as a property of the engine by the next person. And it is unfalsifiable in use: a wrong
fraction produces a plausible number rather than a refusal, so nothing would ever report it.

**Refuse to estimate at all, and rely on the ceilings.** `MAX_OPS_PER_ELEMENT`,
`MAX_OPS_PER_SPAWN`, `MAX_OPS_PER_FRAGMENT` and the fullscreen one already reject a procedure before
it is built, so the argument is that a second gate is not owed. It loses on what those numbers are:
none has ever been measured against a frame time, 4096 was chosen as an order of magnitude above two
spec examples, 16384 is that times four with nothing arguing the factor, and 512 stands in for
`capacity × sprite area × overdraw`, a quantity nowhere counted. **They are a backstop, not a
budget** — they catch absurdity and cannot see capacity or parameters at all, which is exactly what
decides a frame's cost. Refusing to estimate also leaves the risk badge undrawable, and the badge's
last band is a behaviour and not a colour: a slot over budget is stopped. Something has to produce
the number.

**Fit three or more rungs by least squares, and read the residual as a confidence.** Strictly more
information: a third rung the fit did not use is what says whether the cost is affine in the area at
all, and `examples/small_draw.rs` prints exactly that residual. It is deferred rather than refused.
Two rungs already cost 5/16 of a full-size draw against the single rung's 4/16, a third would take
it past a half, and the preparation draw is meant to be the cheap half of priming — *trialling
something extremely heavy does little harm* is the property it exists to have. The residual is also
not free of the machine: on a host clock with this much run-to-run spread it would report noise as
non-affinity. **What would revive it** is a working `GpuTimestamp` path, where the spread is small
enough for a residual to mean something.

**Clamp a negative term to zero instead of refusing.** It always produces a number and it is never
below the measurement. It loses because the number it produces is not an estimate of anything: `b <
0` means the two draws did not lie on one curve, and clamping hands a caller arithmetic it has no
way to distrust. `P-0095` is the same rule one level down — an instrument that cannot measure says
so rather than reporting a number.

**Guess the floor rather than refusing** — assume some smallest `point_rate` and place the rungs
against it. Rejected because every candidate is a number from this corpus rather than from the
language, and because the failure is silent in the dangerous direction: a floor guessed too low puts
a rung under the real one, which drives `b` down and makes the answer undershoot. A refusal that
names what it would need is recoverable by the caller that has it; a guess is not recoverable by
anybody.

## Consequences

- **`crates/karakuri-engine/src/estimate.rs` no longer has an `extrapolate`.** `fit`, `rungs`,
  `floor_rows` and `sub_pixel_floor_rows` are pure and need no device; `estimate` and
  `estimate_above_floor` are the two that draw. `estimate`'s signature is unchanged.
- **`Estimate` changed shape.** `ms: f32` and `area_ratio: f32` are gone; `fit: Result<Fit, Unfit>`,
  `rungs: Option<[Measurement; 2]>` and `floor: Option<u32>` are new, with `Estimate::ms`,
  `Estimate::method` and `Estimate::biased_high` as the accessors. `crates/karakuri-engine/src/lib.rs`
  re-exports `Fit` and `Unfit` beside `Estimate` and `PREPARATION_RESOLUTION`, because an `Estimate`
  cannot be read without them. Nothing else in the workspace constructs or reads an `Estimate`.
- **`estimate` answers `Unfit::FloorUnknown` for every `Points` and `Lines` Set**, without drawing.
  That is the honest state of what this side knows and it is a **gap this record leaves open**: what
  would close it is a Set that can report the smallest rate its renderers emit, which is a static
  analysis of the `point_rate` expression against its params' declared ranges and is not written.
  Until then the estimate is reachable for that material only through `estimate_above_floor`.
- **The shipped `speed_lines` pairing has nowhere to stand even with its rate in hand.** It ships
  `width` at 0.00139 — its own comment calls it *one pixel at 720 rows* — so its floor is 719 rows
  and the reference target is 720. `rungs` returns `Unfit::NoRoomBelowTheTarget`, which is the
  roadmap's own sentence turned into a return value: *the reference size is already at its floor and
  no smaller draw says anything new about it.*
- **Priming costs two draws.** A quarter and a sixteenth of the target's area: the **fragment** work
  goes from 4/16 of a full-size draw to 5/16, and the **invariant** work is paid twice instead of
  once — which for per-element material at the reference capacity is the larger half of the bill,
  and is the same fact this record is about. The whole-capacity uploads `Set::build` and
  `Set::rewind` pay are untouched.
- **The estimate is noisier per call than the number it replaced, and the module doc says by how
  much.** The prediction reduces to `m_lo + 5·(m_hi − m_lo)`, so noise in either rung is amplified
  about fivefold into the answer. **No ratio measured here is quoted as a finding**, and the reason
  is the instrument rather than the rule: driven on 2026-09-06 with the full-size truth interleaved
  between fits in one process, `soft_points` at `point_scale` 0.0222 measured 8.85, 16.47, 21.17 and
  28.00 ms **at the same size with the same parameters**, a spread `examples/small_draw.rs` would
  discard on its own `SPREAD_LIMIT` of 1.25x. What can be said from that run is structural and is
  the design working: about half the attempts on dense material came back
  `Unfit::FragmentTermNegative` rather than with a wrong number, and the fits that did land sat near
  the truth in both directions rather than at 2.58x above it. **A per-call answer is not yet
  trustworthy on this clock**, and that is the second reason this stays unwired — the first being
  that wiring it is the maintainer's decision.
- **`crates/karakuri-engine/tests/estimate.rs` asserts the two draws rather than the one.** The
  device-side properties are unchanged in kind and doubled in number: the probe left at the upper
  rung, the Set back at the caller's viewport and cold, both rungs at the sizes `rungs` placed, and
  a `Points` Set refused with nothing drawn and nothing moved.
- **`crates/karakuri-engine/src/node/renderer.rs`'s `topology` field carries a corrected comment.**
  It claimed `estimate` asked a three-way question of it — *`Fullscreen` and `Lines` cost the
  target's area and `Points` does not* — which nothing ever implemented and which this record says
  nothing should. The field keeps its full value because `Estimate` reports it.
- **`docs/roadmap.md`'s *The preparation slot is the measurement* loses its "this is where to
  resume" paragraph** and its `Topology` decides the extrapolation paragraph, and points here. What
  it still says is thin stays thin: how the extrapolation's confidence is expressed is now the
  spread above and is not settled.
