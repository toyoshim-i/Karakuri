---
id: 0225
title: A menu is a gesture in hand rather than a rectangle on the panel
status: accepted
date: 2026-08-30
supersedes: []
superseded_by: []
principles: [0027, 0061, 0076, 0085]
tags: [console, ui]
---

# A menu is a gesture in hand rather than a rectangle on the panel

## Context

[ADR-0221](0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md) named an
arrangement and gave it a fourth place under the store, and left the two operations owed.
[The operations page](../manual/operations.html) placed them — *"the arrangement family — reset it,
save it, put one back — lives in the transport row, beside the pill that names the MIDI map in use.
That pill is already the shape this family needs"* — and [the console page](../manual/console.html)
now draws `arr · night ▾` and says what is under it. This record is about the two things that
specification did **not** settle, both of which were decided while building it and both of which
somebody will re-propose.

**The console has never drawn a menu.** Its five previous controls are each one capsule that acts on
the press that lands in it: the Outputs row's sink, a fader knob, and the blend, tally and mask minis
([ADR-0176](0176-a-control-the-console-draws-is-the-panels.md),
[ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
[ADR-0195](0195-the-tally-chip-cycles-from-the-request-so-the-parked-case-needs-no-case.md),
[ADR-0203](0203-the-mask-chip-carries-the-angle-it-does-not-control.md)). Every one of them is
hit-tested by re-deriving the shape that drew it, and every one of them fits inside the bay it
belongs to. A menu is neither: it has a state that outlives the press that opened it, and it is
deliberately drawn **outside** the region that owns it.

**The pointer rule this lands in is `crates/karakuri-console/src/input.rs`'s**, and its own
documentation had been warning about exactly this since it was written: `GRAB` widens every boundary
by six pixels either side, *"and those twelve pixels are inside the bays, over whatever the bay draws
at its edge. The first control placed near a bay's edge is under a boundary's grab, and then both do
think they are dragging."* The answer to that for a *capsule* is arithmetic, and it has been made
five times — 7.75 for the Outputs sink, 8.97 for the blend chip, 14.23 for the tally, 41.03 for the
mask mini, and now **15.75** for the arrangement pill, which is the transport row's 48 around a
`.pill`'s 16.5. The arithmetic does not exist for a card that is drawn across the panel on purpose.

**The seam decides where the state can live**, and it is worth citing precisely because it is easy to
cite the wrong record. [ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)
decided that the arrangement is a constraint tree this repository owns rather than the toolkit's
panels — so the panel is *"the same kind of client as MCP is: it reads rectangles and it emits
operations"*, and every one of its questions is answerable with no GPU and no window. What makes
that structural rather than a rule is
[ADR-0214](0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md), and
[architecture.md](../architecture.md) states it as a fact about the manifest: *"`src/` takes no
device and now cannot: the eight dev-dependencies that could reach one left with the example
(ADR-0214), so ADR-0156's seam is enforced by the manifest holding nothing rather than by a rule."*
The console therefore has no store to list names from, no keyboard to take letters from, and — since
it paints and `egui` owns no widget anywhere in it — **no `Response`**, which is why nothing on this
panel has ever had a hover state.

## Decision

**A menu is a gesture in hand.** That one sentence is what both halves below are, and it is why they
are one record rather than two: the first says a gesture in hand owns the pointer, and the second
says a gesture that is one press and one pick has no second level to hold open.

### 1. An open menu takes every pointer event, ahead of the boundary's first refusal

`input.rs`'s rule is now **five clauses**, and the new one is **rule 2**. A reader who greps that
file finds this list:

1. **A drag in hand keeps its claim**, wherever the pointer has wandered to.
2. **An open menu keeps the pointer until it is shut** — every point of the console, not only the
   menu's own card.
3. Otherwise, if the pointer is within `GRAB` of a boundary, it is the panel's.
4. Otherwise, if the pointer is on one of the **six** controls the console draws, it is the panel's.
5. Otherwise `egui` decides.

The two previously-numbered clauses moved down by one: what was rule 2 (the boundary) is rule 3, and
what was rule 3 (a painted control) is rule 4. The sentence *rule 2 before rule 3 is first refusal
meant literally* is now *rule 3 before rule 4*, and it still means what it meant.

**Rule 2 is rule 1 again rather than a second exception to rule 4.** A menu that is down is a hand
mid-choice exactly as a boundary in hand is a hand mid-drag, and the next press is part of that
gesture whichever way it ends: on a row it picks, anywhere else it dismisses. Neither of those is
`egui`'s and neither is a boundary's. Rule 1's own argument — *"a drag is a gesture and not a
position"* — is this one word for word.

### 2. The menu is flat, and the list of names is the *load*

One card, and no second level anywhere in it:

```
save as…                 ← save, and the ellipsis is there exactly while it asks for letters
start a new one          ← the reset
───────────────────────
four_deck                ← the names already filed; picking one is the load
night
rehearsal
```

Two verbs, a hairline, then `Arrangement::filed` in the order the store handed it over. There is no
item called *load*, because the manual does not describe one:

> So *load* is a list — the names already filed, which a hand can pick without typing anything — and
> *save* is the one item on this panel that asks for letters.

*Save* is the same row in two states rather than two rows: with an arrangement in use it emits
`Operation::SaveArrangement` naming it and asks nothing, because *"saving over that name again is
what you mean by saving it again"*; with none it asks for letters. **The word changes with the
state** — `save` against `save as…` — so the one item on this panel that can ask for something says
so before it is pressed rather than after.

**Everything in it is `.lib-row` inside `.lib-list` padding, with `.lib-foot`'s count under it where
the window is too short for the list.** That is
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) taken literally: a menu of
filed names *is* a list of names, the console already draws exactly one of those in the Library bay,
and a second set of numbers for the same box is a second answer waiting to disagree. The foot is
[P-0027](../principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md) — a list that ends
because the window did, saying `12 of 40`, is a different thing from a list that is short, and the
count is what tells them apart.

## Alternatives

### a. The menu is an ordinary painted control under rule 4, and its rows clear every `GRAB` band

**This is the one somebody will re-propose, and it is the right instinct.** It is what the other five
controls do, it keeps the rule at four clauses, it costs the operator nothing — a boundary stays
draggable with the menu open — and this repository has made the clearing argument five times already
and written a test for it each time.

**It cannot be done, and the numbers are what say so rather than a judgement.** Every constant here
is in `crates/karakuri-console/src/room.rs` and `panel.rs`, so this is arithmetic a reader can redo:

- The transport row is **0 to 48**. A `.pill` is `BASE * LINE` = 11 × 1.5 = **16.5**, centred, so the
  capsule is **15.75 to 32.25**.
- The card hangs one `PILL_GAP` (**5**) below it, so its top edge is **37.25**, and its first row
  starts one `LIB_LIST_PAD` (**3**) further down at **40.25** and is `LIB_ROW_H` (**22.5**) tall —
  **40.25 to 62.75**.
- The boundary under the transport row is at **48**, and `GRAB` widens it to **42 to 54**.

So the menu's **first** item — *save*, the one an operator reaches for most — has **twelve of its
twenty-two and a half pixels inside a boundary's grab**. Under rule 4 those twelve pixels are dead,
silently, in the middle of a list somebody is reading; and a press that lands there starts a drag on
a divider the operator cannot see because a menu is drawn over it.

**Moving the card is not a fix, and that is the part worth recording.** Any card long enough to be
useful crosses the boundary under the row on every window, and once it is past that it is over the
body row and its pane dividers too. Clearing them all would mean laying the rows out around
boundaries whose positions the operator drags — the card would grow and shrink as they moved a
divider, which is a list that changes shape while a hand is travelling down it. The clearance
argument works for the other five controls because each of them is a capsule inside one bay; it is
not a general technique and this is where it stops.

**The price of rule 2, stated rather than waved away: a boundary cannot be dragged with a menu
open.** The first press shuts the menu and the second one drags. That is one extra press in a state
the operator put the panel into deliberately and can leave with the same press, against a dead first
item they would have had to discover by experiment — which is the failure
[the console page](../manual/console.html) already names about a different control: *"a control that
quietly declines the last of something is a rule an operator can only find by experiment."*

`tests/arrangement_pill.rs` asserts both halves, because *the menu is modal* and *the grab has gone
missing* look identical from one assertion: a boundary under the open card goes to the panel, **and**
it goes back to being an ordinary boundary the moment the menu is shut.

### b. Rule 2 claims only the card's own rectangle

The narrow version, and it is worse than either end. It fixes the dead rows and leaves a press
outside the card doing something else — dragging a divider, folding a bay, reaching `egui` — while a
menu stands open over the panel. Then the menu has no way to be dismissed except by pressing the pill
again, which is a control an operator has to be taught, and the panel is in a state where one press
does two things at once.

### c. A third item, *load*, that opens a submenu of names

The shape a reader expects from the tooltip's *"Click to save, load, or start a new one"*, and it
lost on the sentence the manual wrote two paragraphs later: *load is a list*. A submenu is a second
open state, a second card, a second set of boundaries to be modal over and a second dismissal rule —
all of it to put one more press in front of the operation the list already performs. The tooltip
names the three things the control does; it does not specify three rows, and the note under it
specifies two and a list.

It also costs something real at the seam. A submenu has to remember which item is open, which is a
third variant on `Menu` that means nothing to anyone but the drawing code, and every one of those
variants is state `crates/karakuri` has to be careful not to write.

### d. The menu's state lives in `crates/karakuri`, beside the name and the list

Tempting, because the name in use and the names filed are already handed in per frame from there —
they are a file and a directory, and the console can reach neither. So *put the whole control's state
on the same side* is one line shorter and looks consistent.

It loses on ADR-0156's argument moved one level along. What the *control* is doing is not something
the program can be the model of record for: the console would then be drawing a state it could not
answer questions about, and `input.rs`'s rule 2 would be reading a flag owned by the binary it must
not depend on. The menu is held in `view::Arrangement` — console-crate state in a console-crate type
the program owns an instance of, exactly as `Panel` holds a drag in hand — and it moves only through
`opened`, `shut`, `asks_a_name`, `typed` and `rubbed_out`. The **name and the list stay handed in**,
because those really are a file and a directory.

That split also keeps the typed name honest.
[P-0076](../principles/0076-a-surface-owns-the-affordance-never-the-authority.md): the buffer takes
whatever the keyboard produces and checks none of it, and a name that is not one path component is
refused where the file is written.

## Consequences

- **`karakuri_console::input::claim` takes the whole `View`** rather than the strips it used to.
  Two of the six controls now read values a caller wrote — a knob sits on the fill's moving edge, and
  the pill is as wide as the name in it — and passing them separately would let a caller mix one
  frame's strips with another frame's arrangement with nothing failing to compile. One argument is
  one frame's answer. Every test that calls `claim` builds a `View` through `common::showing`.
- **The transport row is four readouts and one control**, where `view.rs` has said *"four readouts
  and no controls at all"* since it was written. That paragraph and
  `tests/transport.rs`'s `nothing_in_the_transport_row_is_a_control` were both **narrowed with the
  reason written in**, not deleted: the test was never an argument that a control could not go there,
  it was the statement that none had, made where a control added without a decision about the pointer
  would fail it. It failed, and this record is the decision it was asking for. The test now asks with
  the pill drawn, so it cannot pass on the control having gone missing.
- **A sixth control means `tests/panel_column.rs` demands two badges.**
  [ADR-0213](0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
  made the panel column the Interface milestone's meter, and that file scans this crate's source for
  every `Operation::` a control constructs. *Save the arrangement* and *Put a saved arrangement back*
  are the first emissions ever to come from a row of *Arranging the console*, and the meter moved on
  the day the control landed rather than on the day somebody remembered.
- **The reset asks for `panel::Op::Reset` and not `Operation::ResetArrangement`.** That is
  [ADR-0208](0208-resetting-is-the-default-case-of-restoring-an-arrangement.md) unchanged — reset
  reaches code, restore reaches a file — and it is why *Reset the arrangement*'s panel route is
  checked by `tests/vocabulary.rs` rather than by the scan above. `r` and *start a new one* are one
  operation, asserted by folding two panels identically and comparing every rectangle after each
  route.
- **Three keys are live only while a name is being typed, and they reach no row.** `return`,
  `backspace` and `space` are entered in `main.rs`'s `NO_ROW` with the argument beside them: ADR-0221
  records that **no key is bound to saving or restoring an arrangement**, and that is still true —
  these three do not *name* the operation and cannot be pressed to reach it. Pressing `return` on a
  console nobody has opened the menu on does nothing at all, which is the honest test of the claim.
- **No hover highlight, and it is not an omission to be tidied up later.** A row under the pointer is
  not lit, because a hover state needs `egui` to own a widget and this console paints — the same
  sentence `view.rs` writes about every tooltip the mock draws and this panel does not. It is
  drawable from `Panel::cursor` without a widget, and that is a decision about whether the panel
  starts keeping per-frame pointer state for presentation, which is its own record.
- **The resize cursor still shows over a boundary while the menu is open**, which is the one place
  rule 2 and what is on screen disagree: the press will shut the menu, and the cursor says it will
  drag. `View::cursor` is asked of the layout and knows nothing about the menu. Cosmetic, and named
  here so that whoever fixes it finds it written down rather than finds it on a projector.
- **A store with more arrangements than the window is tall lists what fits and says `n of m`.**
  Scrolling is a control this console does not have, and a scrolling list is a second gesture in hand
  — which is to say another instance of this record's own sentence, and another record when it is
  needed.
- **`checked_name` and `mcp::checked_id` say the same three things and are two functions.**
  [P-0061](../principles/0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md) is
  kept today only because the pill is the sole surface that can name an arrangement: `checked_id` is
  private to `karakuri-environment`'s `mcp` module and its sentences say `id` and `<store>/sets/`, so
  it can neither be called nor quoted from the program. **The day an arrangement name gets a second
  surface** — a map line, an MCP tool, a `--restore-arrangement` flag — the two collapse into one
  shared `checked_name` in that crate. The comment on `checked_name` says so, and is where whoever
  does it should start.
