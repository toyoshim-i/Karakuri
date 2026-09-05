---
id: 0205
title: A question whose reply the vocabulary cannot say gets no row, and the status line was concealing five console gaps
status: accepted
date: 2026-08-28
supersedes: []
superseded_by: []
principles: [0087]
tags: [console, vocabulary, surfaces, docs, cli]
---

# A question whose reply the vocabulary cannot say gets no row, and the status line was concealing five console gaps

## Context

Two surveys ended with the same shape of hole and neither closed it.

[ADR-0197](0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md)
surveyed `karakuri_console::panel::Op` against *Arranging the console* and found four gaps between
eight variants and five rows. [ADR-0204](0204-the-root-and-the-body-row-stay-unnamed-and-a-folded-root-is-not-hit-testable.md)
took one of them. `crates/karakuri-console/tests/vocabulary.rs` pins what is left, and its `NO_ROW`
list names the two variants no row reaches with the sentence that separates them: **"`Reset` is a
change and `Report` is a question"**.

[ADR-0198](0198-a-gesture-converts-in-the-parts-that-are-decided.md) surveyed `karakuri-cli`'s
thirty-nine keys and put three of them — `s`, `h` and `?` — in a group of their own: *the vocabulary
names nothing*. Its §4 declined to invent rows from the implementation and named what would have to
be answered first, *"what a status line is on a console with no terminal is a question for the
page"*, and bundled the three keys into that one question.

**They are one question asked twice, and the answer is the same both times.** A row on
`docs/manual/operations.html` is a promise every surface can be held to — rule 01 of
[the seven](../manual/index.html) is *"Every operation is reachable from the panel, from the
keyboard alone, from a mapped MIDI control, and from a model over MCP"* — so an operation whose
**reply cannot be said in the vocabulary's terms** cannot be given one, however plainly it is an
operation the program performs. That is what `Report` is, and it is what `s` is. `h`/`?` is a
different thing that happened to be sitting beside them.

## Decision

### 1. `panel::Op::Report` gets no row

**It is a question in exactly `karakuri-operation-record`'s sense.** `Silent::Question` is
documented as *"**It asks rather than changes.** Reading a Set, listing the store, reading a
procedure, finding out what a write did. A record is what a replay reconstructs a performance from
and a question changes no performance."* `crates/karakuri-console/src/repaint.rs` reaches the same
conclusion from the other side and returns `Repaint::Never` for it: *"A report is a print and
nothing else"*.

**And its reply cannot be said.** `Outcome::Report` is a `Vec<Placement>`, and a `Placement` is a
`NodeId`, a depth, a `Rect` and a visibility. `NodeId` is `pub struct NodeId(usize)` — a handle with
a private field that only a `Layout` hands out, which is `vocabulary.rs`'s own reason the two types
cannot be merged. The `Rect` is pixels, and a pixel is half of why `Operation::MoveBoundary` carries
`Undecided`: *"`Layout::set_divider` takes a position in the viewport's own coordinates, which is a
pixel: a number a drag produces and a key press or a model has no way to mean."*

**The four question rows that exist all answer prose to a model over MCP.** *List what the store
holds*, *Read what one Set holds and declares*, *Read one node's source* and *Find out what a write
did* are the four operations `written` answers `Silent(Question)` for, and on the page each is
`panel plan`, `key —`, `MIDI —` and `MCP` has the tool (`list_sets` has a CLI flag beside it and no
other route does). That is the working shape of a question in this vocabulary: something a model
asks and gets sentences back from.

So a row for `Report` would be a promise three of the four surfaces cannot be given. A key cannot
say a `NodeId` and could not read the answer; a map line cannot either; and a model over MCP would
be handed rectangles in the coordinates of a window it is not looking at. The `—` on such a row
would never become a badge, because nothing is unbuilt — the reply's type does not cross.

**What it costs, recorded rather than left to be met**: the console keeps performing an operation
the page does not name, and `crates/karakuri-console/tests/vocabulary.rs` goes on pinning it, so the
gap stays visible and cannot be closed by accident in either direction.

**`Reset` is not this and is not decided here.** It is a change rather than a question — a fresh
arrangement at the same viewport — and it is getting a row in a later change. The `NO_ROW` list
keeps both entries until then, and only one of the two is permanent.

### 2. `s` and `h`/`?` get no rows, and they are two questions rather than one

ADR-0198 §4 answered them together. They fail differently, and bundled they hid the second one's
cost for a week.

**`h`/`?` prints a static string that has already been printed twice unprompted.** `--help` prints
`{USAGE}\n{BINDINGS}` and the run prints `\n{BINDINGS}\n` before the first frame; `'h' | '?' =>
eprint!("{BINDINGS}")` reprints it after it has scrolled away. That is a terminal's problem and not
an operation, and **on a console there is nothing to print**: the mock in
`docs/manual/console.html` draws no key hints anywhere, and its tooltip contract carries something
else — *"The tooltip also carries the control's **MIDI assignment**, which is how learning one
works: point at anything with a tooltip and move a knob."*

**The consequence is noted rather than fixed, because it is larger than this record.** The console
as drawn gives an operator no way to learn a key binding at all: rule 01 makes the keyboard one of
the four surfaces, and the panel teaches the MIDI half of a control and not the key half. That
belongs to the roadmap's deferred *a keyboard is mapped and learned the same way a control surface
is*, which already names the neighbouring half of the same hole — *"both learn paths wait on the
same missing mechanism, and it is worth knowing which: **this console draws no tooltips at
all**"* — and it is hooked there rather than turned into a row here.

**`s` is a question in the same sense as `Report`.** `'s' => self.print_status()` calls the same
function the `STATUS_INTERVAL` of 500 ms calls, so it prints exactly what the timer prints and
changes nothing about the performance — it resets the window the frame rate is averaged over, and
nothing else.

It gets no row because **giving it one makes the panel owe an answer it cannot give**. The status
line's fields are not one readout; they are a dozen readouts on one line, and five of them have
nowhere on the console to go.

### 3. The five gaps the status line was concealing

Each is checked against the source rather than repeated from the line's format string. They belong
to `docs/roadmap.md`, where the open work is, and they are recorded there as console gaps.

1. **A slot's simulation clock.** `t0.0s`, printed per slot from `self.deck.slot(slot).set().time()`.
   Nothing in `view::Strip` carries a per-deck time — its fields are `name`, `tally`, `requested`,
   `gain`, `gain_to`, `opacity`, `opacity_to`, `blend` and `mask` — and neither does the mock's
   strip, which is a name, a tally chip, the trim, the fader and meter column, one number and the
   two minis.
2. **An armed transition's destination**, `g>` / `o>` / `w>`. **This one is not a gap but a rule
   already broken**:
   [P-0075](../principles/0075-a-pending-transition-shows-where-it-is-where-it-is-going-and-that-it-has-not-arrived.md)
   requires that *"The destination is identifiable from the surface itself. Not from a tooltip
   alone, and not from a log"*, and the status line is that log. **It is being closed as this
   lands**: `view::Strip` carries `gain_to` and `opacity_to` now, and their documentation cites that
   clause by name. Two things beside them are not covered — `Control::MaskPosition`, which is the
   `w>` of the three, and the armed renderer selection `r>`, which `Deck::selections_on` answers and
   no `view` field carries. No record for the fader half had landed in `docs/adr/` when this was
   written; when one does it is the record to cite here.
3. **Non-finite texels**, `x{n}`. `view::Level` carries `mean` and `peak` only, and the mock's meter
   is a column with a peak mark. The status line says at the field why it is beside them: *"it
   explains the two numbers beside it — they are a mean and a peak over the texels this did not
   count"*, so a console drawing the mean and the peak without it draws two numbers that no longer
   say what they are over.
4. **The tempo source** — its name, its peer count, the anchors it rejected, and its liveness, which
   is deliberately three states and not two (`?` for a source that greeted and has said nothing
   since, `stop`, and running) beside `GONE` for one that ended. Nothing on the console carries any
   of it. `view::Transport` is `bpm`, `beats`, `beats_per_bar`, `fps`, `frame_ms` and `budget_ms`,
   and the mock's transport row is a BPM, an `audio-in` pill, the beat grid, the bar, `tap`,
   `learn`, the map, the frame budget, `landed` and `rec`. **The `audio-in` pill is not this**: a
   tempo source is *"a separate program that knows where the beat is and tells this one"*, of which
   the in-process beat tracker is one of four named kinds, and the pill names an input, carries no
   tooltip, and says nothing about peers or liveness.
5. **Sync mode, anchor and offset** — `T{anchor}` where a deck is tempo-synced and
   `B{anchor}{±offset}` where it is beat-synced, printed only when a deck is synced at all. The page
   routes *Set a deck's sync mode* and *Scrub a deck a quarter beat* to `panel deck head` — **and
   there is no deck head anywhere in the mock**, whose heads are `bay-head`, `half-head`,
   `node-head` and `seq-head`. It is the sharpest of the five because the page names a home that
   does not exist; the other four name no home at all.

## Alternatives rejected

- **Give `Report` a row and mark `key`, `MIDI` and `MCP` as gaps.** The page carries rows with three
  dashes already, so this looks like the consistent move. It is not the same thing: those dashes are
  routes nobody has built, and this one is a reply that cannot be carried. A row promising an
  operator that a key will one day report every node's rectangle is a promise the type system
  refuses, and the page is the specification rather than a wish list.
- **Give `Report` a row and answer it in prose, the way the four MCP questions are answered.** That
  is a second derivation of *where everything is* — the arrangement already answers it as data, to
  the one caller that can use it — and the console has no prose channel to a model at all. It would
  also make the reply's usefulness depend on a rendering nobody specified.
- **Answer *what a status line is* by giving the console a status line.** It is the shape the
  question arrives in and it is the terminal's shape, not the panel's. A line of text under the
  panel is precisely the log P-0075 rules out as a place to identify a destination, and it would let
  all five gaps above stay open while looking closed.
- **Keep `s` and `h`/`?` bundled as ADR-0198 left them.** One has nothing to print and the other has
  a dozen fields to print and nowhere for five of them. Bundled, the second's cost is invisible —
  which is what happened: the gaps below were found by pulling the two apart, not by reading the
  line.
- **Add rows for `s` and `h`/`?` so every key has an operation.** Already rejected in ADR-0198 §4,
  and this record does not reverse it; what it adds is the reason the count will never come out
  even. `h`/`?` is a terminal reprinting its own scrollback, and `s` is a question whose reply is a
  dozen readouts that belong on a dozen regions.

## Consequences

- **`docs/manual/operations.html` is not edited.** Both decisions are *no row is added*, and the
  page needed no change for either.
- **`crates/karakuri-console/tests/vocabulary.rs` keeps both `NO_ROW` entries**, and they now have
  different lifetimes: `Report`'s is permanent and `Reset`'s lasts until its row lands.
- **Three of `karakuri-cli`'s thirty-nine keys stay outside the vocabulary**, and it is now two
  reasons rather than one deferral.
- **Five console gaps are on the roadmap** that were not on it before, and the one thing they had in
  common is that a terminal was covering for all five.
- **One of the five is a broken rule rather than missing work** — P-0075's second clause — and it is
  being closed as this lands, in the parts that have a control to hang on.
- **The console still cannot teach a key binding**, which is rule 01's keyboard surface with no
  teaching path. Hooked from the roadmap's deferred keyboard-map entry beside the tooltip mechanism
  both learn paths wait on.
- **ADR-0198's front matter is annotated and its prose is untouched** (P-0093), the way ADR-0185 and
  ADR-0188 were.
- **No new principle.** P-0075 is what this applies; the rest is a decision about one page.
