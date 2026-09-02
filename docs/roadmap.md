# Karakuri — Vision and Roadmap

**This file tracks milestones and implementation status. It is not a manual and not a design document.**
Before implementing any task here, contributors and coding agents MUST review the mandatory engineering invariants in [principles/](principles/) and adhere to the ADR lifecycle in [Principle 0066](principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md) (when changing behaviour governed by an ADR, create a new superseding ADR rather than revising history).
The code and what the words mean is [architecture.md](architecture.md); the language is [ir-spec.md](ir-spec.md); how to play it is [manual.md](manual.md) and [manual/](manual/); why a thing was decided is [adr/](adr/); milestones that closed are kept whole in [history/](history/). Where a later milestone constrains earlier code, that is stated under **Demands on earlier work**.

**No count, figure or percentage is written into this file.** Every one that ever was went stale,
usually within days and usually in silence. *The instrumentation*, below, is the four commands
that answer them, and there is nothing here to keep in step.

---

## The end state

A performer opens an empty session and types a prompt. Over the course of a set, the
system generates visual material, warms it up out of sight, and offers it at musically
sensible moments. The performer takes control of anything they want and leaves the rest to
the system. Everything generated is saved, searchable, and reusable in later sessions.
Nothing stops working when the network drops or the DJ gear changes.

Four properties define whether this succeeded:

1. **Procedural, not generated frames.** The AI writes procedures that run on the GPU
   every frame, not images or video. What gets saved is an instrument with parameters, not
   a recording.
2. **Manual and autonomous on the same mechanism.** A human and an agent affect the system
   through the identical interface. **Authority is set per node of a Set**, not a global mode
   — [ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md). This
   sentence read *a per-layer setting* until then, which was one of three answers in play and
   was the oldest of them; it loses because no operation in this system addresses a layer of a
   live Set, and per *deck slot* loses to rule 06 of the manual, which rules out any switch
   that hands a whole instrument over. The senses each word carries are in
   [architecture.md](architecture.md#words-that-carry-more-than-one-sense).
3. **Reproducible.** The same record stream and the same seeds produce the same show.
4. **Unbreakable on stage.** Nothing stops working when the network drops or the DJ gear
   changes. Defended continuously rather than built once — see Continuous concerns.

### What this is not

- Not a general-purpose compositor. Video file playback and editing are out of scope
  except as an external source node.
- Not a 3D content creation tool. There is no modelling, no asset import pipeline, no
  animation timeline.
- Not primarily a live-coding environment for humans. The IR is written by LLMs and read
  by humans, not the other way round.
- Not a cloud service. The engine runs locally and keeps running when generation is
  unavailable.

### The reference point

TouchDesigner is the closest existing thing, and the deliberate divergence is worth
stating. TouchDesigner is largely one operator per GPU pass, connected by texture
ping-pong, driven from the CPU. Karakuri compiles a subgraph into a minimal set of passes
and fuses procedural chains into single shaders. The cost is a compilation step and less
immediate feedback; the gain is that primitive-centric generation at high element counts
becomes affordable.

---

## Milestones

Sizes are rough and relative — a sense of which milestone is larger than which, not an
estimate of anyone's calendar.

### M1 — Closing the loop — **closed**

Proved the assumption the whole project rests on: an LLM can generate constrained IR, it can
be validated and compiled to WGSL, and it can be hot-swapped without dropping a frame.
[history/m1.md](history/m1.md).

---

### M2 — Playable — **closed**

A deck of Sets, a mix, transitions on a beat grid, MIDI and audio in, a session recorded and
replayed frame for frame. [history/m2.md](history/m2.md).

What M2 left owed is rescheduled: MIDI *out* and 14-bit control changes are M5.12's, and a
session stream that cannot say what a deck held is under *Mx — TODO*.

---

### M3 — Expressive depth — **closed**

Every `Ln` in the algebra is a node with a `kind` behind it, and a Set holds a chain of them.
`--set` reaches all of it with no new syntax, parameters and bindings address a *node* rather
than a layer, and a Set can say which of them a console sees.
[history/m3.md](history/m3.md).

What M3 left owed is rescheduled: fusing adjacent nodes into one pass is under *Deferred by
decision*, and how a mask names a source is under *Mx — TODO*.

---

### M4 — Library at scale — **closed**

A Set can be named, saved from a running session as the material on screen, listed, read
without compiling anything, round-tripped whole with its layering and fold, and sent to
somebody else as one self-contained file whose every inlined source is checked against the
address it claims. [history/m4.md](history/m4.md).

The goal sentence — *finding the right thing among two thousand artifacts is faster than
generating a new one* — cannot be checked, because there are no two thousand artifacts and
nothing generates one. What closed is the machinery, tested at the scale that exists.

What M4 left owed is rescheduled: `parent` and a deck-slot variant pool are M6's, what a
thumbnail is *of* is M5.3's to judge, and `perf`, dual embeddings, search and grouping are under
*Mx — TODO*.

---

### M5 — Interface

**Goal: the application.** Karakuri is a GUI application for playing a VJ set in real time — you
pick a Set per deck and mix them live — and this milestone is where that application exists.
**MCP is a mouth, not the control stick**: the four properties at the head of this document are
all about what a *person* can do in front of an audience.

**And it has an exit condition, which a goal sentence is not**: M5 closes when everything the
manual describes is implemented — no `plan` badge left in the panel column of [every
operation](manual/operations.html)
([ADR-0226](adr/0226-m5-closes-when-the-manual-is-implemented-and-the-meter-is-progress-rather-than-completion.md)).
The split below decides how that column is reached and in what order; it does not change what
closes M5.

This comes before agents deliberately: an autonomous system that cannot be observed and overridden
is not usable on stage, and building the observation surface afterwards means retrofitting it into
decisions already made.

#### The manual is the reference, and it was written before the panel

[`docs/manual/`](manual/) is the console's manual, written ahead of the implementation on
purpose: a sentence you cannot write about a control is a control designed wrong. Read it before
touching this milestone. Four pages: [the seven rules](manual/index.html), [what the words
mean](manual/concepts.html), [the console](manual/console.html) region by region with a working
mock in it, and [every operation](manual/operations.html) with each way in. Its seven rules are
capped and [`docs/principles/`](principles/) is not — adding a rule there means removing or
merging one, because they are the model somebody holds before the panel makes sense.

#### One vocabulary, and this milestone opened with naming

Every operation is reachable from the panel, from the keyboard alone, from a mapped MIDI control,
and from MCP. That is structural before it is visual: **each operation is named once, and every
surface routes into that name.** An operation only the mouse can reach breaks it; so does one the
map cannot address. A sequencer lane is a fifth route and works for the same reason (ADR-0222).

`crates/karakuri-operation/` is that vocabulary — a leaf crate with no dependencies at all, whose
specification is [every operation](manual/operations.html), checked both ways round by a test
([ADR-0180](adr/0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)), and
`crates/karakuri-operation-record/` is where one becomes a record (ADR-0194). **All four surfaces
are answered rather than pending** — MIDI moved outright (ADR-0196), MCP names its operations and
performs them itself (ADR-0199), the console's `panel::Op` stays permanently (ADR-0197, ADR-0204),
and the CLI's keys are surveyed and partly moved (ADR-0198, ADR-0232). The per-surface breakdown
is `karakuri-operation`'s own crate documentation, which tests check rather than prose
transcribes.

**A model reaches every operation, and what could stop a show is shut until a hand opens it**
([ADR-0235](adr/0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md)).
`gate` is the audit and the one place the split lives, with no wildcard arm, so a new operation
stops the build until somebody says which class it is in; an opening is map configuration rather
than an operation of its own (ADR-0236). **`crates/karakuri` serves MCP**, on `--mcp PORT`, and it
hands `karakuri_environment::mcp::serve` the same `Opening` the four bay-head pills write — so a
pill opens a class the server reads on the next call rather than one only the panel sees. Four
switches still do not cover every closed row: the clock's, the sequencer's, `Quit`, *Select a deck*
and rule 06's authority belong to no class.

**Five operations are named and left `Undecided`**: moving a boundary, choosing where the frame
goes, walking the edit history, *edit the file instead*, and the camera. Where each sits in the
split is under *The estimate is retired rather than re-cut*, below.

#### How the work below is split

**M5.1 to M5.9 are one sub-milestone per console bay, in the order below.** Each is the same job:
complete that bay's GUI components and connect them to operations from the mouse and the keyboard.
MCP, hover tooltips and MIDI cut across every bay, so they are M5.10, M5.11 and M5.12 rather than
a share of each bay's work. **Each names its rows, its exit condition and what it is blocked on**,
and is the one place those are written down. The maintainer, 2026-09-01: *"you have to get to the
point where all the bay GUI components are there and assembled, or you cannot even try it out."*

**The order is the order below, and the work goes top to bottom.** If an order needs changing the
sub-milestones are reordered; if a row needs a note the note goes in the sub-milestone that owns it.

**A sub-milestone's exit condition is a grep over two columns**: no `plan` badge in the **panel**
or **key** column of that bay's rows on [every operation](manual/operations.html). The MIDI and
MCP columns are out of scope for M5.1 to M5.9, being M5.12's and M5.10's. **A row can carry a
`plan` key badge and no `plan` panel badge**, so each sub-milestone below names those rows as well
as the ones the panel column owes. **ADR-0226 is unchanged by the split**: M5 closes when M5.1 to
M5.9 close and the two sections at the end of this list close with them.

##### Every bay ends by rewriting its own prose as tooltips

**The last item of M5.1 to M5.9 is the same job**: the notes about that bay in [the console
page](manual/console.html), condensed into the `data-tip` of the controls the mock draws. The
reason is written here once for all nine. The same prose sits in three places — this file, the
console page and [every operation](manual/operations.html) — and duplication produces gaps and
contradictions; the console page's own layout does not work, its sections long and its columns
sparse with the mock far from the text explaining it; and nobody reads a large manual, so a short
dialog beside the control has to carry the whole of it — what the control is, what state it is
in, what a click will do, and its MIDI assignment.

**And a gap gets a note.** The maintainer's reading rule, 2026-08-31: *the mock is not
exhaustive. A tooltip is something everything should carry in the end, but only the characteristic
ones are worked out. There will be gaps in the functions too — leave a purposeful note for those,
aimed at what the thing is for — **and the more detail something is written in, the more important
it is and the sooner it is wanted.*** **An empty tooltip is not a defect to fill in**; what *is*
owed where a function is missing is a note saying what it would be for, because a gap with no note
is indistinguishable from a decision.

**Doing this at the end of each bay means M5.11 starts with its specification already written**,
and owes only the pointer decision and the drawing.

##### What no sub-milestone owns

Four things the seven-item *Adds* list and the withdrawn estimate carried between them. Each is
real work, none of it closes a badge, and no exit condition above measures it.

- **Adding or removing a node.** The node editor needs it, and there is no such operation and no
  row on [every operation](manual/operations.html). The gap has an arena half: `karakuri-layout`'s
  arena has no insert and no remove, and `NodeId` is a bare index, so a removal shifts every id
  anything is holding.
- **The mock's six body controls**, of which `2 up` names a region of the arrangement and the other
  five add something a bay draws inside its own body. None is an operation and none has a row.
- **Thumbnails and live previews in the Set browser.** The drawing is M5.3's; what a thumbnail is
  *of* is M4's undecided question, handed here to judge. Neither has a row.
- **Reaching a MIDI map while running.** A map is a file saved and recalled per controller. The file
  exists; no row says it can be loaded while the instrument runs.

*An arrangement is saved and restored* has no entry here because it is finished: the record, the
place, the two operations and the transport row's arrangement pill all exist.

##### The estimate is retired rather than re-cut

**The ~10–14 week range does not survive the split and is withdrawn**: it was taken against the
seven *Adds* items, which ADR-0226 has since refused as this milestone's finish line, and every
assumption it carried now sits in the sub-milestone that owns it, stated as a blocker rather than
as weeks. When a range is wanted it is taken from the twelve blocker lists below and from the
panel column, both of which move on their own.

What the range assumed and excluded went to the sub-milestones that own it: the two undrawn bays
to M5.8 and M5.9, the pointer decision to M5.11, M4's thumbnail judgement to M5.3, a second `Sink`
to M5.6, and the sequencer's producer to M5.9. Of the five `Undecided` operations, *choosing where
the frame goes* is M5.6's and *the camera* is M5.5's; *walking the edit history* is in the no-home
section; and *moving a boundary* and *"edit the file instead"* carry no `plan` badge in either
column and are in no sub-milestone. The arena's insert and remove is above.

#### M5.1 — Program

**Rows.** No `plan` panel badge or key badge remains. The two key badges previously planned were
resolved: *Put a deck on air, prime it, or take it off* retired its key shortcut in favor of mixer
controls, and *Choose what the output shows* was retired
([ADR-0240](adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md); see
[docs/history/m5.md](history/m5.md)). **The reason ADR-0240 gave for that retirement is not the
reason.** It said redundancy with the four cells.
[ADR-0243](adr/0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md) settles the
model: the picture is one of the outputs, so once material is on it, it is the broadcast of the
final result and never a preview of anything; the four cells are monitors, each welded to the letter
under it, and nothing routes one. An operation that pointed the output at a deck was wrong from the
start rather than made redundant. `Operation::RouteFrame` names the picture and can never name a
cell, which is one more constraint on M5.6's naming question.

**Exit.** No `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html).

**Blocked on.** Nothing. All panel and key rows are clear, and what remains for the exit is
condensing the bay's tooltip notes. The bay draws one thing less than the mock does, and it is not a
badge and does not block the exit — see the risk badge, below.

**A cell's letter and state are outside the image.** Each cell is an image with a caption band under
it: the deck's letter, one word for what the cell is showing — `material`, or `no slot` — and a
place kept for the risk badge. `karakuri-console`'s `caption_of` derives the band from the image
rectangle rather than taking it beside one, so `preview_rects`, which is what the engine sizes a
texture from, stays the image and never the image plus its label.

**The risk badge is not drawn, and it is not a drawing that is owed.** Its five bands are specified
in [the console page](manual/console.html), under *What a deck preview cell shows, and when* and on
deck A's caption tooltip; the panel draws no dot because nothing in this workspace estimates what a
slot costs. What would produce that estimate is under *Performance discipline*, below.

**The bay's prose, as tooltips.** Three notes now. *Program, sized by height* is the height drag and
the letterbox, the picture being the `program view` sink and on screen exactly when that sink is on,
and why the picture carries no label of its own. *What a deck preview cell shows, and when* is the
cell: its slot's own material with no fader in it, that residency does not gate it, the caption's
three parts, which nothing a cell can be showing is *off*, and the five bands. *What a model is
refused, and where a class opens* is the `mcp` pill: what a shut class refuses, that the call is
answered rather than hidden, and that the pill says the word and is drawn armed. The mock tips
`solo`, `mcp · shut`, `previews 3 of 4` and all four cells and their captions already; the size pill
and the picture carry none. So the second note is largely condensed and the other two are not. That last
note also covers the pills in the Mixer's, the Master's and the Outputs' heads, so it is written
here once and the other three bays take it as it stands.

#### M5.2 — Mixer

**Rows.** Four carry a `plan` panel badge: *Choose the wipe shape, the quantum, the length*, *Fade
a deck out or in*, *Choose which renderer of a deck is live*, *Wipe the next deck in*. Four more
carry a `plan` key badge and no `plan` panel badge: *Gain*, *Opacity*, *Blend mode*, and *Crossfade
to the next deck*,
whose panel cell is a `gap` and whose only owed route is `x`.

**Exit.** No `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html).

**Order inside the bay.** Draw *Choose the wipe shape, the quantum, the length* first. `.xfade`, the
transition row, is drawn nowhere at all, and the other three rows convert only once a surface holds
what that row sets: *Fade a deck out or in* answers `Owed::NotRead` until the transition settings
are handed over, and *Choose which renderer of a deck is live* reads the same way.

**Blocked on.** One row of the eight. *Wipe the next deck in* waits on who says the front shape and
the soft edge are its: the shape is `Operation::SetTransition`'s third setting and nothing has said
it is the wipe's to write, and the soft edge is named by no operation anywhere.

*Set a deck's mask position* carries no `plan` badge in either column and is not in this exit
condition. It is the mixer's under-draw, named in
[ADR-0206](adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md), and whatever
surface gives the mask's front a position closes it.

**The bay's prose, as tooltips.** Two notes and half of a third. *Mixer* is the four strips, the
paged list whose length is a number, the residency chip that rolls toward what was asked for and
never lands, and the three things a fader with a fade scheduled on it says. *The mixer has no
crossfader* is why `x` stays a key. And the selection half of *Two focuses, and they do not look
alike* says why the strip is the one thing in the bay with no capsule of its own to press. The mock
tips the three tallies, the two blend chips and the priming fader; the trims, the meters and the
whole transition row carry none, and that row is where the crossfader note has to land.

#### M5.3 — Library

**Rows.** Four carry a `plan` panel badge: *List what the store holds*, *Read what one Set holds
and declares*, *Load material into a deck*, *Send a Set to somebody, and take one in*.

**Exit.** No `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html).

**Blocked on.** Half of one row. *Send a Set to somebody, and take one in* waits on a destination
the vocabulary can carry: the taking-in half is built and reached — a `presets` row taken in and
loaded on the one press — and `SetTransfer::Send { id }` names a Set the store already holds and no
path at all, while the one control in this bay that takes letters takes a *name* and not a path.
The missing sentence is where a package goes when no shell redirected it, and it belongs to the
vocabulary rather than to this bay (`view::LibraryBay`). *Load material into a deck* is not
blocked: `l` performs the operation
([ADR-0228](adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)), and
the control the page names is the drag from a row onto a strip, of which the page says *"and it is
not drawn"*. The pill beside it is a readout on purpose, so a press on it would be a third route
nobody specified (`view::LibraryBay`).

**Also here.** The Set browser's live previews and the thumbnail beside them.
The bay lists what the store holds already; the previews are what waits, and what a thumbnail is
*of* is M4's decision handed here to judge. Neither has a row on the page, so neither is in this
exit condition.

The scope row is done and the mock puts one shape over it: the list already carries **presets**
beside favourites and a folder, and says the scope list is itself extensible.

**The bay's prose, as tooltips.** The library half of *Library, and staging under it*, and five
notes under it: *A folder scope reads Sets, and a bundle is not a third thing*, *What keeps a
favourite, and where it does not travel*, *Where the presets come from, and why it is told rather
than found*, *A Set has two forms, and loading one is packaging it*, and *How a Set reaches a deck*.
*Every deck runs from its own copy* is here too, because a library load is what writes into
`<store>/scratch/`. The mock tips four of the five scope chips, two stars and the `load → A` pill;
`my sets`, the path, the two filters and the rows carry none.

#### M5.4 — Transport

**Rows.** Six carry a `plan` panel badge: *Tap the beat*, *Halve or double the grid*, *Nudge the
latency offset*, *Set the free-run tempo*, *Find out what a write did*, *Record the session*. Two
more carry a `plan` key badge and name the transport as their panel home: *Tone map* and
*Exposure*.

**Exit.** No `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html).

**Blocked on.** Nothing. This is the cheapest bay on the page: every table row is a control beside a
readout the row already draws.

One thing to know before drawing the first two. *Tap the beat* and *Halve or double the grid* reach
`karakuri_environment::audio` directly rather than going through `written`, because `written(TapBeat)`
and `written(ScaleGrid)` both answer `Owed(NotSettled)`. That is a gap in `karakuri-operation-record`
and not in the panel, and it does not stop either control.

**The bay's prose, as tooltips.** Six notes: *The beat moves, always*, *The octave is a person's,
and only one half of it is ever live*, *Health, in the transport*, *The latency offset, and which
offset it is*, *The arrangement is a file, and the reset is one of them*, and *The look is two
controls, and they sit where a frame leaves* — the tone map and the exposure track, drawn in this
row and belonging to the master chain. This is the bay the mock has already tipped: every control
here but `tap` and the tempo figure carries a `data-tip`. The item is those two, and reading the six
notes against what the tips already say.

#### M5.5 — Inspector

**Rows.** Nine carry a `plan` panel badge: *Write a parameter*, *Attach a signal to a parameter*,
*Take a parameter back*, *Element capacity, seeds, the camera*, *Read one node's source*, *Check
and write one node's source*, *Keep what a deck is playing*, *Set a node's authority*, *Composite a
deck's renderers*. Two more carry a `plan` key badge only, both at the deck head: *Set a deck's
sync mode* and *Scrub a deck a quarter beat*.

**Exit.** No `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html).

**Blocked on.** Two rows. *Set a node's authority* waits on a writer, and a live-session one:
`Set::set_authority` is reached from `swap.rs`'s restatement and from its own test and from nowhere
else, so every node of every Set is `Manual` and the chip can read while nothing can make it change
([ADR-0216](adr/0216-a-node-nobody-has-spoken-for-is-manual-and-a-request-states-only-what-was-said.md)).
*Composite a deck's renderers* waits on a setter: `Set::merge` is `Some` only where
`layering == Layering::Composite` at `Set::build`, nothing writes it afterwards, and neither `Set`
nor `Deck` offers one, so the operation names a state the engine cannot be moved into while running.

The other nine of the eleven wait on nothing. This is the largest bay of the unblocked ones and the most lopsided:
every read it needs answers off a running Set, so what is missing is presses rather than faces.

**Also here.** Three of the four items with no sub-milestone land in this bay. The **node
editor**, whose source half is
`ReadProcedure`, `WriteProcedure` and `WireInput` and is the two *node's source* rows above; adding
or removing a node is the part with no home, listed in the preamble. The **parameter surfaces**,
which are *Write a parameter*, *Attach a signal to a parameter* and *Take a parameter back* — the
MIDI learn half of that item is M5.12 and the tooltip it lives in is M5.11. And the **`man / sug /
auto` control**, which is *Set a node's authority*: the node head draws it since 2026-08-29 and the
writer is what is left. The two deck-head key badges are what remains of the **sync toggles** item,
which is otherwise done — two of the three sync modes are conditional rather than one, which is what
the chip has to be able to say, and a one-off scrub shows a price rather than a disabled control
("8 bars back — 340 ms").

**The bay's prose, as tooltips.** Eight notes: *Inspector*, *A control that names no one node*,
*The deck head, and why it is in the inspector*, *Who is holding a control*, *A knob is bound to a
deck, not to a Set*, *Two focuses, and they do not look alike*, *Authority is per node*, and *The
camera is a node, and it is the one with nothing to turn*. Only the focus half of *Two focuses* is
this bay's; the selection half is the mixer's, above. This is the largest body of prose on the page
and the bay the mock has tipped most: the deck head, the wildcard group, the authority chips, the
renderer row and *take back* each carry one, and the parameter rows and their faders do not.

#### M5.6 — Outputs

**Rows.** One carries a `plan` panel badge: *Choose where the frame goes*.

**Exit.** No `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html).

**Blocked on.** The one row. No output has an identity anywhere in this workspace:
`RouteFrame { output: Undecided }`, and deciding what names an output is the row's whole content
rather than a payload it is missing. The bay draws its sinks and hit-tests the dot; what it owes is
the switchable list, which is the blocked part.

**Also here.** A second `Sink`, which the estimate excluded from its range. The fan-out is built
([ADR-0171](adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md)) and
nothing has been written to put in the slice. A projector window is the one this repository owns.

**The bay's prose, as tooltips.** One note, *Outputs*: every place a frame goes as one switchable
list, which sinks live in this repository and which appear only with a plugin, that the Program
picture is the first row of the list, and that all of them may be off. The mock tips the four sinks
and the `mcp` pill, and `+ add output` carries none. One short note is all the prose this bay has,
so this is the smallest of the nine items.

#### M5.7 — Staging

**Rows.** Two carry a `plan` panel badge: *Keep a candidate*, *Put a node's previous version back*.

**Exit.** No `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html).

**Blocked on.** Both rows, on six items. Nothing in the list is a drawing — the row the two verdict
controls would hang on is already painted — most of it is wiring `karakuri-cli` already has, and one
item is a decision nobody has taken.

1. **A store the build path can write.** `watched()` (`crates/karakuri/src/main.rs`) constructs
   each slot's `watch::Watch` and calls neither `Watch::storing_to` nor `Watch::snapshotting_to`,
   which its own documentation says outright.
2. **A `Built` receiver on the render thread.** The `Sender<Built>` is `storing_to`'s second
   argument and `crates/karakuri` creates no such channel, so the per-node hashes have no reader
   even once item 1 exists.
3. **A baseline**, because the first build of a run has nothing before it. The startup Set is
   `Set::build` in `Engine::new` rather than the watcher, so the first `Built` a run sees would
   read as every node changed; `karakuri-cli` answers this with `history::seed` and the panel calls
   it nowhere. It has to be re-seeded on every `watch::Aim`, since a library load re-points a slot
   at different files (ADR-0228).
4. **An undecided answer: what the lane draws when one build changes two nodes.** A rebuild
   restates the whole stack, so a save touching two files is one `Built` with two changed hashes
   and one `swap::Event`. One row cannot name two nodes, and two rows means the lane's row model
   stops being one-per-slot — which is what `staging()` and `view::Candidate` are built on. **A
   decision and not an implementation**, and it stands in front of both controls.
5. **A reader for the edit history.** `karakuri_environment::history` has `record`, `seed` and
   `stamped_id` and no lister: nothing opens `<store>/history/` to answer *what came before this
   version*. `Operation::WalkHistory` is `Undecided` for the same absence.
6. **A history to read at all.** Item 5 has nothing to list until this program keeps one, and it
   is not item 1's wiring: `storing_to` puts a build's sources under a content address,
   `snapshotting_to` keeps every version that compiled. A restore owes both.

*Keep a candidate* waits on items 1 to 4 and has no hazard — it writes nothing, and what it
changes is the lane. *Put a node's previous version back* waits on 5 and 6; where it writes is
already answered, because `working_copies()` materialises one scratch copy per slot before the
window opens, so a restore moves the deck and the operator's editor names the same file.

**Also here.** The **staging lane**. The bay draws a row per slot and the producer it was gated on
is wired. What the lane still omits is the node address, `origin`, the timestamp and the head's
count, each named at the code and none of them a row on the page.

**Two findings this lane exists for, and neither has a row anywhere.** After
`Event::RolledBack` the watchdog puts the previous *Set* back on screen and **does not put the
previous file back**, while the watcher re-reads every file on every rebuild — so the picture is
the old version, the disk is the over-budget one, and the next unrelated save swaps it in again,
with nothing in the instrument saying so. And **a checker refusal reaches no row at all**: a
`.kir` the checker turns down produces no `Request`, so the disagreement an operator most wants
to see is said on stdout. Closing the second is an engine change, `Source::poll` having no way to
say *I refused*.

**The bay's prose, as tooltips.** The staging half of *Library, and staging under it* — a candidate
waits whether it came from you or from an agent, and a rejected one costs nothing — and *The staging
lane, and the sense in which a candidate waits*, which is the longest note on the page and is where
the absence of a *regenerate* is argued. The mock draws the lane with no `data-tip` anywhere in it,
so every tip in this bay is written from nothing.

#### M5.8 — Master

**Rows.** Three carry a `plan` panel badge: *Feedback*, *Bloom*, *RGB shift*.

**Exit.** No `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html).

**Blocked on.** All three, on an L5 kind the IR does not have: `ast::Kind` is
`L1 | L2 | L3 | L4 | Field`. *Feedback* waits on a second thing as well, a decision nobody has
taken — which cut of the previous frame this one reads. It is the heaviest row on the page and the
one M5 cannot close by drawing. *Bloom* and *RGB shift* carry `params: Undecided`: what their knobs
are has never been named.

The bay is an `out` fader and a head. The fader is built end to end, and the chain the bay is named
for is these three rows.

**The bay's prose, as tooltips.** One note, *Master, and the two levels that are not one level*:
what `out` is the level of, what the tone mapper's `exposure` is the level of, that the chain
between them is what makes them two, and that with nothing in that chain they are today the same
number. The mock tips the `out` fader and the `mcp` pill. The three chain rows carry none, and they
are the rows this bay is blocked on drawing.

#### M5.9 — Sequencer

**Rows.** All five of the bay's rows carry a `plan` panel badge: *Toggle a step*, *Mute a lane*,
*Point a lane at what it drives*, *Choose a pattern's steps and what a step is worth*, *Choose
which pattern the sequencer plays*.

**Exit.** No `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html).

**Blocked on.** All five, on one thing: a pattern and a step grid, neither of which anything in this
workspace holds. *Toggle a step*'s payload is `Undecided` for the same reason. Where a pattern is
*kept* once it exists is settled
([ADR-0227](adr/0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md)) and is
not the machinery. The bay draws nothing but its head.

This is the estimate's excluded item — the sequencer's producer — and it is why this bay is last
rather than because of its weight on the page.

**The bay's prose, as tooltips.** One note, *Sequencer*: a row per lane under the mixer strip it
drives, a lane as a fifth route into the vocabulary rather than a binding, and the step grid as the
beat clock subdivided rather than a fourth clock. The mock tips the four lane labels and nothing
else — the pattern pills, the step controls, the ruler and the cells carry none. This is the thinnest
prose of the nine against the most undrawn bay, so most of what this bay ends up drawing has no note
behind it yet, and writing its tip is where that is found.

#### M5.10 — MCP

**Rows.** The MCP column of every row, which M5.1 to M5.9 leave out. Seven of the page's rows have
an MCP route today and the rest carry a `plan` badge.

**Exit.** No `plan` badge in the MCP column of [every operation](manual/operations.html).

**The server exists in the instrument**, which is a change to what this sub-milestone is. `karakuri
--mcp PORT` runs `karakuri_environment::mcp::serve` against the panel's own `Opening` and publishes
seven tools, so the mechanism
([ADR-0199](adr/0199-mcp-names-its-operations-and-performs-them-itself.md)) is not only settled, it
is running. **What is left here is two things
rather than the server**: the audit's surface, and the operations the panel does not reach.

**The audit's surface.** `gate` is the audit and has no wildcard arm, so a new operation stops the
build until it is classed (ADR-0235, ADR-0236). The console draws four class pills. Four switches do
not cover every closed row — the clock's, the sequencer's, `Quit`, *Select a deck* and rule 06's
authority belong to no class — and an opening is map configuration rather than an operation, so
whether a press covers a class or one operation of it, whether an opening outlives a restart, and
whether an open class shuts itself are open ([the console page](manual/console.html), *What a model
is refused, and where a class opens*).

**Blocked on.** The bays above, for the rest. A row whose operation the engine cannot yet perform has
nothing for MCP to route to. MCP is a mouth rather than the control stick, so it follows the bays
rather than being spread through them.

#### M5.11 — Hover tooltips

**Rows.** None. No operation on the page is a tooltip, so this sub-milestone closes no badge and its
exit condition is not a grep.

**What it is.** Every compact control explains itself on hover. The console draws no tooltip
anywhere and has not half-drawn one: a tooltip needs `egui` to own a widget, and this console
paints. The text is not what is missing. The mock carries about sixty-three `data-tip` attributes
already, and M5.1 to M5.9 each end by rewriting their bay's prose into more of them, so this
sub-milestone starts with its specification written and owes only the drawing.

**Blocked on.** A decision rather than work: who owns the pointer — whether the panel gains `egui`
widgets, or paints its own hover layer.
[`karakuri-console`'s `input`](../crates/karakuri-console/src/input.rs) names it as its own
decision and deliberately does not take it.

#### M5.12 — MIDI

**Rows.** One. *Write a parameter* carries the page's only `plan` MIDI badge; every other row's MIDI
column is `has` or `gap`.

**Exit.** No `plan` badge in the MIDI column of [every operation](manual/operations.html).

**What the badge count leaves out.** Four things, none of which has a row. The map is migrated and
`Action` is deleted ([ADR-0196](adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)),
and what is missing is **reaching a map while running** — a map is a file saved and recalled per
controller. The next is **learn**: a control is bound to a deck and a position in the Set's
published interface, never to a parameter by name, and the assignment lives in the control's own
tooltip. The last two are M2's, and they are the surface itself rather than a route into it:
**MIDI *out***, so a surface's LEDs and motorised faders follow the deck, which matters the moment
two things can move a fader; and **14-bit control changes**, so a fader is more than 128 positions.

**Blocked on.** M5.11, and only for learn: there is no tooltip to learn from until the pointer
decision is taken. MIDI out and 14-bit control changes do not wait on it.

#### Rows the manual has not given a home

Three rows carry a `plan` panel badge over a panel cell reading `—`: *Wire a procedure's input to a
node*, *Narrow the published interface*, and *Walk the edit history*. Nothing on the console is
specified to reach any of them, so no bay holds them and none of M5.1 to M5.9 can close them.
**The manual has to name a home before they can be scheduled.** There is no drawing to owe until it
does.

Two of them have a second blocker behind the missing control. *Narrow the published interface* is
upstream of the Inspector: until something publishes, every control that bay will ever draw is a
wildcard. *Walk the edit history* carries `WalkHistory { step: Undecided }` and needs the same
history reader M5.7's item 5 names.

#### The console's own shape

Four rows have a panel home that is not a bay: *Fold a bay away* at the bay head, *Fold a pane away*
at the pane edge, *Size the window* at a drag, and *Quit* at the `close` control. They are the
panel's own chrome, so no M5.x above owns them, and ADR-0226 does not close without them.

Two are the console's and two are the host's. **The folds want a hit test.** The bay head is painted
and never hit-tested, so the only route is still a key, and they reach the console as
`karakuri_console::panel::Op::Fold` rather than through the operation vocabulary — which is why
`tests/vocabulary.rs` rather than `panel_column.rs` is what checks them. **Sizing and quitting are
the host's**: `crates/karakuri` answers `WindowEvent::Resized` and `WindowEvent::CloseRequested`
itself, and `esc` reaches the second. What is owed there is the `a` binding, which is bound to
nothing, and the `close` control on the panel.

Nothing in this section is blocked.

---

### M6 — Autonomy

**Goal:** the system prepares material without being asked and is safe to let run.

**Adds**

- Agents, and **what each one is attached to is still undecided.** This bullet asks for one per
  *deck slot*; the older statement of the control plane called them *node agents*, and naming it
  either way would settle the question by wording. **What is no longer part of that question is the authority**: it is set per node
  ([ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)), and an
  agent scoped to a deck slot can respect a per-node authority perfectly well — so this
  granularity is a question about agents and not about rule 06 any more. Whatever it is
  attached to, each owns a prompt,
  watches designated inputs, selects from its prepared variants, and queues generation for
  material it expects to need. Autonomous selection means choosing among what already exists,
  never generating on a path the frame waits for
- Director. Coherence across the **kinds** — whether the camera behaviour suits the
  geometry, whether L4 is killing the shape L1 produced
- Mix agent. Set-level arc, energy trajectory, monitoring for staleness
- Authority model. Budget constraints ("camera changes at most once per four bars"),
  and manual intervention instantly demoting to `Suggest` whichever agent was moving the
  control — *which* agent that is depends on the undecided granularity above. **What the
  demotion writes is decided**: `Operation::SetAuthority` on the node, and
  [P-0078](principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md) is
  the rule it is the standing case of
- Generation queue with priority and cost awareness
- **`parent`, recorded from the first generated artifact**, or the genealogy has a hole at its root
  that no later pass can fill. It is M6's because this is where something first generates a
  procedure; an empty one written before then would fill the hole rather than leave it visible.
  **The sharpest demand in this document**, and the one most likely to be missed by arriving late
- A deck-slot variant pool, where alternatives differ at L1 or L2 and so carry state of their own,
  and the unselected ones are primed off air. It is the machinery under *selects from its prepared
  variants* above. The half that lives inside one Set is built

**Demands on earlier work**

- The record stream must be the sole mutation path, so that an agent is structurally
  incapable of doing anything a human could not do through the same interface
- Parameter schemas with ranges on every procedure, from M1 — an agent with no declared
  ranges has nothing to search over

~8–10 weeks.

---

### M7 — Musical foresight

**Goal:** transitions land on phrase boundaries because the system knew they were coming.

Audio analysis is reactive; it reports what already happened. Track analysis is
anticipatory. This is what makes generation-time latency compatible with live performance:
knowing a chorus arrives in 32 bars is enough time to generate, compile, and prime.

**Adds**

- rekordbox integration. Beatgrid and transport position, current track identification,
  and phrase structure (`PSSI`) from the local collection database
- Lookahead timeline as a queryable structure, not a signal — "the first Chorus within 16
  bars"
- Backward scheduling. Transition point fixes priming start, which fixes generation start.
  "Will not make it" becomes a decision to postpone rather than a dropped frame
- Track metadata into visual direction: key to palette, colour tags, and a convention for
  writing VJ hints in the rekordbox comment field
- Optionally PRO DJ LINK via beat-link and crate-digger, if playing on club CDJs

**Demands on earlier work**

- The Set lifecycle must already be schedulable by absolute time, not just triggered.
  **Undischarged by two closed milestones, and deliberately**: a transition schedules on beat
  count and nothing else, which is what makes a fade reproduce on a machine at a different
  frame rate and follow a tempo change mid-fade. Absolute time is a second axis rather than a
  correction, and it arrives with whatever needs it
- Signal confidence must already drive transition policy, since these sources are
  unofficial reverse-engineered integrations that will break

~6–8 weeks.

---

### Mx — TODO

**Work that is owed, has no milestone whose subject it is, and is not scheduled.** It is here
rather than under a closed milestone because work left under one is never picked up again, and it
carries no number because a holding list with a milestone number would claim a goal it does not
have.

- **A session stream cannot say what a deck held.** A Set file describes one Set, so only slot 0's
  material reaches the head of a recording and a multi-slot session replays the rest against a deck
  of one, reporting what it skipped. A format change. From M2.
- **How a mask names a source**, with the `source` uniform it would read — the L2 `mask` block
  comparing a `Source` slot, specified in `docs/ir-spec.md` under *A mask that names the source it
  applies to*. Not the mixer's mask, which is a different sense of the word and already reaches the
  panel. From M3.
- **`perf` on a card.** It is a measurement and wants the stage-7 probe; what a compile pass can
  answer is `ops_per_element`, a different quantity in different units. From M4.
- **Dual embeddings.** The text side needs `origin` and `tag`, which have no producer until
  something generates procedures; the visual side needs the thumbnail M5.3 judges. From M4.
- **Search over more than a node's name, and grouping.** The same absent card fields. From M4.

---

## The handover

**Read this if you are picking the work up.** It says where to start, what that piece needs, and
what nobody has decided. Everything else is in the sub-milestone above, in the code, or in a record.

### Where to start: M5.1 — Program

**The panel is a program.** `cargo run -p karakuri` opens the console over a real deck: the
picture, four deck previews, the transport, the mixer, the Library bay, the Inspector, the
Staging lane and the Master fader. It is [`crates/karakuri`](../crates/karakuri), a binary of its
own since ADR-0214; what it needs that is not a surface is
[`karakuri-environment`](../crates/karakuri-environment)'s (ADR-0215), and
`crates/karakuri-cli/src/` is `main.rs` alone. `--presets DIR` and `--store DIR` name where the
data lives, and with neither given a four-candidate search runs (ADR-0230). `examples/` ships the
`.kset` files a fresh clone opens on; `my sets` is empty until something is saved.

**M5.1 is the current sub-milestone and it is nearly closed.** The Program bay carries no `plan`
panel or key badges on [every operation](manual/operations.html). The preview sizing and arrangement
was settled by ADR-0239 (preserving operator preview size across arrangements, pure area comparison
at crossover 706px, and widening outer panes to 340px / 400px so standard windows open in Below
placement). The two remaining key badges were resolved: *Put a deck on air* retired its shortcut
in favor of mixer controls, and *Choose what the output shows* was retired
([ADR-0240](adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md); see
[docs/history/m5.md](history/m5.md)) — for redundancy with the cells, which
[ADR-0243](adr/0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md) has since
corrected to the operation having been wrong from the start. **That answered what the deck A preview cell shows**, which
was one of the decisions nobody had taken: the panel aims **a sink per cell**, each presented from
`Deck::slot_view` for the slot it is lettered for, so no cell can show another deck's material under
the wrong letter. `Deck::set_preview` and `Deck::preview` are **deleted** — that question is
answered, in
[ADR-0241](adr/0241-auditioning-survives-the-control-that-was-retired-and-is-re-recorded-as-a-property.md),
which also retired P-0070 and re-recorded the requirement as
[P-0080](principles/0080-an-operator-can-see-a-slots-own-material-without-putting-it-on-air.md),
*An operator can see a slot's own material without putting it on air*.

**Residency does not gate a cell, and the two gaps that paragraph used to name are gone.** `Deck`
draws every slot into its own target on every frame, whatever its residency, and the field that
chose one slot is deleted (`crates/karakuri-engine/src/deck.rs`). `Engine::aim` asks whether there
is a slot behind the cell — `slot_bind_groups[slot]`, which is `None` past `slot_count` — and never
what that slot's residency is (`crates/karakuri/src/main.rs`). That closes the second gap. The
first, that `karakuri-cli` cannot audition a slot, is not a gap:
[ADR-0242](adr/0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)
makes the command line test tooling and scopes it out of the instrument's principles, so a rule
written for the person playing the instrument does not reach it. **What P-0080 still records under
*Where it is not met* is prose rather than behaviour.** `crates/karakuri/src/main.rs` carries a
module header and two constant docs that still say a cell is drawn only for a Live deck and that
three slots are neither stepping nor drawn — its *Three of the four preview cells are off at a time*
paragraph, and the docs on `ON_AIR` and on the deck's fullness. That is
[P-0023](principles/0023-a-document-that-describes-replaced-behaviour-is-worse-than-none.md), and it
is the program narrating behaviour it no longer has. The rejected build's cell — black, or the
sentence saying what went wrong — is owed on top of it.

**The panel serves MCP.** `karakuri --mcp PORT` binds `127.0.0.1:PORT` and runs
`karakuri_environment::mcp::serve` with the same `Opening` the four bay-head pills write, so a pill
opens a class the server reads on the next call. Seven tools: `read_procedure`, `write_procedure`,
`wire_input`, `swap_outcome`, `save_set`, `read_set` and `list_sets`. The last three came with a
**save path this program did not have** — `Live::save_set`, which is the one place a Set is written
here and where the `k` key, the MCP tool and whatever control the Library bay grows all end. The
server addresses each deck's working copies rather than the paths the operator typed, so a model
cannot rewrite the preset library, and there is no bind option: reaching it from another machine is
`ssh -L`. **This changes what M5.10 has left** — see that sub-milestone.

**A cell's letter and state are outside the image**, in a caption band under it, with a place kept
for a risk badge that is not drawn. And
[ADR-0243](adr/0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md) settles the
bay's model: the picture is one of the outputs and is in the set `Operation::RouteFrame` has to
name; the four cells are monitors and are not outputs at all. Both are M5.1's, above.

**Nothing above blocks M5.1**: what remains for its exit is condensing the bay's tooltip notes.

**The fate of `karakuri-cli` is open.** The recommendation on record is to shrink it as each M5.x
moves its features to the panel, rather than delete it; ADR-0242 says deleting it is out of that
record's scope.

**The order after M5.1 is the sub-milestone list above.** Work them top to bottom; each one names
its own rows, its exit condition and its blockers. Run the fourth command beside them: a `plan`
badge does not distinguish a drawing that is owed from a machine that is missing, and a row has
already been picked up as the first while being the second. **Master and Sequencer stay last for
machinery rather than for drawing.**

### Built: a sub-pixel primitive is floored at one pixel and compensated in the alpha

**This was the *Decided and not built* item and it is built**
([ADR-0245](adr/0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md),
which supersedes [ADR-0244](adr/0244-a-sub-pixel-primitive-is-drawn-at-one-pixel-and-compensated-in-the-alpha.md)).
A sprite of side `s` pixels with `s < 1` produced no fragment unless its quad happened to cover a
pixel centre, so a small target lost most of its material and lost it *arbitrarily* — measured,
one sprite at the shipped default covered sixteen texels at 1280x720 and none at 128x72. It is now
drawn at one pixel with the fragment's **colour** multiplied by the coverage the floor took: `s²`
for a sprite, `s` for a stroke, carried from the vertex stage as a flat varying. The alpha is left
alone because alpha is coverage — what the mix's `over` hides behind and what leaves the mix into
the master chain — so the price is that a rounded-up primitive occludes as though it filled the
texel. Exact under `additive`, approximate under `over`, and inert at or above a pixel — so no
host-clock figure in this file was taken on material it touches.

**Two things it changed that are not the renderer.** `tests/deck.rs` argued that re-rendering into
a preview cell is *cheaper and wrong*; the *wrong* was this defect, and the cost has moved with it
— a 112x63 render went from 0.6x the whole 720p frame to 1.1x of it, measured as a pair on one
machine and recorded in that file. And a `Points` procedure's fragment count no longer falls with
the target's area all the way down: it bottoms out at one fragment per element, which is what the
*preparation slot is the measurement* extrapolation below has to be read against.

**What it did not touch is the resolution model**, which is still the largest thing nobody has
decided — below.

### The decisions nobody has taken

Four, each with what it blocks. Every one was found by building the thing next to it.
*What the deck A preview cell is showing* was one and is answered in the M5.1 paragraph above.

- **The instrument has no resolution model, and this is the denominator every budget number
  depends on.** `crates/karakuri/src/main.rs` renders at `const CANVAS: (u32, u32) = (1280, 720)` —
  a constant, unrelated to the window and to any output. `karakuri-cli` has `--canvas`; the panel has
  no way to say it at all. No output has an identity anywhere in the workspace either, which is why
  `Operation::RouteFrame` carries `output: Undecided` and why the console page's own mock draws a
  size pill reading `1920×1080` over a program that renders 1280x720. **Every cost figure in this
  file was measured at that constant**, and the maintainer has said 720p is an unrealistically low
  resolution for a venue — so every band, budget and refusal in this project is calibrated against a
  number nobody chose. **What it blocks**: the risk badge's five bands, the whole-frame budget below,
  and M5.6, which cannot name an output without saying what an output is. It is not blocked on
  anything itself.

- **Which cut of the previous frame a feedback effect reads, and what holding it costs.** The
  previous frame is not one thing. It could be a Set's output, the raw frame the mix wrote before
  anything downstream touched it, or the frame as it stands after some effect in the chain, so a
  feedback procedure has to name which cut it reads and the vocabulary cannot say that. A cut that
  is read has to be held, which is a frame-sized target and a copy per frame, so holding all of
  them against the chance that something reads one is the cost nobody would pay. The shape this
  points at is that **the selection recomposes the pipeline**: a cut nothing reads is not retained,
  and a `.kir` that names one adds the retention to the graph the way an edge adds a pass. Nothing
  in `karakuri-engine` does that — the render graph is built from a Set's nodes and the fold is
  fixed. **What it blocks is M5.8, and M5 itself**: the operations page carries a `plan` row for
  `feedback`, so M5 cannot close without either taking this or changing the manual.

- **Whether loading a shipped preset should write into the operator's own library.** ADR-0229
  makes a load a packaging step that stores the bundle, so opening a shipped Set adds an entry to
  `<store>/sets/` — a consequence of two decisions rather than a decision anybody took. It is
  probably right, being copy-on-load exactly as `scratch.rs` performs it for material and what
  keeps `examples/` unwritten (P-0048). **The first symptom if it is wrong** is a `my sets` filling
  up with things the operator did not make.

- **What `expand` under a solo should do.** `Layout::soloed()` can lie: the solve never reads it
  and `check_structure` only checks that it addresses a node, so a solo's exclusivity lives in
  collapsed flags any later `expand` may contradict, and the next `unsolo` restores its snapshot
  and throws the expand away with nothing said. Nothing reaches that state today.

Three more questions are named where they are met rather than here: what the Staging lane draws
when one build changes two nodes (M5.7's item 4), who owns the pointer (M5.11), and whether an
addressed write by an agent onto a node the operator kept is refused, which cannot be decided
until something writes a parameter on an agent's behalf (M5.5). **What an agent is one *of*** is
M6's and is open there.

---

## The instrumentation

**These four commands are the only source of the numbers in this project, and they are not to be
summarised or transcribed into prose.** Every figure ever written into this file went stale — the
meter four times, the vocabulary count six, the panel's allocation figure twice, the program's own
startup legend five ways at once — so the numbers are gone rather than corrected.

**The meter is the panel column** of [every operation](manual/operations.html), where a `has`
badge means an operator running the instrument reaches that operation
([ADR-0213](adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)),
and the key column is the instrument's keyboard rather than the command line's
([ADR-0220](adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)).
The board is the page's rather than this file's: every `plan` badge names where its control
lives, so what is left arrives grouped by region and ordered by weight.

```sh
# the meter — how far each surface has got
for col in panel key MIDI MCP CLI; do
  echo -n "$col: "
  grep -o "class=\"rt [a-z]*\">$col " docs/manual/operations.html |
    sed 's/.*rt //;s/">.*//' | sort | uniq -c | tr '\n' ' '; echo
done

# the board — what is left on the panel, grouped by where the control lives
grep -o 'rt plan">panel <b>[^<]*' docs/manual/operations.html | sed 's/.*<b>//' |
  sort | uniq -c | sort -rn

# the vocabulary — how many operations there are
grep -c '<h3' docs/manual/operations.html

# drawing or machine — the operations nothing in this workspace constructs
sed -n '/^operations! {/,/^}$/p' crates/karakuri-operation/src/lib.rs |
  grep -oE '^    [A-Z][A-Za-z]+( \{[^}]*\})? => "[^"]+"' |
  sed -E 's/^ *//; s/ *(\{[^}]*\})? *=> "/|/; s/"$//' |
  while IFS='|' read -r v t; do
    n=$(grep -rn --include='*.rs' "Operation::$v" crates/ |
        grep -vE 'crates/karakuri-operation(-record)?/' |
        sed -E 's#^[^:]*:[0-9]+:##; s#//.*##' |
        grep -c "Operation::$v")
    [ "$n" -eq 0 ] && echo "$t"
  done
```

**The fourth answers a question the badges cannot.** A `plan` badge says a surface is meant to
reach an operation and does not yet; it says nothing about *why*, and an operation nothing
constructs is one nobody can route to at all. Two cuts make it honest, both the ones
`karakuri-console/tests/panel_column.rs` already makes: the record crate is excluded because it
would answer *constructed* for every operation, and each line is truncated at its first `//`,
because a doc comment naming an operation is not a call site. Seven of what it returns are
expected. Five of those are the console's own shape, which reaches its rows through
`karakuri_console::panel::Op` — moving a boundary, folding a bay, folding a pane, bringing one
back, and resetting the arrangement. The other two, sizing the window and quitting, are the
host's `winit` events.

**The panel measures itself rather than being measured here.** `crates/karakuri` installs a
counting `#[global_allocator]` and holds each run against its own `WRITTEN_ALLOCS`, saying so
when the two part company (ADR-0217); `karakuri-console`'s `tests/schedulable.rs` holds both of
P-0072's schedulability conditions and `tests/parked.rs` holds the still panel's zero. Take any
figure from them as an order of magnitude rather than a quantity two of them can be subtracted
from.

---

## Continuous concerns

Not milestones. These degrade silently if not defended at every step.

### Live safety

- No allocation or compilation on the render thread, ever. **Defended structurally and
  asserted by proxy**, where the audio thread has a counting global allocator behind it: the
  render path's claim rests on where allocation is *possible*, not on a harness that would
  catch it
- Watchdog on new pipelines with automatic rollback
- The show continues when the generation API is unavailable. This becomes meaningful once
  there is a pool to fall back on, so it is a promise from M4 onward rather than from M1
- Graceful degradation for every input source, with confidence driving how conservative
  transition scheduling becomes

### Determinism

Reproducibility is not a nice-to-have; it is what makes the record stream meaningful.
Undo, replay, A/B comparison, and session recording all follow from it. Anything
introducing run-to-run variance — atomic append ordering, real frame delta, wall-clock
seeds — breaks all four at once.

### Performance discipline

Per-element cost is the artifact's intrinsic property; total cost is a function of capacity.
Metadata records the former. Any change touching the frame path comes with a measurement.

**The machine this is developed on is not the baseline, and saying so is a rule rather than
a caveat.** It has been wrong twice in the same direction. Its GPU timestamps advertise a
feature it does not deliver, which is why the probe calibrates instead of trusting a flag.
And it has enough memory that no VRAM figure this project has produced has ever been
uncomfortable — which was then used, once, as evidence that a memory question could wait.
A number measured here is a number from a rich machine: it bounds nothing.

**Development is a desktop and a performance is probably a laptop.** Those are different
machines with different thermal limits and different memory, and the second one is the one
that matters — rehearsal and the night itself run on the same box, and which box that is
belongs to whoever is playing.

#### The budget is measured over one slot, and it has to be measured over the frame

**Decided, not built.** `Deck::govern` builds its `SlotState` list from `swap.measured_cost()`, which
is one slot's Set measured on its own. Nothing measures the composite, the present passes or the
panel, so the number the governor decides on is not the number the frame costs.

**The failure is silence rather than slowness.** Four slots at 262144 elements measured about 10 ms
a frame and registered as no violation at all. The maintainer's point: the problem is not that it is
slow, it is that it is not detected.

**What must not be done about an over-budget slot.** Forcing it to keep drawing, or lowering the
frame rate to fit it, confuses the means with the end — the frame rate is what the show is played at,
not a dial to spend. **A slot over budget is stopped.** That is the last of the risk badge's five
bands and the one that is also a behaviour ([the console page](manual/console.html), *What a deck
preview cell shows, and when*).

**What is thin.** What the whole-frame measurement is taken *with* is not settled — the probe
measures a Set, and a frame is passes this project has never timed together. The instrument question
[P-0012](principles/0012-a-measurement-carries-how-it-was-taken.md) asks of any number applies to
this one and has no answer yet.

#### The preparation slot is the measurement

**Decided, not built, and it is what would produce the risk badge's estimate.** Today a reference
measurement is the hint that admits a Set into a slot at all. The slot is then **drawn small while it
prepares**, and that small draw is a second measurement — a higher-confidence estimate of what the
same material costs at full size, on this machine, in this environment, rather than a number carried
from elsewhere.

**`Topology` decides the extrapolation, and it is known at compile time.**
`karakuri_ir::ast::Topology` is `Points | Lines | Fullscreen`. A `Fullscreen` procedure's cost is the
pixel count, so it scales with the target's area. A `Points` procedure is dominated by primitive
work, so it does not. `Lines` is unmeasured and its scaling is not known. (The maintainer says
*segments*; the enum says `Lines`.)

**The small draw has a floor under it now**, which cuts both ways for this.
[ADR-0245](adr/0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md)
holds a sub-pixel primitive to one fragment rather than dropping it, so a small target's picture is
the same picture and the measurement is of the same material — which is the whole premise of
measuring there. It also puts a floor under the *cost*: below the size at which sprites reach a
pixel, a `Points` procedure costs one fragment per element whatever the target is, so a draw made
smaller past that point stops getting cheaper and stops saying anything new. `tests/deck.rs`
measures the pair: a 112x63 render of the panel's own material went from 0.6x the whole 720p frame
to 1.1x of it.

**Round the estimate toward refusing.** A show is cheaper to protect before it starts than to rescue
during it.

**`Priming` gains a second meaning.** It already means warming buffers
([ADR-0053](adr/0053-priming-runs-the-simulation-and-skips-rendering.md)); this makes it also mean
measuring.

**Two slots live and two preparing is the standard way of working.** That is why drawing a small
target cheaply matters, and it is the premise the risk badge's bands are read against — *blue* is the
ordinary state of good material precisely because two at 60 Hz is the ordinary case.

**What is thin.** How small *small* is, what it is drawn into, and how the extrapolation's confidence
is expressed are all unstated — and the floor above puts a bound on the first of them that nobody has
yet turned into a number. So is what happens to a `Lines` slot, whose scaling nobody has
measured. And every one of these numbers is read against a resolution nobody has chosen — see *The
decisions nobody has taken*.

#### What a machine's size is allowed to decide

**No hard limit, and no ceiling chosen here.** A rich machine should be allowed to spend
what it has: more capacity, more slots resident, longer chains. Capping that to make a small
machine's experience the only experience would be this project choosing against its own
operator.

**What is not allowed is waste that a bigger machine merely hides.** The two are easy to
confuse and the test between them is simple: *does spending it buy anything?* Capacity buys
elements. A resident Set buys an instant transition. Sixteen bytes holding a four-byte
`seed` buys nothing at any size, so it is not a budget question and does not wait for one —
it is tightened because it is slack, and the argument is theoretical rather than measured.

**4 GiB of dedicated VRAM is the reference for "does this fit"**, and it is a reference and
not a cap: it is the number a design is checked against, the machine a default has to be
comfortable on, and the size at which a refusal must carry a sentence rather than a driver
error. Above it, the operator's machine decides.

**"GPU timestamp" is what that used to say, and it is honoured in labelled substitute.** The
probe asks for timestamps, proves them with a calibration workload, and reports which
instrument it actually used — `GpuTimestamp` or `HostWallClock` — because this crate's own
development machine advertises the feature and lies. So the rule is a measurement that names
its instrument, and the headline frame-path numbers in this repository came from a host
clock and say so.

---

## Deferred by decision

Designed, understood, deliberately not scheduled.

**Fusion — the graph compiler's other half.** Folding a chain of stateless stages into one
shader at codegen. The language already makes it legal rather than merely plausible: an L2's
statelessness is *structural*, since every generated `deform` overwrites its output from its
input before the block runs, so there is no previous value left to accumulate onto. Nothing
has to change in the language for this. It is an optimisation over a materialising
implementation that is already correct.

**It is deferred on a measurement and against an argument, not on cost.**

The argument is that fusion is a trade and not a win: it costs no memory and **pays the
stage's cost once per reader**, so a heavy deformation read by three renderers costs three
times. Materialising pays once however many nodes read it — which is the shape M3 just spent
a milestone building. Fusion makes the multi-renderer case *worse*.

The measurement is that nothing is near a budget. A stage is one compute pass and one
read-write of the element buffer; at 262144 elements and a 96-byte stride that is about
50 MB a frame, or 3 GB/s at sixty — a few percent of a modern part's bandwidth — and the
longest chain anything ships is two stages. There is nothing here to reclaim yet.

**What would revive it**, written down so this is an observation and not a mood: a probe
measurement putting a Set over budget where the cost is in a chain of stateless stages with
**one reader**. `probe.rs` already measures at real capacity with real parameters and the
governor already acts on the number, so the instrument to notice this exists — it does not
need building first, and this item does not need scheduling until it fires.

**External shader source compatibility.** Shadertoy-style material does not fit the
primitive-centric model directly: it is a single fullscreen fragment shader, with no
geometry stage and no parameters. Two import modes were designed — wholesale, as an opaque
L4 fullscreen node, and extraction, where an LLM pulls out the SDF or noise function and
discards the raymarch loop so it becomes a composable `Field`. The real value in both is
parameter lifting: hardcoded constants become declared `param`s, turning a frozen image
into an instrument.

This is deferred rather than dropped because `VideoSource` makes it cheap later. A foreign
source only needs to produce a colour texture. It slots in beside a Set without touching
the Set model.

Note that imported material is often extremely expensive. Allowing it makes the budget
governor mandatory rather than advisory.

**A keyboard is mapped and learned the same way a control surface is.** The keys are a `match`
in `karakuri-cli` today, and there is no reason they should be: the vocabulary is what made a
MIDI map possible, and a key is another way to name the same operation. The grammar is already
written — `examples/surface.map` says `<target> <deck> <value>`, and a key is a third `Key`
beside `Cc` and `Note` — so the map file's parser is nearly all of the work, and whether that
lives in `karakuri-midi` or moves out of it is the question that goes with it, since a crate
named for a wire would then be parsing something that never touches one.

**Learning it belongs on the control**, which is the maintainer's own model: a GUI component is a
translator between one operation and N ways in, so *learn this control* is one gesture from one
place and MIDI is only the first of the N. Both learn paths wait on the same missing mechanism,
which is M5.11 — this console draws no tooltips at all, and the mock draws no key hints anywhere
either, so **the console as drawn gives an operator no way to learn a key binding**, which is
rule 01's keyboard surface with no teaching path
([ADR-0205](adr/0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md)).

**What a keyboard map has to name that a MIDI map never did**: twelve of the CLI's thirty-nine
keys write no record at all — the deck selection, the transition settings, the window, quitting
— because they are the surface's own state (ADR-0194's group (c), ADR-0198). Every operation a
map line can produce today writes a record, so a keyboard map is the first map file that would
name operations the record layer never sees, and what a map file may say is then a slightly
larger question than *which keys*.

**Also deferred:** projection mapping and multi-output warping, DMX and Art-Net lighting
sync, collaborative multi-operator sessions, video file playback, and any browser or wasm
target. The engine is native for Syphon, NDI, low-latency audio, MIDI, and GPU timestamp
queries.

---

## Settled decisions

**These moved.** Every rule that was listed here is now one file in
[docs/principles/](principles/) — stated once, so it cannot drift between documents — and the
decision behind each, with the alternative that lost, is one record in [docs/adr/](adr/).
`ls docs/principles/` is the index, because each filename is the rule it states.

This section said *reference, not history*, and that was the right split; what it had no room for was
the history, which is what `docs/adr/` is. It also had to carry supersessions inline — *this said per
layer until a Set could hold two geometries* — which a record's front matter now carries instead.

Nothing was left behind. The one entry that had been held here as unsettled — whether reference
material handed to a generator belongs in `origin` — is settled in
[ADR-0133](adr/0133-what-was-fed-to-a-generator-is-a-field-not-an-object.md): the object stays
rejected, the field does not.

---

## Document map

| | |
|---|---|
| `README.md` | The front door: what this is, a quickstart, and where every other document is |
| [`docs/architecture.md`](architecture.md) | The code: crates, pipeline, threading, the layer model, the three clocks, and the vocabulary |
| [`docs/principles/`](principles/) | The rules in force, one file each — `ls` is the index |
| [`docs/adr/`](adr/) | Every decision, with the alternatives that lost — append-only |
| [`docs/history/`](history/) | Milestones that closed, kept whole. History, never the present tense |
| `docs/ir-spec.md` | The IR: grammar, semantics, lowering, record formats |
| `docs/manual.md` | How to play it: every flag, every key, and what each does |
| [`docs/manual/`](manual/) | The console's manual, published — and the specification M5 is measured against |
| `docs/plugins.md` | The out-of-process boundary, and why those two things are outside it |
| This file | The milestones, what each is waiting on, and the handover |

None of the others should restate this one, and this one should not restate them. **What is
built, part by part, is not here**: the code's own documentation and the two manuals are the
primary sources, and a status page that repeated them went stale faster than any of them.

---

## Reading order for implementation

1. [`docs/principles/`](principles/) — the rules in force
2. [`docs/architecture.md`](architecture.md) — the crates, the pipeline, and what the words mean
3. `docs/ir-spec.md` — the whole thing before writing any parser code
4. This file — *The handover* first if you are picking work up
5. [`docs/manual/`](manual/) — before touching M5, because it is what M5 is checked against
