---
id: 0312
title: The `params` pill is a toggle, and the Library bay scrolls
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0082, 0090, 0091]
tags: [console, ui, library, manual, m5]
---

# The `params` pill is a toggle, and the Library bay scrolls

## Context

The Library bay's foot carried a capsule reading `read`. A press opened, under the row the cursor
is on, what that Set declares — every knob with its range and default, the element count, what the
geometry emits — and a second press put it away.
[ADR-0264](0264-a-reading-is-one-question-and-only-the-opening-asks-it.md) settled that only the
**opening** emits `Operation::ReadSet`: closing performs the console's own state and a cursor move
re-reads without asking again, so the number of `ReadSet`s in a session is the number of times
somebody asked.

**The maintainer read the control and said two things about it**, verbatim, on 2026-09-09:

> あぁ、それってアイテム選んだ時にも展開されて出るやつ？readだと意味わかりづらいな。expandとかparams
> が直感的な気もするけどトグルボタンになって欲しい気もする。

*Is that the thing that expands under an item when you pick one? `read` is hard to make sense of.
`expand` or `params` feels more intuitive — though I also want it to be a toggle button.*

**And one thing the bay could not do, which the same block made worse.** The listing stopped where
the bay ran out of room. `View::walk`'s own doc said so and defended it: *"This bay has no scroll
position and inventing one would be a control ... So the reachable Sets are the listed ones, and a
cursor allowed past them would sit on a row nobody can see."* The foot said `5 of 27` and the
twenty-two were unreachable **by any means** — no key, no pointer, no widening short of a taller
window. Opening a reading made it sharper: the block pushes the rows under it down, and rows pushed
off the end had nowhere to be reached from.

**That argument was answered one bay over three days ago.**
[ADR-0307](0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md) put a position on
an Inspector pane, on the same shape of complaint — *"a short pane drew no knob at all, and a bay
whose whole purpose is the knobs cannot answer a small window with an empty pane"* — and it wrote
down the whole mechanism: the wheel over the region, a position that is the console's own, clamped
where the region is laid out and stored unclamped, `input::wheeled`'s seam beside `input::claim`,
rule 04's count as the mark, and no scrollbar. **What is decided here is that the Library takes
that mechanism**, not what the mechanism is.

**Two decisions in one record**, because they meet in the foot and in the same paragraph of the
manual: the pill's block is part of what scrolls, and the pill's own count — `n of m` — is what
rule 04 answers the scroll with. Neither could be written without saying what the other does to it.

## Decision

### 1. The pill is called `params`

**`read` named what the press does to the file; `params` names the block.** An operator who has not
pressed `read` has no way to know what appears; nine of the ten rows that appear are the knobs the
Set publishes, so the control says what you will be looking at.

**`expand` was the maintainer's other candidate and it loses on a test the panel already applies.**
It names the *motion*, and nothing else on this console is named for one — `load`, `go`, `rec`,
`keep`, `solo` are nouns or verbs about the instrument. A control whose word is the animation it
plays would be the first, and it would go stale the day the block is drawn some other way.

**`Operation::ReadSet` keeps its name and the row keeps its heading.** *Read what one Set holds and
declares* is what is asked for; `params` is what the surface calls the control that asks. This is
the console owning the affordance and never the vocabulary
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)), and it is why none of
`docs/contributing.md` §5's nine places moves except the two badges that name the control.

### 2. It is drawn as a toggle, in mint

**Lit while a reading is open, plain while none is**, which is what every other two-state capsule
on this panel does.

**The argument it replaces was right about the block and wrong about the chip.**
`docs/manual/console.html` said *"the chip itself does not light ... this one pays that debt in a
louder currency: nine lines appear under the cursor, which is not a thing anybody misses."* True,
and it is an argument for the block being **enough**, not for the chip being **wrong** — and rule
03 of [the manual](../manual/index.html) asks a symbol to say what state it is in and what a click
will do. A capsule that reads the same in both states answers the first only by pointing at
something else.

**The light is mint and the colour is part of the decision.** This console spells every state in
one of four: lavender is *the deck the keys are addressed to*, pink is *live*, sun is *priming and
the star*, mint is *armed, on, or held by a signal*. A reading being open is **on**.

- **Lavender is ruled out**, and that is the surviving half of the old paragraph: this bay spends
  lavender on `load`, which is the press it exists for, and a second lavender capsule beside it
  would make the colour mean two things at a width of eight characters.
- **Pink is ruled out on what it means.** `.pill.on` is the pink treatment and is what `rec` and
  `go` wear — a recording running, a wipe that would run. A Set being looked at is not live, and
  the Library bay would have gained its first pink for a control that puts nothing on air.
- **The deck pulldown beside it stays plain**, and that is not an inconsistency: it names a deck
  rather than being in one of two states, so there is no *on* for it to be.

**One derivation and not two.** The paint reads `LibraryBay::reading` — the block this bay is
*drawing* — which is the same value `LibraryBay::read` matches on to decide what a press asks for.
So the capsule an operator is looking at and the answer the press gives cannot come apart, which is
the whole of what a toggle owes.

**P-0090's refusal of *toggle* is not in the way, and it is worth saying why.** That rule rules out
an operation that says *toggle*, *cycle* or *step* — *"a surface that can only step has no way to
arrive, and two surfaces stepping one control disagree about where they are"*. Nothing here is an
operation with two ends: ADR-0264 already decided that the opening emits and the closing emits
nothing, so the two presses are one operation and one console act. **What is drawn is which of two
states this console's own block is in**, which is a readout, and P-0090 is about payloads.

### 3. The bay scrolls, by ADR-0307's mechanism

- **The wheel over the bay** is the way in, three rows a notch — the same `room::size::WHEEL_STEP`,
  because it is the same wheel and `LIB_ROW_H` and `PARAM_H` are the same 22.5.
- **Aimed at the whole bay** and not at the list: a hand over the scope chips turns the listing
  under them, which is ADR-0307's *the whole pane and not its body*.
- **The position is the console's own**, one number, private on `View`, reached by
  `View::library_scroll` and `View::scroll_library_by`. **One and not one per scope**: only one
  scope is ever being read, and the chip press that changes the listing is exactly the moment an
  operator wants the top.
- **Clamped at the draw and stored unclamped.** `library_box` clamps to `0 ..= content - list
  height` and hands the clamped value back as `LibraryBay::scroll`; nothing is written back
  ([P-0082](../principles/0082-looking-never-writes-back.md)). What the *store* clamps against is
  the content, which is a reading of the listing rather than of a viewport.
- **The reading block is part of the content and scrolls with the rows**, because it sits between
  two of them rather than over them. `library_content_h` is one function so the two clamps cannot
  disagree about what a reading costs.
- **The foot counts what is whole.** `LibraryBay::rows` is the `n` of `n of m` and counts rows
  wholly inside the list; `LibraryBay::drawn` is the range that is painted and hit-tested, cut
  edges included. `27 of 27` can never be read off a bay with a row hanging over an edge, which is
  rule 04's *says how much*.
- **A press outside the list reaches no row**, even where the row is drawn: a cut row's hidden half
  lies over the filter fields, and without the bound the bay would answer for a press nobody aimed
  at it. `InspectorPane::grip` is the same guard one bay over.
- **No scrollbar**, for ADR-0307's reasons, which are not re-run here.
- **The cursor is held inside the rows that are drawn**, and that sentence is older than the
  scroll. What changed is that *drawn* is a **range** rather than a prefix, so `View::walk` takes
  one.

## What the keyboard gets, and what it is still owed

**Nothing the keyboard could reach before is out of reach now.** The rows past the end of the list
were unreachable by any means; they are one notch of the wheel away, and the arrows reach every row
inside the window exactly as they reached every row of the list before.

**What is owed is a key that moves the window**, and it is M5.13's with the arrows' walk of a bay's
items — ADR-0307's *"the keyboard's route is not bound here"*, for its reason: a key invented on
this side would be the panel deciding a question
[operations.html](../manual/operations.html) reserves.

**The alternative was to let the cursor walk the whole listing and have the view follow it**, which
would make every Set reachable from the keyboard and is what most list widgets do. It is refused
here rather than dismissed: it makes the arrows a key route to scrolling, which is the decision
ADR-0307 reserved, and it needs `View::walk` — which is called from a key handler with no geometry
in hand — to know how tall the list is before it can say how far to scroll. Both are real, and
neither is a reason it is wrong; it is a decision with a losing alternative and it belongs to
whoever spells the walk.

## Alternatives rejected

### `expand` for the word

The maintainer's own other candidate. **It reads well and it is what the block does.** It loses on
the panel's own convention: it names a motion, every other capsule here names a thing or an act,
and the block could be drawn a different way tomorrow without the control meaning anything
different. `params` names what is in the block, which is what an operator is deciding about.

### Keep the chip unlit

The status quo, with a written argument behind it: the block is nine lines that appear under the
cursor, which is louder than any capsule. **It survives as a fact and fails as a rule.** Rule 03
asks a symbol what state it is in, and *look at something else* is an answer only while the
something else is on screen — a reading opened on a row the bay has since scrolled away leaves the
chip as the only thing that could say a reading is open at all. That case did not exist before this
record and exists now, which is the second decision making the first one necessary rather than
merely tidier.

### `.pill.on` — the pink pressed treatment

The other treatment the mock already has, and the one whose CSS class is literally `on`. **It loses
on what the colour means rather than on how it looks**: pink is *live* on this console, and this
bay would have gained its first pink for a control that puts nothing on air. The palette's own
sentence assigns *on* to mint, and this is that.

### Page the list rather than scrolling it

The mixer's answer one bay up — *"every strip the page holds is visible at once, which is why the
list is paged rather than scrolled"* — and it would keep every row whole, so the foot's count would
never need a second meaning. **It loses on what the two lists are.** A mixer's strips are four
tracks a compositing shader declares, so a page is a real boundary; a library is however many Sets
a store holds, and a page number over it is a coordinate an operator has to hold in their head to
know where they are. It also puts a **control** in the foot — a page forward and a page back — which
owes a row, a key and a map line under rule 01, where a wheel over a region owes none (ADR-0307's
*it is pointer-state, so it has no row*).

### Scroll by whole rows — an index rather than a distance

ADR-0307 turned down the same shape for groups of unequal height and the objection does not
transfer: library rows *are* a stride, so an index would work. **It loses on the reading block**,
which is not a row and not a whole number of them — the top and bottom margins are the block's own —
so an index would either quantise the block into rows it does not have or leave the one thing in
this list that is not a stride unreachable in its lower half. A distance is also what
`Pointer::Wheel` already carries, so an index would convert one back at the seam ADR-0307 built to
avoid exactly that.

### Give each scope its own position

Five numbers where one would do. **Only one scope is ever being read**, unlike the Inspector's two
panes, which are two places in one list at once and are why *that* position is per pane. A position
per scope would be four stale numbers and a fifth in use, and the chip press that changes the
listing is exactly the moment an operator wants the top — so the honest behaviour of the five-number
version is the one-number version's.

## Consequences

- **`docs/manual/operations.html` moved first, under `docs/contributing.md` §5 step 2.** *Read what
  one Set holds and declares* reads `panel has library params` where it read `panel has library
  read`, and its tip names the toggle and says the word changed and why. **No row moved anywhere
  else and no badge changed class**: the operation is what it was.
- **No row for the scroll**, which is ADR-0307's argument unchanged: a position no operation names,
  that nothing outside the console could be the record of, and that `karakuri-layout`'s tree does
  not hold. This is a `docs/contributing.md` §6 change on that half and a §5 one on the pill's.
- **`docs/manual/console.html`**: the foot's capsule reads `params` and wears `.pill.armed`,
  because the mock draws the reading open; its tip is rewritten and carries the word's argument and
  the colour's. *Reading a Set before you spend a load on it* gains two paragraphs and loses the
  *"and the chip itself does not light"* one, whose surviving half — lavender is `load`'s — is kept
  where it is still true. **A new note, *The library scrolls, and the foot says how much it is not
  showing***, and the foot's `5 of 27` at last has a `data-tip`.
- **`crates/karakuri-console/src/view.rs`**: `READ_PILL` is `PARAMS_PILL`, `LibraryBay::read_chip`
  is `LibraryBay::params_chip`, and the chip is painted through `pill_into` off
  `LibraryBay::reading`. `LibraryBay::read` and `view::Read` keep their names, because what they
  answer for is the operation and the operation did not change.
- **`LibraryBay` has three fields it did not**: `scroll`, `content` and `bay`. `rows` keeps its
  name and changes meaning from *how many are drawn* to *how many are whole*, which is
  `InspectorPane::shown`'s own history one bay over — the two are the same number at rest, so every
  reading of the foot that was true before is true now.
- **`LibraryBay::row` answers for every index in the listing** where it used to be a caller's error
  past `rows`, and `LibraryBay::drawn` is the range that is painted and walked.
  `LibraryBay::at_row` is the one bound the hit tests share.
- **`view::library` takes a sixth argument** and every caller passes `View::library_scroll()` — the
  six probes in `input.rs`, `View::draw`, and the press arms and the arrow-key branch in
  `crates/karakuri/src/main.rs`. Tests that are not about scrolling pass `0.0`, which is the bay
  every one of them already had. That is ADR-0307's own churn one bay over, and it is taken rather
  than avoided with a defaulting wrapper: a `library()` that meant *the top* would be a second
  derivation of where a row is, and `input::claim` calling it would hit-test rectangles the frame
  did not draw.
- **`input::wheeled` answers a `Turned` rather than a pane index.** Two regions scroll and this is
  the one place that says which; the Library is asked first and the order cannot matter, because
  the two bays are two leaves of the arrangement and no point is inside both.
- **`View::walk` takes a range rather than a count**, and `crates/karakuri/src/main.rs`'s arrow-key
  branch passes `bay.drawn()`.
- **Nothing declares**, which is ADR-0307's own consequence and is unchanged: a scroll is not a
  live region, `budget.rs` excludes what the operator does by name, and what a scrolled frame costs
  is one `Change::Wheeled(Claim::Panel, true)` per notch.
- **The vocabulary did not change.** `Operation::ReadSet` is what it was and its title string is
  untouched, so `the_manual_and_the_vocabulary_agree` and
  `karakuri-console/tests/panel_column.rs` both hold.
- **`karakuri-console/tests/library.rs` carries six new tests and one whose premise moved.**
  `the_params_chip_says_read_and_never_lights` is
  `the_params_chip_says_params_and_lights_while_a_reading_is_open` and asserts the pair rather than
  one end, because a test of the lit end alone passes against a chip that is always lit.
- **One of the new tests was watched to *pass* against its own defect and was re-aimed**, which is
  worth recording because the mistake is easy to repeat: a press on a row scrolled entirely off the
  top is refused by `drawn()` before any bound is asked, so the bound is load-bearing only for the
  hidden half of a **cut** row. The test now presses there.
- **M5.3's sentence about the `read` pill is rewritten**, and the bay's tooltip inventory names the
  new note and the foot's tip.

## What this does not decide

- **A key that scrolls this bay.** M5.13's, with the arrows' walk of a bay's items.
- **Whether the reading should follow a cursor the wheel has scrolled away from.** It does not
  today: `View::opened` answers `None` the moment the row under the cursor stops being the Set the
  reading is of, and the cursor is held inside the window, so the two cannot come apart by
  scrolling alone. Whether a wheel should carry the cursor with it is the same question as the key
  above and is left with it.
- **What the other bays do.** The Program bay's cells and the Staging lane are each their own, and
  nothing here says a bay that runs out of room must scroll rather than say so.
