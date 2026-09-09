---
id: 0331
title: The key column is spelled two ways, and a built badge keeps its bare letter
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0087]
tags: [docs, keyboard, vocabulary, ui]
---

# The key column is spelled two ways, and a built badge keeps its bare letter

## Context

[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
decided the keyboard and **edited no page on purpose**: a key press is addressed to the bay that has
focus, the six keys inside a bay are rules about kinds of thing, and a letter survives globally only
where the operation it names has no operand. Its own Consequences say what the manual then owes —
*"the key column stops naming only letters"* — and `docs/roadmap.md` M5.13 carries that as item 1.

**The column as it stood said `has key l`, `plan key y`, `gap key —`.** Under the record a digit,
`space` and `enter` reach a different row in every bay, so a badge naming a press and not a bay
names nothing: a reader told to press `space` and not told what it lands on has been told nothing,
which is the failure ADR-0220 rejected a whole program for.

**And the column is not one thing.** Sixteen rows are reached today by the flat letters
`crates/karakuri/src/main.rs` binds, and those letters work. The grammar is designed and none of it
is bound: nothing is on `Tab`, `esc` still quits, and the console takes no keyboard focus at all.
[ADR-0213](0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
fixes what `has` means — *an operator running the instrument reaches it* — so the two halves of this
column are describing two different keyboards, one that exists and one that is decided.

## Decision

**The key column carries two spellings at once, and the page's head says which is which.**

- **A `has` badge names a bare letter** — the letter this instrument binds today, because the
  keyboard it has is one letter per operation and the press goes wherever the letter goes.
- **A `plan` badge names a key of the grammar and the bay it is addressed in**: `space · in the
  Mixer`, `enter · in Staging`, `↑↓ · in the Transport`. The key is spelled the way ADR-0259 spells
  it and the bay is one of the nine.
- **A `gap` badge is a row no grammar reaches**, and the legend gives the record's three reasons
  rather than the retired one: an effect whose parameters have no spelling, an operand that is a
  name or a path no key press can type, or a way in that is a flag before the run or a model's.
  *A letter that ran out* is no longer a reason for anything.

**No badge moves between `has` and `plan` in either direction.** What is built has not changed, so
neither does the meter. Twenty-nine rows were re-spelled on 2026-09-09: ten that were already
`plan`, and **nineteen that moved from `gap`** — the list ADR-0259 reads off the page, because those
rows were `gap` for want of a route and the grammar is one.

**A `gap` row moves to `plan` only where its bay draws the control the key hangs on**, which is
ADR-0259's own condition and is read off the *panel* column. *Set a deck's mask position* is the one
row of the record's twenty that does not move: its panel badge is `gap`, so there is nothing drawn
for a digit to land on and a key badge would name a press with no target.

## Alternatives rejected

### a. Spell the whole column in the grammar now

**The case is that it is what ADR-0259 decided**, and half a column in the old spelling is the state
the record describes as costing the visible half. Twenty-one of the twenty-six letter-naming rows
are focus-relative under it; writing all twenty-six as a bay and a press is one pass over the page.

**It loses on what `has` means, and the loss is not cosmetic.** A built badge is a claim that an
operator reaches the operation today, and `space · in the Mixer` reaches nothing: nothing is bound
to `Tab`, no bay takes focus, and the six keys are not in the `match`. So the honest forms are
either a built badge naming a route that does not exist, or sixteen rows demoted to `plan` — which
would say the instrument's keyboard reaches nothing, and it reaches sixteen rows.

**And it loses on the check, which is the same objection one level down.**
`key_column::every_key_route_the_page_marks_built_is_bound_by_the_instrument` reads a `has` badge as
whitespace-separated key names and resolves each against `ROWS`. A bay written into a built badge is
either a key nothing binds — a panic naming the row — or a check rewritten to parse a badge. ADR-0259
already schedules that rewrite: *"Two are replaced — both directions of the badge check, because a
key alone no longer determines a row and a badge naming a bay needs parsing rather than a word
match."* **That work belongs with the code that binds `Tab`**, not with a page edit, and doing it
here would replace a check with nothing on the other side of it to check.

**What it costs to defer is written down rather than left to be met.** *Bring back what is folded*
keeps a bare `z` and does not gain the focus spelling ADR-0259 gives it — `Tab` to the folded bay
and `space` — and *Quit* keeps a bare `esc` the record retires to the platform accelerator. Both are
badge edits that wait on the same rewrite.

### b. Keep letters everywhere and put the bay in prose

**The case is that a badge is small and a sentence is not.** The head could say *a designed letter is
provisional and the bay it is pressed in is under ADR-0259*, and no badge would have to grow.

**It loses because the badge is where a reader looks**, and five of the ten `plan` rows named a free
letter — `y`, `x`, `c`, `t`, `- = \``. Those are promises the record retired: *no operation is given
a letter for its initial*. A page that goes on printing them reserves eight letters for a scheme
that is not coming back, and the reader who acts on the badge and never reaches the paragraph acts
on the retired scheme.

### c. Fold a bay in the mock and put the dashed ring on it there

**The case is that `console.html` is a drawing and this is a drawing**, so the mark belongs in the
panel with every other mark.

**It loses twice over.** Every bay's body carries panel cells the operations page's *panel* column
names — *Keep a candidate* is `staging row`, *Master out* is `master out` — so folding a bay in the
mock takes a specified control off the page to draw a state. And the mock already draws keyboard
focus once, on deck B's fader; a second dashed ring inside the same panel says focus is in two
places, which is the one thing focus cannot be. **So the mark is drawn beside the note that defines
it** — under *Two focuses, and they do not look alike* — where it is a specimen rather than a second
claim about where the address is.

### d. Move every row the grammar could reach out of `gap`, and ignore whether a control is drawn

**The case is uniformity**: the grammar reaches whatever a bay draws, and what a bay will draw is
already specified, so every row with a designed control has a designed key route.

**It loses on ADR-0259's own clause** — *"each is `plan` only once its bay draws the control the key
hangs on"* — and the clause earns its place on the row that tests it. *Set a deck's mask position*
has no control in any bay, drawn or specified: its panel badge is `gap`. A key badge there would
say a press lands on something, and nothing has said what.

## Consequences

- **`docs/manual/operations.html`'s key column reads 16 `has`, 29 `plan`, 19 `gap`** as of
  2026-09-09, against 16 / 10 / 37 before. The counts are a reading of the page and nothing checks
  them; `grep -c 'rt plan">key' docs/manual/operations.html` is the one M5.13's exit is measured by.
- **M5.13's exit gets larger and stays the same criterion.** *No `plan` badge in the key column* now
  covers nineteen rows that read `gap` and were never going to be picked up under that exit.
- **`key_column`'s six tests are green untouched.** `KEYS`, `ROWS` and `NO_ROW` needed no edit,
  because both badge checks read only `has` rows: one iterates `ROWS` and requires the row's badge
  to be `has` and to name the key, the other filters the page for `has`. **That is what made this
  page edit separable from item 4** — and it is also the reason a built badge cannot yet carry a
  bay, so the two facts are one fact read from either side.
- **`docs/manual/console.html`'s *Two focuses, and they do not look alike* is rewritten.** Focus is
  *the bay a key press is addressed to* and the deck selection is *the Mixer bay's remembered
  address*, word for word with `concepts.html`'s *Focus*. The sentence that said the deck selection
  *"is what a key press is addressed to"* is gone: under ADR-0259 that is focus, and the selection
  is one bay's instance of the mechanism focus moves.
- **The mark for a folded bay holding focus exists**, which is the one drawing ADR-0259 created a
  need for and left open. It is the bay head alone wearing `.wfocus`, saying two things — there is a
  bay here, and `space` opens it.
- **`concepts.html` needed no sentence changed.** It was written after ADR-0259 and already carries
  the distinction the console page has now been brought into line with, which is the direction that
  document is supposed to run in.

## What this record does not do, and what was not checked

**It binds no key and moves no `has` badge**, so nothing an operator does changes. The keyboard is
still one letter per operation and `esc` still quits.

**Three rows were out of this edit's reach and are somebody else's**: *Element capacity, seeds, the
camera*, *Wire a procedure's input to a node* and *Narrow the published interface*. None of the three
is in ADR-0259's list of nineteen, and all three are `gap` for reasons the record puts in its
remaining eighteen — an operand that is a path or a name — so none needs a key badge moved.

**Two rows the grammar plausibly reaches were left `gap` because the record does not name them**:
*Walk the edit history*, whose control landed on 2026-09-08 after ADR-0259 was written, and *Attach
a signal to a parameter*. The record's list is the authority here and re-reading the page against
the grammar is a second reading nobody has taken.

**ADR-0259's own count is off by one and it says so.** It reads *"nineteen of the 37 `gap` rows"*
and then names twenty; the record calls the figure *"a reading of the page rather than a count
anything checks"*. Nineteen moved because the twentieth failed the drawn-control condition, which is
an arithmetic coincidence rather than a resolution.

**Two prose sites elsewhere still name retired letters and are outside this edit**: the transition
row's tooltip on `console.html` — *"From the keyboard the settings are z, n and j, and the wipe is
c"* — and `key_column`'s module documentation on *Save the arrangement* and *Put a saved arrangement
back*, which argues both cannot be bound while the page now marks their key route `plan`. ADR-0259
answers the second: *"both arguments are answered by a mechanism rather than by a letter."*
