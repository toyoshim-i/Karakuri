---
id: 0352
title: The chain's list is the Master bay's items, and a slot is taken out by a glyph on its row
status: accepted
date: 2026-09-13
supersedes: []
superseded_by: []
principles: [0083, 0087, 0090, 0091]
tags: [ui, console, master-chain, keyboard, surfaces, m5]
---

# The chain's list is the Master bay's items, and a slot is taken out by a glyph on its row

## Context

[ADR-0340](0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md) §7 names
three rows for the chain — *Set a chain effect's parameter*, *Add an effect to the master chain*
and *Remove an effect from the master chain* — and says what each addresses. It does not say what
control any of them is on. [ADR-0348](0348-a-chain-slots-cut-is-set-through-the-parameter-row.md)
settled the vocabulary and left the surface to this pass, recording that on a default run all
three rows were inert: the bay drew three fixed rows for three shipped passes and nothing on the
panel put a slot in the chain.

[ADR-0343](0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md) walked the
Master bay as *the out fader and the three effects*, with **the three effects answering no key**,
because *"their operations carry no spelling for a parameter"*. That clause is spent: a slot's
position and a parameter's declared name are what `Operation::SetChainParam` carries.

So three control shapes were open: what takes a slot out, what puts one in, and how a digit counts
what a slot draws.

## Decision

**The Master bay's items are the out fader, then one per slot of the running chain, then `+ add`.
A slot's controls are its parameter rows, its cut chip and the `−` at the end of its row.**

### 1. A slot is taken out by a `−` on its own row

The glyph sits at the far end of the head line, where the parameter rows' figures sit under it, and
`enter` on it is the key route. It is one press, it is addressed to the slot it is drawn on, and it
is visible without a gesture.

### 2. A parameter row is laid out from the range the procedure declares

The track's two ends are the declaration's two ends, the figure is what the slot holds, and
`↑↓` step it a tenth of that range — the Inspector's rule (ADR-0343) read on a range declared in a
`.kir` rather than published by a Set. `space` returns it to the value the procedure declared it at,
which is a value the reading carries because the compiled slot has it.

### 3. `+ add` is a rung of the address, on ADR-0351's terms

`enter` on it puts down a card of the `kind L5` procedures the library holds; a digit names the nth,
`↑↓` walk them, `enter` adds the one the address is on and takes the card away, and `esc` and `Tab`
take the card away. An entry carries the content address of the procedure's source, because that is
what `Operation::AddChainEffect` names, and a cut exactly where the procedure declares `retains` —
`Cut::Mix`, which is the cut `docs/manual/console.html` has always drawn a slot arriving at.

### 4. The drop lands on the whole list and appends

The chain's list is the third set of rectangles a release can land on
([ADR-0273](0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)). It is **one**
rectangle and not one per slot: `AddChainEffect` carries no position, so every point of the list
names the same landing and a ring per slot would promise a position the vocabulary cannot carry.
A row that is not a `kind L5` is refused at the release with the reason, which is the first refusal
a release on this panel makes — the mark still says *where* and never *whether*.

### 5. `focus::Of` grows a suffix, and the cut chip keeps its number

The Master's item rung is a prefix, a repeat and a suffix — `Out`, one `Slot` each, `AddEffect` —
which `Of`'s prefix-and-repeat could not say, so `Of` gains `last` and the nth is resolved from the
end. Every entry of `last` is drawn whenever the rung is, which is what makes that resolution
possible from a single count.

**A slot's suffix is the cut chip and the `−`, and the cut chip is counted on a slot that draws
none.** A slot whose procedure declares no `retains` has no chip, and the digit that would name it
declines with that sentence rather than naming the `−`. So a digit means the same control on every
slot of the chain.

## Alternatives rejected

### a. A row menu on each slot, with *remove* in it

**The case is that it scales.** A slot will grow more acts than one — reorder, bypass, save — and a
menu is where the Library bay already puts a row's second and third act (ADR-0311). It costs no
width on a narrow bay.

**It loses on the address.** A card the grammar can open and cannot walk traps the keyboard
(ADR-0343), and a card it *can* walk is a fourth rung under a bay that is already three deep. It
also loses on what the chain has: one act, not three. Reordering is refused by ADR-0340 and named as
a limit, so a menu would be drawn for a list of one.

### b. Drag a slot out of the chain

**The case is that the carry already exists**: a row comes out of the Library and lands on the
chain, so a slot dragged off the list is the same gesture run backwards, and it needs no glyph and
no width.

**It loses on rule 01 and on P-0083.** A gesture is not a keyboard route, so the row would carry a
`has` panel badge and no key badge, which is the state ADR-0343 spent five letters to get out of.
And a drag that means *delete* when it lands on nothing is a destructive act with no refusal to
carry: a release over an alley would take a pass out of the frame, and there is nothing for the
panel to say that would have stopped it.

### c. Number a slot's controls in draw order, so a slot with no cut chip renumbers

**The case is the one rule the digits have**: they count what was drawn, from one (ADR-0343).

**It loses on what it does to the same key across a chain.** A chain of three slots where the first
declares `retains` would answer `2 1` and `3 1` with different controls — a parameter row on one and
the `−` on the other — so a digit an operator learned on one slot means something else on the next.
Counting the chip on every slot costs one refusal, which names why it declines, and buys a digit
that means one thing down the whole list. The cost is written down rather than hidden: a digit lands
on a control the bay does not paint.

### d. A chooser that lists every procedure and refuses the ones that are not `kind L5`

**The case is that it explains itself**: an operator looking for a file finds it and is told why it
cannot go in the chain.

**It loses on P-0090's own division.** A surface offers, and a card of items most of which cannot be
picked is an offer that is not one. The Library's kind filter is where a list is narrowed by kind,
and the chooser is that narrowing applied once.

## Consequences

- **`karakuri-console::view::master` is a list.** `MasterRow` gains `slots`, `add`, `card` and
  `list`; `FxRow`, `Fx` and `Chain::at` are gone, and `Chain` is `Vec<ChainSlot>` with a
  `SlotParam` per declaration. `Knob::Fx` becomes `Knob::Chain { at, key, range }`, which carries an
  owned key because a chain holds whatever procedures an operator put in it.
- **`karakuri-console::focus` gains `Control::Slot`, `ChainParam`, `ChainCut`, `ChainRemove`,
  `AddEffect` and `ChainProcedure`, and `Act::Remove` and `Act::Add`.** `Control::Fx` is gone.
  `Of` gains `last` and `Items::Controls` carries an `Of` rather than a slice.
- **`Panel::released` takes a `Landing`** — a deck or the chain — and `Released` gains `Refused`.
  `Panel::carried` is what a caller resolves a landing with.
- **`karakuri_engine::master` gains `SlotReading` and `SlotParam`, and `Present::chain_reading`**,
  because a declared range and a declared name are on the compiled procedure and on no `SlotSpec`.
  `karakuri_environment::mix::shipped_rows` and `ShippedRow` are gone and
  `karakuri_environment::mix::l5_offer` replaces them.
- **`docs/manual/operations.html`'s three chain rows read `has` in the panel column and in the key
  column**, so the panel column of the Master bay's rows holds no `plan` — M5.16's exit.
  `panel_column.rs`'s `UNREACHABLE` is empty.
- **Six bays answer all four of the acting keys** where ADR-0351 recorded five.
- **`+ add` leaves the arena's gap list**, on ADR-0340's own terms, and both tips that carried it
  say so.
- **Reordering still has no control and no row**, and neither does a chain saved under a name. Both
  are ADR-0340's limits carried forward rather than met here.
- **A release over a slot appends rather than inserting at that position**, because the vocabulary's
  add carries none. An insert is a payload change to `AddChainEffect` and is not taken here.
