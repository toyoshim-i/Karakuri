---
id: 0332
title: Focus is a pointer the console owns, and the three pointers are instances of it
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0083, 0087, 0090, 0094]
tags: [ui, keyboard, surfaces, console]
---

# Focus is a pointer the console owns, and the three pointers are instances of it

## Context

[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
decided the keyboard and **built none of it**. A key press is addressed to the bay that has focus,
`Tab` and `shift-Tab` walk the arrangement, the keys that act inside a bay are the same six
everywhere because they are rules about kinds of thing, `esc` goes up one level and stops quitting,
and a global letter survives only where the operation it names has no operand.
[ADR-0331](0331-the-key-column-is-spelled-two-ways-and-a-built-badge-keeps-its-bare-letter.md) then
spelled the manual's key column two ways and moved no badge between `has` and `plan`, because
nothing was bound: *"nothing is on `Tab`, `esc` still quits, and the console takes no keyboard focus
at all."*

**`docs/roadmap.md` M5.13 carries the rest as five items.** This record is items 2 and 4's first
half and the binding of `Tab` — the pointer, the ring, the level `esc` leaves — and it deliberately
stops short of the six keys.

**The console has been drawn for this since before it was drawn.**
`docs/manual/console.html` has carried *Two focuses, and they do not look alike* from the
beginning, the mock draws `.wfocus` on deck B's fader, and `karakuri-console/src/view.rs` said why
it drew nothing: *"Nothing draws the dashed one — this console takes no keyboard focus."*

## Decision

**Focus is a pointer the console owns, in a module of its own, and the three pointers that were
already there are three readings of it.**

### The ring is derived from the arrangement, and `REGIONS` is what it is checked against

`karakuri_console::focus::ring` walks the solved tree: a column's children top to bottom, a row's
left to right, and **it stops at a bay rather than descending into it**. That gives the transport,
the left pane's two, the centre's two, the right pane's three and the outputs row — nine, in the
order `view::REGIONS` is written in.

**Derived rather than read off the constant**, which is ADR-0259's own instruction: *"a ring derived
from the solved tree is the honest implementation and the constant is a thing to check against, not
the source."* `tests/focus.rs` is where the constant does the checking, in both directions, so a
region added to the panel and not to the walk fails there rather than being a bay `Tab` never
reaches.

**A folded bay is in the ring, and so is a bay inside a folded pane.** The walk asks
`Layout::children` and never `Layout::placed_children`: what is on screen is the paint's question.
The reason is the narrow one ADR-0259 gives — a folded bay is in the ring so that there is something
to press to open it, not so that it can be operated.

### There is no unfocused state, and the start is the walk's answer

`Focus` holds `Option<&'static str>` and `Focus::bay` resolves it against the ring: the first bay
the traversal reaches where nothing has been focused, and again where what was focused is no longer
a bay of this arrangement. **The `Option` is a storage detail and not a state an operator can be
in** — every read answers a bay.

That is ADR-0259's rule rather than its example: *"an operator who has moved the Transport has moved
the starting position with it, and no second rule has to agree with the first."*

### A bay's remembered address is a path, and the head is not one of the things remembered

`Address` has two fields and they answer two questions. `at` is **where the address is now** — the
dashed ring, pushed by a digit and popped by `esc`. The memory is **where it was** — the solid ring,
keyed by the path it sits under, so a bay remembers the item it was on at every level of itself.

One field could not be both. `esc` from deck B's fader leaves the address at the Mixer and the
selection on deck B; a single path that had been popped would have taken the deck selection with it,
and the deck selection surviving your hands being elsewhere is the whole of what the mechanism is
for.

**`0` — the head — is never written into the memory.** A bay's head is where it keeps the controls
that are about the bay rather than about anything in it, so there is one of it and nothing to
remember; a bay that filed *the head* under the same key as *the third strip* would lose the deck
selection the first time an operator pressed `0`. The head is still a rung the address **descends**
through, which is exactly how the Library's scope is reached — `[0]`, and the scope is the control
remembered under it.

**That clause is what lets one bay hold two of the three pointers.** The Library's cursor is which
*item* it is on and its scope is a control of its *head*, which is how ADR-0259 walks that bay:
*"`0` is the head and its controls are the scope chips … Items are the listed Sets."* They are two
levels of one address and not two numbers, and moving one leaves the other alone.

### The three pointers collapse, and nothing they mean changes

`View::selection`, `View::cursor_row` and `View::scope` were three private fields carrying the same
paragraph — *"nothing downstream can be the model of record for it, and a host that kept a copy
would be keeping the console's state on its behalf"* — written three times. The paragraph is written
once now, at `View::focus`, and the three are readings:

| pointer | bay | where |
| --- | --- | --- |
| the deck selection | `mixer` | the item the bay was last on |
| the library cursor | `library` | the item the bay was last on |
| the marked scope | `library` | the control remembered under the head |

**Every rule about what a bay draws stays where it was.** `View::select` still refuses a deck the
mixer has no strip for, `View::walk` still holds the cursor inside the rows the bay drew and clamps
rather than wrapping, `View::point_at` still refuses a row past the listing, and `View::select_scope`
still refuses a scope with no chip and puts the cursor back at the top. What moved is where the
number is kept. `View::focus` is exposed to read and **not** to write, so those five are still the
only writers.

**Digits count from one.** A remembered rung is the digit that named it, which is ADR-0259's change
from the four deck keys — the step down to a position is written at exactly three private readers,
one per pointer, and each subtracts with a floor rather than bare: zero is unreachable because
`Address::remember` refuses the head, and a wrapped `usize` in a paint path is a panic in an event
handler.

### `Tab` and `esc` are bound, and `esc` stops quitting

`crates/karakuri/src/main.rs` binds `Tab` to `View::tab` and `esc` to `View::focus_up`. Neither names
an operation and neither writes a record: focus is a pointer, `Change::Pointed` is already *a key
moved a pointer*, and the page grows no row, because moving focus is not an operation.

**`esc` at bay level acts on nothing and says so**, which is
[P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md) and the page's own
sentence that *"a key that declines and a key that is not bound are the same experience"* — the line
names the bay the address is on and says how to leave it.

**Quitting is the window's.** `WindowEvent::CloseRequested` was already handled — saves waited for,
recording flushed — so nothing about quitting stopped working, and `event_loop.exit()` is now
reached from exactly one place in this file. *Quit*'s key badge is `gap`, and the fifth cell already
named the route.

### `shift-Tab` costs one field and is not a modifier chord

`winit`'s `KeyEvent` carries no modifier state, so `App::shift` is written from
`WindowEvent::ModifiersChanged` and read at the one arm. `egui`'s own copy is not usable here:
`egui_winit::State` keeps modifiers privately and stamps them onto queued events, and
`Context::input` answers from the last pass — a `shift` pressed since the previous frame would be
invisible.

**It is not the modifier scheme ADR-0259 rejected.** That rejection is *"a modifier is a mode with
no readout"* against rule 04. `shift-Tab` gives no key a second meaning: it reverses a traversal
that is drawn either way, and the ring is on a bay whichever direction you arrived from.

### The ring is drawn on the head, or on the row for a bay that draws none

`focus::mark` is a derivation, `View::focus_mark` asks it with the bay this console has focus on,
and `View::draw` paints from that — which is this crate's arrangement everywhere a mark is drawn. The Transport and the Outputs row are headless (ADR-0159)
and ADR-0259 reads them the same way it reads their `0` — *"a headless row, so `0` names the row
itself"* — so the row stands in for the head and the ring goes round it.

**A folded bay is not ringed, and that is the drawing ADR-0259 left open.** A folded region has no
rectangle and no divider beside it (ADR-0204), so there is nothing on the panel for a ring to sit
on. `console.html` draws the mark that is owed — the bay head alone wearing `.wfocus` — beside the
note that defines it, which is ADR-0331's decision and not this record's to revisit.

## What phase two owes

**The six keys, as a per-bay dispatch.** A digit names the nth thing one level below the address and
`0` the head, the arrows take the neighbour or the next value, `space` is the addressed thing's next
state, `enter` is the act. `Address::down` is the shape a digit writes into and nothing calls it
from the window loop yet.

**`key_column`'s machinery.** `KEYS`, `ROWS` and `NO_ROW` are built around one letter reaching a
fixed set of rows, and the badge checks read the `match` as text; under focus a digit reaches a
different row in every bay, so a dispatch table is invisible to the scan. **That is the thing to
solve rather than to discover**, and it is why no `plan` badge moved to `has` here.

**The letters ADR-0259 retires are still bound, and both routes exist until the grammar reaches the
same row.** Twenty-one keys become focus-relative under that record: `f`, `g`, `s` and `u` take
their region from the pointer through `Readout::target` and are to take it from the focus; `0`–`3`
become the Mixer's `1`–`4`; `[ ] \` and `; '` become arrows and `space` on the addressed trim and
fader; `m` and `e` become `space` on the addressed chip and on the Library's head; `o` and `p`
become arrows on the transport's offset; `up` and `down` become the arrows everywhere; `l` becomes
`enter` on a library row; `k` becomes an act on a control the Library bay does not draw yet.
**None of them is unbound here**, because a letter unbound before the grammar reaches its row is a
row an operator cannot reach at all. Phase two takes the Mixer's and the Library's; the other seven
bays' letters wait for theirs.

**`Readout::target` and `Readout::enclosing` are what actually retire**, and they retire with
`f`, `g` and `s`.

## Alternatives rejected

### a. One number per bay, and the Library keeps two fields

**The case is that it is the smallest change that satisfies the sentence.** Three fields become a
`[usize; 9]` indexed by bay, the deck selection and the library cursor are two of its entries, and
the marked scope stays where it is because it is not an item.

**It loses on the Library, which is the bay that tests the claim.** ADR-0259's walk puts the scope
chips in that bay's *head* and its Sets in its *items*, and the address that reaches a chip is `0 n`
— two rungs. A scheme with one number per bay cannot hold both, so the scope would have stayed a
field of its own and the record's *"three instances of one mechanism"* would have been two
instances and a leftover. **The leftover is the one that proves the mechanism**: if a bay can hold
two remembered positions at two levels, the address is a path; if it cannot, ADR-0259's Inspector —
three deep — has nowhere to go either.

### b. A single path per bay, with no separate memory

**The case is that ADR-0259 says a bay remembers *its address*, singular**, and a path is what an
address is. One `Vec<usize>` per bay, pushed by a digit and popped by `esc`, and the deck selection
is *whatever the path names at the item level*.

**It loses on `esc`, which is the key this record binds.** `esc` from deck B's fader pops to `[2]`
and then to `[]`, and at `[]` there is no item in the path — so the deck selection would be gone the
moment an operator pressed `esc` twice, and the ring on the strip with it. **Focus is transient and
the selection persists**; that is `console.html`'s first sentence about the pair, and a scheme with
one field makes them the same field.

**It also loses the Library's two for (a)'s reason**, one press earlier: descending into the head
would overwrite the row the cursor is on.

### c. Focus starts on the Mixer, at deck A's strip

**The case is the mock, which rings that strip**, and an earlier draft of ADR-0259 argued exactly
this.

**It loses because it is a second rule.** The mock rings deck A because that is the *deck
selection*, which is one bay's remembered address and not where focus is. A named starting bay would
have to be kept in step with the traversal by hand, and the first divider dragged would have parted
them. The record settled this and this is the implementation obeying it, recorded here because the
`Option` in `Focus` is exactly where somebody would put the other answer.

### d. Read `shift` off `egui` at the press instead of keeping a field

**The case is that this loop already hands every key to `egui` before its own `match`**, so the
toolkit has the modifier state and asking it costs no field and no event arm.

**It loses on when the answer is true.** `egui_winit::State` keeps its modifiers privately and
`egui::Context::input` answers from the **last pass**, so a `shift` pressed since the previous frame
is invisible there — and a first `shift-Tab` on a still panel is precisely a key pressed between two
frames. What is readable in time is `RawInput::modifiers`, which `egui-winit` never writes. The
field is one `bool` and one arm, and the arm still hands the event on.

### e. Draw the dashed ring round the whole focused bay

**The case is uniformity**: nine bays, nine rectangles, one rule, and no case analysis for the two
that draw no head.

**It loses on what the mock says the mark is.** `console.html`'s focused-bay mark is *the head
alone*, and a ring round a whole bay is a second reading of a mark that already exists — it would
also enclose the `.strip.focus` ring on a strip inside the Mixer, which is the one pair the page
insists a reader must be able to tell apart. The headless case is not an exception either: ADR-0259
already reads the Transport's `0` as *the row itself*, so *what stands in for the head* is a rule
that record wrote and this one honours.

## Consequences

- **`karakuri-console` gains `focus.rs`**, exporting `ring`, `is_bay`, `mark`, `Focus`, `Address`,
  `HEAD` and the two bay names the three pointers live under. It reaches `egui` for a rectangle and
  takes no device.
- **`View` loses three fields and gains one.** `selection`, `cursor_row` and `scope` are gone;
  `focus: Focus` is where all three are, and `View::focus` is a read-only reading of it.
- **`View::selection`, `View::cursor_row` and `View::scope` answer exactly what they answered.**
  The refusals and the clamps are at the same five methods.
- **`crates/karakuri/src/main.rs` binds two keys and unbinds none.** `KEYS` gains `tab` and `esc`'s
  sentence changes; `key_column::ROWS` gains `("tab", &[])` and `("esc", &[])`, and `NO_ROW` grows
  from six to eight.
- **`event_loop.exit()` is reached from one place in that file**, where it was reached from two.
  `focus_keys` is the test that holds it.
- **`docs/manual/operations.html`'s key column reads 15 `has`, 29 `plan`, 20 `gap`** as of
  2026-09-09, against 16 / 29 / 19 before: *Quit* is the one row that moved, and it moved because
  the key that reached it stopped reaching it. **No other badge moved in either direction.**
- **`egui` is still never asked for permission**, and the invariant `event_response` holds is what
  makes `Tab` arrive at all: `egui-winit` 0.36.1 reports `consumed` for every `Tab` whatever has
  focus. That test was written before there was a `Tab` to swallow, and this is the record that
  makes it load-bearing.
- **`room::size` gains `WFOCUS_RING`, `WFOCUS_OFFSET`, `WFOCUS_DASH` and `WFOCUS_GAP`.** The first
  two are `.wfocus`'s own declarations; the last two are the console's own, because `dashed` is a
  CSS keyword and a browser picks the pattern.
- **`concepts.html`'s *Focus* stops saying focus is not built**, and `console.html`'s *Two focuses*
  note says which half is on the panel. Neither page's definition changed.
- **No operation is added or retired**, so `karakuri-operation`, `gate.rs`,
  `karakuri-operation-record` and `karakuri-store::record` are untouched — checked rather than
  assumed: the two arms this record adds construct no `Operation` at all.

## What this record does not do, and what was not checked

**It binds none of the six keys**, so every letter ADR-0259 retires is still bound and still the
only route to its row. A reader of the key column sees no change but one.

**Three `match`es on the keyboard have an `Escape` arm and the check reads the first one it
finds.** The two letter-taking flows take the keyboard whole while a name is being typed, and each
abandons the name on `esc` — which ADR-0259 reads as a *field*, one of its seven kinds. `focus_keys`
cuts the flattened source at the `Tab` arm before looking for `esc`, because only the window loop's
own `match` binds `Tab`; without that cut it read the arrangement pill's arm and reported the
panel's `esc` missing, which is how it was found.

**Nothing presses a key.** `tests/focus.rs` asks the console's own derivations and `key_column` and
`focus_keys` read `main.rs` as text; that a press on a running panel moves the ring is `mod gpu`'s
question and it is not asked. That is the same boundary `key_column`'s own documentation stops at,
and it is written here rather than left to look covered.

**Whether `Tab` reaches this program on every platform was not run.** It is checked on the code
path — one field read off `EventResponse`, and it is `repaint` — and the toolkit's own source was
read on 2026-09-05. A window manager that claims `Tab` before `winit` sees it is outside anything
this workspace can assert.

**The mark for a folded bay holding focus is still only drawn in the manual.** The panel draws
nothing there, which is honest — there is no rectangle — and it means an operator who tabs onto a
folded bay has no mark until that drawing exists. It is ADR-0259's own open question and this record
does not close it.
