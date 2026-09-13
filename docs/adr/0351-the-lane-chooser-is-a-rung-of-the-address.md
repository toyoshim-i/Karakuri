---
id: 0351
title: The lane chooser is a rung of the address
status: accepted
date: 2026-09-13
supersedes: []
superseded_by: []
principles: [0083, 0087, 0090]
tags: [ui, keyboard, surfaces, console, docs]
---

# The lane chooser is a rung of the address

## Context

[ADR-0343](0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md)
built the grammar in all nine bays and left seven rows of the key column `plan`, each waiting on a
control rather than on a bay. Three of the seven wait on the same sentence: *"a card the grammar can
open and cannot then walk is a control that traps the address"* — the audio-in pill's inputs, the
arrangement pill's menu and the sequencer's `+ lane`.

That record rejected giving the cards a rung, and wrote down exactly what the rejection was made of:

> **It loses on what a card is.** While one is down, `input::claim`'s rule 2 gives **every** press on
> the console to the panel, so a card the address opened would take the keyboard away from the ring
> until something dismissed it — a mode with no readout … The rung is not the hard part; deciding
> what `Tab` means while a card is down is, and that is not this record's to invent.

Two of the three clauses do not survive being read against the code. `input::claim` routes **pointer**
events; the keyboard never passes through it, so a card that is down takes no key away from anything
— `Tab`, `esc` and the four keys of the grammar reach `focus::press` and `Focus::tab` exactly as they
did. And the card is drawn on the panel, so a mode with a readout is what it already is. What is left
standing is the third clause, which is a question and not an objection: **what does `Tab` mean while a
card is down.** This record answers it for one card.

*Point a lane at what it drives* is M5.13's row here. Its panel badge reads `+ lane` and its key badge
read `enter · in the Sequencer` with no press behind it.

## Decision

**The `+ lane` chooser is a rung of the Sequencer's address, under the head control that opens it.
`Tab` and `esc` take the card away.**

### The rung, and where it hangs

`+ lane` is the sixth control of the Sequencer's head, so it is `0 6`. The card's entries — one per
target a lane can drive, which is a fader per mixer strip and the published controls of the load
pulldown's deck (ADR-0327) — are what is drawn under it, so an entry is `0 6 n`.

`focus::Built::under` already carries *the control the address descends through, and what is under
it*, for the Inspector's third rung. It carries a second entry now, and the difference is which rung
it hangs off: the Inspector's is under an **item** and this one is under a **head** control. So
`Addressed` grows `UnderHead` beside `Under`, and `addressed` resolves `[HEAD, through, nth]` before
the item arms — `HEAD` is zero, and a three-digit path beginning with one would otherwise read as an
item, a control and a thing under it.

**How many entries the rung draws is the bay's own reading, and it is zero while the card is up.** A
rung that is not on the panel draws nothing, so every press addressed below `+ lane` with the card up
declines with the one sentence that says which press draws it.

### The six keys, at the chooser

- **`enter` on `+ lane` puts the card down**, and asks for nothing else. What is being added is *what
  the lane drives*, so a lane with nothing to point at emits nothing — the pointer's own reading of
  this press (`Readout::chosen`), unchanged. It is refused where there is nothing to point at, which
  is `View::open_lane`'s existing rule.
- **A digit names the nth entry** and the address descends, counting what the card drew from one.
- **`↑↓` walk the entries** from the one the chooser remembers, with no digit pressed first — the
  bay-level rule one rung down — and the address follows them into the card. Walked and clamped, never
  wrapped. **`←→` are refused** and the refusal names the pair that works: the card is a column of
  items, `LaneCard::item` stacking them by `size::LIB_ROW_H`.
- **`enter` on an entry asks for `Operation::PointLane`**, with the bank this bay drew, takes the card
  away and puts the address back on `+ lane` — the pointer's own two moves in the pointer's own order.
- **`esc` takes the card away**, and the address ends on `+ lane`: one level up where it had descended
  into the card, and where it already was otherwise.
- **`space` declines on both**, because both perform rather than set, and each refusal names `enter`.

### `Tab` takes the card away, and that is the question ADR-0343 left

**A card the address is inside belongs to the address.** Focus moving is the address leaving, so
`View::tab` shuts the chooser on a move that happens. The bay keeps its remembered address, which is
every other thing `Tab` does not disturb.

**The five cards the address does not walk are left alone**, and the distinction is not an exception:
they belong to the pointer, because nothing but a pointer can be inside them. The chooser is the one
card focus can be in, so it is the one card focus leaving takes away.

### The badge

*Point a lane at what it drives* reads `has` with the badge it already carried, `enter · in the
Sequencer`, and `key_column::ROWS` gains the pair `(sequencer, enter)` that makes it true in both
directions.

## Alternatives rejected

### a. `enter` lands the address on the first entry

**The case is the brief's own wording** — *`enter` on `+ lane` opens the chooser and the address
descends into it* — and it reads better on the panel: the dashed ring lands on a target rather than
staying on the pill that opened the card.

**It loses on the one rule the digits have.** A digit names *the nth thing one level below the
address*, and with the address already on an entry there is nothing below it — so a digit would have
to name a **sibling**, which is a second meaning for the key in one bay and the only place on the
panel where it would mean that. ADR-0343 turned down numbering the Sequencer's cells from one for
exactly this: *"a digit counting cells from one would skip a control the bay draws, which is the one
rule the digits have: they count what was drawn, from one."* The address descending on the digit and
on the arrow is what keeps that rule whole, and the card being down is the readout that says the rung
is there.

**What it costs to reject is written down rather than left to be met**: after `enter` the ring is
still on `+ lane`, and an operator learns the rung is open from the card rather than from the ring.

### b. Give all three cards the rung at once

**The case is three `plan` rows for one mechanism.** The audio-in pill's inputs and the arrangement
pill's menu are lists too, and a list is what a digit counts.

**It loses on what the other two cards are for.** The arrangement pill's menu carries *save*, which
takes a name a bare key press cannot type — `key_column`'s own two-row note, and ADR-0221's. The
audio-in card lists devices the console did not read and cannot re-read, which is a disk question and
not this crate's (ADR-0156). Neither is the chooser's shape, and building a rung for all three would
have put two of them in the built column on the strength of the third. Each is its own record when
somebody takes it.

### c. Leave `Tab` alone, so the card survives a move of focus

**The case is that it changes nothing.** `Tab` moves focus and has never touched anything else; the
card is drawn, so an operator can see it is there and dismiss it with a pointer press.

**It loses on what it leaves behind.** `input::claim`'s rule 2 gives every **pointer** press on the
console to a card that is down, so a `Tab` that left one standing would put the keyboard in one bay
and the whole pointer in a card the operator has walked away from. The way back is to `Tab` round to
the Sequencer and press `esc`, which is a recovery nothing on the panel says. Shutting it is one line
and one sentence, and it is the answer ADR-0343 said was the hard part.

## Consequences

- **`karakuri-console::focus` gains `Control::LaneTarget`, `Act::Open`, `Act::Point`,
  `Addressed::UnderHead`, `add_lane_nth`, `open_chooser`, `walk_chooser` and `point_lane`**, and
  `Control::AddLane` stops being an `Answers::Card` and becomes `Answers::Act(Act::Open)`. `Answers::Card`
  keeps its two inhabitants, the audio-in pill and the arrangement pill.
- **`Built::under` holds two rows**, and `Built::reaches(Enter)` is true for the Sequencer — so
  `focus::reaches` declares thirty-two routes where it declared thirty-one.
- **`View::focus_up` and `View::tab` take the card away**, which is where `esc` and `Tab` reach this
  crate from `crates/karakuri/src/keymap.rs`. Neither key binding changes.
- **`key_column::ROWS` gains `(sequencer, enter) → Point a lane at what it drives`**, and
  `docs/manual/operations.html`'s badge for that row moves from `plan` to `has`.
- **Four bays answer all four of the acting keys now and five do not**, where ADR-0343 recorded three
  and six. `grammar.rs` asserts the list rather than the claim.
- **No operation is added or retired**, so `karakuri-operation`, `gate.rs`, `karakuri-operation-record`
  and `karakuri-store::record` are untouched: the key route ends in `Operation::PointLane`, which the
  pointer's pick already emitted, through `View::shut_lane` and the same payload.
- **`docs/manual/console.html` and `docs/manual/operations.html` both listed `+ lane` among the cards
  the address does not descend into**, and both say what it is instead now. The `+ lane` tip carries
  the keyboard route.

## What this record does not do, and what was not checked

**It leaves two cards where they were.** The audio-in pill's inputs and the arrangement pill's menu
decline and name what is in them, and their three rows stay `plan`.

**Nothing presses a key.** `grammar.rs` asks the console's own derivations and `key_column` reads the
page against the table; that a press on a running panel puts the card on screen is `mod gpu`'s and was
not asked. **The card was not looked at.** That the ring is drawn on an addressed entry is not drawn
at all — `View::draw` paints the dashed ring on the focused bay's head, and an address inside the
chooser wears no mark of its own, exactly as an address on a strip's fader wears none.
