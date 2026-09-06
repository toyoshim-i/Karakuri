---
id: 0264
title: A reading is one question, and only the opening asks it
status: accepted
date: 2026-09-06
supersedes: []
superseded_by: []
principles: []
tags: [console, operations, library, mcp]
---

# A reading is one question, and only the opening asks it

## Context

`65c9e34` drew the reading a Set gives up before a load is spent on it: a press on the Library
bay's `read` chip opens a block under the cursor's row, and the block follows the cursor. The
control was specified by `7eba97a` and `docs/manual/console.html`'s note *Reading a Set before you
spend a load on it*; the mock says *"move the cursor and the reading moves with it, so one row is
open at a time"* and says nothing about the vocabulary.

A reading therefore has three gestures — the press that opens it, a cursor move while it is open,
and the press that puts it away — and exactly one operation in the vocabulary,
`Operation::ReadSet { id }`, which `karakuri-operation-record` classes `Silent(Question)`: *"it asks
rather than changes, and a question writes no record."* Which of the three gestures emit it was
undecided, and each of the two that are not the opening had a plausible answer that emits.

The two are not independent questions. Both ask where a reading's operation boundary is, and both
are answered by what the operation *is*: `ReadSet` names a Set and asks what it declares. So they
are one record.

## Decision

**The opening is the only gesture that emits `ReadSet`.** A reading is one question with one
answer, and neither moving its operand nor ending it is a second asking.

- **Opening emits.** `view::Read::Open(Operation::ReadSet { id })` leaves the console; the host
  answers it with `read_reading`, which reads the store the console may not (ADR-0156).
- **Closing emits nothing.** `view::Read::Shut` is performed where it is pressed —
  `App::asked_to_read` calls `View::shut_reading` and returns `Acted::Nothing`. What it changes is
  which rows this bay draws, which is the console's own state and no more an operation than a fold
  is.
- **A cursor move with a reading open re-reads and emits nothing.** The host calls `read_reading`
  again for the row it arrived at, off the same cards, on the press and never on a frame (P-0091).
  The key still names no operation, which is `key_column`'s `("up", &[])`.

The move genuinely produces a new answer, and it is still not a new question: what the operator
asked was *what does the Set under the cursor declare*, and the cursor is that question's operand.

## Alternatives rejected

**Emit `ReadSet` when the reading is put away.** Symmetric, and it would make the block's two states
two operations, which is how a control with an on and an off is usually spelled. It loses on what it
would say: a `ReadSet` on the close asserts that a question was asked at the moment one stopped
being. The operation carries an id and means *what does this Set declare*; emitted on a close, it
means nothing, and a model watching the surface would see a Set read twice as often as it was read.
The vocabulary has no closing half of a read, and inventing one to be the second door of a chip is
the kind of second answer `docs/contributing.md` §4's *A statement is held true by the thing it
describes* is against — the reading a model gets from `mcp::read_set` and the reading an operator
opens would no longer be one thing.

**Emit `ReadSet` on every cursor move while a reading is open.** The stronger of the two, and the
one somebody will re-propose: the move really does read a different Set, and a stream in which every
reading is an operation is a stream that says what the operator looked at. It is refused because of
what the arrow keys are. `docs/manual/operations.html` marks the *Read what one Set holds and
declares* row's key column `—` with the `gap` class, and `console.html`'s note says why: *"Which
keys exist is the operations page's to say and it names none for this row, so the chip is the only
way in until it does — which is a gap and not a decision."* A key that emitted `ReadSet` would be an
operator reaching that row from the keyboard while the page says nobody can — the panel deciding a
question the page reserved, silently, and without the row on the page changing. The page moves
first (`docs/contributing.md` §5, §6), so if the arrow keys are to reach this row, that is an edit
to the key column and not a line in `main.rs`.

**Close the reading on a cursor move, so that the only reading open is the one that was asked
for.** This makes the emission question disappear by deleting the second gesture. It loses to the
mock, which is the specification for the drawing: *"move the cursor and the reading moves with it."*
It also costs the control its point — the reading exists so that a run down the listing is a run
down what each Set declares.

**Keep the reading pinned to the row it was read of and let the cursor leave it behind.** A second
way to avoid re-reading. It loses to the same sentence, and to what the block would then be: a
reading drawn under a row that is not its own, which `View::opened` exists to make impossible.

## Consequences

- `crates/karakuri-console/src/view.rs`'s `Read` has two variants and only `Read::Open` carries an
  `Operation`. `Read::Shut` carries nothing, so a host cannot emit for a close by mistake — the
  refusal is in the type.
- `crates/karakuri/src/main.rs`'s `asked_to_read` returns `Acted::Emitted(Some(op))` for `Open` and
  `Acted::Nothing` for `Shut`, and the arrow-key branch calls `read_reading` under
  `moved && self.readout.view.reading_open()`. `View::reading_open` exists for that branch: it is
  the flat question *is anything open at all*, where `View::opened` answers the drawing's question
  and is `None` once the row has been rewritten out from under the reading.
- **The reading is not put away when its row goes.** `View::opened` answers `None` while the row
  under the cursor is no longer the Set the reading is of, and the reading is kept — so a narrowing
  that takes the row away and a second that brings it back are one gesture to the hand that made
  them. Nothing emits on either.
- **The operation count means readings taken.** One `ReadSet` per press on the chip, and a walk down
  the listing with a reading open emits none — so the number of `ReadSet`s in a session is the
  number of times somebody asked, not the number of rows they passed.
- **Three tests hold the three gestures, and each is where the seam it crosses is.**
  `crates/karakuri-console/tests/library.rs`'s `the_read_chip_asks_for_the_set_under_the_cursor`
  holds the chip answering `Read::Open(Operation::ReadSet { .. })` with nothing open and
  `Read::Shut` with a reading open. `crates/karakuri/src/main.rs`'s
  `a_press_on_the_read_chip_asks_for_the_set_under_the_cursor` drives the press handler and asserts
  the second press answers `Acted::Nothing` — the console's own tests cannot see it, because the
  press handler is in the host. The key half is the `key_column` table's `("up", &[])` and its
  `NO_ROW` list, which is checked against
  [operations.html](../manual/operations.html): a day the arrow keys reach this row, that entry is
  what fails.
- **The keyboard route to this row stays owed to the page**, not to this record: the key column is
  `gap` and this decision does not close it. If the page ever names a key, this record is what says
  the emission question comes back with it.
