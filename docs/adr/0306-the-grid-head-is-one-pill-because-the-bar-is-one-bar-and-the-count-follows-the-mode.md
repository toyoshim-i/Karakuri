---
id: 0306
title: The grid head is one pill because the bar is one bar and the count follows the mode
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0087, 0090]
tags: [console, manual, vocabulary, sequencer]
---

# The grid head is one pill because the bar is one bar and the count follows the mode

## Context

The sequencer bay's grid head drew three pills — `16`, `1/8`, `2 bars` — and two lines below them
the ruler drew four numbers over sixteen cells, which is a beat every four cells: **one bar of
sixteenths**. The two do not describe the same pattern. Sixteen cells of an eighth is two bars, and
under two bars a beat falls every two cells and a bar every eight, so the ruler's groups are wrong
if the pills are right and the pills are wrong if the ruler is. Every lane repeats the ruler's
groups as an outline on the cell a group starts on, so the wrong count is drawn on every row of the
bay rather than in one strip.

**Both were in the mock's first commit** (`5311dc7`, 2026-08-23) and neither had moved since.
`Operation::SetPatternGrid`'s documentation later rationalised **the pills** — *"sixteen steps of an
eighth apiece is two bars, so any two of the three fix the third"* — which is arithmetic that is
correct and **taken from the wrong two**: it fixes a length out of a count and a subdivision, where
the length was never a hand's to set. `docs/roadmap.md`'s M5.9 records the inconsistency and says
the design is a mode — eighths or sixteenths, both one bar, the cell width halving in the finer one
— and left two questions open: whether one pill replaces three, and what the row is then called.
The maintainer answered both on **2026-09-08**, which is this record.

Nothing in this workspace holds a pattern, and the panel draws nothing of this bay but its head, so
this is a decision about a drawing and a title. That is the order the manual is written in: the page
is the specification and the implementation is checked against it.

## Decision

- **A pattern is one bar.** Fixed, and not a control.
- **A step is a sixteenth or an eighth.** Two values, and the list is closed.
- **The grid head draws one pill**, reading `1/16` — the mode the ruler and the cells were already
  drawn in. A press asks for the other of the two by naming it, a state and never a flip, which is
  the rule the cells below it already carry.
- **The count follows the mode**: sixteen cells at a sixteenth, eight at an eighth. The row keeps
  its width, so the cells halve in the finer one — which is what the *Sequencer* note has said about
  eight and sixteen steps from the beginning.
- **The row is *Choose what a step is worth***, on `docs/manual/operations.html` and as
  `Operation::SetPatternGrid`'s title string, which is the same words by
  `the_manual_and_the_vocabulary_agree`.
- **The payload stays `Undecided`.**

**Why the payload does not move, when the list it was waiting for now exists.** `SetPatternGrid`'s
`Undecided` was argued on two things, and one of them is gone: a subdivision was *"a list this crate
has to own, on `Curve`'s terms, that nothing anywhere holds yet"*, and a two-valued mode is exactly
that list. What is left is
[ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)'s rule —
an operation asks for what a *surface* can say — and no surface can say either value: the panel
draws nothing of this bay but its head, and there is no pattern for a mode to be of. So the payload
waits on a control rather than on a decision, and its documentation says that rather than leaving a
reader to infer it from a marker.

**The beat-clock example this decision hands back.**
[ADR-0222](0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md) and
[ADR-0255](0255-three-clocks-run-at-once-and-a-slower-ones-work-never-lands-on-a-faster-one.md) both
take **a 1/8 step at 128 BPM, 234 ms**, against the beat clock's stated band of 0.5–4 s, and neither
has the finer case. **A sixteenth at 128 BPM is 117 ms** — half of it — and the sixteenth is the
mode the head now reads, so the worse case against that band is the one drawn by default. Neither
record needs correcting: ADR-0255 says the bands are descriptive and that what fixes a clock is what
may happen on it rather than its period, and 117 ms is further outside a band that was never the
test. It is written here so the figure exists somewhere.

## Alternatives, and why they lost

### Three pills with the arithmetic corrected — `16`, `1/16`, `1 bar`

The smallest change, and it keeps the head reading in whichever unit a hand is counting in. It
loses because **once the arithmetic is right the pills are redundant**. With the length fixed at a
bar and the count following the mode, two of the three figures are derivations of the third, so the
head would draw two readouts dressed as controls: a hand pressing `1 bar` has nothing to move to,
and a hand pressing `16` is choosing a subdivision by another name. That is
[P-0087](../principles/0087-name-the-property-never-the-shape.md) exactly — the property is *what a
step is worth*, and a count and a length are two presentations of it.

The old tip's own open question disappears rather than being answered: it said *which two a hand
sets and which one follows is undecided*, and *a press that moved two figures at once would have to
say which one it was leaving alone*. With one control there is no second figure to leave alone.

### A length control, so `2 bars` becomes something a hand sets

This is the reading that would have made the three pills honest, and **nothing in the design wants
two bars**. The ruler is one bar. The lanes are sixteen cells against that ruler. The *Sequencer*
note describes eight and sixteen steps of one row rather than a row that grows. No record asks for
a pattern longer than a bar, and the operations page carries no row for setting a length. Adding one
would be a requirement invented by a drawing, which is the failure this bay has just been through
once.

If a longer pattern is ever wanted it arrives as it should: a control drawn on the console, a row on
the operations page, a variant behind it — and this record is then the thing to supersede.

### Keeping the title *Choose a pattern's steps and what a step is worth*

Refused. It names two things where one is left, and **the half it names first is the half that
stopped being chosen** — a title that says an operation sets a step count is a description of
replaced behaviour, which is
[ADR-0031](0031-a-document-describing-replaced-behaviour-is-worse-than-none.md)'s subject. The
shorter title also happens to be what the operation always was: even under the three pills, the
subdivision is the figure the other two were read against.

## Consequences

- **`docs/manual/console.html`'s grid head is one `pill armed` reading `1/16`**, where it was three
  spans. Its `data-tip` says what the mode is, its two values, that a press asks for the other by
  naming it, that the bar is one bar so the count follows — sixteen cells at a sixteenth, eight at
  an eighth, the cells halving — what the three pills were and why their arithmetic came from the
  wrong two, the 117 ms against the 234 ms, and that there is nothing to press because nothing holds
  a pattern. Its MIDI line still reads *unassigned*, and its reason has changed: a two-valued mode
  **is** a word from a closed list, so what is missing is the pattern rather than the spelling.
- **The ruler's tip no longer reports a disagreement.** It said *"it does not agree with the head
  above it"* and asked which of the two gives way; it now says four numbers over sixteen cells is
  one bar of sixteenths at the mode the head reads, and that the head was the side that gave way.
- **The *Sequencer* note keeps its three paragraphs.** Its sentence that the step grid is the beat
  clock subdivided and not a fourth clock is unchanged, and its last clause gained the sixteenth:
  117 ms, twice as far outside the band as the eighth it already named.
- **`docs/manual/operations.html`'s row is `<h3>Choose what a step is worth</h3>`**, its tip states
  the mode, the two values, the fixed bar and the arithmetic that was taken from the wrong two, and
  its four badges are unchanged — panel `plan`, key gap, MIDI gap, MCP `plan`. The panel cell names
  the control that exists now: **grid mode pill**, where it said *sequencer head*.
- **`karakuri-operation`'s `SetPatternGrid` carries the new title string and `grid: Undecided`.**
  Its documentation quotes the sentence it used to make, says the list is closed and that what is
  left is ADR-0192's rule, and carries the 117 ms beside the 234 ms.
  `the_manual_and_the_vocabulary_agree` was watched to fail on the title — *`Choose what a step is
  worth` is an operation in `docs/manual/operations.html` and no variant of `Operation` carries that
  title* — with only the page changed, and passes with the string.
- **The classification does not change and neither does what it writes.**
  `karakuri-operation/src/gate.rs` still reads
  `SetPatternGrid { .. } => Standing::ClosedUnclassed(Unclassed::Lanes)` with the sequencer's five
  together, and `karakuri-operation-record` still answers `Written::Owed(Owed::Undecided)` for all
  five. Neither file's prose named the old title or the three pills, so neither needed a word
  changed; the record crate's *sixteen a bar* was already the mode this record settles on.
- **`docs/roadmap.md`'s M5.9 says both open questions are answered** and points here. Its row list
  carries the new heading; its *What that moves* paragraph says the heading *was* the old one; the
  rest of the entry — the five `plan` badges, the exit, the blocked-on, the arithmetic and the
  prose item — is unchanged, because none of it moved.
- **Nothing is built by this.** No control is drawn on the panel, no route is added, no badge moves,
  and `Undecided` is still `Undecided`. What this record buys is that the mock stops specifying two
  different patterns, and that the operation waiting behind it is waiting on one thing rather than
  two.
