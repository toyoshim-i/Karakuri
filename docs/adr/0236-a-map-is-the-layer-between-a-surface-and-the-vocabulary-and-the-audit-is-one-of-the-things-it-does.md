---
id: 0236
title: A map is the layer between a surface and the vocabulary, and the audit is one of the things it does
status: accepted
date: 2026-08-31
supersedes: []
superseded_by: []
principles: [0085, 0090, 0093, 0094]
tags: [architecture, surfaces, midi, mcp, vocabulary, live]
---

# A map is the layer between a surface and the vocabulary, and the audit is one of the things it does

## Context

[ADR-0235](0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md)
decided the same day that MCP is connected to all 64 operations and that four classes are **closed
by default**, opened from an indicator in the head of the bay each class belongs to. It decided
where the opening is drawn and what the audit refuses. **It did not say what kind of thing the
opening is**, and it left that in *What this leaves undone* as a question with a circularity in it:

> - **The opening setting has no operation.** Nothing in `karakuri-operation` names it, so there is
>   either a 65th row on the operations page or a reason it is not one — and if it is a row, rule 01
>   says it is reachable from four surfaces, which walks straight into the question above.

*The question above* is whether `SetAuthority` and the opening are ever openable to MCP, which that
record recommends against and does not decide: **a permission an actor can grant itself is not a
permission.** A 65th row would make the opening reachable from the surface it governs, and the whole
mechanism would then be one call from being turned off by the thing it exists to hold.

Asked what the opening *is*, the maintainer answered on 2026-08-31:

> 操作ではなく操作のマップの監査機能の制御。つまりmcpから操作へのマップの間で監査する、マップ機能の
> 一つ。キーボードやMIDI のマップがツールチップにあるアイコンから変更できるって話しと一緒。

— *it is not an operation; it is control of the **map's** audit function. The check sits between MCP
and the operation it names, and it is one of the things a map does. It is the same story as the
keyboard and MIDI maps being editable from an icon in a tooltip.*

The summary he approved is this record's spine:

> Every surface reaches the vocabulary through a map. That layer holds three things: **which surface
> gesture names which operation**, **whether a surface may reach a class of operations at all** (the
> audit), and **helpers that carry time** — a curve, an automation — which is what ADR-0232 called
> *"a helper where the control map already sits"*. The bay-head toggle and the tooltip icon are two
> entrances to the same layer.

### The position this names was already occupied, and ADR-0232 is what put something in it

[ADR-0232](0232-a-control-is-thrown-because-the-request-is-asynchronous-and-a-curve-is-a-helper-on-the-control-map.md)'s
fifth part settled where curve control goes if it is ever wanted:

> **5. Curve control, if it is ever wanted, is automation: a helper where the control map already
> sits, never a feature of the engine.** A trigger becomes a timed series of the same operations,
> emitted on the beat clock, off the render path — one more writer standing where `karakuri-midi`
> stands, depending on `karakuri-operation` and nothing else, unable to say anything a hand could
> not say.

That record named the position by pointing at the one thing standing in it. **This record is what
the position turns out to be**, and the pointing is the reason it can be named at all: `karakuri-midi`
is not merely *an example of* the layer, it is the only built instance of it, so *"where the control
map already sits"* was a description of one crate and not yet of an architecture.

### What is actually there today, surface by surface, and it is one of four

Rule 01 of [the manual](../manual/index.html) names four:

> Every operation is reachable from the panel, from the keyboard alone, from a mapped MIDI control,
> and from a model over MCP. There is no operation only the mouse can reach, and none the map cannot
> address.

**MIDI has a map, and it is a file.** `crates/karakuri-midi/src/map.rs` states the reason it is one:
*"**A surface's numbers are the surface's.** There is no controller this engine knows the layout of,
and inventing one would be a table that fits one device and misleads about every other. So the
mapping is a file, the file is the operator's, and what this module owns is the translation — not the
layout."* `examples/surface.map` is that file — `cc 1 -> gain 0`, `note 36 -> residency 0 live` — and
the crate header says where it stops: *"It produces a [`karakuri_operation::Operation`] — 'the
operator asked for gain 0.7 on deck 2' — and stops."* `karakuri-cli`'s `--midi-map` takes the path.

**The keyboard's assignment is a `const` in the panel's own source.** `crates/karakuri/src/main.rs`'s
`KEYS` is *"Every key this window binds, and the sentence the legend prints for it"*, and it is a
legend rather than a binding: what binds is the `match` in `window_event`, and
`key_column::the_keys_this_file_lists_are_the_keys_the_window_loop_binds` **reads that `match` out of
this file's own text** and asserts it is exactly `KEYS`. `karakuri-cli` has its own second table,
`BINDINGS`, a `const &str` that is both what `--help` prints and what a run prints at startup, over
its own match. **There is nothing an operator can change in either.** The assignment is in the source
twice and in data nowhere.

**MCP has neither a map nor an audit.** `crates/karakuri-environment/src/mcp.rs`'s `asked()` turns a
tool name and a JSON object into an `Operation` and `perform()` *"dispatches on the operation rather
than on the tool's name"* — which is the seam the audit will read, and it is a `match` on a `&str`
with seven arms, not a table anybody edits. Nothing between the call and the operation asks whether
this caller may make it.

**The panel reaches operations directly from the surface that draws them.** `window_event` answers
`Acted::Emitted(Some(Operation::SelectDeck { deck }))`, `Operation::LoadSet { deck, set }`,
`Operation::TapBeat` and the rest from the `match` arm the press landed in. There is no layer, and
the console's own five arrangement operations do not even go through the vocabulary —
[ADR-0197](0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md) costed
that and `karakuri_console::panel::Op` stayed.

**So the layer exists for one surface of four**, and the drawn version of it exists for none: the
console page's `learn` pill and its `map · nanoKONTROL2 ▾` menu are both mock, and
`karakuri-console/src/view.rs` says so in the plainest terms — *"`learn` is MIDI learn… **There is no
map, no learn mode, and exactly one control on the panel to point at**"*, and *"`map · nanoKONTROL2 ▾`
is the map in use, **and it is a file**. None is loaded, saved or recalled anywhere here."* The panel
binary does not open a MIDI port at all: `crates/karakuri/Cargo.toml` names no `karakuri-midi`, and
the file's own missing-features list says *"`karakuri-midi` and `examples/surface.map` exist and
`--midi-in` `--midi-map` drive them. **No operation names attaching one**, so this is a hole in the
vocabulary before it is a hole here"* — and the same sentence again for MCP, *"**no operation names
opening it**"*.

**That last pair is the third instance of the thing this record is about**, found while checking the
first two, and it is what makes the answer general rather than a special case for one toggle:
*attach a MIDI port*, *open the MCP server* and *open a class to MCP* are all configuration of how a
surface reaches the vocabulary, and **none of the three has an `Operation` today.** The opening
setting is not an anomaly in a vocabulary that otherwise names everything; it is the third member of
a group nobody had noticed was a group.

### The tooltip is where the other entrance already is, written down

[The console page](../manual/console.html) has carried it since it was written. The `learn` pill:
*"Click, then point at any control and move a knob on your surface, and the two are mapped. Every
control that has a tooltip can be learned, so nothing is unreachable by accident."* And its rule,
under *Every icon explains itself*: *"The tooltip also carries the control's MIDI assignment, which is
how learning one works: point at anything with a tooltip and move a knob. Coverage then takes care of
itself — **a new operation gets a tooltip, and a tooltip is a thing a map can reach**."* Every control
on that page ends its tooltip with `⊕ MIDI: cc 2` or `⊕ MIDI: unassigned` and a sentence saying why.

**That is a map editor already specified, distributed across every control on the panel.** What the
maintainer's analogy says is that the bay-head opening is the same editor reached from the other end
— per class of operations rather than per control — and not a new kind of setting.

## Decision

**A map is the layer between a surface and the vocabulary, and it is one layer with three jobs.**
Every surface reaches `karakuri-operation` through it: which gesture names which operation, whether
this surface may reach a class of operations at all, and helpers that carry time. ADR-0235's opening
is the second of the three, and it is **map configuration rather than a member of the vocabulary the
map addresses.**

**So the opening setting needs no `Operation`, and ADR-0235's open item is closed by that.** The
circularity it feared arises only if the setting is a row: rule 01 makes a row reachable from four
surfaces, so a row that gates MCP is reachable from MCP, and a model would be one call from opening
its own gate. It is not a row. **The map is not the vocabulary**, and a setting that decides what a
surface may say cannot be said in the language the surface is speaking.

**The rule is narrower than *map configuration is never an operation*, and it has to be, because the
vocabulary already contains a piece of map configuration.** `karakuri-operation`'s
`PointLane { target: Undecided } => "Point a lane at what it drives"` is a map line written as an
operation — the first of the three jobs, an operator saying which operation a source names — and it
is a row on the operations page today. So the line that settles ADR-0235's item is this one:

**A setting that decides whether a surface may reach a class of operations cannot itself be one of
those operations.** Not because of where it lives, but because rule 01 is true: any row is reachable
from the surface it would govern. `PointLane` gates nothing, so it can be a row and is one. The
opening gates, so it cannot be.

**This is [P-0090](../principles/0090-a-surface-offers-it-never-decides.md) applied
one level out.** That principle says a control decides *what a press asks for* and never *what may be
asked*, and that a constraint lives where the record is applied so every way in meets the same wall.
The opening is a constraint on **what may be asked, by whom**, and the map is where a surface's
askings are already translated — so it is the one place a rule about a surface can live without being
a rule *held by* that surface. A per-surface lock is exactly what P-0090 rules out; a per-surface
entry in a common layer is not, because the layer is the same code for all four.

**Why there is an audit at all is not this record's**, and it is worth saying so plainly: that is
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
and ADR-0235, which asked *what does this do at its worst, on the frame it goes wrong* of each class
and closed four of them. **This record decides only where the mechanism lives**, and it changes
nothing about which operations are in which class or what a refusal says.

**And the two entrances are one layer.** The bay-head indicator ADR-0235 places in Mixer, Master,
Outputs and Program, and the icon in a control's tooltip that the console page has always specified,
are two views of the same configuration: one per class of operations, one per control. Neither is a
new mechanism.

### A sequencer lane, under this reading

[ADR-0222](0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md) decided *"A sequencer lane is
a fifth route into `karakuri-operation`, not a signal binding. It emits operations on the beat, the
way the pointer, the keys, a MIDI map and MCP emit them."* Asking whether that makes a lane a map
entry is worth doing rather than assuming, and **the answer is that a lane is not one map entry: it
is two things, one on each side of the layer's own list.**

**The case for a map entry is strong and is not the whole answer.** A lane has a left side and a right
side exactly as a map line does — `cc 5 -> opacity 0` in `examples/surface.map` and a lane pointed at
deck A's opacity are the same sentence — and `PointLane`'s own title, *"Point a lane at what it
drives"*, is that sentence spelled as an operation. A lane sits where the map sits by every structural
test ADR-0232 used: on the beat clock, off the render path, emitting operations, *"unable to say
anything a hand could not say"*.

**The case against is that a map entry translates a gesture, and a lane has none on its left side.**
Every line of `surface.map` begins with something a hand did to hardware. A lane begins with the beat
clock. And a lane carries **authored state** — ADR-0222's central finding, *"a pattern is authored
state"*, which is why the stateless signal bus could not carry one — where no map line carries any.

**So the lane decomposes across the layer, and both halves are in it.** `PointLane` is the first of
the three jobs, a map entry an operator authors at runtime. The pattern, its grid and its steps are
the third — a helper that carries time, which is precisely the position ADR-0232 gave an automation
helper, and that record's own open question (*"whether an automation helper and a sequencer lane are
one thing is not settled"*) is this record's answer to the extent that they are **the same job of the
same layer**, with steps where the helper has a curve. Whether they are one mechanism or two remains
that record's question and is not settled here.

**Read this way, ADR-0235's sharpest worry costs nothing.** It wrote *"the route that would defeat it
is a lane, not a tool"* — a model that cannot write `SetOpacity` but can point a lane at a fader and
unmute it has written the mix one beat later. Under this record a lane's emission crosses the same
layer an MCP call crosses, so the audit that reads the operation as it lands reads a lane's without a
second copy of the table. That is what ADR-0235 asked for and this is the structure that supplies it.

### What this record does not do is build the layer

**Stated flatly so the record cannot be read as a description of the present:** *every surface reaches
the vocabulary through a map* is the shape being named, not the shape that exists. It is true of MIDI
today and of nothing else. The keyboard's assignment is a `const` and a `match`, the panel emits from
the arm the press landed in, and MCP has neither a map nor an audit.
`docs/contributing.md` §4 is the rule that makes that
sentence mandatory rather than decorative — *an invariant is a claim, and a claim has a truth value on
a date* — and its second half is the instruction: the clause is a marker, not a resting place.

## Alternatives

### a. The opening is the 65th operation

The reading ADR-0235 offered first, and it has a real argument: rule 01 would then cover it, an
operator could open a class from the keyboard as well as from the bay head, and the setting would
write a record like everything else
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)) — which is not nothing, since
*the operator opened the mix to a model at 23:40* is exactly the sort of thing a session should be
able to say.

**It loses on rule 01 being true.** A row is reachable from four surfaces, so the row that governs
MCP is reachable from MCP, and the mechanism holds only for as long as a model does not call it.
Saving it needs a permanent exception — a row that is closed to one surface for ever — and ADR-0235
already flagged that as the one place *connect everything* and *the operator opens it* genuinely
collide. **This reading is what would force that collision to be resolved; the map reading dissolves
it**, and leaves the collision standing only where it really is, on `SetAuthority`, which is a row.

It also loses on company: *attach a MIDI port* and *open the MCP server* have no operation either, and
under this alternative all three become rows, each one reachable from the surface it configures.

### b. The audit belongs to the MCP server

Put the check in `karakuri-environment::mcp`, beside `asked()`, where the only caller that needs it
lives. It is the smallest change on this page and needs no new concept at all.

**It loses to P-0090 and to ADR-0235's own finding.** A rule held by one surface binds one surface —
*"a lock held in the panel makes the instrument behave differently depending on which hand touched
it"* — and the lane is the proof rather than the hypothetical: a second automatic route already
decided (ADR-0222) would arrive with no audit on it, and the fix would be a second copy of the table
in a second crate. ADR-0235 said the same thing from the other end: *"an audit skipped on one path is
the whole mechanism gone."* The map is one layer, so the check is written once in it.

### c. There is no layer — *map* is MIDI's word for a file and nothing more

The conservative reading, and it has
`docs/contributing.md` §4 behind it: a name means one
thing, and *map* today means `karakuri-midi`'s `Map`, a pure function from a hardware message to an
operation. Widening it to cover an audit and an automation helper risks the failure that principle is
named for.

**It loses because the widening has already happened, in ADR-0232, unnamed.** *"A helper where the
control map already sits"* is a claim about a position, and a position that holds a translation table
and a timed emitter is not a translation table. Refusing the name leaves that sentence pointing at a
crate rather than at an architecture, and leaves the opening setting with nowhere to be. Naming it is
what makes P-0093 satisfiable rather than what threatens it: *map* means **the layer between a surface
and the vocabulary**, one thing, and `karakuri-midi::Map` is its first and only instance.

### d. One map per surface, decided now

Four maps, one each, each with its own file, its own audit column and its own editor. It is the shape
MIDI already has and it needs no new question answered.

**It is not rejected — it is refused as premature.** A file per surface may well be right, and so may
one table with a surface column; what decides it is where an opening has to be read from, whether a
map travels with an arrangement, and what a keyboard map is when the operator has not written one.
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) says check what a feature is
made of before it gets a subsystem, and the honest answer today is that three of the four maps have
nothing in them. It is in *What this leaves undone*.

## Consequences

- **ADR-0235's open item is closed and its recommendation shrinks to one row.** The opening needs no
  `Operation`, so there is no 65th row and no permanent exception for it. What survives of that
  record's *"whether `SetAuthority` and the opening setting itself are ever openable"* is
  `SetAuthority` alone — a genuine row, closed by default, and still the maintainer's to decide.
- **The layer is true of one surface of four and this record builds none of it.** MIDI has a map,
  the keyboard and the panel have source, MCP has nothing. No code in this workspace changes on the
  day this is recorded.
- **The keyboard's `const` becoming data an operator can edit is a real change with a real cost, and
  it is named rather than designed.** `KEYS` is a legend, not a binding: the binding is the `match`
  in `window_event`, and the test that keeps the two honest works by **reading that `match` out of
  the source text**. A keyboard map that an operator can change means the `match` stops being the
  binding — so that test has nothing to read and needs replacing, `karakuri-cli`'s `BINDINGS` is a
  second table with the same problem in a second binary, and every legend either becomes generated
  from the map or becomes a document that describes a default nobody is running. None of that is
  hard; all of it is work, and it is the price of the sentence rather than a detail of it.
- **Rule 01's *"none the map cannot address"* is a claim about a map that three of the four surfaces
  do not have.** The clause reads today as a statement about MIDI's file and is written as though it
  were about all four. Under this record it becomes true of the architecture and stays false of the
  build until the other three maps exist. **The manual is another writer's and this record proposes
  no edit to rule 01** — ADR-0235 already found it stands as written, and it still does; what is owed
  is that the sentence's scope is now larger than the code, which is P-0093's clause and belongs
  wherever the map is described rather than in the seven rules.
- **The tooltip icon is the console page's and is a specification task already owed.** ADR-0235 handed
  that page four bay heads to specify; this record adds nothing to the count, because the per-control
  entrance is already on that page in prose — the `learn` pill, the `map · nanoKONTROL2 ▾` menu and
  the `⊕ MIDI:` line every tooltip ends with. What is owed is the same task the maintainer has already
  named for MIDI: what the icon is, what it opens, and what it shows for a control no map can reach.
- **Three settings turn out to be one kind of thing**: attaching a MIDI port, opening the MCP server
  and opening a class to a model. Each is written in the panel's own missing-features list as *no
  operation names it*, and each is map configuration under this record rather than a hole in the
  vocabulary. That is a re-reading of two existing notes and not a new decision; whoever next reaches
  for a 65th row should reach here first.
- **[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) is
  unaffected and worth checking rather than assuming.** *An operation says what it wants,
  never which way to move* is a rule about the vocabulary, and the opening is not in the vocabulary —
  but the toggle that draws it is still a control, and a control that means *the next state after this
  one* is the affordance P-0090 permits over operations it does not own. The pill can be a toggle; the
  thing it writes still names a state.

### What this leaves undone

- **What the map is as data, and where it is kept.** A file like `surface.map`, a table in the store,
  something in an arrangement — nothing here chooses, and the MIDI map's own argument for being a file
  (*which knob is which is a property of the hardware in the room*) does not obviously carry to a
  keyboard or to an audit.
- **Whether it is one map or one per surface.** Alternative d, refused as premature rather than
  wrong.
- **Whether a map travels with an arrangement, with a Set, or with neither.** `karakuri-midi` says the
  MIDI map is emphatically not in the session stream — *"replaying one room's wiring in another is not
  replaying a performance"* — and whether that argument covers an audit state is not obvious, because
  an audit is about who is in the room rather than about what is plugged into it.
- **What a map edit does mid-performance.** A gesture re-pointed while a hand is on it, a class closed
  while a call is in flight, a lane re-pointed on the beat it fires.
- **Whether the audit's state persists across a restart.** ADR-0235 asked this of the opening and it is
  the same question of the layer: *a setting that persists is one an operator can forget they left on;
  one that does not is one they have to set again before every show.*
- **What the tooltip icon looks like and what it opens.** The console page's, and the specification
  task the maintainer has already named as owed for MIDI.
- **Whether the keyboard's map is ever authored by an operator at all**, which is the question behind
  the cost above. A MIDI surface has to be mapped because no two are alike; a keyboard arrives the
  same on every desk, and *the assignment is data* and *the assignment is editable* are two decisions
  rather than one.
