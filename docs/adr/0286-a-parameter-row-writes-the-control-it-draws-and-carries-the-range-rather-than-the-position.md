---
id: 0286
title: A parameter row writes the control it draws, and carries the range rather than the position
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0090]
tags: [console, inspector, params, m5.5]
---

# A parameter row writes the control it draws, and carries the range rather than the position

## Context

The Inspector draws parameter rows and hit-tests none of them. That sentence has been true since
the bay's first pass and is written down in three places —
[ADR-0268](0268-a-vector-parameter-is-driven-one-component-at-a-time.md) (*"`view::param_into` is
paint only … there is no claim for a `.param` row"*), `docs/roadmap.md`'s M5.5 (*"the bay where an
operator turns a knob and the bay with no way to turn one"*), and
`docs/manual/operations.html`'s *Write a parameter* row, `plan` on the panel.

Everything under the control is now built.
[ADR-0280](0280-a-parameter-written-to-a-live-set-is-a-session-record.md) put
`Deck::write_param` over `Set::write_param` and `Record::Ride` under it, so an operation reaches a
live Set; [ADR-0282](0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md)
made a value somebody moved survive a rebuild. What was owed was the console's half: **a pointer
landing on a row's fader has to answer which deck, which parameter and what value.**

Two of those three were questions rather than transcriptions, because `crate::view::Param` — the row
as the console holds it — carried neither answer. It was `ord`, `name`, `value` and `at`, a position
on `[0, 1]`, and the harness computed `at` from the published range on the way in.

## Decision

### 1. The row writes the control the interface published, not the group the row was drawn in

`Param` carries `karakuri_operation::ParamAt` — `Published::at` and `Published::key`, carried over
unchanged. A **wildcard** control stays a wildcard: `node: None` means every node that declares the
key, exactly as `--param exposure=2.0` does and as `Record::Param` does.

The row's *place* on screen is a different answer, and the console already computes it: a wildcard
covering exactly one node is drawn in that node's group, because the mock draws no `.param` outside
a `.node-group` and there is no second group the row could go in. `crates/karakuri/src/main.rs`'s
`node_of` is that resolution, and it is documented there as being about where the row goes.

**Reading the placement back as the address is the alternative, and it loses on three counts.**

- **It narrows the control to the node it happens to reach today.** *Every node that declares
  `exposure`* is a set the Set determines, and it has one member in the pair a bare run opens on. A
  save of a `.kir` that adds an `exposure` to the second renderer makes it two, and the operator's
  knob would go on moving one of them — silently, because the row is still drawn and still moves.
  The control an operator was given is the published one.
- **It routes around the refusal that exists for this exact case.**
  [ADR-0223](0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md) refuses a
  bare-name write whose nodes are not under one authority, and names them, so that one knob cannot
  hand a node the operator kept over through a node an agent acts on. An addressed write *"meets
  nothing — it says which node it means"*. A surface that addressed its wildcards would be a surface
  that never met that rule, which is
  [P-0090](../principles/0090-a-surface-offers-it-never-decides.md) exactly inverted: the console
  would be deciding what may be asked for, in the one case the engine was built to answer.
- **It is a second derivation of an address the row was handed.** `Published::at` is in the reading
  the pane was built from. Rebuilding it out of the group the row landed in — or worse, out of
  `Node::addr`, which is a display string — is the shape *a statement is held true by the thing it
  describes* forbids.

**What the alternative had going for it, kept honest**: the addressed write is the one that cannot be
refused, so a console that always addressed would have a knob that always moves. That is real, and it
is the wrong trade at exactly the moment it matters — a refusal that names the two nodes and their
authorities is a sentence an operator can act on, and a write that quietly moved one of two is not.

### 2. The row carries the published range and derives the position, rather than the other way round

`Param::at` was a field. Its own argument was written at it:

> A position and not a range plus a value, because the fader is the only reader and a second
> derivation of *where along the track* is a second answer. Whoever publishes the control has the
> range.

**The premise stopped being true the moment the fader became a control.** A fader a hand can move has
a *second* reader — the grab, which turns a pointer back into a value — and that one needs the range
whichever way the field is spelled. Keeping the position and adding the range beside it is the
literal *two statements about one thing* the original argument refused; keeping the position alone
puts the inverse map in the harness, one crate out from the forward map, where nothing checks that
they agree.

So the field is `range: [f32; 2]`, and `Param::at` and `Param::valued` are the two directions of one
map in one place. It is the arrangement `filled` and `Grab::value` already have — the same map
written forwards where a fader is drawn and backwards where one is grabbed, and `Grab::value`'s
documentation already says which one it is the inverse of.

Two things fall out of having one place to put them:

- **A published range of no width is answered once.** The harness carried that guard (*"a range of
  no width is a control with one position, and the fader sits at its start rather than at a division
  by zero"*); it is now `Param::at`'s, and `Param::movable` is the same fact read as a control —
  such a row is **drawn and not taken hold of**, which is `Grab::new`'s refusal of a track with no
  travel, read on the value axis instead.
- **Both ends of the published range are exactly reachable**, for `Grab::value`'s reason, and a
  position past either end is clamped rather than extrapolated: a published range *"narrows and never
  redefines"*, so there is nothing outside it a control may ask for.

### 3. The knob is the target and the track is not

`Mixer::grab`'s rule, taken as read rather than re-decided: a press on the track, off the knob, takes
hold of nothing. A parameter at 0.2 whose track was clicked would jump to the far end of its
published range, on stage, because a hand landed three pixels off a handle. The mock draws a
`.fader s` on every `.param` and deliberately draws none on the transport's exposure track, which is
what tells a control with a handle from one that is set outright — `MasterRow::grab` argues the pair
out and this is the third instance of it.

**The manual does not say this for this bay.** The mixer's faders each carry it in a tooltip
(*"a press on the track off the knob does nothing, which is every fader in this bay's rule"* — the
Mixer bay's rule, by its own words); `docs/manual/console.html`'s `.param` rows and their faders
carry no tooltip at all. That is a gap in the page rather than a question this record settles, and it
is named in *What this does not decide*.

## What it costs

**`crate::view::Param` is no longer a small copy of a number.** It carries a `String` key and an
optional node address per drawn row, rebuilt whenever the harness re-reads a pane — which is at
startup and on the frame a build lands, never per frame, because `Set::published` allocates and says
so. The row was already a `String` for its name, so this is a second one on a path that already had
one.

**Two spellings of the same address exist in a pane**: `Node::addr`, the mock's `L1:0`, and
`Param::param`'s `node`. They are not two answers to one question — the first is the group's, is a
display string, and is the layer word `docs/ir-spec.md` owns; the second is the control's, is the
vocabulary's type, and is `None` for the row the first could not describe at all. The wildcard case
is what makes that unmistakable rather than a distinction being maintained by care.

## Alternatives rejected

### a. Address the row at the node it was drawn in

Above, §1. It is the smaller diff — `node_of` has already resolved the node, so the row could carry
the pair it was grouped by and `ParamAt` would never be `None` on this route. It loses because the
resolution is *where the row goes* and the write is *what the control is*, and the two come apart
exactly when a rebuild adds a second node declaring the key.

### b. Keep `at` and add the range beside it

The purely additive change: nothing that reads a position moves, and the grab gets what it needs.
Rejected because it makes the row state one fact twice — the position and the two numbers it was
computed from — with the forward map in `crates/karakuri` and the inverse in
`crates/karakuri-console`, so a harness that computed `at` against the *declared* range while the
console mapped a drag against the *published* one would draw a fader that jumped under the hand. One
field and two methods cannot do that.

### c. Send the position and let the harness turn it into a value

`Operation::WriteParam` would carry a `[0, 1]` and whoever applies it would read the range off the
live Set. It has a real argument: the range is the Set's and the console is holding a copy of it.

Rejected because the vocabulary already says otherwise and for a reason that is not this control's —
`ParamValue` is *"a value and never a range, since a range is the procedure's declaration and not an
operator's to write"*, and every other route into `WriteParam` (a `--param` flag, a `param` record, a
`ride`) carries a number. A position-carrying operation would mean one variant whose meaning depended
on which surface sent it, and a MIDI control change — seven bits, learned against a position in the
published interface — would have to be turned into a value somewhere anyway.

### d. Press-to-set, with no drag

The transport's offset and exposure tracks are set outright by a press (*"it is 80 pixels for the 80
five-millisecond steps"*), and a parameter fader could be. Rejected by the mock: those two tracks are
drawn with no `.fader s` and every `.param` is drawn with one. A handle that jumped to the pointer is
a lie about what a handle is, and the console has that argument written down once already.

## Consequences

- **`crates/karakuri-console/src/view.rs`'s `Param` is `ord`, `name`, `value`, `range` and `param`**,
  with `Param::at`, `Param::valued` and a private `Param::movable`. The doc at the old `at` field is
  kept as the argument it was and marked with what changed it, rather than deleted.
- **`param_fader` is the row's fader, derived once**, and `param_into` paints from it. The track is
  `.param`'s `1fr` and `positive` is asked there — which is where ADR-0279's measurement lands as
  code.
- **`param_rect` is where a row is**, and `node_into`'s running sum is gone. That sum was the same
  shape the deck head had before ADR-0218: a press had nowhere to ask what it had landed on.
- **`InspectorPane::grip` and `InspectorPane::owns`** are the derivation and the hit test, over
  `0..shown` so a group the pane had no room for is not reachable by a press either.
  `ParamGrip` is which deck, the row, and the track it took hold of.
- **The console does not construct `Operation::WriteParam`.** The grip stops at the row; the
  translation belongs with the other three knobs, in `crate::panel::Knob`, whose `operation` is the
  one place a fader's position becomes an operation. `tests/panel_column.rs` reads the emission out
  of this crate's source, so that arm and the panel badge on
  `docs/manual/operations.html`'s *Write a parameter* row land together.
- **`crates/karakuri-console/tests/param_fader.rs`** covers the row's place against `group_h`, the
  fader's four tracks, the painted knob against the grabbed one, the track not being a target, every
  row being its own control, the wildcard staying a wildcard, the map both ways over a range that is
  not `[0, 1]`, a range of no width being drawn and not grabbed, and a group the pane did not draw
  not being reachable.

## What this does not decide

- **Whether the panel badge moves.** `docs/manual/operations.html`'s *Write a parameter* row is
  `plan` on the panel, and it stays `plan` until a press on this control reaches
  `Deck::write_param` — which is the `Knob` arm, the `PROBES` row and the `apply` arm, and none of
  them is here. The page moves first, on `docs/contributing.md` §5's terms.
- **What the manual says about this fader.** `docs/manual/console.html` gives the `.param` rows and
  their faders no tooltip at all, so the page does not say that the knob is the target and the track
  is not, and does not say what a bare-name row's refusal looks like when it comes. Both are the
  page's to state.
- **What a `.param.bound` row's fader does.** A bound parameter shows its source instead of a number
  and nothing in `crates/karakuri` binds anything, so it is a state this program cannot enter
  (ADR-0191) and there is no row to grab. Whether a hand may move a knob a signal is holding is
  *Take a parameter back*'s question.
- **Whether a pane scrolls.** Below roughly 534px of Inspector bay not one parameter row is drawn,
  and `grip` answers `None` there because `shown` is zero. That is the console's first genuine want
  of a scroll position and it is untouched here.
- **How a vector reads.** Three rows, `glow.x`, `glow.y`, `glow.z`, one `f32` each — ADR-0268 —
  and each is a control of its own on this route, with no grouping and no wide write. Whether the
  page groups them is that record's open question and still is.
