---
id: 0360
title: over_budget stays the Live slots' sum, and no deck total is drawn
status: accepted
date: 2026-09-14
supersedes: []
superseded_by: []
principles: [0091, 0095]
tags: [engine, governor, console, measurement, m6]
---

# `over_budget` stays the Live slots' sum, and no deck total is drawn

## Context

`Report::over_budget` is `committed_ms > budget_ms - chain_ms`, where `committed_ms` sums the
budgeted cost of the slots whose requested residency is Live. Since
[ADR-0269](0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md) every
slot is stepped and drawn into its preview cell whatever its residency, so the three off-air slots'
step and draw are in no sum. [ADR-0313](0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)
supplied `Report::deck_over_period` as the deck-level alarm and wrote that *"what `over_budget`
should say once the deck's total is computed … is the maintainer's"*; the old roadmap carried the
question as M5.14 item 3 and then as an M6 item.

[ADR-0356](0356-the-worker-estimates-what-it-built-and-an-estimate-is-a-fit-rather-than-a-number-at-one-size.md)
made a deck total computable for the first time: an estimate is a fit, so an off-air slot's cost
at its preview cell's size is `Estimate::at(cell)` and no draw. The question was put to the
maintainer with that option.

## Decision

The maintainer's, on 2026-09-14: **nothing new is needed.**

The instrument already exists in three readings, and they are enough:

- **Each slot's own band**, the dot the Mixer bay draws from the number the governor spent on that
  slot ([ADR-0298](0298-the-badge-is-the-band-of-the-number-the-governor-spent-and-how-it-was-taken-crosses-the-seam-undrawn.md)),
  which since ADR-0356 is an estimate at the output's size for every slot, cold or swapped in.
- **The rate**, which says whether the display is being kept.
- **The CPU figure**, which says what this program's own code costs and is labelled as the CPU's
  ([ADR-0359](0359-the-transport-rows-frame-figure-stays-the-cpus-and-says-so.md)).

So `over_budget` keeps its meaning — *the on-air material alone does not fit one frame* — and is
drawn nowhere in the console; `deck_over_period` stays the measured alarm on the CLI's status
line. No deck total is summed, no headroom figure is drawn, and no record is changed.

## Alternatives rejected

**Sum every slot at the size it is drawn at.** Live slots at the output's size, off-air slots at
the preview cell's, plus the chain — a predicted deck total that the measured period could be
checked against. Rejected as a number nobody would act on: the governor never touches a Live slot
(ADR-0054), a total over the budget would warn where the dots already warn per slot and the rate
already shows the drop, and a second warning about the same frame is a second thing to keep true.

**Draw `Report::headroom_ms` on the transport row.** The budget-side prediction beside a CPU
measurement — two currencies on one row, which is what ADR-0359 keeps off it.

## Consequences

- The roadmap's last M6 item is settled by this record; M6's exit is its three commands.
- `Deck::frame_period_ms` reading slot 0's watchdog, and the GUI calibrating the compute budget
  from slots 0 and 1 (`bridge/engine.rs`), stay as they are: facts, not defects, under this
  decision, and named here so they are not rediscovered as one.
