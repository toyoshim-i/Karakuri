---
id: 0195
title: The tally chip cycles from the request, so the parked case needs no case
status: accepted
date: 2026-08-27
supersedes: []
superseded_by: []
principles: [0074, 0076]
tags: [ui, decks, vocabulary]
---

# The tally chip cycles from the request, so the parked case needs no case

## Context

The mixer strip's residency chip was drawn before it was wired, on purpose:
[ADR-0190](0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
made it say that a request has not landed — the word rolls part of the way toward the residency that
was asked for and falls back, about once a second, and never lands — and `docs/roadmap.md` recorded
why that half went first: *"a control that could not yet say it is pending would look dead for
exactly as long as one commit."*

So what was left is the pointer. The blend chip is the worked example and the shape is settled by it
([ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)): the
console translates a gesture into an `Operation` and applies nothing, the harness turns it into a
`Record`, the record moves the deck, and the strip draws what the deck says.

**What is not settled by it is which of two values the cycle counts from.** The tally is the first
control on this panel that reads *two* — `Deck::residency` is what the slot is doing and
`Deck::requested_residency` is what it was asked to do — and they part on one state the engine can
produce: **parked**, asked to prime and held at allocated because the budget has no room. A chip
that cycles has to step from one of them, and on a parked slot the two answers are opposite.

## Decision

**A press emits `SetResidency { deck, residency }` naming the next of `live → priming → allocated →
live`, counted from the residency that was *requested*.**

The order is the mock's own, stated on every tally tooltip in `docs/manual/console.html` — *"one of
three residencies — live, priming, allocated"* — and it is the order `view::Tally::ALL` and
`karakuri_operation::Residency` are both written in.

### Counting from the request is what makes the parked case come out right with nothing in the code about it

A parked slot's request is `Priming`, so the next is `Allocated` — and `Allocated` **is** the
withdrawal of the prime request. The behaviour anyone would write a branch for falls out of the
ordinary arithmetic:

- the chip shows `alloc` and was asked for `prim`;
- a press asks for `alloc`;
- `Deck::set_residency(slot, Allocated)` writes the request, the governor has nothing left to hold
  back, and the roll stops because the two halves agree again.

That is exactly what
[P-0076](../principles/0076-a-surface-owns-the-affordance-never-the-authority.md) permits a surface
and marks the boundary of: *"a press on a control whose transition is pending may ask for the
withdrawal"* — **a control choosing which destination a press names, which is an affordance and not
a lock**. The chip refuses nothing; every destination is handed over and the engine decides. P-0076
was written with nowhere it held yet — *"nowhere yet, because nothing has tried it"* — and this is
the first control that tries it.

**It is also what `karakuri-cli` already does**, which is the check that the argument is about the
instrument rather than about this chip: `w` is `toggle_priming`, and it reads
`Deck::requested_residency` to decide which way to go, so a parked slot's `w` withdraws rather than
re-asking. Two surfaces reading different halves of the pair would disagree about what a press means
on precisely the slots where it matters, and *one vocabulary, four ways in* is the thing that would
be broken.

### What a MIDI map is offered is the three values, not the cycle

Unchanged from ADR-0187 and settled before it by
[ADR-0186](0186-one-operation-names-one-of-three-residencies.md): `SetResidency` names one of three
destinations and there is no *next* in the vocabulary to learn. The pointer is shown a cycle because
a pointer is one control on a 53-wide strip; a pad is offered `SetResidency { deck, residency: Live
}` and can say what it means. **Nothing MIDI is implemented by this change** — `karakuri_midi`'s map
still has its own actions, and moving it is the migration `docs/roadmap.md` lists.

## The alternative that lost

### Cycling from the effective residency — the one the chip is showing

It is the obvious reading of a cycling control: what you see is where you are, and a press takes you
to the next one. It is also what every other control in this bay does, because every other control
in this bay reads one value.

It loses on the parked slot, and it loses loudly. The chip shows `alloc`, so the next is `live`:
**a press meant to take a prime request back puts the deck on air.** That is the most expensive
mistake this panel can make — it is the operation the manual describes as the one that always
lands, since nothing demotes a Live slot
([P-0033](../principles/0033-the-governor-never-takes-a-live-slot-off-air.md)) — and it happens in
front of an audience, from the control whose whole reason for animating is to say *your request is
still waiting*.

It is worse than a wrong destination, too: it makes the request **unwithdrawable from this surface**
without going the long way round the cycle and through `Live` on the way. There is no press that
means *stop asking* at all.

**And the argument that saves it costs more than it saves.** A branch — *if the slot is parked, ask
for the withdrawal; otherwise cycle from what is shown* — gets the same behaviour and is a special
case for a state that is already fully derived (`Strip::pending`, which is an inequality and
deliberately not `Deck::is_parked`'s pair, so that a second kind of disagreement is drawn without
being taught). The branch would have to be re-derived the day a second pending state exists, and
the version with no branch would not. One rule that produces the right answer everywhere beats two
rules that agree today.

### Rejected earlier and recorded elsewhere, so only named here

**A lock in the panel** — forbidding every operation but the withdrawal while a transition is
pending — is
[ADR-0188](0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md) and
P-0076, decided before this chip had a pointer at all. **Three targets or a picker** instead of a
cycle is ADR-0187's measurement, and the residency words are wider than the blend's: `PRIM` beside
`ALLOC` alone is 85.125 against a 53-wide row.

## Consequences

- **`input.rs`'s rule 3 takes a fourth control and did not change to hold it** — the Outputs sink,
  a fader knob, the blend chip and now the tally chip, each asked the same way: the derivation that
  draws it, asked whether the point is on it, with nothing stored, and the caller that acts on the
  press asking the same function again rather than copying its answer. **The bay is still derived
  once per event** and asked for all three of its controls; a third derivation would be a third
  answer.
- **The console gained a conversion and `karakuri-operation` gained nothing.** `view::Tally` is the
  console's word for a residency and `karakuri_operation::Residency` is the vocabulary's, so
  `view::residency` is a three-arm match, and the cycle is `view::next` beside it — the same
  division ADR-0187 made for the blend: the vocabulary owns the values and whoever draws the control
  owns the order a pointer walks them in.
- **The chip is the first control on this panel whose target does not move with its own value.**
  The capsule is sized to the widest of the three words — `LIVE` 34.06, `PRIM` 37.59, `ALLOC` 44.53
  including `TALLY_PAD_X` either side — which ADR-0190 did for the drawing's sake, and which now
  buys a hit target that stands still while the deck moves under it *and while a word rolls through
  it*. The blend chip, sized to the word it shows, cannot say either.
- **Its clearance from a boundary's grab was measured rather than inherited**, for the fourth time,
  and the number is not the blend chip's. The nearest boundary to either is the pane divider down
  the left of the bay: the blend chip is as wide as its whole mode row and clears it by **8.97**,
  where the tally's 44.53 capsule in a 61 track clears it by **14.23** — both against a `GRAB` of 6,
  and a grab widened to 9 would kill one and not the other. `tests/tally.rs` asks `Layout::hit`
  directly as well as `claim`, and carries the guard that makes a bay with no strips fail rather
  than pass.
- **A folded bay was already handled and by `mixer` rather than by a control.** A bay with no room
  for its row of strips answers `None` before any rectangle is hit-tested
  ([ADR-0193](0193-a-region-that-is-not-laid-out-declares-nothing-rather-than-being-dropped-later.md)
  is the same fact on the declaration side), so a chip in a folded bay is not claimed and there is
  no per-control check for it. That is now asserted rather than assumed.
- **The harness's `apply` runs a governor pass after a residency record, and it is the only record
  here that needs one.** `Deck::set_residency` writes the request *and grants it*, so a harness that
  stopped there would draw a primed deck the governor never admitted — which is
  [ADR-0191](0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)
  read forwards. It is `karakuri-cli`'s own order, where `mix::Change::Residency` sets the level and
  calls `govern` beside it.
- **The mask mini is the last readout in the strip**, and it is waiting on a decision rather than on
  work: there is no `SetMask`, so the row has to be settled on the operations page before the chip
  can be a control.
- **`examples/panel.rs` now writes four records by hand** where it wrote three, and the sentence
  ADR-0185 attached to that function still stands: the day `Operation` becomes `Record` somewhere
  that is not an example, this function is deleted rather than moved. The record's `level` is
  `karakuri_operation::Residency::name` rather than a fourth copy of the three wire words —
  the vocabulary grew that spelling in the same session, for
  [ADR-0194](0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md), and the
  wire words are `Record::Residency`'s rather than the chip's `live`/`prim`/`alloc`.
