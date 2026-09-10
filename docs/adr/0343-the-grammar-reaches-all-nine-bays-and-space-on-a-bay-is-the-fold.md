---
id: 0343
title: The grammar reaches all nine bays, and space on a bay is the fold
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0083, 0087, 0090, 0094]
tags: [ui, keyboard, surfaces, console, docs]
---

# The grammar reaches all nine bays, and `space` on a bay is the fold

## Context

[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
decided the keyboard and walked it across all nine bays;
[ADR-0332](0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)
built the pointer, the ring, `Tab` and `esc`; and
[ADR-0333](0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md)
built the four keys that act, in **two bays of nine**. That record wrote down what it owed and this
is that list: the seven remaining bays, the transition row, a library row's own controls, the axis
nothing was checking, `space` on a bay, and the mark a folded bay wears.

**`space` on a bay was declined for one reason and the reason has expired.** ADR-0333: *"Folding is
a rule about all nine bays, so binding it in two would put *Fold a bay away* in the built column with
a badge naming two of the nine places it works — which is a badge that misleads in the direction the
whole column exists to prevent."* With all nine built the badge names all nine, and the sentence
that refused it is the sentence that asks for it.

## Decision

**The grammar is built in all nine bays. `space` at bay level is the fold, in every one of them, and
the page spells that `space · in any bay`.**

### The seven bays, as the walk reads them

Each bay's row in `focus::BUILT` is what ADR-0259's walk says it is made of, and where the walk finds
a bay with nothing of a kind that is a row in the table rather than an omission.

- **Transport.** Headless, so `0` names the row itself and its items are its controls left to right:
  the tempo, the halve-and-double pair, the offset, the beat, the bar, the frame cost, the audio-in
  pill, the arrangement pill, the tone map and the exposure. `space` on the tone map cycles the four
  operators; `↑↓` step the exposure and the offset and `space` returns each to what it was declared
  at.
- **Staging.** Items are the candidate rows and a row's two controls are its two acts — `n 1` keeps
  and `n 2` puts the node's previous version back, which is the record's own spelling. **Nothing here
  has a state at all**, so this bay has no `space` route below the fold.
- **Program.** `0` is the head, whose controls are `solo` and the class pill. Items are the picture
  and the four preview cells, and **neither answers a key**: a cell is a monitor, and the picture's
  on and off is the Outputs row's one control.
- **Inspector.** Three deep. Items are the panes; a pane's things are its deck head and then a group
  per node; a group's controls are its authority chip, its renderer chips and then its parameter
  rows. `space` cycles the sync chip, the composite chip, an authority and which renderer is live;
  `↑↓` scrub the anchor a quarter beat and write a parameter a tenth of its published range.
- **Master.** Items are the out fader and the three effects. `↑↓` step `out` and `space` returns it to
  unity. **The three effects have no addressable controls**, because their operations carry no
  spelling for a parameter — a digit reaches the effect and stops, which is the walk's one place where
  a bay is drawn and its operations are not sayable.
- **Sequencer.** `0` is the head: the grid mode pill, the four bank pills and `+ lane`. Items are the
  lanes; a lane's controls are its label and then its cells. **Sixteen steps outrun ten digits**, so
  the digits reach the label and the first eight cells and the rest are walked.
- **Outputs.** Headless again. Items are the sinks and each has exactly one state, so `space` is the
  whole of this bay — the simplest of the nine, and what the grammar looks like with one kind in it.

### `space` on a bay is the fold, and the badge says *in any bay*

**One route and not nine.** Folding is a rule about *a bay*, so `focus::reaches` declares it once,
under `focus::ANY`, and `key_column::ROWS` carries `(any, space) → Fold a bay away`. The reverse badge
check reads *in any bay* as the strong claim it is: a badge saying it fails unless the dispatch table
declares all nine bays.

**A folded bay answers `space` and nothing else**, whatever the address had reached inside it, and a
digit, `enter` and the arrows decline and say why — which is the narrow reason a folded bay keeps its
place in the ring at all.

### The mark, and the panel paints it now

`docs/manual/console.html` has specified it since ADR-0331: **the head alone, wearing the dashed
ring**, saying two things and no more — there is a bay here, and `space` opens it. The panel drew
nothing, because a folded region has no rectangle.

**It has an edge.** A closed child keeps its divider, so it still solves to a rectangle — one with no
extent along its parent's axis, sitting exactly where the bay was. `focus::folded_head` grows that
edge to a head's height and holds it inside the parent, and `View::draw` paints a head there and puts
the ring on it. **Painted over the bays and only while that bay holds focus**, which is the five
cards' arrangement: the edge belongs to whatever is drawn next to it, and this is only ever drawn
because an operator deliberately tabbed onto the bay that is not there.

### The transition row is the Mixer's head

ADR-0333 left where it sits open — *"drawn in the bay's body under the strips — neither the head's nor
a strip's"*. **It is the head's**, and the argument is the head's own definition: a head is *"where a
bay keeps the controls that are about the bay rather than about anything in it"*, and
`Operation::SetTransition` is the one row of that bay that carries no slot, because the settings decide
what the next move means wherever it lands. So `0 1`, `0 2` and `0 3` are the shape, the quantum and
the length, and `0 4` is `go` — an act on the **addressed strip**, which is the deck selection.

**Only the wipe moves.** The row draws one capsule and it asks for `Operation::Wipe`; *Fade a deck out
or in* and *Crossfade to the next deck* have no control on that row and their panel badges already say
so, so they stay `plan`.

### A library row's controls are its star and its `params` chip

ADR-0259's *"a row has no state, so `space` reaches nothing in this bay below its head"* was written
before the star was drawn, and **a star is a state**. `space` on `n 1` stars the Set or takes the star
off; `enter` on `n 2` opens what it holds and declares.

**The row menu is drawn and is not addressed.** Its items are a card, `input::claim`'s rule 2 gives
every press on the console to the panel while one is down, and **a card the grammar can open and
cannot then walk is a control that traps the address**. The same sentence covers the audio-in pill's
inputs, the arrangement pill's menu and the sequencer's `+ lane`: each declines and names what is in
the card rather than opening it.

### The axis is checked where a bay uses both

ADR-0333: *"`↑↓` and `←→` both resolve to *the arrows* … the axis is `Built::across` and nothing holds
the page against it."* **The Sequencer is where both are used at once** — its lanes are a column and a
lane's cells are a row — so a control carries its own axis (`Answers::Cells { across }`) beside the
bay's, and `grammar.rs` holds both directions: `←→` are refused on the lanes and `↑↓` on a cell, and
each refusal names the axis that works. **What is still not checked is which pair a *badge* spells**,
which is one reading further out and is written down rather than left to look covered.

### Five letters go, and the one that stays takes the focus

A letter goes when the grammar reaches the same row, which is ADR-0333's own condition.

- **`f`** folded the region under the pointer. `space` on a bay is that row, and the other row `f`
  reached — *Fold a pane away* — is `g`'s. Unbound.
- **`s` and `u`** solo and unsolo the region under the pointer. `space` on the Program head's `solo` is
  *"`s` and `u` collapsed into the one control they always described"* (ADR-0259). Unbound.
- **`o` and `p`** nudged the latency offset. `↑↓` on the Transport's offset step by the same five
  milliseconds, off the same constant. Unbound.
- **`g` stays and stops taking the pointer.** It folds the split enclosing the **focused bay**, which
  is a pane every time — a bay's parent is a split and never another bay — so it reaches *Fold a pane
  away* and nothing else. Its badge is `g · in any bay`.

**`Readout::target` and `Readout::enclosing` retire from the binary**, which is ADR-0259's own named
consequence: they resolved a region from the pointer, and the focus replaces them.

**Rule 01 is what this buys.** *Every operation is reachable from the keyboard alone.* `f`, `g`, `s`
and `u` needed a pointer to be somewhere, so four rows carried a `has` key badge while relying on a
mouse to say so — which is the objection ADR-0259 calls decisive against sharing a letter by what is
under the pointer, met on this program's own keys.

## Alternatives rejected

### a. Keep `f`, `g`, `s` and `u` on the pointer and leave the fold rows' badges as letters

**The case is that it takes nothing away.** `f` folds any region an operator can point at, including
`inspector-1` and the preview row, which no bay's address reaches; `s` solos any of the thirteen where
the Program head's pill solos the picture alone. Binding `space` on a bay costs nothing on its own,
and the page could go on printing `f g` on *Fold a bay away*.

**It loses on one badge per row, which is not a rule anybody can bend here.** *Fold a bay away* would
be reached by `f`, by `g` and by `space` in nine bays, and `key_column::ROWS` maps a route to the rows
it reaches in **both** directions — so either the table says `f` does not reach a row it reaches, or
the badge names one route and the check that reads the other fails. A column whose entries are allowed
to be incomplete is a column that stops catching the thing it exists to catch, which is the defect
`mod press_handler` was written for one seam along.

**And it loses on rule 01**, which is the half worth paying for: the four letters were the four `has`
badges in this column that a keyboard alone could not reach.

**What it costs to reject is written down rather than left to be met.** Folding `inspector-1`,
`inspector-2` and `deck-previews` from the keyboard is gone, and so is soloing any region but the
picture. Each was pointer-only before — the pane edge and the bay-head grip still do all of it — so
what is lost is a keyboard route that rule 01 did not recognise as one.

### b. Make the Program bay's picture the sink toggle, as the walk says

**The case is ADR-0259's own sentence**: *"`space` on the picture turns that sink on and off."*

**It loses on the same one-badge rule.** *Choose where the frame goes* is one row, the Outputs row is
where the page names it, and a second press in another bay would be a route no badge could carry. The
picture declines and names the row that does — which is a refusal that carries what the next attempt
needs (P-0083), and it is cheaper than a row with two badges.

### c. Give the cards a rung of their own

**The case is four `plan` rows**: the audio-in pill's inputs, the arrangement pill's menu and the
sequencer's chooser are lists, and a list is what a digit counts.

**It loses on what a card is.** While one is down, `input::claim`'s rule 2 gives **every** press on the
console to the panel, so a card the address opened would take the keyboard away from the ring until
something dismissed it — a mode with no readout, which is the objection ADR-0259 rejects modifier keys
on. The rung is not the hard part; deciding what `Tab` means while a card is down is, and that is not
this record's to invent.

### d. Number the Sequencer's cells from one so the digits reach the first nine

**The case is ADR-0259's own arithmetic**: *"the digits reach the first nine of them"*, which only
holds if a cell is the first thing a lane draws.

**It loses on what a lane draws.** The label is drawn first and it is a control — *"click to mute the
lane and keep the pattern"* — so a digit counting cells from one would skip a control the bay draws,
which is the one rule the digits have: they count what was drawn, from one. The record's *"honest and
very nearly useless"* is honest at eight cells too.

## Consequences

- **`karakuri-console::focus` gains `Answers`, `Of`, `Items`, `ANY`, `OPENS`, `Control::acts`,
  `Built::item`, `Built::beneath` and `folded_head`**, and `BUILT` grows from two rows to nine.
  `Addressed` grows a third rung and `addressed` takes a closure that answers *how many things does
  this bay draw under this path*, because a bay three deep cannot be resolved against one count.
- **`Asked` gains `Panel(Op)` and `Routed(Operation, Op)`**, and `Stepped` loses its `deck` field to a
  `Level` that carries what each level needs: the trim and the fader carry a slot, and the master out,
  the exposure and the offset carry nothing because there is one of each.
- **`View` gains `folded_mark`** and `view::folded_head_into` paints it. `next_tonemap`, `next_sync`
  and the three `TransitionSettings` cycles become `pub(crate)`, which is what makes the key and the
  chip one cycle rather than two.
- **`crates/karakuri/src/main.rs` unbinds five letters and rebinds one.** `KEYS` goes from
  twenty-three entries to eighteen; `Readout::target` and `Readout::enclosing` are gone; `out_key`,
  `exposure_key` and `offset_key` join `gain_key` and `opacity_key`, and `offset_step` retires into
  the third of them.
- **`key_column::ROWS` holds thirty-one routes** over nine bays and the any-bay pair, `NO_ROW`
  seventeen, and `parsed` resolves a tenth bay spelling that is not one of the nine.
- **`docs/manual/operations.html`'s key column reads 38 `has`, 7 `plan`, 23 `gap`** as of 2026-09-10,
  against 17 / 27 / 24. Twenty rows moved from `plan`, one from `gap` — *Read what one Set holds and
  declares*, which the `params` chip reaches — and four were re-spelled without moving.
- **ADR-0259's *no bay uses all six keys* has stopped holding**, and it is a finding rather than a
  drift: the Library's star and the Mixer's `go` are two rows that arrived after that walk, and the
  Inspector's parameter rows are a level that also performs. Three bays answer all four of the acting
  keys now and six do not, and `grammar.rs` asserts the list rather than the claim.
- **No operation is added or retired**, so `karakuri-operation`, `gate.rs`,
  `karakuri-operation-record` and `karakuri-store::record` are untouched — checked rather than
  assumed: every variant the new code constructs is one a control on this panel already emitted.

## What this record does not do, and what was not checked

**Seven rows stay `plan`**, and each waits on a control rather than on a bay: three on a card the
address does not descend into, one on the tempo figure being a track a press positions rather than a
level with a step, one on the sequencer's chooser, and two on the transition row drawing one capsule
that asks for a wipe.

**A parameter's step is a tenth of what it publishes and is this crate's**, where the trim's tenth is
`karakuri-cli`'s. Nothing in that program steps a parameter by key, so there is no second keyboard to
agree with; the number is the trim's read on a range that is declared rather than fixed, and it is
written down here so that the day one arrives the two are compared rather than discovered.

**A parameter row and the Inspector's chips are named off the pane the frame drew**, which is the
staleness ADR-0333 keeps the three mix keys away from. There is no second reading to take — the
address, the range and the value are all on the pane — and it is `view::ParamGrip`'s own arrangement
for the pointer, so a key and a hand read the same thing.

**Nothing presses a key.** `grammar.rs` asks the console's own derivations and `key_column` and
`focus_keys` read `main.rs` as text; that a press on a running panel folds a bay is `mod gpu`'s and it
was not asked. **The mark was not looked at.** Its rectangle is asserted — a head's height, the width
of the bay it stands for, inside the parent — and that it reads as a head with a word in it is a
drawing nobody ran the panel to see.
