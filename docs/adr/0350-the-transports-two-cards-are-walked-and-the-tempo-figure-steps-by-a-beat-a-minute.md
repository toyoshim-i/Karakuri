---
id: 0350
title: The Transport's two cards are walked, and the tempo figure steps by a beat a minute
status: accepted
date: 2026-09-13
supersedes: []
superseded_by: []
principles: [0083, 0087, 0090, 0094]
tags: [ui, keyboard, surfaces, console, docs]
---

# The Transport's two cards are walked, and the tempo figure steps by a beat a minute

## Context

[ADR-0343](0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md) built the
grammar in all nine bays and left seven rows of the key column `plan`, each waiting on a control
rather than on a bay. Four of the seven are the Transport's:

- *Attach a beat source*, on the audio-in pill's card of inputs.
- *Save the arrangement* and *Put a saved arrangement back*, on the arrangement pill's menu.
- *Set the free-run tempo*, on the tempo figure being *"a track a press positions rather than a level
  with a step"*.

The first three wait on the sentence that record wrote for all the cards: *"a card the grammar can
open and cannot then walk is a control that traps the address"*. Its rejection was made of three
clauses, and
[ADR-0351](0351-the-lane-chooser-is-a-rung-of-the-address.md) read them against the code a day later
and found two of them false: `input::claim` routes **pointer** events, so a card that is down takes no
key from anything, and a card is drawn, so it is a mode with a readout. What was left was a question —
what does `Tab` mean while a card is down — and that record answered it for the sequencer's `+ lane`,
leaving the other two cards out on the grounds that each was its own record. **This is that record.**

The fourth waits on something no record decided. `Answers::Nothing` on the tempo said *"nothing in
this workspace names a step for a tempo — so there is nothing for an arrow to move it by"*, and that
is a true reading of the workspace: `karakuri-cli`'s `--bpm N` names a tempo outright and binds no key
for it, the halve-and-double pair beside the figure moves by an octave, and
[ADR-0291](0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md) settled what a
*press on the figure* may ask for and said nothing about a key. So the step is a choice with
alternatives, and this is where it is made.

## Decision

**Both of the Transport's cards are a rung of its address, and `↑↓` on the tempo figure step it by one
beat a minute.**

### The two rungs, and where they hang

The Transport is headless, so its items are its controls left to right and the audio-in pill is `7`
and the arrangement pill `8`. What each card lists is what is drawn under it, so a row of the audio-in
card is `7 n` and a row of the arrangement menu is `8 n`.

`focus::Built::under` carries *the control the address descends through, and what is under it* — the
Inspector's third rung, and ADR-0351's chooser. It carries the Transport's two now, and the difference
is again which rung they hang off: the Inspector's is under an **item**, the chooser's is under a
**head** control, and these two are under a control of a bay whose *items are its controls*. So
`Addressed` grows `InCard` beside `Under` and `UnderHead`, and `addressed` resolves `[through, nth]`
in a bay whose items are its controls.

**The arrangement menu's rung is a prefix and a repeat**, which is what `Of` already is: *save* and
*start a new one* are drawn first and every row after them is one of the names filed. The audio-in
card has no verbs, so its rung is the repeat alone.

**How many rows a rung draws is the control's own reading**, and it is zero while the card is up —
`AudioIn::rows` and `Arrangement::rows`, the same numbers the two pills lay their cards out from.
A menu asking for a name draws none of them either, because that card is a field and not a list.

### The six keys, at the two cards

- **`enter` on a pill puts the card down**, and asks for nothing else. What the *host* does about it
  is read the machine's inputs or the names filed, and neither is a thing `karakuri-console` can do
  at all (ADR-0156) — so the press leaves as the control's own word for it, `AudioAsk::Open` or
  `Ask::Open`, and `crates/karakuri` performs it through `Readout::listened` and `Readout::arranged`.
  Those are the methods a pointer press on the same rectangle already reaches, so the key and the hand
  are one derivation asked twice.
- **A digit names the nth row** and the address descends, counting what the card drew from one.
- **`↑↓` walk the rows** from the one the card remembers, with no digit pressed first — the bay-level
  rule one rung down — and the address follows them into the card. Walked and clamped, never wrapped.
  **`←→` are refused** and the refusal names the pair that works: both cards are a column,
  `AudioInPill::row` and `ArrangementPill::row` stacking their rows by `size::LIB_ROW_H`.
- **`enter` on a row performs what that row is for**, in the control's own words again:
  `AudioAsk::Operation` naming `Operation::AttachBeatSource`, `Ask::Operation` naming
  `Operation::SaveArrangement` or `Operation::RestoreArrangement`, and `Ask::Name` where *save* has no
  name in use. The address goes back to the pill as the answer leaves, because the host takes the card
  away as it performs it.
- **`esc` takes the card away** and the address ends on the pill: one level up where it had descended
  into the card, and where it already was otherwise. **`Tab` takes it away too**, which is ADR-0351's
  answer to its own question, met here for the other two cards: a card the address descends into
  belongs to the address, and focus moving is the address leaving it.
- **`space` declines on a pill and on a row**, because each performs rather than sets, and each
  refusal names `enter`.

### *Start a new one* is drawn and is not performed

The menu's second row is the reset. *Reset the arrangement* is one row of the page and `r` is the key
that reaches it, from anywhere; a second route to it would be a route no badge on that page names, and
`key_column::ROWS` maps a route to the rows it reaches in **both** directions. So the row is counted —
the digits count what is drawn, from one — and `enter` on it declines and names `r`.

### Saving takes letters, and the field already has the keyboard

*save* with a name in use is that name: *"saving over it is what saving it again is"*, which is
`ArrangementPill::ask`'s own rule and is reached here unchanged. With none in use it asks for one, and
what then happens is not this record's to invent: `window_event`'s first letter-taking branch takes
the keyboard whole while `Arrangement::naming` is `Some`, `return` files the name and `esc` leaves it
unsaved. That branch predates the grammar and is ADR-0259's *field*, which is one of the seven kinds
the grammar is written in.

**So *Save the arrangement* is reachable from the keyboard alone**, which is rule 01's own test, and
the badge is honest: `enter` puts the menu down, a digit names *save*, and the field takes the letters.

### The tempo steps by one beat a minute

**A difference and not a ratio.** The exposure steps by a quarter stop because an exposure is a ratio;
the latency offset steps by five milliseconds because a delay is a difference. A tempo is drawn as a
number of beats a minute and named in the vocabulary as one (`Operation::SetFreeRunTempo { bpm }`), and
one beat a minute is the smallest change the figure draws as a different whole number — `.bpm`'s
`128.0` is one decimal place, and a step of a tenth would be a press an operator cannot see.

**It is inside the guard the hand is held to.** `view::TEMPO_BAND` is ±15% of the tempo at the press,
which is 9 bpm at the slowest tempo the tracker searches, so a key press asks for less than a press on
the figure may — the two surfaces are under one rule rather than one of them going round the other.

**Floored at one beat a minute**, which is `karakuri_signal`'s own clamp. The step decides what the
*record* says, which is why the floor is at the key rather than left to the oscillator — `gain_key`'s
reason, one control along.

**`space` reaches nothing on the figure.** A level's `space` is *"the value it was declared at"*, and
what the grid runs at is not a value the Transport row declares: a run starts at whatever the engine's
oscillator was built with and the tracker moves it from there. So the tempo is a third thing beside a
state and a level — `Answers::Track`, a continuum the arrows step with no value worth naming — and the
refusal says so.

### The badges

*Attach a beat source*, *Save the arrangement* and *Put a saved arrangement back* read `has` with the
badge they already carried, `enter · in the Transport`; *Set the free-run tempo* reads `has` with
`↑↓ · in the Transport`. `key_column::ROWS` gains the pair `(transport, enter)` and the three rows it
reaches, and `(transport, arrows)` gains the tempo — which is what makes all four true in both
directions.

## Alternatives rejected

### a. Step the tempo by a tenth of the band, as the trim and a parameter step

**The case is the family the rest of the console is in.** A trim, a fader, the master out and a
parameter row all step by a tenth of what the control publishes, and the tempo figure publishes a
band: a tenth of ±15% is 1.5% of the tempo, which is 1.92 bpm at 128 and puts ten presses exactly
where one press of the pointer may go.

**It loses on what the figure draws.** The readout is one decimal place of beats a minute, so a step
that is a percentage lands on 129.9 from 128 and on 195.0 from 192.3 — the same key press moving the
number by a different amount each time, on the one control of this row that an operator reads as a
number rather than as a position. The band is a guard on a hand's *aim* and not a scale anything is
stepped along; a step taken as a fraction of it would make the guard into a unit, which is exactly what
ADR-0291 says it is not: *"a guard against a mis-click and not a bound on what a tempo may be"*.

**And it loses on the two keyboards that already exist.** The offset's five milliseconds is a round
number in the unit its readout is written in, and `karakuri-cli` steps by the same one; the tempo has
no second keyboard to agree with yet, so the number is chosen to be the one a second keyboard would
pick too — the unit the operation carries, one of them.

### b. Make the tempo a level, so `space` returns it to the tempo the run was declared at

**The case is uniformity.** Every other continuum on this panel is an `Answers::Level`, `space`
returns each to its declared value, and `karakuri_engine::binding::DEFAULT_BPM` is a real number a run
starts from — so the tempo could be the fifth of them and the grammar would need no third kind.

**It loses on what the press would do.** 120 is the signal crate's default and not something the
Transport row declares: a grid that has been tapped, tracked or named is not *at* a declared value it
could return to, and one press of `space` would move it by however far it had come — past the ±15% a
press on the figure is trusted with, in one key, in front of an audience
([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
The uniformity is a property of the type and the hazard is a property of the show.

**What it costs to reject is one variant**: `Answers::Track`, whose only inhabitant is the tempo, and
whose whole content is the sentence `space` says. The alternative to the variant is a `Level` that
declines on one of the two keys a level answers, which is a type that lies about one member.

### c. Let a digit inside a card name a sibling row

**The case is the menu with four rows.** Having descended to *save*, naming the third row means `esc`
— which takes the card away — and `enter` again, where a digit would do it in one press.

**It loses on the one rule the digits have**, which is ADR-0343's own reason for turning down
numbering the Sequencer's cells from one and ADR-0351's for leaving the ring on `+ lane`: a digit names
*the nth thing one level below the address*, and a digit that named a sibling would be a second meaning
for the key in one bay. The arrows are what move along a level, and they walk a card's rows exactly as
they walk a bay's items.

**What it costs to reject is written down rather than left to be met**: an operator who has descended
to the wrong row walks to the right one rather than naming it.

### d. Leave the audio-in card to the pointer, because the console cannot read a device

**The case is ADR-0351's own sentence** for leaving these two out: *"the audio-in card lists devices
the console did not read and cannot re-read, which is a disk question and not this crate's."*

**It loses because the pointer cannot read a device either.** `AudioIn::inputs` is written by whoever
opened the input, and the press that opens the card is the moment the host re-reads it —
`Readout::listened`, which is where `karakuri_environment::audio::inputs()` is called. The key asks for
the same `AudioAsk::Open` the pointer asks for and the same function answers it, so the rung costs this
crate no device and no disk: what crosses the seam is a word for what was pressed.

## Consequences

- **`karakuri-console::focus` gains `Control::Input`, `Control::Save`, `Control::New`,
  `Control::Filed`, `Answers::Track`, `Act::Attach`, `Act::Save`, `Act::Restore`,
  `Addressed::InCard`, `Level::Tempo`, `Asked::Listened`, `Asked::Arranged`, `Address::to_row`,
  `card_of`, `card_down`, `card_short`, `open_card`, `walk_card` and `card_enter`**;
  `Control::Audio` and `Control::Arrangement` stop being `Answers::Card` and become
  `Answers::Act(Act::Open)`, and `Control::Tempo` stops being `Answers::Nothing`.
  **`Answers::Card` is gone**: ADR-0351 took one of its two inhabitants and this record takes the
  other, so nothing on the panel is a card the address declines to open, and a variant nothing
  constructs is a kind this grammar has not got.
- **`walk_card` and ADR-0351's `walk_chooser` are the same walk written twice**, over two rungs that
  differ only in the path they hang from. They landed in one working tree from two slices and are one
  function's worth of code; folding them into one is the reviewer's, and it is written down here rather
  than left to be found.
- **`Built::under` holds the Transport's two rows**, and `Built::reaches(Enter)` is true for the
  Transport — so `focus::reaches` declares thirty-three routes.
- **`View::focus_up` and `View::tab` take these two cards away as well**, through
  `View::open_transport_card` and `View::shut_transport_card`. Neither key binding changes.
- **`AudioIn::rows` and `Arrangement::rows` are public**, because the rung's count is the card's own
  count and a second reading of it would be a second answer.
- **`crates/karakuri` gains `tempo_key` and `TEMPO_STEP_BPM`** beside the other five level keys, and
  `answered` gains the two arms that hand a card press to `Readout::listened` and `Readout::arranged`.
- **`key_column::ROWS` gains `(transport, enter)` with three rows and `(transport, arrows)` gains
  *Set the free-run tempo***, and four badges on `docs/manual/operations.html` move from `plan` to
  `has`. **Its module note that this program deliberately binds no key to the two arrangement rows is
  gone**: no *letter* names either, which is what ADR-0221 settled, and the route is the pill that
  lists them.
- **Five bays answer all four of the acting keys now and four do not**, where ADR-0351 recorded four
  and five. `grammar.rs` asserts the list rather than the claim.
- **`grammar.rs` loses `a_control_that_opens_a_card_declines_and_names_what_is_in_it`** — the claim it
  held has no inhabitants left — and gains seven tests: the tempo's step and its refusals, each card's
  descent and act, the naming flow's entry, the column the arrows walk, and `esc` and `Tab` taking a
  card away.
- **No operation is added or retired**, so `karakuri-operation`, `gate.rs`, `karakuri-operation-record`
  and `karakuri-store::record` are untouched: every key route ends in an operation the pointer already
  emitted from the same control.

## What this record does not do, and what was not checked

**Three rows of the key column stay `plan`** as this lands: *Fade a deck out or in* and *Crossfade to
the next deck*, which wait on the transition row drawing one capsule that asks for a wipe, and *Set a
chain effect's parameter*, which is another slice's and arrived in this working tree beside it. The
count is `grep -c 'rt plan">key' docs/manual/operations.html` and not a number written here, because
three other slices are editing that page.

**A name is typed and not walked.** The field the menu's *save* opens takes letters and the grammar
does not reach it: `return` and `backspace` are that flow's, and the four keys of the grammar are not
live while it is asking. That is ADR-0259's *field* kind working as written rather than a gap.

**Nothing presses a key.** `grammar.rs` asks the console's own derivations and `key_column` reads the
page against the table; that a press on a running panel puts a card on screen and attaches an input is
`mod gpu`'s and was not asked. **The two cards were not looked at.** That the ring is drawn on an
addressed row is not drawn at all — `View::draw` paints the dashed ring on the focused bay, and an
address inside a card wears no mark of its own, exactly as ADR-0351 records for the chooser.

**The tempo's step was not measured against a hand.** One beat a minute is chosen from what the figure
draws and what the operation carries; whether it is the right number of presses to walk a set is a
judgement an operator makes at the panel, and the constant is one line.
