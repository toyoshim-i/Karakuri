---
id: 0186
title: One operation names one of three residencies
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: []
principles: [0090]
tags: [vocabulary, decks]
---

# One operation names one of three residencies

## Context

The vocabulary landed with two rows for a deck's residency
([ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)):

```rust
SetOnAir   { deck: u8, on_air:  bool }  // "Put a deck on air, or take it off"
SetPriming { deck: u8, warming: bool }  // "Ask a deck to warm off air"
```

**Two booleans are four combinations for three states, and both `false` destinations were
undefined by the vocabulary.** Nothing in `karakuri-operation` said where `on_air: false` left a
deck, and nothing said where `warming: false` left one. The answers exist — `space` moves a slot
between Live and Allocated (and puts a priming slot on air at whatever `t` it warmed to), `w` moves
it between Priming and Allocated and is refused outright while the slot is Live — and they exist in
exactly one place, `karakuri-cli`'s key handler around `toggle_on_air` and `toggle_priming`.
**Knowledge that lives in one surface is what the vocabulary exists to take out of the surfaces**:
a MIDI map, an MCP call and a panel chip each had to re-derive it, and nothing would have caught
two of them deriving it differently.

The engine has never had this problem. `Deck::set_residency(slot, Residency)` is one call naming
one of three, and `karakuri_engine::deck::Residency` is the enum it names them from.

[ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md) is where this
surfaced as a consequence rather than a theory: *"The tally is two operations for three states."*
The first control that had to draw residency could not tell what it would be pressing.

## Decision

One variant, and the vocabulary owns the list:

```rust
SetResidency { deck: u8, residency: Residency }  // "Put a deck on air, prime it, or take it off"

pub enum Residency { Live, Priming, Allocated }
```

`Residency` is this crate's mirror of `karakuri_engine::deck::Residency`, carrying a doc that names
the engine type it mirrors, exactly as `BlendMode`, `Sync`, `Tonemap`, `Curve` and `WipeKind`
already do. It is a sixth copy of a list that exists elsewhere and that is the standing price:
**a vocabulary that names a destination must own the values a destination is drawn from**, or it is
back to toggles. `karakuri-operation` still has **no dependencies at all** — `std` only — which
mirroring rather than importing is what preserves.

This is [P-0090](../principles/0090-a-surface-offers-it-never-decides.md)
applied, not amended: *an operation says what it wants, never which way to move*. **No new
principle.** The rule already covered this; what changed is that P-0074 cited `SetOnAir { deck,
on_air: bool }` as its worked example of naming a destination, and that citation is now the wrong
one — a `bool` that cannot name the third state is a weaker example of the rule than the variant
that replaced it. The citation was corrected in place, which is wiring rather than revision, and
the number is unchanged because the rule is.

## What loses, and it is a real argument

**The two rows encoded a distinction the merge dissolves into prose.** *Put a deck on air* was
`op-when: immediate` and *Ask a deck to warm off air* was `op-when: request`, and that split is
true of the engine rather than decorative — `karakuri-engine/src/deck.rs`, "Residency: requested
and effective":

- The governor may hold a slot **below** its request and never above.
- **Live is never demoted**, so requested Live and effective Live are the same set of slots: asking
  for Live is answered.
- A slot asked to prime with no room is **parked** — effective Allocated over a request of Priming.
  The request is not refused and not remembered; it is recomputed on every `Deck::govern` pass, so
  it takes effect the first pass after the deck empties, with no operator action.

After the merge, one operation carries both timings. The page says
`immediate; priming is a request` — free text on the `op-when` base class, which 32 other rows
already use and which *console setting* is precedent for — and the row's prose carries what the two
rows used to carry between them. **The cost is that a reader now has to read the row to learn that
half of it is a request**, where before the badge said it at a glance and a surface could branch on
the variant. A tally chip that wants to draw *asked for, not yet granted* has to read the deck's
effective residency back and compare, rather than knowing from which operation it sent.

That was judged the smaller loss. The distinction is a property of **one of three values**, not of
two operations: `Live` is honoured, `Priming` may park, `Allocated` is immediate. Splitting the
vocabulary along it puts a governor's behaviour into the shape of the type, which is the engine
leaking into a crate that must not be engine-shaped, and it buys that at the price of leaving two
destinations unnamed.

## Alternatives rejected

- **Keep the two booleans and document the `false` destinations.** A comment cannot be routed
  into. `SetOnAir { on_air: false }` and `SetPriming { warming: false }` are still two names for
  what is one destination, and a MIDI map with a pad per state still cannot say *allocated*.
- **Three variants — `PutOnAir`, `Prime`, `Hold`.** No toggles, no undefined destinations, and it
  splits one value across three rows of the manual. The engine's setter takes a value; a surface
  wanting a three-way control would have to know which of three names to send rather than which of
  three values, and `SetBlendMode` would be the only reason not to split *that* into three too.
- **Carry `karakuri_engine::deck::Residency` directly.** It is the same three names, and importing
  it costs `karakuri-operation` its charter: a dependency on the engine makes every surface that
  parses a MIDI map depend on `wgpu`. ADR-0180 settled this for the whole crate.

## Consequences

- **45 operations, not 46.** The manual's own count line, the in-file floor in `lib.rs`, the floor
  in `tests/the_manual_and_the_vocabulary_agree.rs`, `README.md`, `architecture.md` and the roadmap
  all state it and all moved. The two floors are floors rather than counts and were lowered here
  for the first time — that is what a merge does, and the comment beside each says so, because a
  floor lowered silently is a scan that can come back short.
- **48 of 195 ways in, recounted from the page** rather than adjusted by arithmetic: the merged row
  has four routes where the two rows had eight, and the union keeps `key` and `MIDI` built.
- **ADR-0185's open question is narrower, not closed.** *What the tally shows while the two
  disagree* is still a decision nobody has taken, but it is no longer *two operations for three
  states*. The disagreement has a name in this workspace already — `Deck::is_parked`, drawn by
  `karakuri-cli`'s status line as `park` and spelled out as *parked (asked to prime, waiting for
  room)* — so the console has a state to draw rather than one to invent.
- **`karakuri-midi`'s `Action::ToggleOnAir` / `Action::TogglePriming` are untouched**, as is
  `karakuri-cli`'s key handler. Nothing has migrated onto the vocabulary yet; this changes the
  target the migration aims at, which is the whole reason for landing the target before the moves.
  When `Action` does move, the two toggles become one operation each surface names a destination
  for, and the map's `on-air N` and `prime N` targets stay two ways in to one row.
