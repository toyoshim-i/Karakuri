---
id: 0333
title: The console resolves the address and the window loop names the operation
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0083, 0090, 0091]
tags: [ui, keyboard, surfaces, console, docs]
---

# The console resolves the address and the window loop names the operation

## Context

[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
decided the keyboard: a press is addressed to the bay that has focus, and the keys that act inside a
bay are the same six everywhere because they are rules about kinds of thing rather than about bays.
[ADR-0332](0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)
built the first half — the ring, the address, `Tab` and `esc` — and stopped before the six keys,
leaving two things owed: **the grammar itself**, and **`key_column`'s machinery**, which is built
around one letter reaching a fixed set of rows and reads this program's `match` as text.

That second half is why nothing could move. ADR-0331 wrote the designed spelling onto twenty-nine
rows and moved no badge to `has`, and said exactly why: *"a bay written into a built badge is either
a key nothing binds — a panic naming the row — or a check rewritten to parse a badge … that work
belongs with the code that binds `Tab`."*

## Decision

**The six keys are built in the Mixer and the Library. The console resolves the address and says
what a press landed on; the window loop names the operation.**

### The seam, and why it falls there

`karakuri_console::focus::press` takes the focused bay's address, the key, and a way to ask what the
deck is holding. It answers one of six things: nothing and why, the address moved, one operation, a
level and which way, the scope, or the load.

**What the console can say on its own is the affordance.** Which control a digit named, whether an
arrow has an axis to run along, and what state a chip cycles to are all the surface's — P-0090's
*a toggle is an affordance built over operations by whoever draws it* — and the console already owns
those cycles, at `view::after`, `view::next` and `view::next_shape`, because the pointer needs them.
**The key asks the same three functions the chip asks**, so a press and a click cannot disagree
about which mode comes next.

**What it cannot say is anything that needs the world.** The value a level steps from is on the
deck; the tenth it steps by and the clamp on it decide what the *record* says and are
`karakuri-cli`'s parity argument; a scope step re-reads a directory and a load reaches the store.
`karakuri-console` takes no device and no disk (ADR-0156), so those four stay in
`crates/karakuri/src/main.rs`, where they already were.

**The deck is read at the press and handed in as a closure.** `view::Strip` is the same reading
copied once a frame, and a scheduled fade landing between the frame and the press would cycle from a
state the deck has already left behind — which is the rule the three mix keys this replaces already
kept. It is a closure rather than a value because *which* strip to read is what the address says,
and the address is what `press` resolves.

### Every write goes through the method that already refused it

A digit that names a strip calls `View::select`; one that names a row calls `View::point_at`; an
arrow that walks the library calls `View::walk`; `space` on a chip calls `View::step_scope`. So a
deck the mixer draws no strip for, a row past the listing and a scope with no chip are refused
exactly where they were refused before, and the address descends only where the refusal did not
fire. **The grammar adds a route and no exception**, which is what makes `View::focus_mut` a
`pub(crate)` door for the address path rather than a way past the five methods.

### The dispatch table is data, and the check reads it

`focus::BUILT` is one row per bay whose grammar is built: the controls of its head, the controls of
one of its items, whether `enter` on an item is an act, whether naming an item is the deck
selection, and which way the arrows walk. **Which of the four keys act in a bay is derived from
that** rather than listed beside it — a second list would be a second answer to *what does `space`
do here*.

`key_column::ROWS` is now keyed by a **(bay, key)** pair, with the globals under no bay, which is
ADR-0259's own consequence. The rows stay written down, because a page heading is what a check reads
and is not something this program says to anybody; the *pairs* are derived from the table, and
`the_grammar_the_page_names_is_the_grammar_the_console_declares` holds the two against each other in
both directions.

**Both badge checks parse the badge.** A badge is `<keys>` or `<keys> · in the <Bay>`; every digit
resolves to the one key `digit`, both arrow pairs resolve to `arrows`, and a bay name resolves
through the nine the arrangement has. A row is then looked up by the pair rather than by the key.

**`bound` still reads this file's text, and gains one key from the table.** The named keys are
literals in the `match` and are found as they always were; the digits are a guard, deliberately,
because ten literals would say the digits are bound and say nothing about what they reach. So
`digit` is contributed by `focus::BUILT`. **The seam is one key wide**, which is the smallest it
could be made and is written down rather than left to be discovered.

### The arrows walk from the remembered item, including at bay level

At bay level there is no addressed item, so *the neighbour of the addressed thing* has no answer —
the bay's neighbour is another bay, and that is `Tab`'s. The arrows walk the bay's items **from the
one it remembers**, which is the only place the two fields of an `Address` are read together on
purpose.

**The alternative was to decline until a digit had been pressed**, and it loses on what it takes
away: `up` and `down` walk the Library's rows today with no descent at all, and a grammar that made
an operator press `1` before the arrows worked would be worse at the one bay that already had them.

**The strips are walked and clamped, not wrapped**, which is `View::walk`'s rule one bay over: a
walk is not a cycle, and a key held down must not jump the length of the row.

### `space` at bay level is not bound

ADR-0259 makes `space` on a **bay** the fold, and that is the narrow reason a folded bay stays in
the ring. It is not bound here. Folding is a rule about all nine bays, so binding it in two would
put *Fold a bay away* in the built column with a badge naming two of the nine places it works —
which is a badge that misleads in the direction the whole column exists to prevent. It waits for the
walk over the other seven bays, and `f` still folds the region under the pointer.

**So a folded bay is still in the ring with no key that acts on it**, which is what ADR-0332 already
owed and still owes.

### Twelve letters are unbound and five are kept

Unbound, each because the grammar reaches the same row in one of the two built bays: `0`–`3` are the
Mixer's `1`–`4`; `[ ] \` and `; '` are the arrows and `space` on the addressed trim and fader; `m`
is `space` on the blend chip; `e` is `space` on the Library's head; `l` is `enter` on one of its
rows.

Kept: the seven globals the operand rule keeps, plus **`f`, `g`, `s` and `u`**, which take their
region from the pointer and are owed the focus in bays this slice does not build, and **`k`**,
because ADR-0259 makes it *"an act on a control the Library bay does not draw yet"* and a grammar
key cannot be addressed to a control nobody draws.

### Two rows move from designed to built, and no row moves the other way

*Put a deck on air, prime it, or take it off* and *Set a deck's mask shape* were `plan` with the
panel drawing the control and no key reaching it; `space` on the addressed tally and mask mini
reaches both now. **The other six re-spelled rows keep the badge they had** — what changed is how
an operator gets there, not whether they can.

### Two smaller things that fall out

**`after_blend` is deleted.** This program had two blend cycles — one here and one in the console —
saying the same three in the same order, with a test each. There is one now, and
`karakuri-console/tests/blend.rs` is what holds it.

**`gain_key` and `opacity_key` take a step rather than a letter**, and are total rather than
answering `None` for a letter they do not know. The tenth, the floor on the trim and the clamp on
the fader are unchanged and are still `karakuri-cli`'s. **The fader gains a default it never had**:
`\` had no partner on that control, so `space` on an addressed fader is the first way back to unity
it has ever had — ADR-0259's *"on a level, the one state worth naming is the value it was declared
at"*.

## Alternatives rejected

### a. The console names every operation

**The case is that it is one seam instead of two**, and it is what P-0090 seems to ask for: the
surface owns the affordance, so let the surface hand over a finished `Operation` and let the window
loop apply it.

**It loses on what the console would have to hold.** Naming `SetGain` needs the value the deck is
standing on; naming the load needs the store, the presets root and the folder; stepping a scope
needs a directory read. Every one of those is a device or a disk, and `karakuri-console` has neither
by construction (ADR-0156) — *"a device stays a dev-dependency so `src/` cannot reach for one"*, and
that manifest now has no entry to reach for at all. The console would have to be handed the deck's
whole reading per press to name two operations, which is `view::Strip` arriving through a second
door.

### b. The window loop resolves the address

**The case is symmetry with the pointer**: `input::claim` decides whose a press is and the window
loop acts, so let the keyboard do the same and keep `focus` a plain store.

**It loses because the address is the console's model and nothing else has it.** Which control is
the third of a strip's five, which bay is drawing how many items, and which way its items run are
all facts about what the console drew, and a resolver in `crates/karakuri` would be a second model
of them — the thing `karakuri-layout`'s own header describes as *"a caller with a model of its own …
and a second model of one tree is two answers that drift"*. The pointer's symmetry is real and
points the other way: `claim` is in the console too.

### c. Key `ROWS` by the key alone and put the bay in the row list

**The case is that it is a smaller edit**: leave the table shaped as it is and write *Gain (Mixer)*
into the row strings.

**It loses on the check it exists for.** `space` reaches five rows in the Mixer and one in the
Library; a mapping from `space` to six rows cannot say that pressing `space` in the Library reaches
the scope and not the blend, so the badge check would accept `space · in the Library` on the *Blend
mode* row. The pair is the thing being checked, so the pair is what the table is keyed by.

### d. Ten digit arms, so the text scan sees the digits

**The case is that it keeps `bound` whole**: ten `Key::Character("…")` literals and the scan needs no
help from the console at all, with no new seam between a check and another crate.

**It loses because it would be true and useless.** The scan would report that `0` to `9` are bound
and could say nothing about what they reach, and *what a digit reaches* is the entire question this
column asks — a digit names deck B in the Mixer and the second library row one bay over. The table
is what knows, so the check reads the table; a literal that made the scan complete would be a
literal whose completeness meant nothing.

### e. Unbind `f`, `g`, `s` and `u` now

**The case is ADR-0259's list**: those four take their region from the pointer through
`Readout::target`, and that record names them as the two functions that actually retire from the
binary.

**It loses on when.** They fold and solo *regions* — bay heads and pane edges — which is a route
through bays this slice does not build, so unbinding them now would take away four keys and put
nothing in their place. The record's own condition is the one being kept: a letter goes when the
grammar reaches the same row, and not before.

## Consequences

- **`karakuri-console::focus` gains the grammar**: `Grammar`, `Press`, `Arrow`, `Control`, `Act`,
  `Built`, `BUILT`, `Addressed`, `addressed`, `Held`, `Step`, `Level`, `Asked`, `press` and
  `reaches`. It still takes no device and no disk.
- **`View` gains `focus_mut`, `pub(crate)`**, and `Address` gains `to_the_bay` and `to_item`.
- **`view::after`, `next`, `residency`, `next_shape` and `wipe_kind` are `pub(crate)`**, which is
  what makes the key and the chip one cycle rather than two.
- **`crates/karakuri/src/main.rs` binds four keys as one arm and unbinds twelve letters.** `KEYS`
  goes from thirty-two entries to twenty-three; the legend prints the grammar first and the globals
  after it.
- **`key_column::ROWS` is keyed by (bay, key)**, `NO_ROW` with it, and both badge checks parse the
  badge instead of matching words. Two checks are added:
  `the_grammar_the_page_names_is_the_grammar_the_console_declares` and
  `the_bay_names_the_page_uses_are_the_arrangements_own`.
- **`docs/manual/operations.html`'s key column reads 17 `has`** as of 2026-09-10, against 15 the day
  before: two rows moved from `plan` and six were re-spelled without moving. The `plan` and `gap`
  counts are a reading of a page other work is also editing, so `grep -c 'rt plan">key'` is the
  figure and not a number written here.
- **Three tests go with what they were about.** `m_cycles_the_three_modes_in_the_order_the_chip_cycles_them`
  goes with `after_blend`; `neither_pair_of_mix_keys_answers_for_a_letter_that_is_not_its_own` was
  about an `Option` two total functions no longer return. The claims survive in
  `karakuri-console/tests/blend.rs` and in the types.
- **`reading_follows_the_cursor` keeps both branches and one of them moved.** The keyboard's half is
  now paid once, in the single grammar arm, because a digit and an arrow both move the Library's
  cursor and two arms would have been two copies of the rule.
- **No operation is added or retired**, so `karakuri-operation`, `gate.rs`,
  `karakuri-operation-record` and `karakuri-store::record` are untouched — checked rather than
  assumed: every variant the new code constructs is one this program already emitted from a chip.

## What this record does not do, and what was not checked

**It builds the grammar in two bays of nine.** The Transport, Staging, the Program bay, the
Inspector, the Master chain, the Sequencer and the Outputs row have no row in `BUILT`, so the four
keys decline there and say so. Their rows' key badges stay `plan`, which is what they were.

**The transition row is not addressable.** ADR-0259 hangs the fade, the crossfade and the wipe on
its three controls as acts on the *addressed strip*, and that row is drawn in the Mixer's body under
the strips — neither the head's nor a strip's. Where it sits in the address is left open, and those
four rows stay `plan`.

**A row's own controls are not addressable either.** The Library draws a star, a `params` chip and a
row menu on each row, and ADR-0259 says *"a row has no state, so `space` reaches nothing in this bay
below its head"* — written before the star was drawn. What a digit under a row should name is a
question the record does not answer and this one does not invent, so *Star a Set* stays `plan`.

**Which arrow pair a badge names is not checked.** `↑↓` and `←→` both resolve to *the arrows*,
because a row can be reached by walking items along one axis or by stepping a level along the other
and the pair depends on which. The axis is `Built::across` and nothing holds the page against it.

**Nothing presses a key.** `tests/focus.rs` and `karakuri-console/tests/grammar.rs` ask the
console's own derivations, and `key_column` and `focus_keys` read `main.rs` as text; that a press on
a running panel moves a deck is `mod gpu`'s and those tests were rewritten to the new arm rather
than re-run against a device here.
