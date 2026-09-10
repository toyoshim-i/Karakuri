---
id: 0329
title: An input is wired on the node that declares it, and the number is the publish mark
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0085, 0086, 0087, 0090]
tags: [console, engine, operations, watch, inspector, m5.5]
---

# An input is wired on the node that declares it, and the number is the publish mark

## Context

*Wire a procedure's input to a node* and *Narrow the published interface* were the last two rows of
[roadmap.md](../roadmap.md)'s *Rows the manual has not given a home*, and that section said what it
was waiting for:

> **Two rows carry a `plan` panel badge over a panel cell reading `—`** … Nothing on the console is
> specified to reach either, so no bay holds them and none of M5.1 to M5.9 can close them. **The
> manual has to name a home before they can be scheduled.** There is no drawing to owe until it
> does.

That is true and it was being read as *the manual will have to invent one*. **It did not have to.**
Three records already say where each goes, and the third row that left this list left it the same
way — *"by being read again rather than by being decided"*.

- **Rule 05** of [the manual](../manual/index.html): *"Everything about what a deck **is** lives in
  the inspector."* Both of these are what a deck is.
- **[ADR-0152](0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md) and
  [P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md)** make an input a
  procedure's own declaration — `uses far : Geometry` — answered by an `edge` the *Set* supplies. So
  wiring is a fact about **one node**, and the Inspector already draws one group per node.
- **[ADR-0100](0100-a-published-interface-is-a-choice-of-attention.md)** makes publishing a
  per-control choice of attention. The Inspector already draws one row per control, and that row
  already carries the control's **position in the published interface** — the number a MIDI knob is
  learned against.

The section also carried a second blocker on the narrowing row, and it dissolved on reading:

> *Narrow the published interface* has a second blocker behind the missing control: it is upstream
> of the Inspector, and until something publishes, every control that bay will ever draw is a
> wildcard.

True of the *default* interface and not a blocker on the control, because **narrowing is what turns
the default into an authored one**. The bay is not downstream of publishing; it is where publishing
is chosen.

## Decision

### 1. An input is wired on a `uses` line under the node that declares it

`.uses` is a line in the node group, between the head and that node's rows: what the procedure calls
the input at the left, and a capsule naming the node filling it at the right. A press on the capsule
brings down [ADR-0305](0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)'s
card; a pick emits `Operation::WireInput { deck, node, slot, to }` and the host hands it to
[`rewired`], which is the function a model's `wire_input` already goes through.

- **The line is on the node and the slot is the procedure's word.** `slot` is `far` and `node` and
  `to` are node names, which is `Record::Edge`'s two halves exactly and is why the operation is *the
  one addressed by name at both ends*. Nothing in the `.kir` refers to a node, and nothing here
  makes it.
- **The card lists the nodes of the kind the input takes**, with the declaring node and the node
  already wired left out of its own list. A press on a capsule with nothing to offer says so and
  opens nothing.
- **It replaces and never clears**, and that is the language's shape rather than a control that
  forgot an *off*: an input takes one node, `SetError::SlotBoundTwice` refuses two edges on one
  input, and **a Set with an unbound declared input does not build at all** (ADR-0152's second
  half).
- **Nothing is validated on the console's side**, which is the same nothing `mcp::wire_input`
  applies: it queues the edge and the wall is at `Set::build`, by name and with what the Set does
  hold ([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)).

**The `uses` lines are read off the edges, and no reader was added to the engine.** Nothing on a
built `Set` says which inputs a node declares — the declaration is read at `Set::validate` and
dropped — and nothing has to, because an unbound input is refused there. So **a slot that is running
has an edge for every input it declares**, and the wiring a pane draws is that slot's own aim
(`Aiming::at.edges`) filtered to the nodes its Set holds, which is exactly how the build filters
them. The candidates are the other nodes on the layer the input already reaches, which is a list
every entry of which the build accepts, because the build refused anything of another kind.

### 2. The number at the left of a parameter row is the publish mark

`.param`'s leftmost cell is the control's position in the deck's published interface. A press on it
takes the control off the interface; a press again puts it back at the end of the list. **Off, the
row keeps its place and loses its number, its fader and its figure**, and the number becomes a dot.

- **One cell, two states, and the state *is* whether it is published.** A control off the interface
  has no *position*, and a position is exactly what a knob counts — so the mark and the number are
  not two things drawn beside each other. It also costs the mock no new grid track, which the
  alternative did.
- **The row stays.** The Inspector draws every control the material declares and publishes the ones
  the interface carries, so a choice of attention is made where it can be seen. A row that vanished
  would be a choice nobody could unmake, and the panel would be able to narrow and never widen.
- **On a Set nobody has narrowed nothing changes**, because an empty interface publishes everything:
  every declared control has a number and a fader, and no row is faint. **That is where every deck
  opens.**
- **A press names the whole list.** `Operation::Publish { deck, controls }` carries the interface it
  is asking for, which is the vocabulary's own sentence at the variant — *"the whole ordered list,
  not one entry … an interface that publishes nothing publishes everything, which is a statement
  about the list and not about an entry"*. The list is built in **interface order** and not in the
  pane's: a wildcard control is placed in whichever group it resolves to and numbered by its
  position, and the two are not one walk.
- **What it changes is what is shown, never what can be reached** (ADR-0100). `--param`, a `param`
  record and a model naming the address all still reach an unpublished value.

**`Set::declared_interface` is the one thing this added to the engine**, and it is `Set::published`'s
own else-branch given a name: the default interface, whether or not one is authored. It is what the
mark is chosen *from* — a console that could see only `Set::published` could take a control off and
never offer it back, because the name, the address and above all the **declared range** it would be
republished over are gone with it.

### 3. Both are re-aims, and for narrowing that is the whole decision

Each is a field of what the slot's watcher is pointed at — `Aim::edges` and `Aim::published` — so a
press restates the rest and sends it, the worker recompiles the slot off the render thread, and the
Staging lane carries the verdict. That is ADR-0314's mechanism a third and fourth time
(ADR-0328 was the second and third).

**For the wiring it was already the only route.** `rewired` re-aims, and `wire_input` has gone
through it since it was written.

**For narrowing it is a decision against a cheaper one, and the cheaper one undoes itself.** A live
write through the deck — `Deck::set_interface` over a `Set` setter, on
[ADR-0280](0280-a-parameter-written-to-a-live-set-is-a-session-record.md)'s road, which
[ADR-0319](0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md) walked
three times — is immediate and free and is **wiped by the next rebuild**, because a rebuild restates
`Aim::published` and that list has been empty in every run this program has had. The next rebuild is
the operator's own next save of any `.kir` in the deck, or a capacity press, or a re-salt. That is
the defect ADR-0280 §6 named for parameters and ADR-0282 closed with `Set::carry_moved_from`; there
is no carry for an interface, so the aim is where a narrowing has to live. The cost is a recompile
for a choice about a display, and it is named on both pages.

### 4. What each writes, and the two silences are not the same

`written` answers `Silent(NoRecord)` for both and did before this, and the reason differs:

- **`WireInput` is `SetProperty`'s shape.** `Record::Edge` exists, is a **Set file's** statement
  about a Set and carries no slot — so a session cannot say *the Set in slot 3 wires `far` to
  `sphere_shell`*, and **a keep carries it**, because a keep writes the run's edges into the file it
  saves.
- **`Publish` is weaker and is a gap in the format.** Nothing in this vocabulary says what a Set
  publishes, in a Set file or in a session. So a narrowing is reproduced by **no replay and no
  keep**, and it lives in the run that made it. Inventing a spelling here would be inventing the
  record stream ([ADR-0046](0046-a-flag-writes-into-the-record-it-does-not-invent-one.md)), which is
  the same refusal `SetCompositing` and `LoadSet` already carry — but this is the one row where the
  keep does not close it, and both manual pages say so.

## Alternatives

### a. Un-publishing removes the row, and the way back is a `publish everything` capsule

The cheapest design, and the one that needs no engine reader: the pane goes on being built from
`Set::published`, a press takes a control off and its row disappears, and a capsule somewhere
restores the default by asking for `Publish { controls: [] }` — which ADR-0100 already defines as
publishing everything.

It loses on what an operator would do with it. Narrowing is a set-up act, done a few rows at a time,
and this makes the *only* correction a full reset: take three off, want the second back, and the
answer is to restore all of them and start again. It also makes the mark not a mark — a cell you can
press once, whose effect is to remove the thing you pressed — which is the one shape the console's
other stepping controls are written against (ADR-0262's *"every state either field can be in is one
a press can leave"*).

### b. A separate publish mark beside the number

A dot or a checkbox at the row's left with the ordinal after it, which is the obvious reading of
*"the smallest mark at the row's left"*. Refused because `.param` is a four-track grid whose columns
are transcribed constants, and a fifth track changes the row's arithmetic, the pane's minimum and
therefore the centre's declared minimum — ADR-0279's number, and the measurement ADR-0328 has just
been caught by one row up. It is also two spellings of one fact: a control's number and whether it
has one are the same question.

### c. Narrowing as a live write through the deck

Argued in §3 above and rejected on the one thing that decides between them: it is silently walked
back by the next rebuild, and the next rebuild is an act about something else entirely. It is worth
recording that it is otherwise **better** — immediate, free, no watchdog verdict, no chance of a
display choice being rolled back with a build. **What would revive it** is a carry: the day `Set`
remembers an interface across a swap the way `carry_moved_from` remembers a moved value, the live
write is the right one and the aim can go back to stating what a file said.

### d. Wiring as a stepping capsule over the candidates

The console's own answer to a value a hand should not type, and it is what ADR-0262 and ADR-0328
both reached for. Refused here on ADR-0262's own recorded cost: *"the cycle grows with the store …
there is no number at which it breaks — it degrades"*. A deck's geometries are few, but a `Source`
input's candidates are not bounded by anything this console knows, and a card is what the console
already has for a list somebody picks one of. The capsule is a pulldown for the same reason the
Library bay's deck is one.

### e. A `uses` line on the folded renderer head

The renderers of a deck are drawn as one group under a bare `L4` head, and a renderer may declare an
input (`examples/second_eye.kir` declares `uses view : Camera`). A line there would be one node's
declaration drawn on a head that stands for several, which is exactly what
[ADR-0216](0216-authority-is-per-node-and-a-head-over-several-draws-no-chip.md) refuses for the
authority chip: one of several answers drawn as the answer. So the folded head takes no line, and a
renderer's input is reached from the file and from a model. Named as a limit rather than left to be
found.

### f. An engine reader for a node's declared inputs

The honest-looking fix for §1's inference, and it is unnecessary: the refusal at `Set::validate`
already guarantees that a running slot's edges *are* its declarations. A reader would be a second
answer to a question the refusal settles, and would have to be kept in step with it.

## Consequences

Checked against the tree, file by file.

- **`docs/manual/console.html`** — deck A's pane gains an `L1:1 sphere_shell` group whose two rows
  are drawn **unpublished**, and `L2:0 swirl_warp` gains a `uses far` line wired to it, so one pane
  demonstrates both controls and the pane's count reads `6 of 6`. The Inspector's opening claim is
  rewritten — which rows carry a *fader* is what a Set publishes — and two notes are added: *An
  input is wired on the node that declares it* and *The number is the mark, and publishing is a
  choice of attention*.
- **`docs/manual/style.css`** — `.uses` and its `.slot`, and `.param.unpub`'s two colours. No change
  to `.param`'s grid, which is alternative **b**'s cost avoided.
- **`docs/manual/operations.html`** — both rows read `has` in the panel column, `inspector uses
  line` and `parameter row`; *Narrow the published interface*'s `op-when` is *on a worker* where it
  was *launch*. Both tips say what the control is, what it cannot do, and what is not recorded.
- **`crates/karakuri-engine/src/set.rs`** — `Set::declared_interface`, which is `Set::published`'s
  else-branch extracted and made public. No new logic and no new state.
- **`crates/karakuri-console/src/view.rs`** — `Uses` and `Node::uses`; `UsesLine`, `uses_chip_in`,
  `uses_rect`, `uses_h` and `uses_into`; `Wiring`; `InspectorPane::uses_line`, `uses_chip` and
  `wired`; `View::wiring_open`, `open_wiring` and `shut_wiring`; `Param::ord` is an `Option`,
  `Param::control` builds an interface entry, `ord_cell` is the mark's target and
  `InspectorPane::publishing` is what a press on it asks for; `UNPUBLISHED` is the dot.
- **`crates/karakuri-console/src/room.rs`** — the `.uses` row's five constants, transcribed from
  the CSS this record adds.
- **`crates/karakuri-console/src/input.rs`** — two probe rows, *a parameter row's publish mark*
  claiming 1 and *a node group's `uses` capsule and its card* claiming 2, and `claim`'s rule 2 has a
  fifth card in it.
- **`crates/karakuri-console/src/hover.rs`** — two `TIPS` entries, because that table is keyed to
  `PROBES.len()`. It is the hover pass's file and the two rows are the seam forcing an edit rather
  than this record reaching into it.
- **`crates/karakuri/src/main.rs`** — `inspector` takes the aims, fills `Node::uses` from the slot's
  own wiring and adds the rows `Set::declared_interface` has that `Set::published` does not;
  `uses_of` is the reading; `Readout::wired` is the card's arm and `Wiring` its three answers;
  `wired_input` performs a pick, which is `rewired` with one request; `attended` performs a
  `Publish` as a re-aim; the press handler derives the card **inline** before every other control
  and the publish mark with the pane's own — inline because `press_handler::ASKED` reads that body,
  and a derivation in a helper is a control this window claims and then declines.
- **`gpu::a_pane_reads_a_running_set` reads the run's own aims**, so the `uses` lines come off what
  each slot is pointed at, and asserts what the launch pair is: no node declares an input, and every
  row carries an interface position because nothing has narrowed it.
- **The run's edge list moved from `Keeping` to `Engine`**, and that is what let a press reach it.
  `App::performed` is where every emitted operation is performed and is handed a `Gfx`; the press
  handler is the console's `Readout` and holds no engine at all. A rewiring needs the list *and* the
  aims, and `Engine` already holds the aims — so the list belongs beside them rather than beside the
  saves, and all three of its readers (`Keeping::rewire` and `playing_values` twice) were already in
  methods handed an `&mut Engine`. Three call sites and a field.
- **The vocabulary did not change.** `WireInput` and `Publish` are what they were, `written` answers
  what it answered, and both title strings are untouched — so
  `the_manual_and_the_vocabulary_agree` and `panel_column.rs` hold on the badges alone.
- **`docs/roadmap.md`** — M5.5's rows list is ten where it was eight, its exit is re-stated on this
  bay's rows rather than on a whole-page grep, and *Rows the manual has not given a home* is
  **empty** and says how the last two left it.
- **What is not closed.** A narrowing survives no replay and no keep, which is a gap in the record
  format rather than a decision. A renderer's declared input is unreachable from the panel
  (alternative **e**). And the candidate list is inferred from the layer the input already reaches,
  so a kind the deck currently reaches by no edge offers nothing — a control offering less than the
  language allows, which is the side of P-0090 to be wrong on.

## What was watched fail

Each was run alone against its own injected defect, `docs/contributing.md` §3's terms, and every
substitution was checked to have landed before its result was read.

- **Three of the eight went green the first time, and the reason is the same reason in all three:
  they asserted a consequence rather than the property** (§3's first bullet). It is the most
  useful thing that happened here and it is written out because the tests as first drafted looked
  perfectly convincing.
  - `group_h` with `uses_h(node)` deleted — a group whose height does not count its own lines —
    passed *the groups do not overlap*, because the node head is taller than a `uses` line, so a
    height short by one line still leaves two groups clear of each other. What catches it is the
    group's **own** height: a head, one line and two rows, checked against the constants.
  - `param_rect` with `uses_h(node)` deleted — rows drawn over the line above them — passed
    everything, because `publish_mark` resolves a press through the **same** arithmetic that draws
    the row. A self-consistent error is invisible to a test that asks the derivation where the
    control is; what catches it is asking `uses_rect` and `param_rect` about each other, which is
    the first row's top against the line's bottom.
  - `movable` with its `ord.is_some()` guard dropped — a fader on a row that is off the interface —
    passed a twenty-step sweep across the row, because a knob is about ten pixels wide and the
    sweep stepped over it. It is a one-pixel sweep now, and it **carries a negative control**: the
    published row beside it is swept first and a knob must be found, so a probe that has stopped
    working fails on that row instead of passing on the row with no handle.
- `wiring::the_card_is_shut_until_it_is_opened`, with `UsesLine::rows` set to the candidate count
  whether the card is open or shut: `left: 1 right: 0`. A card nobody opened handing out a
  rectangle is what `Load::rows` was shaped to stop one bay along, and this is the same guard
  checked rather than inherited.
- `wiring::taking_a_control_off_names_the_rest_in_interface_order`, with `kept.sort_by_key` removed
  so the list is built in the pane's order: `left: … [detail, exposure] right: … [exposure,
  detail]`. That is every knob on the deck renumbered by a press about a different row, and the
  fixture is built to catch it — `exposure` is numbered 1 and drawn last, `amount` is numbered 3 and
  drawn first, so a pane whose two orders agreed could not tell the two walks apart.
- `wiring::putting_a_control_back_lands_it_at_the_end`, with the re-published control inserted at
  the front instead of appended: `left: … [twist, exposure, …]`, which renumbers everything for a
  row coming back.
- `wiring::a_pick_names_the_node_the_input_and_the_deck`, with `wired` naming `uses.to` instead of
  the candidate that was picked: `left: … to: "sphere_shell" right: … to: "drift_shell"` — a pick
  that re-wires the input to what it was already wired to, which is a rebuild of the whole slot for
  nothing and would have looked like a control that does not work.
- `panel_column::every_operation_a_console_control_emits_has_a_panel_route_marked_built`, run with
  the console emitting both operations and both badges put back to `plan`. It named the rows and
  asked for the badge or an exemption, which is the seam saying which half of the change was
  missing without being told which half to look for.
