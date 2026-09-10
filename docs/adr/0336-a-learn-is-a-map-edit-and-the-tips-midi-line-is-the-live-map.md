---
id: 0336
title: A learn is a map edit, binds a position, and the tip's MIDI line is the live map
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0090, 0092, 0094, 0087, 0096]
tags: [midi, console, vocabulary, store]
---

# A learn is a map edit, binds a position, and the tip's MIDI line is the live map

## Context

[ADR-0335](0335-the-panel-opens-the-first-surface-there-is-and-the-map-is-two-tiers-under-the-store.md)
gave the panel a port and a map. What it did not give it was a way to *change* the map, and
`docs/roadmap.md`'s M5.12 named the one row that needs it: **`Write a parameter` carries the page's
only `plan` MIDI badge**, and its cell read `learned control`.

**That badge could not be earned by writing a map line by hand**, and the reason is the grammar
rather than the gesture. A map line names a slot number, a range, or a word from a closed list, and
a parameter is none of those: `Operation::WriteParam` carries a `ParamAt` — a node address and a
**key** — and a key is a property of the Set that happens to be in the deck. The manual has said for
a year what the address has to be instead:

> A MIDI control is learned against **the deck and the position in its published interface** — knob
> 3 is knob 3 whatever Set is loaded — which is why the inspector numbers its rows. Binding to a
> Set's parameter by name would make the mapping a cost you pay again on every swap, and a learn
> flow exists precisely to make it a cost you pay once.

That is also why a vector parameter publishes as `x`, `y`, `z` and never as one row
([ADR-0268](0268-a-vector-parameter-is-driven-one-component-at-a-time.md)): *one row would be a
position no control change can set.*

Three things had to be decided to build it.

## 1. What a map line says, and who finishes it

**Decision: the grammar gains `param <deck> <position>`, and the *router* resolves the position.**

`karakuri_midi::Map` reads nothing back — *"a pure function of one message, which is this module's
whole test story"* — and a position becomes a `ParamAt` only against the Set that is in the deck
right now. So the map answers what it can and stops, which is
[ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)'s shape
one field wider: `cc -> exposure` names an exposure and not a whole look, because *"the place that
writes the record fills it in."*

- `Map::operation` answers `None` for a `param` line, and `Map::parameter` answers the deck, the
  position, and where the knob is on its span.
- **The two are disjoint by target**, and `every_mapped_message_answers_exactly_one_of_the_two` is
  what holds it — so a target added to the grammar and to neither accessor is a compile-time list
  short rather than a knob that goes quiet.
- `karakuri_environment::midi::Interface` is the readback, `Decks` is the implementation over a real
  deck, and **both programs use it**: `karakuri-cli`'s router resolves a `param` line exactly as the
  panel's does. A map file means one thing in both programs or it means nothing, which is ADR-0335's
  own sentence about the MIDI column.

**The position counts from one**, which is the number the Inspector draws (`view::Param::ord`). It
is the only place a position is visible at all, so a learned line an operator cannot check against
the pane is a line they cannot fix by hand. A `param N 0` is refused with that sentence.

**And the range is the Set's.** Every other continuous target has a range this grammar can state,
because what it moves is the console's own control; a published control's range is
`Published::range`, which narrows the procedure's declaration. So the range on a `param` line is
optional and `None` means *what the Set published it over* — a control declared `0 – 8` reaches 8 at
the top of the fader, and a knob that stopped at 1.0 would be a fader that cannot reach what the
procedure says is in range.

## 2. Is a learn an operation, or a map edit?

**Decision: a map edit.** No `Operation`, no `Record`, no row on
[every operation](../manual/operations.html) — the same `Acted::Opened` the four `mcp` pills hand
back, and for a stronger version of their reason
([ADR-0236](0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)).

**The case for calling it an operation is real and is rule 01**: a learn *changes something*, and
rule 01 says everything that changes something is reachable from all four surfaces. Three things
answer it, and the third is mechanical rather than a matter of taste.

- **Its operand is the pointer.** Which control a knob binds to is *what the pointer is on*. A model
  has no window ([ADR-0315](0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md))
  and a map line has no pointer, so an operation for it would read `gap` in three of the page's four
  columns. **That is a gesture, not an operation** — and the page already has the precedent:
  `Operation::SelectDeck` writes no record because it is a surface's own state, and this is one step
  further out.
- **A permission an actor can grant itself is not a permission**, which is the `mcp` pills' own
  sentence read one layer along. Those settings decide *which classes* a surface may reach; a learn
  decides *which control a surface reaches at all*. A learn over MCP would let a model rewire the
  operator's hands.
- **It must not reach the session stream, and an operation would put it there.** A session recorded
  from a controller replays with neither controller nor map attached
  ([P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)), because which knob is
  which is a property of the room's hardware
  ([ADR-0007](0007-a-set-file-is-a-projection-and-a-session-stream-is-the-timeline.md)). An
  operation writes a record; a record of a learn would put the room's wiring in the timeline, and a
  replay would re-learn against whatever map was there. **Replaying one room's wiring in another is
  not replaying a performance.**

**So rule 01 is not weakened, it is not reached**: the map is the layer every surface reaches the
vocabulary through, and configuring that layer is not a member of the vocabulary it addresses.

## 3. Where the line is written, and how

**Decision: `<store>/maps/default.map`, always, and the line replaces the knob's own line in place.**

**Always the store, whatever map the run loaded.** A run playing the shipped
`examples/surface.map` and learning a control writes into the store —
[P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md) held rather
than argued, and the first learn is what promotes a map into the operator's tier. The file is seeded
with the map in force where it does not exist yet, rather than created holding one line: a map that
shrank to a single control on a press would be the map going quiet.

**It replaces rather than appends, and the test is what decided that.** This was written as a plain
append, on the strength of `Map::parse`'s *the later line wins*. The file is then still correct and
`a_learn_appends_and_seeds_a_file_that_is_not_there_with_what_is_playing` went red anyway, on the
**note**: a re-learned knob leaves two lines, and the loader reports the shadowed one. A knob
learned five times printed four complaints on every start, about a file the operator never wrote by
hand. So a line whose left-hand side is the same message is replaced where it sits, and a knob
nothing is mapped to is appended.

**Everything else in the file is bytes.** Comments, blank lines, the order of the rest, another
knob's line — none of it is parsed, re-emitted or moved. Rewriting the file from the table is the
alternative, and it loses the two-thirds of the shipped map that is prose explaining what a line
means: the first press would turn the one document that teaches the format into forty bare lines.

**A learned line carries no channel**, which is the map's own default and the forgiving one: a
surface is usually the only thing plugged in, and a binding that stopped working because the
controller was moved to another channel is a mapping an operator has no way to see. Writing `ch N`
by hand still narrows it and still wins.

## 4. The tip's MIDI line reads the live map

**Decision: taken as recommended in ADR-0335, and now built.** The `⊕ MIDI:` clause is derived from
the map in use; **where nothing is mapped the page's own sentence stays**.

The page's ten assigned lines are the **mock's** assignments and no operator's. A run whose map puts
`cc 5` on gain A read *cc → gain A* in the tip because the page said so — the tip confidently wrong
about the one thing somebody hovers to check
([P-0087](../principles/0087-name-the-property-never-the-shape.md): the property is *this knob moves
this control*, and *the mock was drawn with cc 2 on it* is a picture of one arrangement).

**Where nothing is mapped the page wins**, because its sentence says *why* — *a map line names a
slot, a range or a word from a closed list, and a Set id is none of the three* — and no reverse
lookup can produce that. A tip the mock is silent about is untouched: inventing a clause for one
would be the console writing the manual, which is
[ADR-0330](0330-the-console-paints-its-own-hover-layer-and-the-tips-are-the-manuals-own-words.md)'s
whole rule.

**And it crosses the seam as a value.** `karakuri-console` cannot read a map: `karakuri-midi`
depends on `midir`, and [ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)
is that this crate takes no device. So `crates/karakuri` derives it and hands it in beside the
`View`, which is how every other value reaches the console.

**It costs one lookup per pointer rest.** `Map::bound` is a linear scan keyed by the target's own
spelling — `Target::spelled`, the right-hand side of the line, which is the one key both directions
can agree on without `karakuri-midi` learning what a console control is. A tip that is up asks for
no frames, so this runs on the frame a tip appears and on the frame a learn changes one.

## 5. The gesture, and the two pills

`learn` is drawn and is the control; `map · <name>` is drawn and is a **readout**.

- **Arm, point, turn.** The pill arms; the hover layer already knows which control the pointer is on
  and answers it before the dwell, because pointing at a control and reaching for a knob is not a
  decision the way reading a tip is; `asked_at` turns that into the operation a press there would
  ask for, and `target_of` spells it as a map line's right-hand side. **The identity of a control is
  what a press on it asks the deck for**, so a knob learned against it moves exactly what a click
  moves.
- **It stays armed until pressed again.** Mapping a surface is *turn every knob once*; re-arming
  between each would be a click per knob. Nothing is hidden by it — rule 04 — because the pill is
  lit for exactly as long as it is true.
- **Nothing is played while it is lit.** A knob already mapped to deck B's gain, turned with the
  pointer on deck A's, would otherwise move B on the way to being bound to A.
- **Every refusal is out loud** (P-0094): the pointer on nothing, a control no map line can name, a
  map file that will not open. The operator is looking at the controller rather than the screen,
  which is exactly when a silent failure is invisible.
- **The `map` pill has no chevron and no menu.** Reaching a map while running is not built; `▾` on
  this console means *there is a menu under this*, and drawing one over a readout is the scaffolding
  `view::transport` refuses.

## Alternatives rejected

**Learn binds a `ParamAt` — the node and the key.** The obvious spelling, and the manual has refused
it for a year: the mapping becomes a cost paid again on every swap, and the same Set loaded into two
decks would be one address rather than two.

**`Map::operation` completes a `param` line by taking the deck.** It would make one accessor instead
of two, and it costs `karakuri-midi` its charter — *a pure function of one message*, which is that
module's whole test story and the reason its tests need no world to set up.

**A learn is an operation with a `Record`.** Rejected in §2. It is the reading somebody will
re-propose, and the mechanical answer is the session stream.

**Rewrite the map file from the table on every learn.** Rejected in §3: it loses the comments, and
the shipped map is mostly comments.

**Leave the tip's MIDI line as the page's text.** Rejected in §4, and it is what ADR-0335 left
standing on the grounds that the map could not change while running. Learn is exactly what makes it
change.

## Consequences

- **`docs/manual/operations.html` has no `plan` badge in its MIDI column**, which is M5.12's exit
  condition. *Write a parameter* reads `cc → param N M`.
- **That page's legend now says which program the MIDI column measures**, which ADR-0220 left open
  and ADR-0335 answered without writing it on the page.
- **`karakuri-midi` gains `Target::Param`, `Map::parameter`, `Map::bound`, `Map::learn`,
  `Map::lines`, `Target::spelled` and `Key::spelled`**, and `Map::operation` gains an arm that
  answers `None` with its reason.
- **`karakuri-environment::midi` gains `Interface`, `Decks`, `Router::resolved`, a third dedup set
  for a position past the end of an interface, `Surface::learning`, `Surface::learn`,
  `Surface::bound` and `appended`**, and `Router::route`/`Surface::take` take the interface.
  `karakuri-cli`'s one call site passes `Decks`.
- **The coalescing key gained the position.** A deck has one gain and one exposure and as many
  parameters as its Set published, so two knobs on two parameters of one deck would have collapsed
  into one operation — and the Set's third control would have been written with the fifth's value.
- **`karakuri-console` gains `View::learn`, `View::map`, `MapPill`, `LearnPill`, `MapRow`,
  `learn_pill`, `map_pill` and two `PROBES` rows**, and `arrangement` and `look` take the map so the
  three pills that are laid out one from the next stay in the mock's order. Every call site of those
  two passes `None` unless it is the panel.
- **`Hover` gains `resting` and `assign`**, and its galley cache is keyed on the assignment as well
  as the control — a learn changes the last line of a tip that is on screen.
- **Nothing was added to `karakuri-operation` and no badge but one moved.** A learn is not an
  operation, so the vocabulary is untouched.
- **`<store>/maps/` is created by the first learn** and by nothing else. `Store::open` still does not
  establish it, which is ADR-0335's own note.
