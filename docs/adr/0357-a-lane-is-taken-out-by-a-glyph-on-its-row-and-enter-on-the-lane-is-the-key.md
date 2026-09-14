---
id: 0357
title: A lane is taken out by a glyph on its row, and `enter` on the lane is the key that reaches it
status: accepted
date: 2026-09-14
supersedes: []
superseded_by: []
principles: [0083, 0085, 0090]
tags: [ui, console, sequencer, keyboard, operations, manual]
---

# A lane is taken out by a glyph on its row, and `enter` on the lane is the key that reaches it

## Context

Three records name removing a lane as an open gap rather than a decision.
[ADR-0321](0321-a-lanes-target-is-an-operation-with-its-value-elided.md)'s *what it does not
decide* closes on it — *"Removing a lane, which has no control, no row and no operation, and is
owed a purposeful note rather than an invention"* — and
[ADR-0327](0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md)
repeats it word for word after drawing the chooser that puts lanes there. `docs/manual/console.html`
carried the note both were owed, under *two things here are gaps rather than decisions*, and
`Operation::PointLane`'s documentation carried the same sentence a third time.

**The gap had a shape by the time it was paid.** `+ lane` appends and a bank holds whatever an
operator put in it, so a pattern can only grow; the mute keeps a lane and stops its writes, which is
a take-back and not a removal. And
[ADR-0352](0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)
had just answered the same question one bay up: the Master chain's list grows by `+ add` and a slot
comes out by a `−` at the far end of its row, with `enter` on that glyph as the key route. So the
control's shape was settled by precedent and only two things were open — **where the glyph goes on a
lane's row, and which key reaches it**.

## Decision

### 1. A lane is taken out by a `−` at the far end of its row

`Operation::RemoveLane { pattern: u8, lane: u8 }`, addressed the way `SetStep` and `SetLaneMute` are
addressed: the bank is named and never implied, and the lane is the position the bay drew it at. The
row is three columns — the label, the track of cells, and the glyph — so the cells stop short of the
glyph and nothing on the row moves when the mode halves the count. `docs/manual/style.css`'s
`.seq-row` is `30px 1fr auto`, which is the mock saying the same thing.

**A lane index is a position and not an identity.** `Pattern::remove(at)` hands the lane back and the
lanes after it move up, so every index above the removed one names a different lane afterwards. That
is the chain's property (ADR-0352) and it is written into the method's contract rather than argued
at the call sites.

**Nothing is refused for a lane that is driving.** A lane's writes are its whole record
([ADR-0322](0322-the-sequencer-is-polled-like-a-transition-live-only-and-its-writes-are-its-record.md)),
so they stop at the removal and the control keeps the value the last step wrote it — there is no
scheduled thing to collide with, which is what ADR-0323 had to refuse on a *fade*. A lane index the
pattern does not hold is refused in `karakuri_environment::no_such_slot`'s own sentence: the thing
named, then what there was to name.

### 2. `enter` on the lane is the key route, and the digits do not reach the glyph

**A lane's controls outrun the digits.** ADR-0352 makes the `−` the last of a slot's controls and a
digit names it, which works because a slot draws one or two parameter rows, a cut chip and the
glyph. A lane draws its label, sixteen cells and the glyph — eighteen controls at a sixteenth and ten
at an eighth — and the digits count from one to nine. A suffix on this rung is a control no key can
reach in either mode.

So the act is the **item's** rather than a control's: `focus::Built::act` for the Sequencer is
`Act::RemoveLane`, and `enter` with the address on a lane takes that lane out. It is the Library
row's `enter` read on a lane — the one key that does not have to count past nine — and it needs no
new letter, because `enter · in the Sequencer` is a badge this bay already carries for `+ lane`.

**`focus::Control` therefore gains nothing.** The glyph is the pointer's control and the lane is the
keyboard's address for it; a `Control::LaneRemove` in the ladder would be an address the grammar can
spell and no key can type.

### 3. The panel row is the sixth of *The sequencer*

*Remove a lane*, `panel has`, `key has`, MIDI and MCP `gap` beside the other five — a map line names
a slot, a range or a word from a closed list, and one lane of one pattern is none of the three; and a
route into a surface's own state is a route into a window the model is not looking at (ADR-0235,
ADR-0315). `written` answers `Silent(Surface)` with the other five and `gate::standing` classes it
`ClosedUnclassed(Unclassed::Lanes)` with them: what it takes out is a row of a pattern, so the route
it would open is a lane.

## Alternatives rejected

### a. The `−` as the last of a lane's controls, named by a digit — ADR-0352's shape literally

**The case is that it is the same control in the same place on both bays**, so a digit means the same
kind of thing under a chain slot and under a lane, and `Of::last` already exists to say it.

**It loses on arithmetic.** The glyph is the eighteenth control of the rung at a sixteenth and the
tenth at an eighth; `Press::Digit` carries one digit and `0` is the head. The address would be
spellable by the grammar and unreachable by a keyboard, so the row would carry a `has` panel badge
and no key badge — the state ADR-0343 spent five letters getting out of, arrived at from the other
side. It was built this way first and the gap is what rejected it.

### b. Make the arrows walk a lane's controls, then walk to the end of the row

**The case is that the ladder already says the cells are walked** — `Answers::Cells { across: true }`
— and the Sequencer is the one bay that uses both axes, so the machinery reads as though it is there.

**It loses on what is actually there.** `chooser::stepped` answers `Cells` with *"the cells of a lane
are walked from the cell the address is on — press a digit to name one first"*, so nothing walks
within an item today, in any bay. Building that walk is a change to the grammar for every bay that
has items with controls, made in order to reach one glyph — and it would still put the glyph sixteen
presses from the label. It is the answer to re-propose the day walking within an item is wanted for
its own sake.

### c. A second press on the lane's label, or a long press on it

**The case is that it costs the row no width**, which is the argument `SetLaneMute` itself makes for
being on the label.

**It loses on rule 04 and on what the label already is.** The label is the mute, the mute is the
take-back, and a second meaning on it would make *stop this lane* and *destroy this lane* one
rectangle apart by a gesture. A destructive act reached by a modifier is a control whose first
effect is invisible, which is ADR-0327's own reason for a lane arriving muted.

### d. A row menu on the lane, with *remove* in it

ADR-0352 §a, one bay along, and it loses for that record's reasons plus one of this bay's: a card
the grammar can open and cannot walk traps the keyboard, and the Sequencer's one card — `+ lane`'s —
is already a rung under the head. A second card under an item would be a fourth rung in the bay that
is already the hardest to address.

### e. Drag the lane out of the bay

ADR-0352 §b verbatim: a gesture is not a keyboard route, and a drag that means *delete* when it lands
on nothing is a destructive act with no refusal to carry.

## Consequences

- **`karakuri_pattern::Pattern::remove(at) -> Option<Lane>`**, with the shift stated as a contract.
  `Banks` is untouched: a bank is a position and a pattern is what holds lanes.
- **`Operation::RemoveLane { pattern, lane }`** — *Remove a lane* — sits between *Point a lane at
  what it drives* and *Choose what a step is worth*, which is where the page puts it. `gate::standing`
  and `written` file it with the sequencer's other five; `karakuri-mcp`'s `spelled` carries it with
  `make` and `shape` `None`, and `sayable` answers `Sayable::Window`.
- **`view::SeqRow` gains `remove`**, the bay's controls count gains one per lane, and
  `input::SEQ_CONTROLS` is `DECKS * (SLOTS + 2) + 1 + BANKS + 1`. The probe row is renamed to name
  the glyph, which is the five places that string is written.
- **`focus::Act::RemoveLane` and the Sequencer's `act`**; `focus::Control` is unchanged.
- **`docs/manual/console.html` draws a `−` on each of its four lane rows** and the *Sequencer* note's
  *two gaps* paragraph is one gap — the channel fader that does not say who is holding it — with the
  removal moved into a paragraph of its own. `hover.rs` cites the third `.minus` on the page.
- **What is still open in this bay is what ADR-0327 left open beside it**: a channel fader that says
  who is holding it, and whether two Inspector panes are enough to point a lane at any deck's
  parameters. Re-ordering lanes has no control and no row, which is this record's own limit and the
  chain's (ADR-0340) arrived at from the other bay.
