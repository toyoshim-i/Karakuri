---
id: 0292
title: The pane head's name takes letters and the keep capsule stays a stamp
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0090]
tags: [console, ui, operations, library, inspector]
---

# The pane head's name takes letters and the keep capsule stays a stamp

## Context

**Two records were resting on one count.** *The console has exactly one letter-taking flow* is a
sentence written twice, three days apart, and each time it decided something:

- [ADR-0262](0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)
  made the Library bay's `holds` and `layer` fields *step* through what the store already holds,
  because a filter that took letters would need a second flow — *"a decision about the console, not
  about this bay"*.
- [ADR-0287](0287-the-keep-pill-files-under-a-stamp-because-the-consoles-one-letter-taking-flow-is-an-arrangements-name.md)
  made the Inspector's `keep` capsule file under a stamp for the same reason, and said in its own
  last line that the question *"is now asked by two bays, and this record is the second reason to
  expect it to be asked again"*.

It has been asked a third time, and this time it is answered. **The console is to be able to name a
Set**, and where it takes the letters is the place that already displays a name: the Inspector pane
head's `.what`, which reads `deck A · drift_night` between the word `showing` and the `keep` capsule
at the other end of the same row.

**That head is where the question came from.** The capsule whose inability to be named is what
ADR-0287 recorded sits in this row; the name it would have typed is the run three items to its left.
Nothing else on the panel is both a name and a place a hand already goes.

## Decision

**A press on the pane head's name puts that head into a naming state; the commit files the deck
under what was typed; escape leaves it alone. The `keep` capsule beside it is unchanged.**

`crates/karakuri-console/src/view.rs` carries `DeckName` and `deck_name`, and `View` carries
`Naming` behind five methods. What the two routes emit differs in exactly one field:

A press on the capsule emits `Operation::SaveSet { deck, id: None }`; the name, committed, emits
`Operation::SaveSet { deck, id: Some(typed) }`. One operation, one row of
[every operation](../manual/operations.html), and the two presses differ in exactly that field.

**They are not two decisions. They are
[ADR-0128](0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)'s two, drawn on one row**:
*"an operator's own act gets the name it asked for. A key press cannot type one and takes a stamp."*
The capsule is the press that types nothing, so the capsule takes the stamp — and the argument
holding it there is now that sentence rather than the absence of a field.

**What this does to ADR-0287 is nothing, and that is the point.** Its decision stands as written: the
capsule files under `None`, the store names the file, the wash reads the deck's residency, and the
pill keeps the deck its pane is showing. What has changed is one premise of its *Context* — there
are two letter-taking flows now — and that premise was never what the capsule rested on. ADR-0287 is
history and is not revised (`docs/contributing.md` §4); this record is the one beside it, and it
names ADR-0287's decision as **the unnamed route** of a pair.

**What this does to ADR-0262 is take its premise away.** Its refusal of letters in the filter fields
was explicitly not on the merits: *"it is refused **here** rather than on its merits: the first
control that wants a thing is not where that thing is decided."* That deferral is now spent, and
the record itself says the `holds` half is *"what a text-entry model would fix"*. **So the filter
fields are a thing now open rather than a thing decided**, and this record is where that is written
down. It does not reopen them: what a filter field should do is the Library bay's question, on a
field whose rule is *unbounded fragment* where this one's is *one path component*, and nothing here
answers it. The `layer` half is untouched either way — a closed list of five, where stepping is the
right control whatever happens beside it.

### Where the boundary is

**The mock's head is `showing`, the name, `▾`, `.sep`, `keep`, and the `▾` is a control this console
does not have.** It means *point this pane at another deck*, which is a per-pane pointer nothing
keeps ([ADR-0200](0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md), and
`View::inspector`'s own note). It is not drawn.

**The name target is the run's own ink and stops there.** `DeckName::chevron` holds the rectangle one
`.half-head` gap after the run, at the arrangement pill's own `CHEVRON_W` × `CHEVRON_H`; nothing
paints it and nothing hit-tests it. A target that had run to the capsule would have swallowed the
chooser's place before anybody drew it, and the day the chooser lands it would be taking that place
*back* from a control an operator's hand had already learned. The gap between the two is claimed by
neither.

**And the run is one target rather than two.** `deck A · drift_night` is one `.what` in the mock and
one galley here. Claiming the material and leaving `deck A ·` a readout would be a boundary inside a
run of text with nothing on screen drawing it.

**Measured against `panel::GRAB`**, as the capsule at the other end of this row is: the capsule's
nearest boundary is the pane divider on its right and clears it by `.half-head`'s right-hand padding
of 10 against a grab of 6; the name's nearest boundary is the divider on its *left* and clears it by
that padding plus the word `showing` plus a gap. `karakuri-console/tests/deck_name.rs` asserts both,
by asking `Layout::hit` at the run's four corners rather than by arithmetic alone.

### What the head draws while it is asking

`showing` becomes `keep as`, and `deck A · drift_night` becomes `deck A · glass_sh▏`.

- **The label is what says what the letters are for.** A caret says letters are going *somewhere*;
  only the word in front of the run says they are naming the Set the capsule at the other end of the
  row files. It is `keep`'s own word rather than a new one, which is `save as…`'s arrangement three
  bays along.
- **The deck stays and the material goes.** The half of the run being replaced is exactly the half a
  name is, so the row goes on saying *whose* name is being typed. A field that had cleared the run
  would take that word off the screen at the moment an operator is looking hardest at it.
- **The field starts empty rather than from the material's name.** A buffer seeded with what was
  there is a name an operator commits by pressing return once, which is the shape of an overwrite
  nobody typed. What ADR-0128 makes an instruction is a name that was *typed*.
- **A tint under the field and no pink.** `.node-head`'s own `--c-tint`, so no new colour is spent;
  the pink a capsule is lit in means *on air* two controls away.

### One field at a time, and it is the keyboard that says so

`Menu::Naming` is one because a menu is one. This is one because **the keyboard is one**: whoever
holds the keys takes them whole while a name is being asked for — `s` is an `s` in a name and not a
solo — so two open fields would be two places one keystroke could go with nothing on the panel
saying which. `View::naming` is therefore an `Option` on the console and carries the pane it is
drawn in, not a buffer per pane.

**It is on `View` and not on `Pane` because a pane is rewritten whenever a Set lands.** A buffer kept
there would be a name that vanished mid-word. This is `Arrangement::menu`'s argument on a second
control: what a *control* is doing is this crate's, nothing about a half-typed name is saved,
restored or reset, and a host that kept a copy would be keeping the console's gesture on its behalf.

### The deck is read at the commit

`KeepPill` carries the deck it was measured for, because its press is one instant. This gesture spans
frames, so `View::named_set` looks the pane up again: what is filed is the deck the head says it is
filing *now*. A pane that has gone — a console handed a shorter `View::inspector` while somebody was
typing — emits nothing and the field is taken away, rather than a keep landing on a deck whose head
is no longer on screen.

## What it costs

- **An overwrite is not announced.** ADR-0128 is unchanged: a Set id typed twice replaces what is
  under it, and nothing on this panel says so before it happens. On the MCP route that is a
  documented behaviour a model reads in a tool description; here it is a hand, a word it has typed
  before, and a file it does not get back. **This record does not decide what the panel should say**
  — the console cannot know what the store holds ([ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)),
  so any warning is a reading the host would have to hand in, and that is a control this record does
  not design. It is named here as owed rather than left to be discovered.
- **An operator-typed id reaches the store unvalidated today, and this is the route that makes it
  reachable from a hand.** `karakuri_environment::filed_as` answers `(Asked::Operator, Some(id)) =>
  id` and `mcp::checked_id` runs only on the tool's side. Until the operator route gets the same
  wall, a name with a `/` in it is a path this console handed to a file write. The surface owns the
  affordance and never the authority
  ([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)), so the fix is the wall and
  not a filter in the head — `checked_name`'s arrangement one bay along, where the refusal is said
  out loud by whoever refuses it. **The console emits the empty string too**, for the same reason and
  with the same expectation.
- **The console is in a mode while a head is asking, and the panel says so with a caret and a
  word.** Every letter goes into the field, so every key that is an operation the rest of the time is
  not one while it is open. That is `Menu::Naming`'s cost paid twice rather than a new one, and it is
  what makes a second flow a real addition to what an operator has to hold in their head.
- **Two flows now mean two things by *name*, and the two rules differ.** An arrangement's name is
  refused by `checked_name` and **overwrites by design** (ADR-0221); a Set id is refused by
  `checked_id`, is a different length, and is written into a different directory. ADR-0287 rejected
  *extend `Menu::Naming` to take a Set id* on exactly this ground and it is still right — which is
  why this is a **second flow** and not the first one widened.
- **The chooser is now boxed in.** `▾` has a rectangle reserved for it and a neighbour that will not
  move, so whoever draws it inherits a place rather than a choice. That is deliberate and it is
  still a constraint on a control nobody has designed.

## Alternatives rejected

**Put the letters on the `keep` capsule — a press opens a field, a long press stamps, or the capsule
grows a second half.** The obvious place, and it loses on ADR-0128: the capsule is the press that
cannot type, and giving it a field makes one control mean both halves of a sentence whose whole point
is that they are two. It also puts a text field inside a capsule whose wash already means *on air*.

**The mixer strip's name.** The other place on this panel that displays a Set's name, and it is the
wrong one twice over. A strip's name is the deck's material read off the same seam, but the strip is
also *the selection* — a press anywhere on it that no knob claimed means *address the keys here*
(`console.html`), so a press on the name would have to be carved out of the one control whose whole
job is that it is the whole column. And a strip is four-wide at every window this panel is drawn at,
where a pane head is half the console: a name being typed into a 60-pixel column is a field that
cannot show what is in it.

**The Library bay's rows.** Rejected on what the bay *is*: the list is a listing of what the store
holds, and a name typed into one of its rows is a **rename**, which is an operation the vocabulary
does not have and `console.html`'s *What keeps a favourite* already says this workspace has no field
for. Naming a keep is a thing that happens before a file exists; the library is the list of files
that do.

**A console-wide text-entry model, with this head as its first user.** What ADR-0262 and ADR-0287
each deferred to. Still not taken, and now for a smaller reason than either of theirs: two flows with
two rules is what the console *has*, a third abstraction over them would have to be a rule that is
neither's, and there is no third caller. `Menu::Naming` and `Naming` are the two, they are eleven
lines each, and the day a fourth control wants letters is the day the shape of the general answer is
visible. Building it now is an abstraction with two call sites that disagree.

**Draw the `▾` while we are in the row anyway.** Refused by ADR-0200 and by this repository's own
history: the chevron is a control with no route, and a drawn, claimed, inert control is a defect this
tree has shipped once already. It is *reserved* instead, which costs a rectangle and no ink.

**Let the name target run from the label to the capsule.** Cheaper to derive and it is the wrong
rectangle: it takes the chooser's place, it claims the gap that separates two controls, and it makes
a press three pixels off the ink into a press that opens a field over a name nobody was pointing at.

**Seed the field with the material's name, so *keep this again under the same name* is one press.**
It is the case ADR-0128 makes destructive, offered as a convenience. A field that arrives full is a
field whose default action is an overwrite, and the operator who wanted it can type it.

## Consequences

- `crates/karakuri-console/src/view.rs` carries `DeckName`, `deck_name`, `NAMING_LABEL`,
  `head_label`, `naming_text_in_head`, `deck_letter`, and `Naming` with `View::naming` behind
  `naming_set`, `naming_set_in`, `name_set`, `stop_naming_set`, `type_into_name`, `rub_out_of_name`
  and `named_set`. `inspector_into` takes what the head is asking for and paints the run from the
  same derivation `claim` will hit-test.
- **`KeepPill`'s documentation is amended and its behaviour is not.** The paragraph that argued the
  `None` from there being one flow now argues it from ADR-0128 and points here.
- **The registration is not in this crate and the control claims nothing until it lands.** A press on
  the run reaches `egui` until `crates/karakuri-console/src/input.rs` carries a `Probe` row for it,
  `crates/karakuri/src/main.rs` carries the press arm and the `press_handler::ASKED` row, and the
  window's key path routes letters to `View::type_into_name` while a head is asking. All three are
  in files this record's author does not own, and they are owed by it.
- **`input::claim`'s rule 2 is owed a clause.** A card that is down is a hand mid-choice and every
  point of the console is part of that gesture; a field that is taking the keyboard is the same
  state, and without the clause a press on the arrangement pill can open a second letter-taking flow
  while this one is open. That is the one way two fields can be open at once and it is closed there
  rather than here.
- **`karakuri-console/tests/panel_column.rs` needs no new arm**: the named route constructs the same
  `Operation::SaveSet` the capsule does, and the row's panel badge is already `has`. Its comment on
  the `SaveSet` sample says the capsule types no name *because* this console's one flow is an
  arrangement's, and that half of the sentence is now this record's.
- **The manual is owed three sentences** (`docs/contributing.md` §5 step 3), and they are in this
  record's report rather than in the pages: the `keep` capsule's tooltip in `console.html`, which
  says *"nothing here types a name, and the one letter-taking flow this console has is an
  arrangement's"*; the two filter fields' tooltips and *List what the store holds*' row on
  `operations.html`, which each give *the console has one letter-taking flow* as the reason they
  step; and `console.html`'s *How a folder is chosen*, which calls a typed path *"refused by the
  console's one letter-taking flow"*.
- **`docs/roadmap.md` states the count too**, at M5.3's note on the filter fields, and it is where
  ADR-0262 being open rather than closed has to be visible.
- **Nothing here decides the console's text entry.** There are two flows and no model over them; the
  question is smaller than it was and it is still open.
