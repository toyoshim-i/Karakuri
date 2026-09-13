---
id: 0349
title: The chain is the frame's, so it reads the session clock and is charged against the frame's budget
status: accepted
date: 2026-09-13
supersedes: []
superseded_by: []
principles: [0091, 0092]
tags: [engine, master-chain, governor, estimate, clock, m5]
---

# The chain is the frame's, so it reads the session clock and is charged against the frame's budget

## Context

[ADR-0340](0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md) says a chain
slot's L5 reads `t`, `beats` and `dt` (§1) and that a chain's cost is the sum over its slots of
`ops_per_fragment` against the frame's area at the largest enabled output's size, charged against
the frame beside the decks (§5). Neither sentence says where either number comes from, and on
2026-09-13 neither had a caller: `Present::set_chain_clock` was written and called by nothing, so
every chain slot ran at `Clock::default()` — `t`, `beats` and `dt` all zero — and
`Present::chain_ops_per_fragment` was read back by a test and spent by nobody.

Three facts constrain the answers.

**A chain has no Set.** A Set's `t` is its own step count times its own `dt`, kept per slot so that
a Set loaded mid-session starts at zero. The chain is one level out from every Set and one level in
from the tone map: there is no per-slot count for it to read, and four decks running at four
different local times have no common one to offer it.

**`estimate.rs` cannot price a chain.** It fits `a + b·area` through two probe draws at reduced
resolutions, which means submitting and waiting twice — the thing that may not happen on the frame
path ([P-0091](../principles/0091-cost-is-known-before-it-is-paid.md),
[ADR-0255](0255-three-clocks-run-at-once-and-a-slower-ones-work-never-lands-on-a-faster-one.md)) —
and a chain changes on a press. What a chain does have is what `estimate` does not: an exact static
op count per texel, computed by `karakuri_ir::cost` before anything is built
([ADR-0252](0252-the-language-is-bounded-so-a-price-can-be-computed-before-anything-is-built.md)).

**There is one measurement.** The three shipped procedures in one chain: 3947 ops per fragment,
0.98 ms at 1280x720, taken on 2026-09-10 on a host clock on one machine.

## Decision

**The master chain is a citizen of the frame, so `frame::compose` hands it the frame's clock and
charges the frame's governor what it costs — and no host does either.**

### 1. The clock is the session clock, and `dt` is the fixed simulation step

`Deck::chain_clock(steps)` is `t` and `beats` of the session oscillator *after* this frame's
`steps`, which is the instant every Live slot's own last substep lands on, and `dt` is `set::DT`.
All three come from the `tick` this frame was committed with and none from a wall clock
([P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)), so a replay reads the
same three numbers the performance did. `seed_salt` is zero: an L5 reads no `seed`.

`dt` is the fixed simulation step and not `steps * DT`, which is
[ir-spec.md](../ir-spec.md)'s *On `dt` and simulation time* applied without an exception — `dt` is
the same number on an L5 as on an L1, and a procedure moved between the two layers reads one
quantity rather than two. The limit it leaves is real and is named in *Consequences*.

### 2. `compose` is the one caller, and it writes both

`frame::compose` writes the chain's clock and the deck's chain charge in the two statements after
the commit closure returns, beside the tone map's own write. The GUI, the CLI's live loop, the
CLI's replay and the offscreen renderer all reach the chain through `compose`, so all four are
correct by construction and none of them mentions either quantity.

`Present::set_chain_clock` therefore takes `&self`, as `Present::set_tonemap` does and for the same
reason: it is a `queue.write_buffer` into buffers sized at build. The clock is kept beside the
slots in a `Cell` so that a repack of a whole uniform block — which is what a parameter write is —
carries the clock that is running rather than a default.

### 3. The price is one rate, calibrated on the one measurement, and it is named as such

`estimate::chain_ms(ops_per_fragment, target)` is linear in both terms, at the rate
`CHAIN_REFERENCE_MS / (CHAIN_REFERENCE_OPS · area(CHAIN_REFERENCE_SIZE))`. The three constants are
the measurement above, so the resolution it was taken at is part of the answer
([ADR-0303](0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md))
and re-taking it is editing three named constants rather than finding a magic number.

`Deck` holds the chain's `ops_per_fragment` and derives the milliseconds against its own current
size at `govern` time, so a resize moves the charge with nothing else written.

### 4. The governor reserves it ahead of every slot

`Governor::set_chain_ms` reserves the chain out of the compute budget before any slot is
considered. `Report::spendable_ms` is what is left, `Report::headroom_ms` is measured from it, and
`Report::chain_ms` reports it. It is **not** summed into `Report::committed_ms`, which still means
the Live slots and only them, and it is charged to no deck — a badge on a deck cell that moved
because somebody added a chain effect would be a reading about the wrong thing.

A chain can put a deck over budget on its own, and that is a warning like any other: the governor
suspends priming and takes nothing off air
([ADR-0054](0054-the-governor-budgets-from-the-probe-and-never-touches-a-live-slot.md)).

## Alternatives rejected

**Give the chain its own clock, counting frames from when the chain was installed.** It is what a
Set has, and it makes an effect start at zero when it is added, which is what an operator dropping
a delay onto the chain would expect to see. Rejected on
[ADR-0227](0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md)'s reading:
the chain is one, at the master, and is not any deck's material. A clock of its own would be a
fourth thing to record, restore and replay — `Record::MasterChain` carries slots and nothing else —
and a chain reordered or a slot removed would move the clock of every other slot with it.

**`dt` as `steps * DT`, the frame's own advance.** It is the number an L5 would need to decay by a
rate per second, and a chain slot runs once per frame while an L1 runs once per substep, so the
fixed step is not the interval between two runs of a chain slot. Rejected because `dt` is one
quantity across the language and splitting it by kind makes a `.kir` mean different things in
different slots. The frame's advance is derivable from the difference between two frames' `t` where
a procedure wants it; the language has no way to keep that across a frame yet, and that is the
limit rather than a second `dt`.

**Probe the chain and fit it, as a Set is fitted.** The better instrument, and it is the one
`estimate.rs` already is. Rejected on when it would have to run: a chain changes on a press, a fit
is two submissions and two waits, and the frame path may take neither. Moving it to a worker is
possible and is the same gap ADR-0296 already records for a swapped-in Set; it is not bought here.

**Charge the chain to the decks, pro rata or to the first Live slot.** It needs no new field and the
badge machinery already exists. Rejected because the number would then be wrong about the thing it
labels: a deck's badge would move when nobody touched that deck, and a deck taken off air would
appear to make the chain cheaper.

**Charge nothing until the rate is measured properly.** Honest, and it is what stood before this
record. Rejected because a chain that costs 0.98 ms and is budgeted at zero is the governor
admitting priming into headroom that does not exist, which is the failure the governor exists to
prevent (P-0091). One rate from one measurement, named as one measurement, is a statement a reader
can check; silence is not.

## Consequences

- **`crates/karakuri-engine/src/estimate.rs`** gains `chain_ms`, `CHAIN_REFERENCE_OPS`,
  `CHAIN_REFERENCE_SIZE` and `CHAIN_REFERENCE_MS`.
  `tests/master.rs::the_three_shipped_procedures_price_the_chains_rate` asserts on a GPU that the
  three shipped procedures still cost `CHAIN_REFERENCE_OPS`, so a re-weighting of
  `karakuri-ir`'s per-builtin weights fails there rather than silently moving every chain's price.
  The tap weights themselves are not re-weighted here; that is the measurement `cost.rs` names for
  whoever takes it.
- **`crates/karakuri-engine/src/governor.rs`**: `Governor` gains `chain_ms`, `set_chain_ms` and
  `spendable_ms`; `Report` gains `chain_ms` and `spendable_ms`, and its `Display` line gains a
  `+ N.NN chain` clause. `Report::headroom_ms` and `Report::over_budget` are now measured against
  the spendable budget rather than the whole one, which is a behaviour change to every caller.
- **`crates/karakuri-engine/src/deck.rs`**: `Deck` gains `chain_ops_per_fragment`,
  `set_chain_ops_per_fragment`, `chain_ms` and `chain_clock`.
- **`crates/karakuri-engine/src/master.rs`**: `MasterChain::clock` is a `Cell`, and `set_clock`
  takes `&self`.
- **A chain slot cannot ask how far this frame advanced.** It has `t` and `dt`, and the frame's
  advance is `steps * DT`, which no ambient carries. A frame effect whose amount should scale with
  a dropped frame cannot express it. This is the price of §1's second half and is recorded rather
  than worked around.
- **The rate is one machine's reading and is linear.** It says nothing about a chain whose slots
  are bandwidth-bound rather than issue-bound, and `karakuri-ir` prices a cached tap about four to
  one high on the machine the measurement was taken on. A second measurement at a second size, or
  at a second chain shape, is what would turn the rate into a fit.
- **Nothing charges the chain outside `compose`.** A caller that installs a chain and draws it by
  hand — which is every GPU test in `tests/master.rs` — is charged nothing and told no clock. That
  is correct for a test and would be wrong for a fifth host; the rule is that a host composes.
