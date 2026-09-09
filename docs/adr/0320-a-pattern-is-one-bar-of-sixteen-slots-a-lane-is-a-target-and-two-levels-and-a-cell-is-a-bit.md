---
id: 0320
title: A pattern is one bar of sixteen slots, a lane is a target and two levels, and a cell is a bit
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0087, 0090]
tags: [console, sequencer, vocabulary, manual]
---

# A pattern is one bar of sixteen slots, a lane is a target and two levels, and a cell is a bit

## Context

Three records already fence this bay in and none of them says what a pattern *is*.
[ADR-0222](0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md) settled the wire — a lane is
a fifth route into `karakuri-operation`, emitting operations on the beat, and *"a lane needs no new
operation to drive anything"*. [ADR-0227](0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md)
settled the tier — library data in two tiers on the arrangement's shape, a name the operator typed,
one path component, bytes the store does not parse — and left the file form open *"for the record
that has something to serialise"*.
[ADR-0306](0306-the-grid-head-is-one-pill-because-the-bar-is-one-bar-and-the-count-follows-the-mode.md)
settled the head — one bar, a step is a sixteenth or an eighth, one pill, the count follows the
mode.

What is left is the shape, and its absence is what every one of the bay's five `Undecided` payloads
is waiting on: *"nothing in this program holds a pattern"*
(`crates/karakuri-operation/src/lib.rs`), which is also why the bay draws nothing beyond its head
under [ADR-0200](0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md).

**The mock leaves one question open twice and in the same words**, which is what makes it a
question rather than an omission:

> *"what an on cell is worth is open: a lane pointed at a fader emits the same set-the-opacity a
> hand on the strip emits, which carries a level, and a cell drawn on or off carries none."*
> — lane A's tip

> *"A parameter has a range and a write carries a number, where a cell is on or off, so an on cell
> here means writing something and which something is undecided."* — lane B ∿'s tip

**And two things the mock has already settled**, which this record takes as given rather than
re-arguing. A cell press is *"a state and never a flip, because a map with a button per direction
has to be able to say this step is on and mean it, and a control that could only flip has no way to
arrive."* And the mode belongs to the pattern: *"It is armed because it is what the pattern is
rather than a preference the head is holding."* So the mode is a field of a pattern, not of the
session and not of a lane.

The maintainer took the design on **2026-09-09** — 「進める」 — against a written proposal whose
recommendations are what is decided here and in
[ADR-0321](0321-a-lanes-target-is-an-operation-with-its-value-elided.md),
[ADR-0322](0322-the-sequencer-is-polled-like-a-transition-live-only-and-its-writes-are-its-record.md)
and [ADR-0323](0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md).

## Decision

```rust
/// One bar. ADR-0306.
pub struct Pattern {
    mode: StepMode,     // what the pattern is, not a preference the head holds
    lanes: Vec<Lane>,   // in the order the rows are drawn
}

pub struct Lane {
    target: LaneTarget, // ADR-0321
    steps: u16,         // sixteen slots; bit k is slot k
    on: f32,            // what an on step writes
    off: f32,           // what an off step writes
    muted: bool,
}

pub enum StepMode { Sixteenth, Eighth }
```

**`StepMode::name()` is a `match` and not a table**, on `Sync::name`'s and `BlendMode::name`'s
stated terms — *"a mode added to the enum does not compile until it has a name"* — returning `1/16`
and `1/8`, which is what the head's pill reads. `count()` is 16 and 8; `steps_per_beat()` is 4.0 and
2.0, which is the multiplier in ADR-0222's step index.

### The store is sixteen slots in both modes, and an eighth reads slot `2k`

A mode change is a change of **reading** and nothing else. `1/16` → `1/8` → `1/16` returns exactly
what was there, and an eighth-mode step sits at the same musical instant as the sixteenth it is
drawn over. That is ADR-0306's own sentence — *"the row keeps its width, so the cells halve in the
finer one"* — the row is one bar either way and the cells are a division of it. It has a witness in
the mock: the fourth lane (`B ∿`, `L2:0 twist`) is on every other cell, *"so it lands on each
outline and again halfway between"*, which is an eighth-mode lane drawn in sixteenth mode.

`u16` and not `[bool; 16]`: sixteen bits is the whole bar, a lane stays small enough to copy, and
the pattern is a value the console holds per frame.

### A cell is a bit, and the level is the lane's

Two numbers, `on` and `off`, and they live on the lane because a level only means anything against a
target. A lane on a fader with `on = 1.0, off = 0.0` is a gate; one with `on = 0.8, off = 0.5` is a
pulse. **The drawing does not change** — a cell stays a lit square, which is the whole reason this
answer is available.

**An off step writes `off`; it does not write nothing.** A lane that wrote only on the on-steps
would leave its target wherever the last on-step put it, which makes a pattern a set of impulses.
The mock's lane A — nearly full, with two gaps — reads as a gate and not as impulses, and a gate is
what a lane on a fader is for.

**Where `on` and `off` come from at `+ lane`.** A fader's are 1.0 and 0.0. A parameter's are the
bottom and top of the control's **published** range, which the console has at the moment of the
press — `view::Param` carries the published range since
[ADR-0286](0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)
— and which the lane must not read later, because a pattern outlives the Set that was loaded when it
was written. The console fills them in at the press, exactly as the exposure operation is completed
from the running look. The write is still checked where it lands
([ADR-0223](0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md),
`Set::write_param`), which is
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md): the surface offers two numbers
and does not decide whether they are allowed.

### A bank is a slot, a name is a file, and they are not the same thing

The mock asks and leaves it: *"whether a bank and a saved name are the same thing has never been
asked."* **They are not.** A bank is a position in the session — `seq 1 · seq 2 · +` — and a name is
what a save files it under, exactly as a deck slot and a Set id are not the same thing. Three things
fall out and all three match what is drawn: the `+` *"lands on an empty one rather than being an
operation of its own"*, because an empty bank needs no name and requiring one would make the `+` a
dialog; `SelectPattern` names a **bank index**, which a pill can say and a model can say; and saving
a pattern and putting a saved one back are two later rows on `SaveArrangement`'s and
`RestoreArrangement`'s shape, which are the rows that introduce the name.

**Four banks, fixed**, on the deck's precedent. A bank list that grows is a control nobody drew, so
`+` means *the next empty one* and is disabled with a reason (rule 04) when all four are full.

### Where the type lives, and what this record does not decide

`Pattern` and `Lane` go in a new crate, **`karakuri-pattern`**. Not the console — a pattern is not a
drawing, and the CLI must be able to load one. Not `karakuri-operation`, whose `Cargo.toml` has no
`[dependencies]` entries at all and says why — *"a dependency here is a dependency each of them
pays"* ([ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)) — so it cannot
hold a serialiser. `LaneTarget` and `StepMode` are the exception and go in `karakuri-operation`
beside `NodeAt` and `ParamAt`, because two of the five payloads name them and a payload may name
nothing a surface cannot construct; that is ADR-0321's decision and its reason.

**The file format and the store directory are not decided here.** ADR-0227 leaves them open *for the
record that has something to serialise*, and nothing built on this record serialises: the first
slice holds a pattern in memory and saves nothing. One record when saving lands. **The bank count is
not a record either** — it is a drawing decision, taken in the console and noted in the tip.

## Alternatives rejected

**A `Vec<bool>` sized to the count.** Switching the mode destroys eight steps or invents eight. The
mode is one press with nothing to confirm against, so this makes a press destructive with no
warning — which rule 04 forbids and which
[ADR-0023](0023-regeneration-is-destructive-in-a-slot-and-safe-in-the-library.md) already refuses in
the library, destructive in a slot and safe in the library.

**Sixteen slots, and at an eighth slots `0..8` are the eight steps.** A mode change then moves the
pattern in time: the second half of the bar becomes the second half of nothing. The `2k` reading is
the one under which the mode is a reading rather than an edit.

**A cell carries a level.** The cell becomes a small bar rather than a lit square, which is a
drawing the mock does not make and a control the operations page has no row for. Adding it would be
*a requirement invented by a drawing* — ADR-0306's own reason for refusing a length control, which
it calls *"the failure this bay has just been through once."*

**A cell is a bit; on writes the top of the target's declared range and off the bottom.** No new
state, and it is wrong for a fader: a lane that slams the fader to 1.0 and back is then the only
fader lane anyone could make. It also makes a lane's behaviour depend on a Set it may outlive, which
is the same mistake as reading the published range at emission rather than at the press.

**A lane writes only its on-steps.** Cheaper on the stream and it is the impulse reading of a
sequencer. It loses to what the mock draws: lane A is a gate, and a gate that stops asserting is not
one.

**A bank is a saved name, and the list is an unbounded `Vec`.** It collapses two identities that
behave differently — a position a pill can say against a string the store files under — and it buys
nothing until a pattern can be saved, which is a later record. The mock's own tip says *"how many
patterns this row holds … [is] not decided"*, and four is the count that draws.

## Consequences

**The pages moved first and nothing was built on the day this was written.** What it bought then was
that the bay's five payloads were waiting on one crate rather than on an unasked question, and that
the console's tips could stop asking twice what a cell is worth.

**What was built the same day, and each clause is a description of the tree** (2026-09-09):

- **`crates/karakuri-pattern`**, a new workspace member (`members = ["crates/*"]` picks it up), with
  `Pattern` — one bar, a `StepMode` and a `Vec<Lane>` — `Lane`, `Banks` (the session's four, and
  which is armed) and `Playhead` (the poll's memory of the step index, which is *not* in the pattern
  because a pattern is what gets saved). `SLOTS` is 16 and `BANKS` is 4. It depends on
  `karakuri-operation` and on nothing else, and it holds **no serialiser and no path**.
- **`karakuri-operation` gains `LaneTarget` and `StepMode`**, which is ADR-0321's consequence and is
  listed there. `StepMode::slot_of` is where *an eighth reads slot `2k`* lives, and
  `an_eighth_reads_every_second_slot` and `a_mode_press_keeps_every_slot` are what hold it.
- **The five payloads carry what this record decided**: `SetStep { pattern, lane, step, on }`,
  `SetLaneMute { pattern, lane, muted }`, `PointLane { pattern, target }`,
  `SetPatternGrid { pattern, grid }`, `SelectPattern { pattern }` — each naming the bank rather than
  implying the armed one. `SetStep`'s `step` is a stored slot and not a step index, and the console
  sends `2k` in the finer reading (`a_cell_press_sends_the_stored_slot_and_not_the_drawn_step`).
- **The console draws the bay**: `view::Sequencer` — the mode pill, the `step n of m` readout, the
  ruler, the playhead column and a row per lane — off a `view::Sequenced` the host writes per frame.
  The cells and the labels are controls and the ruler, the readout and the playhead are readouts.
- **`crates/karakuri` holds the four banks and polls them**, and seeds bank 0 with one lane over
  deck A's fader, **muted**. The mute is not decoration: an unmuted lane writes its target at every
  boundary, on-steps and off-steps alike, so a lane seeded live would hold deck A at `off` from the
  moment the window opened — see `demonstration_banks`, which is where that is argued.
- **`docs/manual/console.html`'s cell tip lost *"Two things a cell still cannot say"***: the
  address is a bank, a lane and a slot, an on cell writes the lane's `on` and an off cell its `off`.
  The two bank pills and the `+` lost *"whether a bank and a saved name are the same thing has never
  been asked"* and *"a pattern has no identity anywhere yet"*; the grid head lost *"there is
  nothing to press here yet … nothing in this workspace holds a pattern"*.
- **The *Sequencer* note gained two paragraphs**: what a lane's two levels are, and the two gaps
  this bay owes a note rather than a fix.
- **`docs/manual/operations.html`'s five rows name what their payloads carry**, and three panel
  badges are `has`: *Toggle a step*, *Mute a lane* and *Choose what a step is worth*. *Point a lane
  at what it drives* and *Choose which pattern the sequencer plays* stay `plan`, because the target
  chooser and the bank pills are not drawn.

**What is still not drawn, and why each one is scope rather than a gap.** The bay head's
`seq 1 · seq 2 · +` bank pills: four banks exist and `SelectPattern` names one, and what is missing
is a head that can say *which* pill is armed — `view::Head::words` carries `&'static str`s, so the
arming is a change to the head machinery rather than to this bay. The foot's `+ lane`, which needs a
target chooser nobody has drawn and a bay body the arena can grow. The `Param` lane arm, the store
directory, the file format and saving.
