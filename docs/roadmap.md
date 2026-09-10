# Karakuri — Vision and Roadmap

**This file tracks milestones and implementation status. It is not a manual and not a design document.**
Before implementing any task here, contributors and coding agents MUST review the mandatory engineering invariants in [principles/](principles/) and adhere to the ADR lifecycle in [contributing.md](contributing.md) §4 (when changing behaviour governed by an ADR, create a new superseding ADR rather than revising history).
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

What M4 left owed is rescheduled: `parent` and a deck-slot variant pool are M6's; what a
thumbnail is *of* went to M5.3 to judge and that bay closed without judging it, so it sits under
*Mx — TODO* beside the drawing it waits on; and `perf`, dual embeddings, search and grouping are
under *Mx — TODO* as well.

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

**Some operations are named and left `Undecided`**, and the list has shrunk: choosing where the frame goes is decided (ADR-0324), walking the edit history is built (ADR-0308), and the camera's numbers are parameter rows (ADR-0318), so what is left `Undecided` is *moving a boundary* and *edit the file instead*. Where each sits in the split is under *The estimate is retired rather than re-cut*, below.

#### How the work below is split

**M5.1 to M5.9 are one sub-milestone per console bay, in the order below.** Each is the same job:
complete that bay's GUI components and connect them to operations from the mouse and the keyboard.
MCP, hover tooltips, MIDI and **the keyboard** cut across every bay, so they are M5.10, M5.11,
M5.12 and M5.13 rather than a share of each bay's work. The keyboard joined them on 2026-09-05 and
was a share of each bay's work until then: it stopped being one when
[ADR-0259](adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
made a key press a consequence of one mechanism — the bay that has focus — rather than a letter each
bay chose for itself. **Each names its rows, its exit condition and what it is blocked on**,
and is the one place those are written down. The maintainer, 2026-09-01: *"you have to get to the
point where all the bay GUI components are there and assembled, or you cannot even try it out."*

**The order is the order below, and the work goes top to bottom.** If an order needs changing the
sub-milestones are reordered; if a row needs a note the note goes in the sub-milestone that owns it.

**A sub-milestone's exit condition is a grep over one column**: no `plan` badge in the **panel**
column of that bay's rows on [every operation](manual/operations.html). The key, MIDI and MCP
columns are out of scope for M5.1 to M5.9, being M5.13's, M5.12's and M5.10's.

**It was two columns until 2026-09-05, and the key column left for a reason rather than for
convenience.** While a key was a letter a bay chose, a bay owed its own keys and the exit said so.
ADR-0259 makes a key press an address into whichever bay has focus, so no bay chooses letters any
more and none can be held to them: the column is now one mechanism's, the way the MIDI column is the
map's. M5.2 is the sub-milestone this was noticed in — it had finished its panel column and was
being held open by four key badges naming letters the record had just retired. **ADR-0226 is unchanged by the split**: M5 closes when M5.1 to
M5.9 close and the two sections *Rows the manual has not given a home* and *The console's own
shape* close with them — named rather than placed, because M5.14 now sits after both and *at the end
of this list* stopped locating them.

**What is still open, as of 2026-09-09.** Of the bays, **M5.5 — Inspector alone**: M5.1 to M5.4 and
M5.6 to M5.9 have closed, and so have M5.14 and *The console's own shape*. What is left beside the
Inspector is the cross-cutting run — **M5.10 MCP, M5.11 hover tooltips, M5.12 MIDI and M5.13 the
keyboard** — each of which is a column of [every operation](manual/operations.html) rather than a
bay, and none of which the panel column's grep reads. Each closed entry above is a stub naming what
it decided, when its exit was met and where every owed item went;
[history/m5.md](history/m5.md) keeps them whole.

**M5.15 is the first sub-milestone whose rows are in two bays, and its exit says so.** It is a
change to what a library row *is* — a procedure is one, and loading one writes a single layer over
what a deck is playing
([ADR-0338](adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md))
— so it lands in the Library and in the Inspector at once and belongs to neither. Its exit is
therefore a grep over the **panel** column of the four rows it names rather than over a bay's, which
is the same test read on the list the sub-milestone owns. **The four rows are M5.15's and not the
two bays'**, which has to be said rather than assumed: M5.3 closed its column against the rows that
existed then, and M5.5 is still open, so two `plan` panel badges arriving under *Inside a Set* would
otherwise enlarge an exit somebody is working towards. M5.5's rows are **enumerated by name** — ten
of them, spread over five sections and deliberately not *Inside a Set* — and neither of these two is
on that list. They are named in M5.15 and nowhere else.

**M5.16 sits after the bays and after the cross-cutting run, and it is neither of those things.**
It is a language change wearing a bay's surface — `kind L5`, and the master chain as an ordered
list of them, decided on 2026-09-10
([ADR-0340](adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)) — so
it is not a share of any bay's work and no column of [every operation](manual/operations.html) is
its subject. It carries M5.8's exit a second time rather than an exit of its own, because retiring
that bay's three rows is what turns that grep red.

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
- **Thumbnails and live previews in the Set browser.** The drawing was M5.3's and what a thumbnail
  is *of* was M4's undecided question, handed to that bay to judge; it closed without judging
  either, so both are under *Mx — TODO*. Neither has a row.
- **Reaching a MIDI map while running.** A map is a file saved and recalled per controller. The file
  exists; no row says it can be loaded while the instrument runs.

*An arrangement is saved and restored* has no entry here because it is finished: the record, the
place, the two operations and the transport row's arrangement pill all exist.

##### The estimate is retired rather than re-cut

**The ~10–14 week range does not survive the split and is withdrawn**: it was taken against the
seven *Adds* items, which ADR-0226 has since refused as this milestone's finish line, and every
assumption it carried now sits in the sub-milestone that owns it, stated as a blocker rather than
as weeks. When a range is wanted it is taken from the blocker lists still open below and from the
panel column, both of which move on their own.

What the range assumed and excluded went to the sub-milestones that own it: the two bays undrawn
when it was taken to M5.8 and M5.9, both of which have since closed; the pointer decision to M5.11;
M4's thumbnail judgement to M5.3, which closed without taking it, so it is under *Mx — TODO*; a
second `Sink` to M5.6, which built it; and the sequencer's producer to M5.9, which placed it. Of the
five operations that were left `Undecided`, *choosing where the frame goes* is not one any more —
ADR-0324 gives `RouteFrame` a destination from a closed list and an on/off, and M5.6 closed on it;
*the camera* is M5.5's; *walking the edit history* is M5.3's, is built, and left the no-home section;
and *moving a boundary* and *"edit the file instead"* carry no `plan` badge in either column and are
in no sub-milestone. The arena's insert and remove is above.

#### M5.1 — Program — **closed**

The picture is one of the outputs — the set `Operation::RouteFrame` has to name, and it can never
name a cell — and the four cells are monitors welded to the letters under them, each drawing its own
slot's material whatever that slot's residency
([ADR-0243](adr/0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md)).
[history/m5.md](history/m5.md).

**Exit, met on 2026-09-03**: no `plan` badge in the panel or key column of this bay's rows on
[every operation](manual/operations.html). It closed one drawing short of the mock — the risk badge,
undrawn because nothing could then estimate what a slot costs — which did not block the exit and
which M5.14 exists to name and has since drawn.

What M5.1 left owed is rescheduled: the rejected build's cell — black, or the sentence saying what
went wrong — is
[ADR-0258](adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)'s
and is stated under *Where to start*, below.

#### M5.2 — Mixer — **closed**

The mixer draws the deck: a trim, a fader, a blend button, a meter and a mask mini per deck the
session has, no strip at all for a track nothing fills
([ADR-0178](adr/0178-the-mixer-draws-four-tracks-and-as-many-strips-as-the-deck-has.md)), and under
them a transition row that sets the wipe's shape, quantum and length and emits the wipe. A fade has
no control of its own, because the fader is already the control for moving an opacity — *Nothing on
a strip starts a fade* on [the console page](manual/console.html). [history/m5.md](history/m5.md).

**Exit, met on 2026-09-05**: no `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html). It closed short of the key column on purpose.

What M5.2 left owed is rescheduled: **the key column, on four rows — *Fade a deck out or in*,
*Crossfade to the next deck*, *Wipe the next deck in* and *Choose the wipe shape, the quantum, the
length* — is M5.13's**, because
[ADR-0259](adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
addresses a key press to the bay that has focus: what those rows want is one pass over the keyboard
and not a letter apiece, the maintainer has decided the global shortcuts get that pass together, and
the letters those badges used to print are not the debt. **And *Set a deck's mask position* is under
*Mx — TODO*** rather than here: it carries no `plan` badge in either column, so no exit condition in
this file reads it, and a debt only a closed milestone names is the failure this file has now been
caught by twice.

#### M5.3 — Library — **closed**

The bay lists what the store holds, most recent first, because the listing is the operation's and
not the surface's
([ADR-0263](adr/0263-the-library-bay-lists-most-recent-first-because-the-listing-is-the-operations-and-not-the-surfaces.md)),
and its filter fields step through what the store already holds rather than taking letters
([ADR-0262](adr/0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)).
A folder is chosen by dropping one on the **window** rather than on a bay
([ADR-0275](adr/0275-a-folder-is-chosen-by-dropping-one-on-the-window-and-the-drop-is-the-windows-rather-than-a-bays.md));
a reading is one question and only the opening asks it
([ADR-0264](adr/0264-a-reading-is-one-question-and-only-the-opening-asks-it.md)), reached by a
`params` pill that is a toggle, over a list that scrolls
([ADR-0312](adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md)); `load` is a
button and a pulldown, and the deck it names is not the selection
([ADR-0305](adr/0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md));
a carried Set names its deck at the release and the panel refuses no drop
([ADR-0265](adr/0265-a-carried-set-names-its-deck-at-the-release-and-the-panel-refuses-no-drop.md),
[ADR-0273](adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)); a row's own
menu loads onto a named deck and sends a Set through the system's own save dialog
([ADR-0311](adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md),
which supersedes ADR-0267 and leaves the `.path` row the folder scope's readout rather than an
outbox); and a scope chip walks one Set's edit history, a row landing that version on a node
([ADR-0308](adr/0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)).
[history/m5.md](history/m5.md).

**Exit, met on 2026-09-09**: no `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html). *Send a Set to somebody, and take one in* was the last
row still reading `library`, and moved with ADR-0311. **The exit rule's one defect is unchanged**:
*Keep what a deck is playing* is one of this bay's rows by the page's grouping and is the
Inspector's control, which is said in [history/m5.md](history/m5.md) and in M5.5.

What M5.3 left owed is rescheduled: **the keyboard's routes to the controls this bay grew — the
split `load` button and its pulldown, a row's own menu, the landing on a `history` row, and a key
that moves the scroll window — are M5.13's**, which is what each of those records says of its own;
**the structured reading in `karakuri-environment` that both the tool and the panel render is
M5.10's**, where the model-facing surface is, this bay's stake in it having closed the day *Read
what one Set holds and declares* went `has`; and **whether `l` should follow the pulldown's deck
rather than the selection — ADR-0305's one assumption, left for the maintainer — together with
thumbnails and live previews in the Set browser and M4's question of what a thumbnail is *of*, and
what the filter fields should be, are under *Mx — TODO***, none of them being read by any exit
condition in this file.

#### M5.4 — Transport — **closed**

The transport row is the grid and what drives it. The latency offset is a track rather than the
capsule the mock had, because a press has to name a value
([ADR-0277](adr/0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md)); the
tap and the octave leave `App::performed` before `written` is reached, which is what lets one press
and one key be one route
([ADR-0278](adr/0278-an-operation-no-record-can-be-written-for-leaves-the-window-before-it-is-written.md));
the free-run tempo is the figure itself, and what a press is trusted with is a band round the
number under it — a guard on a hand, in the hit test and nowhere an estimate travels
([ADR-0291](adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)); `rec`
is a record/stop toggle and each start takes a fresh stamp
([ADR-0289](adr/0289-the-rec-pill-is-a-record-stop-toggle-and-each-start-takes-a-fresh-stamp.md));
and the health capsule is a readout of what the last write did, declaring nothing because a save is
not a rate ([ADR-0283](adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)).
A row the page marks `read` may have its built badge met by a **drawing** this crate can name and
find rather than by an emission
([ADR-0288](adr/0288-a-read-rows-built-badge-is-met-by-an-emission-or-by-a-drawing-this-file-can-find.md)),
which is what let the readout in this row be counted at all. [history/m5.md](history/m5.md).

**Exit, met on 2026-09-08**: no `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html). Every row of this bay reads `has`, including the two the
page files under other groups — *Find out what a write did* and *Record the session*.

What M5.4 left owed is rescheduled, and all of it is under *Mx — TODO*: **`written(TapBeat)` and
`written(ScaleGrid)` answering `Owed(NotSettled)`**, which is a gap in `karakuri-operation-record`
rather than in the panel and closes by saying what a `Current` carries about a beat lock; **which
number the frame readout carries** — the period in place of the CPU figure, both side by side, or
the tooltip's promise rewritten, a decision nobody has taken and a page change before it is code;
and **the tooltip item**, which is not a drawing but a reading of this bay's own notes against what
its tips now say.

#### M5.5 — Inspector

**Rows.** The rows whose panel badge names a control this bay draws, which is **not** the page's
*Inside a Set* section and is where the count has to be derived from: they are spread over five
sections. **Ten of them** — *Composite a deck's renderers*, ***Choose which renderer of a deck is
live*** (moved here from M5.2 on 2026-09-05, because the mock draws its chips directly under
*Composite* and the console page describes the two as one control and the choice it turns into),
*Write a parameter*, *Attach a signal to a parameter*, *Take a parameter back*, *Element capacity,
seeds, the camera*, *Set a node's authority*, *Keep what a deck is playing*, and — since
2026-09-09 — ***Wire a procedure's input to a node*** and ***Narrow the published interface***.

**The last two joined this list rather than being scheduled onto it.** They carried a `plan` badge
over a panel cell reading `—` and sat in *Rows the manual has not given a home*, which said the
manual had to name a home before they could be scheduled. It did, and the home was already implied
by rule 05 and by the mock: a node's declared input is a line on that node's group, and whether a
control is published is the number at the left of its own parameter row — the position a MIDI knob
counts, present or absent
([ADR-0329](adr/0329-an-input-is-wired-on-the-node-that-declares-it-and-the-number-is-the-publish-mark.md)).
Both are `has` on the panel, both re-aim the slot, and that section is now empty.

***Read one node's source* and *Check and write one node's source* were the ninth and tenth and are
neither**, dropped on 2026-09-09 with the maintainer's decision that they stay model-only —
「モデル専用のまま (panel は gap)」. Both panel badges are `gap` now and the two rows say so: a source
editor is a third letter-taking flow on a console that has two, each bounded to one path component
(ADR-0292), and a procedure is unbounded; the operator's editor is the file, which `--watch` and
this panel's own watchers already pick up. So this bay owes them nothing rather than owing them a
control it has not built — [ADR-0314](adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md).

***Keep what a deck is playing* is claimed twice**, here and inside M5.3's rule. The mock draws
`keep` in this bay's pane head, one per pane, so the control is this bay's; M5.3's paragraph and
`crates/karakuri/src/main.rs`'s *"the Library bay has no keep pill drawn"* are the other spelling.

***Set a deck's sync mode* and *Scrub a deck a quarter beat* were counted here and are M5.13's**,
dropped on 2026-09-05. Both sit at the deck head and both carry a `has` panel badge — the chip and
the scrub are drawn — so the only thing this bay held them for was the letters on their key badges
(`y`, `u i`), and under
[ADR-0259](adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
a key press is addressed to the bay that has focus rather than to a letter a bay picked, so there is
no letter here to bind.

**Exit.** No `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html). **A `gap` meets it as a `has` does**, and the two source
rows are the first use of that: the exit asks that no row of this bay is still *waiting* on a
control, and a row saying the panel is not a way in is not waiting on one.

**The grep is met as of 2026-09-09, and this section is not closed here.** The exit is a grep over
**this bay's rows**, and all ten read `has`: the last two `plan` badges on them were *Wire a
procedure's input to a node* and *Narrow the published interface*, which are this bay's now and are
built. `grep -c 'rt plan">panel' docs/manual/operations.html` does **not** return 0 — it is counting
rows added elsewhere the same day, on other bays' lists — so the whole-page reading of that command
is not this section's exit and never was. Closing the sub-milestone is the maintainer's call and
nothing here does it.

**Blocked on. Nothing, since 2026-09-09.** It was five until 2026-09-08, four, three and then one
on 2026-09-09, and the last of them — *Element capacity, seeds, the camera* — was closed the same
day by
[ADR-0328](adr/0328-the-inspectors-deck-head-steps-a-slots-capacity-and-re-salts-it.md): the
deck head grew a capacity chip that steps and a `re-salt` capsule, both re-aims on the fold's own
mechanism, and the vocabulary's `Property` lost the node address that was half the reason the row
read as blocked. Every row of this bay carries `has` or `gap`.
[ADR-0319](adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md)
took the last three together, because the sentence below said they were one decision and they were:
**what each records**. *Attach a signal to a parameter* records `Record::Source` — `Record::Bind`'s
session twin, on the terms `Record::Ride` is `Record::Param`'s — and *Take a parameter back* is the
same record with its attachment absent, which is what made the third row's *nothing to call* stop
being true: `Set::unbind` is `Set::bind`'s inverse and a take-back **removes** the attachment,
because there was never any suspended state for it to leave behind. *Set a node's authority* needed
no new record at all; what it needed was `Deck::set_authority`, and the three chips have been drawn
and unclaimed since 2026-08-29. All three are `has` on the panel.

**Two questions the rows had left open were answered on the way, and both are on the page.** A hand
on a bound parameter's fader **writes and the attachment stays** — order-independent by
construction, which is what a blend on confidence buys — so a knob cannot detach a signal by
accident; what the console does instead is draw the fader and not take hold of it, because at full
confidence the number a hand would write carries no weight and a handle that moved without moving
the picture is the one thing a handle must not be. And **an authority enforces nothing yet, which
the row says rather than implies**: nothing writes a parameter on an agent's behalf, so what a level
reaches today is the bare-name refusal — granting one renderer and keeping the other narrows what
one knob may do from the next press — and the record is kept so an agent's write can be refused
against it later.

*Write a parameter* landed with the parameter row's fader, on the route
ADR-0280 and ADR-0282 had already laid through the engine
([ADR-0286](adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)),
and it is what the sentence below about the writer was written for. Then *Composite a deck's
renderers* landed and *Element capacity, seeds, the camera* stopped being blocked at all, for a
reason that is worth reading before anything else here: **two of the bullets below were wrong, and
they were wrong in the same way**
([ADR-0314](adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
Then that row's remaining undecided third was answered and turned out to be no operation at all
([ADR-0318](adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md)): the
built-in camera's three placement numbers are parameters of its node, so the Inspector draws them
as parameter rows and the vocabulary lost an arm rather than gaining a payload.

**A layering, a capacity, a seed and the salts are not writes and never wanted a setter.** They are
fields of `watch::Aim` — the description a slot's watcher is pointed at — so a control that moves
one restates the other thirteen and sends the aim, the worker recompiles the slot off the render
thread, and the build lands at a frame boundary and is judged against the budget like an edited
file or a library load. That is ADR-0228's mechanism, unchanged and eleven days old when these
bullets were written; what made them read as blockers is that each stops one clause short of its
own answer. *The engine has no writer* is true, and the sentence it belongs to ends *because a
live write is not how material changes here*.

**There is no public route from a `&mut Deck` to a live `Set`.** Every control this panel has built
lands operation → `Record` → `apply` → a `Deck` setter, and `Deck`'s public surface has no param,
binding or authority writer. `Deck::slot` hands back a `&HotSwap` and says why — *"Deliberately not
mutable"* — and `HotSwap::live_mut` is `pub(crate)` with the same argument written out. The engine's
writers all exist and are all reachable only at a build: `Set::write_param`, `Set::set_published`,
`Set::bind`, `Set::set_authority`. **`Set::write_param` no longer is**:
[ADR-0280](adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md) put `Deck::write_param`
over it and `Record::Ride` under it, so *Write a parameter* had a route through the engine and what
the panel owed was the control. **The control is built**, so this bay is no longer the bay where an
operator turns a knob and the bay with no way to turn one. **And the hole is closed for the other
two**: `Deck::bind`, `Deck::unbind` and `Deck::set_authority` reach `live_mut` by the same road and
for the same argument — nothing renders and nothing replaces, so *which Sets is this frame made of*
has the answer it had before the call — and each was the one line ADR-0280 predicted once the record
was decided (ADR-0319). `Set::set_published` is the writer nothing has needed yet, and *Narrow the
published interface* is in *Rows the manual has not given a home* rather than here.

- ***Take a parameter back* had nothing to call, and the reason was the answer.** `Set::bind` had
  no inverse anywhere and `Binding` carried no suspended state — and the second half of that is why
  the row's own tip promised something nothing could do (*"the binding is kept and stops driving, so
  you can hand it back"*). `Set::unbind` is the inverse; there is no suspension, because *not
  driving this parameter* is already written and it is the absence — P-0084's blend writes the
  param's own value at confidence 0.0. What is given up is handing it back in one press; what
  replaces it is that the stream carries the source, the curve and the range on the line that
  attached it. Panel badge `has`, the control is the `.sens` row's last chip, and the page's promise
  is corrected rather than left standing (ADR-0319).
- ***Composite a deck's renderers* is built**, and it is the demonstration of the paragraph above
  rather than an item in this list. The mock had drawn the chip all along; what was missing was the
  press. `DeckHead::compositing` names the layering the deck is *not* in, `composited` in
  `crates/karakuri/src/main.rs` re-aims the slot beside the arm the library load uses, and the
  Staging lane carries the verdict because a composite press is a build — which matters here, since
  compositing costs a frame-sized target per renderer. Panel badge `has`, and `op-when` is *on a
  worker* where it was *launch*.
- ***Element capacity, seeds, the camera* is built, and its two questions were answered by
  precedent rather than by a new idea**
  ([ADR-0328](adr/0328-the-inspectors-deck-head-steps-a-slots-capacity-and-re-salts-it.md)). The
  engine was never what stood in the way — a capacity and the salts ride the same aim the layering
  does — and what was left was the control and the two things that decided its shape. **The
  affordance**: a capacity is a number in a declared range, which on this panel is a parameter
  row's track, and a track dragged is a **rebuild a frame** where a parameter's is a uniform write,
  so the drag is what P-0091 refuses. The answer is one bay over — a field that **steps** a closed
  list and emits the value it arrived at (ADR-0262) — so the deck head's capacity chip steps the
  powers of two inside the range the deck's geometries declare, wrapping, and `Set` grew the one
  reader that makes the offer honest: `declared_capacities`, which is the same declaration
  `capacity_in_range` refuses against. **The address**: `Aim::capacity` is one `Option<u32>` for
  the whole slot where `Property::Capacity` addressed a node, which is ADR-0228's recorded limit —
  closed by narrowing the payload to the deck it already named rather than by widening the aim,
  since no route filled the address and nothing could have honoured it. A pairing Set whose two
  geometries want two capacities is what would revive it. **The seeds** are a `re-salt` capsule
  beside the chip: it asks for the next salt in the slot's own sequence, derived by the host from
  the salt the slot is running, because a console that invented one would draw a picture no later
  run could draw again (P-0092). Panel badge `has`, `op-when` *on a worker*, and `written` answers
  `Silent(NoRecord)` on `SetCompositing`'s and `LoadSet`'s terms — a session replayed does not come
  back at a stepped capacity, and **a keep does**, since `capacity` and `seed` are Set-file
  records. **The camera is no longer the third** and is not this row's any more
  ([ADR-0318](adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md),
  2026-09-09, on the maintainer's 「Orbit の 3 値をパラメータ行として」). The two live answers were
  never two: the built-in orbit's `radius`, `speed` and `height` are parameters of the camera node,
  the engine declaring them where a `.kir` would, so *Write a parameter* is the operation, the
  Inspector draws three faders under the `orbit` group, a knob is learned against their positions,
  a `bind` drives one, a `Record::Ride` replays one and a rebuild carries one — **and not a line of
  `karakuri-console` changed**, because the pane is built from `Set::published`. `Property::Camera`
  is deleted.
- **What this bay owes next is room, and it is a measurement rather than a row.** The deck head is
  seven chips now and an Inspector pane at the console's declared minimum window holds five: the
  row needs about 296 pixels and a pane at 1244 wide gets 237.5, so the two build chips are dropped
  and the five that were there are kept. The panel opens at 1440 and they are drawn from about
  1360 up with two panes, so nothing an operator launches into is short of them — but this is the
  first control on the console that comes and goes with the window, and it is the warning under
  *Mx — TODO* arriving: **a region's declared minimum is a reading of its content, and a bay
  growing a control invalidates it.** Both manual pages say the limit rather than leaving it to be
  found by dragging.

**Three more this bay owes, and none of them is a row.** They were recorded, verified and left in
prose, which is how a bay's exit — a grep over one column — could be met while its pane drew
nothing an operator could reach. **Two of the three are now done** — the centre's declared minimum
and the pane's scroll, both below — so what is owed here is one, and it is an engine item.

- **A pane drew a node group whole or not at all, and there was no scroll position anywhere in the
  crate — done on 2026-09-08.** With the pair a bare run opens on, the first group is the L1's at
  26.5 + 19 x 22.5 = 454px against a bay minimum of 151.5, so below roughly 534px of Inspector bay
  **not one parameter row was drawn**, in the bay whose whole content is parameter rows. The
  decision in front of it — scroll, or a group that can be part-drawn — was this bay's rather than
  the arrangement's, and the maintainer took it: **the pane scrolls**
  ([ADR-0307](adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)). The
  body moves under the two heads, the wheel over a pane turns it, the head reads `n of m` for the
  groups it is showing whole, and the position is the pane's own state — clamped at the draw and
  never written back, so a resize does not lose an operator's place (P-0082, ADR-0250). **It
  answers the *reachable* half and not the whole of the item**: what is left over is the keyboard,
  which is ADR-0259's arrows walking this bay's items and is M5.13's, and until it exists the
  scroll has one route and that route is the pointer's. That is a gap on this bay's list and not a
  `plan` badge, because no row on the operations page names a scroll — it is pointer-state, like
  the Library's cursor, which ADR-0264 and ADR-0265 refuse a row for and for the reason the record
  gives: nothing outside the console could be the model of record for it.
- **`centre`'s declared minimum was the mock's CSS track and not a reading of its content, and is
  now the second — done.** 340 where the `.param` grid wants 207 in a pane before its fader has any
  width, and a divider drag reaches a 340 centre at *any* window width, so no window minimum could
  hold it (ADR-0272).
  [ADR-0279](adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)
  closed it on the axis it was on: an inspector pane declares **208** — one parameter row with a
  fader in it — and the centre declares 2 x 208 + 9 = **425**, taking `MINIMUM_VIEWPORT` to
  777 x 658.5. **It is not the 423 and 775 this entry used to name**: the fader is the `1fr` track
  and is drawn only where what is left over is positive, so at a pane of exactly 207 there is no
  fader and the figure closes nothing — the record carries the measurement. **It was the item below
  on the other axis**, and that one is still owed: the Inspector's declared minimum does not fit the
  Inspector's contents down the column, and no minimum can fix that one.
- **Adding and removing a node**, which *What no sub-milestone owns* names and which the node
  editor below is the drawing half of. **The engine half is not this bay's and should not be
  smuggled in**: `karakuri-layout`'s arena has no insert or remove and `NodeId` is a bare index, so
  a removal shifts every id anything holds. That is an engine item, and naming it here is not the
  same as this bay carrying it.

**Six of the ten are built now.** Three waited on nothing and landed on 2026-09-08; the three the
list above was written for landed on 2026-09-09 with ADR-0319 — *Attach a signal to a parameter* is
the sensitivity row's curve chip, *Take a parameter back* is the chip beside it, and *Set a node's
authority* is the three words on a node head that had been drawn and unclaimed for eleven days.
*Composite a deck's renderers* and *Choose which renderer of a deck is live* are the other two of
the six. **What is left is one row and it is `plan`**: *Element capacity, seeds, the camera*, two
thirds of one thing — a capacity control and a seed control, both waiting on a decision about the
affordance rather than on anything in the engine.

**Three of the ten waited on nothing, and all three are built, on 2026-09-08.** *Write a parameter*
is the pane's fader; *Choose which renderer of a deck is live* is a claim on the chips and the
`Record::Select` arm in `apply`; *Keep what a deck is playing* is the capsule in the pane head,
which files under a stamp because a press types no name
([ADR-0287](adr/0287-the-keep-pill-files-under-a-stamp-because-the-consoles-one-letter-taking-flow-is-an-arrangements-name.md))
— and the other half of that row, a **name** typed into the head beside it, landed the same day
([ADR-0292](adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).
**The keep was one short of what this said**: the call cannot be made from the press handler, which
has no engine, so it is a rectangle, a claim, an operation and a branch at the pointer's call site.

**One thing to know before drawing the first control, and it is not a press — and it is now done.**
A live parameter write used to be **discarded on the next rebuild**, and a rebuild is any save of
any `.kir` in that slot, because every slot is watched. `Request::params` was restated on every
rebuild from `Watch::overrides`, which only a re-point writes, so a knob walked back on the next
save and a slot loaded from a Set file — which states every declaration of every node — could not be
ridden at all.
[ADR-0282](adr/0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md)
closed it where the values already were: `Set` remembers which of them somebody stated, a rebuild
carries those across at the install and reads the rest from the code it just recompiled, and the
watcher states its aim's values on the build that aim causes and on no other. **So a parameter
control owes a writer into the Set and nothing else** — the writer into the watcher this entry used
to ask for is what was removed.

**There used to be no row on screen to press, and that was the first thing this bay's own work had
to fix.** A pane drew a node group whole or not at all and there was no scroll position anywhere in
the crate, so with the pair a bare run opens on the first group is the L1's at 26.5 + 19 x 22.5 =
454px against a bay minimum of 151.5, and below roughly 534px of Inspector bay not one parameter row
was drawn. **The pane scrolls since 2026-09-08**
([ADR-0307](adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)), so a short
bay draws part of a group rather than none of it and every row is one notch of the wheel away. What
is still owed is the keyboard's route to it, which is ADR-0259's grammar and is M5.13's rather than
this bay's — see the entry above.

**Also here.** Three of the seven *Adds* items land in this bay. The **node
editor**, whose source half is
`ReadProcedure` and `WriteProcedure` and is the two *node's source* rows above; adding
or removing a node is the part with no home, listed in the preamble. The **parameter surfaces**,
which are *Write a parameter*, *Attach a signal to a parameter* and *Take a parameter back* — the
MIDI learn half of that item is M5.12 and the tooltip it lives in is M5.11. **What each of them
addresses is a component**, since
[ADR-0268](adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md): a `param glow : vec3`
is three rows — `glow.x`, `glow.y`, `glow.z` — and the engine holds one `f32` under each. That
record closes nothing here; it is written down beside the item because the three surfaces are where
the spelling is first felt, and because **a surface can write a parameter live since 2026-09-08**, which is
what makes these three the item they are. And the **`man / sug /
auto` control**, which is *Set a node's authority*: the node head draws it since 2026-08-29 and the
writer is what is left.

**The sync toggles item is done, as far as a bay can own it.** The deck head draws both controls —
two of the three sync modes are conditional rather than one, which is what the chip has to be able
to say, and a one-off scrub shows a price rather than a disabled control ("8 bars back — 340 ms") —
and the two key badges that were the remainder of it left with the two rows above, for M5.13.

**The bay's prose, as tooltips.** Eight notes: *Inspector*, *A control that names no one node*,
*The deck head, and why it is in the inspector*, *Who is holding a control*, *A knob is bound to a
deck, not to a Set*, *Two focuses, and they do not look alike*, *Authority is per node*, and *The
camera is a node, and three of its six numbers are rows* — retitled with ADR-0318, which is what
put the rows there. Only the focus half of *Two focuses* is
this bay's; the selection half is the mixer's, above. This is the largest body of prose on the page
and the bay the mock has tipped most: the deck head, the wildcard group, the authority chips, the
renderer row and *take back* each carry one, and the parameter rows and their faders do not.

#### M5.6 — Outputs — **closed**

An output is a named destination with a size and an on/off, and the three are answered in three
different places
([ADR-0324](adr/0324-an-output-is-a-named-destination-with-a-size-and-an-on-off.md)):
`Operation::RouteFrame` names one with a word from a closed list and never a label, because a
projector's label carries the display it is on; what it writes is nothing, on a fifth `Silent`
variant saying that publishing is not the performance. The frame is composited at the largest
enabled output's size and the program view's is the rectangle the Program bay gives the picture, so
a divider drag moves what the frame is rendered at — a derivation settled by measuring the resize
rather than by arguing it
([ADR-0325](adr/0325-the-frame-follows-the-largest-enabled-output-and-a-resize-costs-0-145-ms.md)),
which leaves `CANVAS` the smaller job of the shape every output is fitted to and the size a run
starts at. The projector window is the second `Sink`
[ADR-0171](adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md) said was
owed, opened and dropped by the route operation itself. [history/m5.md](history/m5.md).

**Exit, met on 2026-09-09**: no `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html). *Choose where the frame goes* was the one row that
carried it, and ADR-0324 and ADR-0325 between them moved it.

What M5.6 left owed is rescheduled. **The `+ add output` chip is the arena's insert and is already
under *What no sub-milestone owns***, as the arena half of *Adding or removing a node*: the panel
cannot grow a bay's body while it runs, and what the chip owed was the note, which is written.
**The plugin sinks are [`docs/plugins.md`](plugins.md)'s** — `Output::Plugin(n)` exists, nothing
constructs one, and that page already says of itself that the distribution machinery is a design
rather than a description; the chips say so rather than the row omitting them. **Fullscreen on a
chosen display**, **[`docs/manual.md`](manual.md)'s *The canvas is fixed for a run*, now false for
the panel and still true for `karakuri-cli`'s `--canvas`**, and **the estimate a slot drops on every
frame of a drag**, which nothing sets today and which whoever wires the estimate meets, are under
*Mx — TODO*. The MCP route is M5.10's column and MIDI reads `gap` on the page — no chip carries a
target — so neither is a debt this bay leaves.

#### M5.7 — Staging — **closed**

A staging row is a node a build changed rather than a slot, so a save touching two files draws a row
each with the build's one verdict repeated on them; the row itself is the keep, and a `back` capsule
at its end is the put-back
([ADR-0326](adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md)). A source can say
it refused, which is what let the lane draw the disagreement an operator most wants to see instead of
printing it to stderr
([ADR-0310](adr/0310-a-source-can-say-it-refused-and-the-lane-draws-it.md)), and an over-budget
candidate now stays in the slot with the slot stopping rather than anything being put back
([ADR-0316](adr/0316-an-over-budget-candidate-stays-in-the-slot-and-the-slot-stops-updating.md)) —
so the disagreement between the picture and the disk that this lane was built to show cannot arise,
and what the lane says instead is which slot stopped and why. [history/m5.md](history/m5.md).

**Exit, met on 2026-09-09**: no `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html). Both rows read `has`, and **the exit rule's second defect
closed with them**: *Put a node's previous version back* names two controls in one badge — this
lane's capsule and the Library bay's `history` row — which is the shape M5.3 still carries in the
other direction.

What M5.7 left owed is rescheduled, and it is under *Mx — TODO*: **the watcher's other two dead ends
go unreported** — a file that will not read and a stack that will not sort into a Set both still
print and return nothing, ADR-0310 having declined to give either a word because `did not compile`
is the wrong one; and **the lane omits `origin`, the timestamp and the head's count**, the first
having no producer anywhere and the second waiting on the spelling the Library bay's own time column
waits on. **ADR-0310's other consequence — a refusal arriving while a candidate is on trial waits
for the verdict — is moot rather than owed**: ADR-0313 deleted the trial and states that the
sentence is false in both halves, and a refusal reaches the lane on the first frame boundary after
the worker sends it.

#### M5.8 — Master — **closed**

The chain is **three fixed passes in the engine** — feedback, then bloom, then rgb shift, between the
composite's write and the present pass, each with parameters the vocabulary names and none of them a
`kind` a `.kir` can declare — and *Feedback* reads **either cut** of the previous frame, `mix` as the
mixer wrote it or this chain's own `exit`, chosen by a parameter of the pass with only the chosen cut
retained
([ADR-0317](adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md), which
annotates `docs/ir-spec.md`'s paragraph on why `L5` is absent rather than overturning it). The `out`
fader at the chain's entry and the tone mapper's `exposure` at its far end stay two levels that
multiply in different places, which is why the far end is a control in the transport row and not here
([ADR-0224](adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)). An
amount of zero records no pass at all, so with all three at zero the frame is bit for bit the frame
this program drew before the chain existed. [history/m5.md](history/m5.md).

**Exit, met on 2026-09-09**: no `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html). All four rows read `has` — *Master out* since it landed
with ADR-0224, and *Feedback*, *Bloom* and *RGB shift* since ADR-0317 flipped the three together.

What M5.8 left owed is rescheduled, and none of it is a control this bay has not built. **A writable
`L5`, and the operator-written frame effects it would buy, is deferred rather than refused, and it is
a deferral on record rather than a debt**: ADR-0317 makes the case — a language change bought to draw
three rows the console already draws — and [`docs/ir-spec.md`](ir-spec.md) states what would revive
it, somebody writing the compositing down, after which this chain becomes three procedures in
`examples/` and nothing in the vocabulary or the record moves. **The bay's `+ add` is the arena's gap
and is already under *What no sub-milestone owns***, as the arena half of *Adding or removing a node*
and as *The mock's six body controls*: the panel cannot grow a bay's body while it runs, and
[the console page](manual/console.html) calls it one gap drawn wherever a control would add a region — this row, the Outputs row's `+ add output`, the library's `+` on the scope list and the inspector's `2 up`; the sequencer's `+ lane` left that list on 2026-09-09, a lane being a row from `Pattern::lanes` and not a region. What that row owed was the note, and the note is written. **What a master chain
saved under a name looks like — ADR-0227's library tier, which ADR-0317 leaves untouched — is under
*Mx — TODO***, having no row on the operations page and so never having been in this exit. **The
*decisions nobody has taken* entry on the feedback cut has left that list by being decided**, which
was that section's own pass and is done. And **two things this item carried as owed are delivered
rather than rescheduled**: the CLI's `MasterOut` and `MasterChain` replay arms, which landed with
ADR-0317 and closed a stream this program wrote and could not reproduce, and the bay's tooltip item,
which `71419c2` found delivered rather than owed and ADR-0317's pass rewrote again to say what a
press does.

#### M5.9 — Sequencer — **closed**

A pattern is one bar, a mode and a list of lanes — sixteen slots in both modes, a cell a bit, the
two levels the lane's, and four fixed banks that are not saved names
([ADR-0320](adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md));
a lane's target is an operation of the vocabulary with its value elided, which is what lets a fader
lane and a parameter lane sit in the same list
([ADR-0321](adr/0321-a-lanes-target-is-an-operation-with-its-value-elided.md)); the producer is the
render thread, polled against `beats` like a transition one row finer, live only, emitting through
the same `operate` a hand does, so its writes are its record
([ADR-0322](adr/0322-the-sequencer-is-polled-like-a-transition-live-only-and-its-writes-are-its-record.md));
a scheduled move is refused on a control a lane holds, and the refusal names the lane
([ADR-0323](adr/0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md)); the `+ lane` card
lists one deck's published keys and the bank pills are the banks themselves, so there is no `+`
([ADR-0327](adr/0327-the-lane-chooser-lists-one-decks-keys-and-the-bank-pills-are-the-four-banks.md));
and the grid head is one pill because the bar is one bar and the count follows the mode
([ADR-0306](adr/0306-the-grid-head-is-one-pill-because-the-bar-is-one-bar-and-the-count-follows-the-mode.md)).
[history/m5.md](history/m5.md).

**Exit, met on 2026-09-09**: no `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html). Two slices the same day moved every row of the bay — the
grid, the lane label and the mode pill first, then the `+ lane` card and the bank pills — from a bay
that drew nothing but its head.

What M5.9 left owed is rescheduled. **ADR-0323's refusal is still uncalled** — the lookup is built
and the caller is not, so a live lane on a deck's fader kills a fade onto that deck, on any deck the
mixer draws — and it is an engine item rather than a badge, under *Mx — TODO*. **A channel fader
does not say who is holding it**, which rule 02 asks for and which no row carries, and **removing a
lane** has no control, no row and no operation: both are under *Mx — TODO*, with the console page's
purposeful notes standing until they are drawn. **What a pattern saved under a name looks like —
ADR-0227's pattern tier — is under *Mx — TODO* beside the master chain's half of the same record**,
neither having a row on [every operation](manual/operations.html). And **the `Param` lane arm
reaches only the decks the Inspector holds a pane for**, so a target deck no pane is showing offers
its fader and no parameters, which is under *Mx — TODO* as well. The bay's five MCP badges are `gap`
rather than debt on ADR-0315's sentence; what reopens them is a model handed a view of this console,
which that record leaves to the day it happens and which is **M6 — Autonomy**'s.

#### M5.10 — MCP

**Rows.** The MCP column of every row, which M5.1 to M5.9 leave out.

**Exit.** No `plan` badge in the MCP column of [every operation](manual/operations.html).
`grep -o 'rt \(plan\|has\|gap\)">MCP' docs/manual/operations.html | sort | uniq -c` is the
count and it is not written down here.

**The forty `plan` badges are thirty-eight `has` and nine `plan`**, since
[ADR-0334](adr/0334-mcp-names-an-operation-by-the-vocabularys-own-name-and-the-frame-performs-it.md):
an eighth tool, `operate`, takes an operation of the vocabulary by its own heading, is audited by
`gate`, and is performed on the frame the panel performs a press on. **A closed class is a built
route** — ADR-0235's *"a closed class is reached and answered with a refusal"* — so thirty of the
thirty-one are refused today and reachable the moment their pill is pressed.

**What stays, and it is nine rows in three groups.** Three carry an `Undecided` payload and no
surface can say them: *Walk the edit history*, *Edit the file instead*, *Move a boundary*. Five are
named by the vocabulary and performed nowhere on the frame the drain lands on, each of them a
performer sitting in the window's own press arm or behind the event loop: *Star a Set*, *Send a Set
to somebody*, *Narrow the published interface*, *Choose where the frame goes*, *Record the
session*. **Moving a performer onto that frame is one line each**, and *Star a Set* is the
shortest — ADR-0301 already wrote what it answers a model, in a function the drain could call. The
ninth is *Set a deck's mask position*, and it is a defect rather than a scope:
`crates/karakuri`'s `reading` supplies the mask for a shape and a wipe and not for a position, so
the conversion answers *not read* and nothing moves. One arm.

***Element capacity, seeds, the camera* was a tenth and left the day it was written**, which is
what these nine are: ADR-0328 gave the Inspector's deck head two chips, `resized` and `re_salted`
went onto the frame this drain lands on, and the row went `has operate` without anything on this
surface changing. That is the shape each of the five above leaves in.

**The server exists in the instrument**, which is a change to what this sub-milestone is. `karakuri
--mcp PORT` runs `karakuri_environment::mcp::serve` against the panel's own `Opening` and publishes
seven tools, so the mechanism
([ADR-0199](adr/0199-mcp-names-its-operations-and-performs-them-itself.md)) is not only settled, it
is running, and so is the eighth tool. **What is left here is the audit's surface**, the
operations the panel does not reach being the ten above.

**The audit's surface.** `gate` is the audit and has no wildcard arm, so a new operation stops the
build until it is classed (ADR-0235, ADR-0236). The console draws four class pills. Four switches do
not cover every closed row — the clock's, the sequencer's, `Quit`, *Select a deck* and rule 06's
authority belong to no class — and an opening is map configuration rather than an operation, so
whether a press covers a class or one operation of it, whether an opening outlives a restart, and
whether an open class shuts itself are open ([the console page](manual/console.html), *What a model
is refused, and where a class opens*).

**Every write that compiles is a version, and this program keeps them now.**
`mcp::write_procedure` runs `compile::check` and writes the file, and touches nothing else; what
keeps the version it replaced is the watcher, at the compile-success point a hand at an editor and a
model's write both pass through. `crates/karakuri/src/main.rs` builds one `history::Snapshots` for
the run, seeds every slot's launch files into it before the window opens, and hands it to every
watcher — so a panel run, which is where MCP is served from, accumulates versions under
`<store>/history/` on the same terms a `--watch` run does, and what `write_procedure` already tells a
client about the version it replaced is true on this surface as well. **It belongs here rather than
in the bay that would read them**: what makes a version worth keeping is that something other than
the operator's own hands wrote it, and this is where that something is. M5.3's *Walk the edit
history* reads what this keeps. **The Set id goes in with the record** — the slot names it, so
recording without it would file a version under a node and lose which Set it belonged to, which is
the one thing the bay reading it needs
([ADR-0276](adr/0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md)):
`Snapshots::record` takes the Set the slot is running and writes it into the name, and
`watch::Watch::snapshotting_to` carries it per slot. **And in this program a load moves it**
([ADR-0304](adr/0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md)):
every slot launches on the pair the command line settled, which is no Set at all, and a library load
sends the id on the `watch::Aim` that re-points the slot, so the versions written after a load are
filed under the Set that was loaded and a rewiring restates it. `Gfx::material` is not that answer
and was never a candidate for it — that field is a *readout*, one name per slot, and at launch it is
the pair rather than any id. **The reading was M5.3's task 2 and it is built** — a fifth scope chip
in the Library bay whose rows are one Set's versions, with a landing on them
([ADR-0308](adr/0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md))
— so nothing here is owed to it.

**Blocked on.** Nothing, and that changed with ADR-0334. It read *the bays above, for the rest* —
a row whose operation the engine cannot yet perform has nothing for MCP to route to — and the bays
have closed except M5.5. What is left is nine rows, and not one of them is work on this surface:
five performers to move onto the frame the drain lands on and one reading to supply, all six in
`crates/karakuri`, and three payloads for the vocabulary to settle. MCP is still a mouth rather
than the control stick, which is why the list is short.

#### M5.11 — Hover tooltips

**Rows.** None. No operation on the page is a tooltip, so this sub-milestone closes no badge and its
exit condition is not a grep.

**What it is.** Every compact control explains itself on hover. The text was never what was
missing: the mock carries 144 `data-tip` attributes as of 2026-09-09, and M5.1 to M5.9 each end by
rewriting their bay's prose into more of them, so this sub-milestone started with its specification
written and owed only the drawing.

**Blocked on. Nothing.** The decision it waited on — who owns the pointer, whether the panel gains
`egui` widgets or paints its own hover layer — is taken:
[ADR-0330](adr/0330-the-console-paints-its-own-hover-layer-and-the-tips-are-the-manuals-own-words.md).
**The console paints its own layer**, on the two records that were already here: `input::PROBES`
already answers what is under the pointer, and ADR-0156's seam is that the console paints and the
toolkit receives a rectangle. The `egui`-widget alternative is rejected there rather than dismissed,
and what would revive it is written down — `egui` owning a control on this console for some other
reason.

**What is built.** [`karakuri-console`'s `hover`](../crates/karakuri-console/src/hover.rs): the
page is embedded and parsed at start-up, so `console.html` stays the only copy of a tip and what is
written in Rust is a **citation** per control rather than its words. The table is
`[(&str, &[Tipped]); PROBES.len()]` — a control added to that crate's own register of what the
pointer reaches arrives here as a compile error — and a row the mock is silent about carries an
empty slice, which is the reading rule above said as a value. The box is the mock's
`[data-tip]::after`, the dwell is `egui`'s own `tooltip_delay`, and the layer asks for a frame when
a tip appears or goes and for nothing at all while one is up (ADR-0283).
`crates/karakuri-console/tests/hover.rs` is where all of that is held.

**Exit, and it is not a grep.** *Every compact control explains itself on hover* is **not met
yet**, and what is left is entries rather than a decision. The compile-time half is met: every row
of `input::PROBES` has an entry, and `tests/hover.rs` fails if one loses its tips or if a citation
stops resolving against the page. What is short is inside the rows that claim several controls and are
tipped once: the Sequencer bay's five kinds of control, the four class pills, the four preview
cells, the Master bay's three effect rows, a sensitivity row's second control, and the two the deck
head gained on 2026-09-09. Each is one more `Tipped` entry carrying the sub-question the press
handler already asks — forty-seven controls are tipped today, across twenty-six of the thirty rows.

**And one clause of rule 03 is paid in the mock's tense.** *Density is bought with hover* asks a
symbol for three things — what it is, what state it is in, what a click will do — and a tip quoted
from the page answers the first and third exactly and the second in the state the mock is drawn in.
Reading the live value would mean composing the sentence rather than quoting it, which is a change
to the page rather than to the console (ADR-0330's consequences).

**What it unblocks.** M5.12's learn, which is the one thing that section says waits on this: *the
assignment lives in the control's own tooltip*, and there is now a tooltip for it to live in.

#### M5.12 — MIDI

**Rows.** One, and **it is closed as of 2026-09-10**. *Write a parameter* carried the page's only
`plan` MIDI badge and now reads `cc → param N M`
([ADR-0336](adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md)).
`grep -c 'rt plan">MIDI' docs/manual/operations.html` is 0.

**Exit.** No `plan` badge in the MIDI column of [every operation](manual/operations.html). **Met.**

**The panel takes a port and a map, as of 2026-09-10**
([ADR-0335](adr/0335-the-panel-opens-the-first-surface-there-is-and-the-map-is-two-tiers-under-the-store.md)).
It opens **the first MIDI input there is** at start-up — no flag, for ADR-0220's reason one column
along — resolves a map in two tiers (`<store>/maps/default.map`, then the `examples/surface.map`
that ships), and drains what arrives into `App::performed` beside the MCP drain, so **a mapped knob
writes the record a fader writes**. The port and the map are both fixed for the run. **That closes
what ADR-0220 left open for this column**: MIDI now measures the instrument, as the panel and key
columns do, rather than `karakuri-cli` alone — and **no badge moved**, because the grammar is one
file that both programs read.

**What the badge count leaves out.** Four things, none of which has a row. The map is migrated and
`Action` is deleted ([ADR-0196](adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)),
and what is missing is **reaching a map while running** — a map is a file saved and recalled per
controller. **The pill is drawn now and it is a readout**: it says which file is loaded, and the
`View` field, the probe row and the tip landed with learn (ADR-0336). What is left is the *menu*
under it — a press that opens *save · load · new*, and a `Surface::reload` beside
`Surface::first`. The arrangement pill is that menu over a different file and is built
([ADR-0221](adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)), which
is what makes this an entry rather than a design. **Learning does not wait on it**: a learned line
goes into `<store>/maps/default.map` and is loaded from there on the next start.

The next was **learn**, and it is built (ADR-0336). A control is bound to a deck and a position in
the Set's published interface, never to a parameter by name, and the assignment lives in the
control's own tooltip. That position is why a vector parameter publishes as `x`, `y`, `z` in that
order and never as one row — one row would be a position no control change can set
([ADR-0268](adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)). What landed with it:
the grammar's `param <deck> <position>`, resolved against the deck by
`karakuri_environment::midi::Interface` so that **both** programs read a map file the same way; the
transport row's `learn` pill as the control and `map · <name>` as a readout; the tip's `⊕ MIDI:`
line read off the live map, with the page's sentence kept where a control is unmapped; and
`<store>/maps/default.map` as where a learned line goes, whichever map the run loaded. **A learn is
a map edit and not an operation** — its operand is the pointer, a permission an actor can grant
itself is not one, and a record of a learn would put the room's wiring in the session stream, which
P-0092 is exactly about. So the vocabulary gained nothing and no row was added.

The last two are M2's, and they are the surface itself rather than a route into it:
**MIDI *out***, so a surface's LEDs and motorised faders follow the deck, which matters the moment
two things can move a fader; and **14-bit control changes**, so a fader is more than 128 positions.

**And the tip's MIDI line is the live map's, as of 2026-09-10.** Every tooltip on [the console
page](manual/console.html) ends with a `⊕ MIDI:` line and it used to be the page's own words, which
are the *mock's* assignments and no operator's — the tip confidently wrong about the one thing
somebody hovers to check. It is derived from the map in use now, with the page's sentence kept
where a control is unmapped, because that sentence carries the *reason* and no reverse lookup can.
The seam is the part that would have been got wrong: `karakuri-console` cannot read a map, since
`karakuri-midi` pulls `midir` and
[ADR-0156](adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md) is that this crate
takes no device — so the assignment is derived by `crates/karakuri` and handed to the hover layer,
beside the `View`.

**Blocked on. Nothing**, and that changed with ADR-0330. This read *M5.11, and only for learn:
there is no tooltip to learn from until the pointer decision is taken.* The decision was taken and
the layer built — `karakuri-console`'s `hover` — so there was a tooltip for an assignment to live
in, and learn now lives in it. **What is left of M5.12 is MIDI out and 14-bit control changes**,
neither of which ever waited on anything here.

#### M5.13 — The keyboard

**Rows.** The key column of [every operation](manual/operations.html), which is not this bay's list
or any bay's — `grep -c 'rt plan">key' docs/manual/operations.html` is the count and it is not
written down here.

**Exit.** No `plan` badge in the key column of [every operation](manual/operations.html).

**The maintainer has deferred the letters to this milestone and taken four cells off the page in
the meantime.** On 2026-09-08 *Scrub a deck a quarter beat* (`u i`), *Fade a deck out or in*
(`f g`), *Choose the wipe shape, the quantum, the length* (`z n j`) and *Choose which renderer of a
deck is live* (`r`) were changed to read `&mdash;`. Every letter they named was already bound in
`crates/karakuri/src/main.rs`'s key match to something else — `u` to `Op::Unsolo`, `f`/`g` to the
folds, `z` to `Op::UnfoldAll`, `r` to `Op::Reset` — and that match is flat, first arm wins, so the
`has` side took every collision and the `plan` side could never arrive. **`n` was bound to the
day/night room toggle with no row on the page claiming it at all.** The cells stay `plan` rather
than `gap` because rule 01 still owes a write a keyboard route; what they no longer do is promise a
letter that was already spent. His instruction: *global shortcuts get thought about together, and
anything doubtful comes off in the meantime.*

**The letters still promised and still free**, checked against that match on the same day: `y`,
`x`, `c`, `t`, `-`, `=`, the backtick, and `a`. **The page stopped promising any of them on
2026-09-09**, when the key column was re-spelled under item 1 below: the five rows that named a free
letter name a bay and a grammar key instead, and no `plan` badge anywhere in the column reserves a
letter now. The list stays because it is what is free to bind, which is still the question a global
key asks.

**What it is.**
[ADR-0259](adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md):
a key press is addressed to the bay that has focus, `Tab` walks the arrangement down a column and
then across, and the keys that act inside a bay are the same six everywhere because they are rules
about kinds of thing rather than about bays. A global letter survives only where the operation it
names has no operand, or where its only operand is the choice the key itself spells — seven keys of
the thirty-one this file bound when the record was written. **It binds twenty-three now**: `tab`,
`esc` and the four keys of the grammar, the rub-out, and thirteen letters. Twelve of the twenty-one
the record makes focus-relative have gone, each on its own condition — the grammar reaches the same
row in the Mixer or the Library — and the other nine wait for the bays they are in (item 2).

**Blocked on. Nothing.** The one uncertainty the record carried was whether `egui` swallows `Tab`
before the window loop's `match` sees it, which would have been the failure that looks like nothing
at all. It does not, and the reason is that `App::to_egui` reads `repaint` and nothing else —
`egui-winit` reports `consumed` for every `Tab` and this program never asks. That is now an
invariant a test holds.

**What it owes beyond the badges**, and none of it is drawing:

1. **The manual, and it is done.** ADR-0259 edited none of it on purpose; the three pages were
   written on 2026-09-08 and 2026-09-09. **`operations.html`'s key column is spelled two ways now,
   and its head says which is which**: a `has` badge names the letter this instrument binds today,
   because the keyboard it has is still one letter per operation, and a `plan` badge names a key of
   the grammar and the bay it is addressed in — `space · in the Mixer` — because a press goes to the
   bay that has focus and a key named on its own does not say what it lands on. Twenty-nine rows
   were re-spelled. **Ten were already `plan`** — the five that still promised a free letter (`y`,
   `x`, `c`, `t`, `- = \``) and the four cells taken off the page on 2026-09-08, plus the star —
   and **nineteen moved from `gap`**, which is the record's own reading of the page: they were `gap`
   for want of a route and the grammar is one. *Set a deck's mask position* is the twentieth the
   record names and it did **not** move, because the record's condition is that a row is `plan`
   *"only once its bay draws the control the key hangs on"* and that row's panel column reads `gap`.
   **No badge moved between `has` and `plan` in either direction**, so nothing about what is built
   changed and `key_column`'s six tests are green untouched — they read a `has` badge as
   whitespace-separated key names and never look at a `plan` one, which is why a bay could be
   written into the designed spelling and not into the built one. **That is what item 4 is for**:
   until both directions of the badge check are replaced, a built badge cannot carry a bay, so
   *Bring back what is folded* keeps a bare `z` with no focus spelling beside it. **The second of
   those two rows has since been settled the other way**: *Quit* read a bare `esc` when this was
   written and reads `&mdash;` since 2026-09-09, because `esc` stopped quitting rather than because
   a badge was re-spelled (item 2, ADR-0332). **The exit gets larger, on purpose**: nineteen rows that read
   `gap` now read `plan`, and the count is still the `grep` above rather than a number here.
   **`console.html` has the mark for a folded bay holding focus** — the head alone with the dashed
   ring on it, since a folded region has no rectangle — drawn beside its *Two focuses, and they do
   not look alike* note rather than inside the panel, because the panel draws every bay open and a
   second dashed ring in it would put focus in two places. That note now reads focus as *the bay a
   key press is addressed to* and the deck selection as *the Mixer bay's remembered address*, word
   for word with `concepts.html`, which is item 2 arriving in the prose before the code.
   **`concepts.html` gained *Focus* in `cdedcad`** and needed no sentence changed by any of this.
   `index.html`'s seven rules are capped and this fits inside rule 01 rather than adding one; rule
   03's reword — *when you point at it or land on it* — is still owed.
   [ADR-0331](adr/0331-the-key-column-is-spelled-two-ways-and-a-built-badge-keeps-its-bare-letter.md).
2. **The three pointers collapse, and they have.** `View::selection`, `View::cursor_row` and
   `View::scope` are three readings of one field — a bay's remembered address — which is the
   record's strongest structural result and the reason the deck selection persisting *"while your
   hands are in the library"* stops being a special property of the mixer. **The console has a
   focus pointer and `Tab` walks it**, in `karakuri-console/src/focus.rs`: the ring is derived from
   the arrangement's own tree rather than read off `view::REGIONS`, a folded bay stays in it so
   that there is something to press to open it, and the dashed `.wfocus` ring is drawn on the
   focused bay's head — on the row itself for the two bays that draw no head. **`esc` goes up one
   level and has stopped quitting**: the window's own close was already handled, so
   `event_loop.exit()` is reached from one place in `crates/karakuri/src/main.rs` where it was
   reached from two, and *Quit*'s key badge is `gap` — the one badge in the column that moved, and
   it moved because the key that reached it stopped reaching it (15 `has` / 29 `plan` / 20 `gap`).
   [ADR-0332](adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md).
   **The six keys are built in the Mixer and the Library since 2026-09-10**, which is the rest of
   this item: a digit names the nth thing one level below the address and `0` the bay's head, the
   arrows take the neighbour or the next value, `space` is the next state and `enter` the act.
   **The console resolves the address and the window loop names the operation** — the affordance is
   the surface's, so the key asks the same three cycles the chips ask, and the deck's value, the
   size of a step and the store are the host's. **Twelve letters are unbound**, each because the
   grammar reaches the same row: `0`-`3`, `[ ] \`, `; '`, `m`, `e` and `l`. `f`, `g`, `s` and `u`
   keep theirs, because they fold and solo *regions* and that is a route through bays this slice
   does not build; so does `k`, because the Library draws no *keep* control for the act to be
   addressed to. **`space` on a bay is the fold and is not bound**: it is a rule about all nine, and
   a badge naming two of them would mislead in the direction this column exists to prevent — so a
   folded bay is still in the ring with no key that acts on it. The seven bays with no grammar
   decline the four keys and say where the six are.
   [ADR-0333](adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md).
3. **`map.rs` moves to `karakuri-map`.** A global key is a complete line exactly as a `cc` is, so it
   is a third `Key` beside `Cc` and `Note`; the crate holding the layer between a surface and the
   vocabulary then stops being named for a wire. Keys reached under focus are deliberately not
   customisable, which is a property rather than a gap.
4. **`key_column`'s machinery changes shape.** `KEYS`, `ROWS` and `NO_ROW` in
   `crates/karakuri/src/main.rs` are built around one letter reaching a fixed set of rows; under
   focus a digit reaches a different row in every bay. The check reads this file's own `match` arms
   as text, so a keyboard that becomes a per-bay dispatch table is invisible to it — which is the
   thing to solve rather than to discover. **It is solved.** `ROWS` is keyed by a **(bay, key)**
   pair with the globals under no bay, `NO_ROW` with it, and both badge checks *parse* the badge —
   a key, a separator and a bay — where they matched words. `bound` still reads this file's text
   for every key that is a literal and takes the one it cannot see, the digit, from
   `karakuri_console::focus::BUILT`; a new check holds the rows written down here against the
   pairs that table declares, in both directions, so a bay whose grammar is built with no rows
   written down fails and a pair written down that the console does not declare fails. **A built
   badge can carry a bay now**, which is what ADR-0331 deferred to *"the code that binds `Tab`"*,
   and eight rows carry the grammar spelling in the built column. What is still owed is the axis:
   `&uarr;&darr;` and `&larr;&rarr;` both resolve to *the arrows*, so a badge naming the wrong pair
   passes.
   [ADR-0333](adr/0333-the-console-resolves-the-address-and-the-window-loop-names-the-operation.md).
5. **The routes closed bays have rescheduled onto it, none of which is a letter.** The Library bay
   closed owing these: the split `load` button and its pulldown
   ([ADR-0305](adr/0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)),
   a row's own menu
   ([ADR-0311](adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md)),
   the landing on a `history` row
   ([ADR-0308](adr/0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)),
   and a key that moves that bay's scroll window
   ([ADR-0312](adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md)) — all of them
   rescheduled here when that bay closed, and ADR-0305, ADR-0311 and ADR-0312 name this
   sub-milestone themselves. The sequencer's first slice drew the grid's cells, the lane labels and
   the mode pill and named a route for none of them
   ([ADR-0320](adr/0320-a-pattern-is-one-bar-of-sixteen-slots-a-lane-is-a-target-and-two-levels-and-a-cell-is-a-bit.md)
   to
   [ADR-0323](adr/0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md)), so the arrows'
   walk of a bay's items has to reach a cell of the grid and a lane. All of it is ADR-0259's focus
   grammar rather than a letter each bay picked, which is why the rows carry no letter to keep.

**And it takes `z` with it.** *Bring back what is folded* unfolds everything rather than one region
because *"a region a pointer cannot reach is a region a key has no way to name either"*, and focus
can name a folded bay. The key may still earn its place as a bulk act; its recorded reason is gone.

#### Rows the manual has not given a home

**This list is empty as of 2026-09-09**, and the two rows that were on it are the Inspector's
([ADR-0329](adr/0329-an-input-is-wired-on-the-node-that-declares-it-and-the-number-is-the-publish-mark.md)).
It is kept because the next row the manual has not placed is the next entry, and because how these
two left it is the reading the next one gets.

**They were homeless because nobody had asked where they went, not because it was hard.** The
section said *"the manual has to name a home before they can be scheduled"*, which is true and was
being read as *the manual will have to invent one*. It did not: rule 05 puts **what a deck is** in
the Inspector, and both of these are what a deck is. *Wire a procedure's input to a node* is a
node's own declared input, so the home is the node group, on a line under its head — `uses far`,
with a capsule naming the node that fills it and ADR-0305's card under that. *Narrow the published
interface* is a per-parameter decision that the parameter row already draws: the row's leftmost cell
is the control's **position** in the interface, which is what a MIDI knob counts, so the number
being there or not **is** whether it is published, and the cell is the mark. Neither needed a
control invented; both needed the page read against the mock it already had.

**The second blocker on *Narrow the published interface* dissolved the same way.** It read *"it is
upstream of the Inspector, and until something publishes, every control that bay will ever draw is a
wildcard"* — true of the *default* interface and not a blocker on the control, because narrowing is
what turns the default into an authored one. The bay is not downstream of publishing; it is where
publishing is chosen ([ADR-0100](adr/0100-a-published-interface-is-a-choice-of-attention.md)), which
is why the Inspector now draws the rows the interface leaves out rather than only the ones it
carries: a choice nobody can see is one nobody can unmake.

**What each cost, said plainly.** Wiring cost nothing in the engine: an unbound declared input is
refused where the Set is built (ADR-0152), so a *running* slot has an edge for every input it
declares, and the run's edge list filtered to the nodes a Set holds is the list of `uses` lines
exactly. Narrowing cost one reader — `Set::declared_interface`, which is the default interface
whether or not one is authored, and is what the mark is chosen *from*. Both re-aim the slot, which
is the fold's mechanism a third and fourth time (ADR-0314, ADR-0328). **Neither writes a session
record**, and the two silences differ: an edge is a Set file's record with no slot, so a keep carries
it; a published interface has no record in this format at all, so a narrowing survives neither a
replay nor a keep. That is a gap on both manual pages rather than a decision.

**A third was here and it has been built.** *Walk the edit history* left this list on the reading
that it is a listing and a load rather than undo, became M5.3's, and landed on 2026-09-08 as a fifth
scope chip in the Library bay with a landing on its rows
([ADR-0308](adr/0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)).
The half-finished page edit this section used to describe — the row in *The library*'s group with
its panel cell and tip still unmoved — is finished: the badge reads `library history chip`.

**This list held three, and the third left it by being read again rather than by being decided.**
*Walk the edit history* was here because the operations page calls the surface over those files
*undo* and *a later milestone*, and undo is nobody's bay. It is not undo. A Set's history is a list
of the versions it has had, walking it is a listing, and landing on one is a load — all three of
which the Library bay already does — so the row became M5.3's. **What it needed was a lister**:
`karakuri_environment::history::list`. **What it needed next was a history to read, and a panel run
writes one** — one `Snapshots` for the run, seeded before the window and handed to every slot's
watcher, with the Set a version is filed under moved by a library load. That was M5.10's, and
neither the Library bay's task 2 — now in [history/m5.md](history/m5.md), that sub-milestone having
closed — nor this paragraph said the walk depended on it. **A blocker that names a milestone rather
than a mechanism is a blocker nobody can check**: this one survived two readings of
this file before the question *what actually stops it* was asked of it, and naming the mechanism is
what got it built. **The page then named the control and the bay drew it**, which is the whole of
the distance between *homeless* and *built* and took three records to cross.

#### The console's own shape — **closed**

Four rows have a panel home that is not a bay — *Fold a bay away* at the bay head, *Fold a pane
away* at the pane edge, *Size the window* at a drag and *Quit* at the window's own close — so no
M5.x above owns them and ADR-0226 does not close without them. A pane folds by dragging its boundary
out and comes back by dragging it in, which is why that row needed no probe, no claim and no press
arm
([ADR-0300](adr/0300-a-pane-folds-by-dragging-its-boundary-out-and-comes-back-by-dragging-it-in.md)),
while a bay's grip was painted and never hit-tested and now is. A model has no window, so the
surface rows' MCP badges are `gap` rather than debt, *Quit*'s route is the window's own close with no
`close` control drawn on the panel, and `a` is bound to nothing so *Size the window*'s key badge is
`gap` too
([ADR-0315](adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)). The
window declares a minimum, the declared minima summed along each axis
([ADR-0272](adr/0272-the-window-has-a-minimum-and-only-one-of-adr-0250s-three-cases-is-real.md)),
with the centre's term set by what a pane needs before its fader has any width
([ADR-0279](adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)).
[history/m5.md](history/m5.md).

**Exit, met on 2026-09-09**: ADR-0226's grep read over this section's four rows — no `plan` badge in
the panel column of *Fold a bay away*, *Fold a pane away*, *Size the window* and *Quit* on
[every operation](manual/operations.html). The first two read `has` at the bay head and the pane
edge; the last two read `gap`, which is finished because the page has already said the panel cannot
reach them. ***Size the window* was met by M5.6 rather than by this section**: a window drag sets an
output's size (ADR-0325), and the fifth cell carries `window drag` the way *Quit*'s carries `window
close`.

What this section left owed is rescheduled. **`esc` still quits, and what a global letter is under
ADR-0259's grammar is M5.13's** — that row's key badge reads `has` and stays there until the one
pass over the keyboard takes it. **The window minimum is a measurement a bay growing a control
invalidates**: a region's declared minimum is a reading of its content, and `centre`'s first number
was a CSS track rather than a reading, which is the warning; it is under *Mx — TODO*. **Three of the
seven headed regions carry no grip**, so the mixer, the staging lane and the sequencer fold from the
keyboard alone, and whether that is the exit met or a page that has to change is not this file's to
say. ***Move a boundary*'s MCP badge stays `plan`**, because a row filed under `Owed(Undecided)` is
not in the arm ADR-0315 took the twelve off, and it moves the day somebody settles the payload.

#### M5.14 — The frame's cost — **closed**

A renderer's floor is bounded from its declared ranges or refused
([ADR-0285](adr/0285-a-renderers-floor-is-bounded-from-its-declared-ranges-or-refused.md)), and a
rung may sit under the floor because what it hides is bounded and paid for
([ADR-0293](adr/0293-a-rung-may-sit-under-the-floor-because-what-it-hides-is-bounded-and-paid.md));
a frame's cost is the period, and a measurement names which resolution it is about
([ADR-0303](adr/0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md));
the governor budgets on the estimate where it answers and on the measurement where it does not,
which changes admissions rather than only reports
([ADR-0296](adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md));
the badge is the band of the number the governor spent
([ADR-0298](adr/0298-the-badge-is-the-band-of-the-number-the-governor-spent-and-how-it-was-taken-crosses-the-seam-undrawn.md));
and a region declares when its picture next changes rather than that something is pending
([ADR-0283](adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)).
It exists because the risk badge M5.1 left undrawn was hooked from *Performance discipline*, which
closes nothing. [history/m5.md](history/m5.md).

**Exit, met on 2026-09-08**, and it is deliberately not a badge grep: the panel draws a dot whose
band a test can predict from a measurement, held from both sides — the engine's, which fails when
the number stops arriving, and the console's, which predicts a band from a number. This
sub-milestone has no column and no row of [every operation](manual/operations.html), which is why no
exit of the shape M5.1 to M5.9 use could hold it; its precedent is M5.11.

What M5.14 left owed is rescheduled, and all of it is under *Mx — TODO*: **item 3 —
`committed_ms` as the deck's total and what `over_budget` then says**, now with
[ADR-0313](adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)'s
`Report::deck_over_period` beside it as a second flag rather than as an answer to it; **what is left
of item 6, caching a bay to a texture**, whose precondition and whose measurement are both built;
and **the badge's estimated/measured mark**, which is what mark the page gains and is the
maintainer's.

#### M5.15 — Procedures in the library

**Rows.** Four, all new and all written on 2026-09-10:
*Filter the library by kind* and *Load a procedure over a layer* under **The library**, and
*Keep a node's procedure* and *Point an Inspector pane at a deck* under **Inside a Set**
([every operation](manual/operations.html)).

**Exit.** No `plan` badge in the **panel** column of those four rows. That is M5.1 to M5.9's grep
narrowed to a list of rows rather than to a bay, because the four sit in two bays — the Library and
the Inspector — and neither bay's own column is what this sub-milestone is. The key, MIDI and MCP
columns are out of scope for the reason they are there, being M5.13's, M5.12's and M5.10's: three of
the four carry a `plan` MCP badge and are M5.10's owed list rather than this exit's.

**What it is.**
[ADR-0338](adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md),
from the maintainer's request of 2026-09-10. A procedure is a row of the library in the two tiers
every other kind of library data is in — what ships under `examples/`, the operator's under
`<store>/procedures/` — and a badge at the right of a row's name says which layer it implements: one
kind for a procedure, the layers its slots fill for a Set. Six toggles say which kinds show, OR
across the ones that are on. **A procedure loaded over a layer re-aims the slot with that one file
replaced and every other field restated**, which is ADR-0314's derivation and ADR-0228's path, and
what the deck plays afterwards is a **derived Set with no name**: the strip reads `base + kir`,
`keep` is what names it, and its versions go on being filed under the Set it started from so the
snapshot every compile takes stays alive. The Inspector keeps one node's source into the operator's
tier, and a pane is pointed at a deck by a pulldown per pane, A–D.

**Blocked on. Nothing.** Both passes are work in files that exist, against decisions that record
settles, and neither waits on the other: the Inspector pass writes the tier the Library pass lists,
and an empty `<store>/procedures/` is a listing with nothing in it rather than a broken one — the
presets tier fills the list from the first run.

**The two implementation passes, named by the files they touch.**

1. **The Library and host pass.** `crates/karakuri-store/src/store.rs` — a `procedures/` directory
   established by `Store::open` the way `arrangements/` is, and the listing that reads it.
   `crates/karakuri-environment/src/places.rs` — the presets root's `.kir` files listed beside its
   `.kset` files, each carrying the kind `history::declared_kind` scans off it.
   `crates/karakuri-console/src/view.rs` — the row kind and its badge, the `.lib-kinds` row and its
   six chips, and `Field::Layer` retired with the `layer…` field.
   `crates/karakuri-console/src/room.rs` and `src/input.rs` — the new row's arithmetic and its
   claim. `crates/karakuri-console/src/hover.rs` — a `TIPS` entry per chip and per badge, and the
   `layer field` entry deleted with the field it cites.
   `crates/karakuri/src/main.rs` — `listing` handing the two new populations across the seam, and an
   arm beside `played` that re-aims a slot with one file replaced and leaves `Aim::set` where it is.
   `crates/karakuri-environment/src/mcp.rs` — `SPELLED`'s `Load a procedure over a layer` row given
   a `make` once the frame performs it.
2. **The Inspector pass.** `crates/karakuri-console/src/view.rs` — the `keep` capsule on a node
   group's head, the pane head's pulldown, and the per-pane target beside `InspectorPane`'s scroll
   position. `crates/karakuri-console/src/room.rs` and `src/input.rs` — both controls' rectangles
   and their claims. `crates/karakuri-console/src/hover.rs` — a `TIPS` entry for each.
   `crates/karakuri/src/main.rs` — the writer that puts a node's source under
   `<store>/procedures/`, and the sandbox arm for a model's.
   `crates/karakuri-environment/src/mcp.rs` — the same for `Keep a node's procedure`.

**What it owes beyond the badges.**

1. **The `layer…` filter field goes when the chips land.** The page supersedes it now and still
   draws it, because the panel still draws it — *a control the panel draws that the mock does not is
   a defect* (`docs/contributing.md` §5) — so the field stays on both until one pass takes it off
   both. Its tip says which of the two it is, and `hover::TIPS`' `layer field` entry goes with it.
   `Operation::ListSets` keeps its `layer` field either way: `list_sets` and `--list-sets` are where
   *which Sets hold a node on this layer* lives once the panel stops asking it.
2. **Two limits are recorded and not fixed**, and either is a later record rather than a defect. A
   procedure load lands on **node 0** of its kind, so the second renderer of a three-renderer Set is
   unreachable from a library row; and a `.kir` in a folder somebody dropped on the window is **not**
   a row, because a folder row is a take and nothing takes in a bare `.kir`.
3. **Listing procedures over MCP is nobody's yet.** *Filter the library by kind* is `gap` because a
   model has no window, and the question a model actually has — *what procedures does this store
   hold* — is *List what the store holds*' to grow. It is written here because a `gap` standing in
   front of a real gap is what this list exists to stop.
4. **A node head with nothing to keep draws no capsule**, and there are two of those. A head
   standing over several nodes carries none, which is *Set a node's authority*'s own rule on the
   same head; and the **built-in camera** carries none, because it is a node with no procedure
   behind it and there is no source to write. The mock draws both absences, and the pass reproduces
   them rather than drawing a capsule that refuses.

#### M5.16 — L5, the writable frame effect

**`kind L5` joins the language, the master chain becomes an ordered list of L5 slots, the three
fixed passes ship as procedures under `examples/`, and a Library row is dropped onto the chain.**
The maintainer's, on 2026-09-10, asked whether — with M5.15 making a procedure a row that can be
loaded over a layer — an L5 could be dragged from the Library onto the master chain:
*進める（M5.15 の後）*, **proceed, after M5.15**.
[ADR-0340](adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md) is the
record and [ir-spec.md](ir-spec.md)'s *L5, written — not built* is the specification, which moves
first for a file format the way the operations page moves first for an operation
(`docs/contributing.md` §6). **This meets
[ADR-0317](adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md)'s own
revival condition** — *somebody writing the compositing down* — which that record wrote into its
own rejection, so the deferral is taken up rather than reversed.

**Rows**, to be named on [every operation](manual/operations.html) by M5.16's first pass, which is
where the work starts (`docs/contributing.md` §5): **three retire** — *Feedback*, *Bloom* and
*RGB shift*, which name passes that are always in the chain — and **three arrive**: *Set a chain
effect's parameter*, addressed by position in the chain and parameter key, *Add an effect to the
master chain* and *Remove an effect from the master chain*. That is
[ADR-0192](adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)'s
rule read once more — a surface asking for *feedback* is asking for a pass that may not be in the
chain, and what a surface can say is *this slot's this parameter*. **Reordering a chain has no
control in the mock**, and this entry names it as what the page will owe rather than deciding it.

**Exit: no `plan` badge in the panel column of the Master bay's rows** — M5.8's own exit, met a
second time, and this sub-milestone exists partly because it has to be. M5.8 closed on that grep
with three rows reading `has`; retiring them for three that start at `plan` turns it red, so the
page and the panel land together here or a closed sub-milestone reopens with nobody owning it.

**Blocked on M5.15 and on nothing else.** The library row, the `L5` badge, the kind filter's
seventh toggle and the drag are
[ADR-0338](adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md)'s
and none is worth building twice. **Half of that record's load transfers literally and half does
not**: the re-aim writes one file over one layer of a deck's material, which is exactly right for a
**nested** L5 at `L5:0` of a Set and wrong for the master chain, which is no deck's material and
which no aim carries — a drop there appends a chain slot and writes `Record::MasterChain`.

**The passes, by file** (split by file rather than by phase,
[ADR-0115](adr/0115-split-work-by-file-not-by-phase.md)):

1. **IR and codegen.** `karakuri-ir`: `Kind::L5`, the `frame` block, the `retains` declaration,
   `uses … : Texture`, and `texel` / `tap` / `frame_step` in `Builtin::ALL` — which is also how
   they reach a model, `mcp.rs` serving the checker's own table rather than a second copy. Every
   refusal in the specification with the case that must be accepted beside it
   (`docs/contributing.md` §3). `cost.rs` prices an L5 on `ops_per_fragment` alone against the
   fullscreen ceiling, which means `fragment_ceiling` — today a test on `topology == Fullscreen`,
   which an L5 does not declare — switches on the kind as well. `karakuri-codegen`: `l5.rs`, and
   the three shipped procedures through `naga_test.rs`, which is not optional for that crate.
2. **The engine's chain as slots.** `master.rs` from three fixed passes to a list, `master.wgsl`
   retired with them, `present.rs`'s targets two plus one per retained cut instead of four, and the
   retained frame sanitised where it is copied instead of where it is read.
3. **The record and the vocabulary.** `Record::MasterChain` from four scalars to a list of slots
   written whole; `mix::change`'s arm; `karakuri-cli`'s replay driver resolving each slot's address
   against the store before the first frame; three operations out and three in, `gate.rs` classing
   them with no wildcard arm and `karakuri-operation-record` saying what each writes; and a refusal
   naming the four-field record rather than defaulting past it, which here would play an empty
   chain where the session had three passes.
4. **Console and host.** The chain's rows as a third set of rectangles a release can land on
   ([ADR-0273](adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md), and the mark
   says *where* and never *whether*), the Master bay's `+ add` appending a slot, and the bay's
   tooltips rewritten to say what a chain is now.
5. **The three shipped effects.** `examples/feedback.kir`, `examples/bloom.kir` and
   `examples/rgb_shift.kir`, in the presets tier nothing in this program writes (P-0096). Two are
   the hand-written bodies term for term; **`bloom.kir` is not**, because a chain slot cannot name
   the intermediate target a separable blur needs, so it is one 9×9 pass at **81 fetches per texel
   against the pair's 19** — same radius, same picture, different price. **This pass owes a fresh
   measurement**: ADR-0317's 0.80 ms was taken on the two-pass form and does not describe this one.

**The Master bay's `+ add` leaves the arena's gap list**, and for the Sequencer's reason rather
than a new one: `+ lane` left it on 2026-09-09 because a lane is a row from `Pattern::lanes` and
not a region, and a chain slot is a row from the chain. `karakuri-layout`'s arena still has no
insert and no remove, and nothing here needs one.

**What this does not do**, each named so it is met as a decision rather than as a surprise: **a
per-deck chain** — [architecture.md](architecture.md)'s *a per-deck effect and a master effect
become one node at different points* is what the kind makes possible, and there is one chain, at
the master; **reordering**, above; and **the library tier of a chain saved under a name**, which is
[ADR-0227](adr/0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md)'s and
sits under *Mx — TODO* where that record left it — ADR-0340 says what the file holds (the record's
slot list and nothing else) and leaves the directory's name there.

**And two sentences of [architecture.md](architecture.md) go wrong the day this lands**, both about
a count rather than about the layer: *"There are five"* under *Words that carry more than one
sense*, with `karakuri_store::record::Layer` and `karakuri_operation::Layer` named beside it, and
[ir-spec.md](ir-spec.md)'s *A note on the word* saying it from the other side. Correcting them is
this sub-milestone's, not a note to leave behind it.

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
  [P-0094](principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) is
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

- **The mixer's mask has no front position, and no column asks for one.** *Set a deck's mask
  position* carries no badge in either the panel or the key column, so no sub-milestone's exit reads
  it, and it left M5.2 by being unreadable rather than by being done. What it wants is a surface
  that gives the mask's front a position —
  [ADR-0206](adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md) is the record.
  **It is here for the reason the risk badge should have been**: an unfinished task under a closed
  milestone is never picked up, and that is exactly what happened to the badge between 2026-09-03
  and 2026-09-08.

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
  something generates procedures; the visual side needs the thumbnail below, which M5.3 closed
  without judging. From M4.
- **Search over more than a node's name, and grouping.** The same absent card fields. From M4.
- **MCP offers the effective limit beside the declared one.** A model writing a procedure is told
  what the *language* permits — the ceiling `cost::estimate` refuses at — and never what this
  machine has actually been able to afford. The second number is inferable from what has run: the
  preparation slot's measurements and the governor's history. The two do not compare directly, a
  ceiling being in ops and a measurement in milliseconds, so what a model would be handed is closer
  to a rate than to a limit and the shape of it is not settled here. **Polishing rather than a
  feature**, and it wants two things first: the estimate *The preparation slot is the measurement*
  builds, and a run long enough to have a history worth quoting. From M5.

- **An artifact card states no default for a vector param.** `meta.rs` writes
  `default: p.default_scalar()`, which is `None` for a `vec3` since ADR-0268 gave the engine three
  keys and three numbers. The card and the run cannot disagree — both folds share one literal
  reader — but the card is silent where the run is not, and the Library's reading draws that field.
  What it waits on is a decision the metadata format owns: whether `Record::ParamDecl`'s `default`
  grows an array, or a card carries one record per component. **M6 is what makes it matter** — an
  agent with no declared ranges has nothing to search over. From M4's three card fields, which are
  the same absence.

- **A Set installed part-way through a session starts at `t = 0` against a clock that has run.**
  So material reading `beats` jumps when that slot goes on air, which is the last live case of the
  lag correction and a preview-honesty defect of exactly the kind ADR-0258 and ADR-0269 exist for.
  Recorded in ADR-0269 and hooked from nowhere until now; the bay that would have owned it, M5.1,
  is closed. From M5.

- **Whether `l` loads onto the deck selection or onto the load pulldown's deck.** ADR-0305 split
  `load → A` into a button and a pulldown and left the key on the selection, saying outright that
  this is its one assumption and the maintainer's to confirm — nothing in the manual, in this file
  or in ADR-0228 says which mark the key should read. Changing it is a line in
  `crates/karakuri/src/main.rs` and a paragraph on two pages; leaving it takes the deck selection's
  one reason for existing away from the row that has always read it. From M5.3.

- **Thumbnails and live previews in the Set browser, and what a thumbnail is *of*.** The drawing was
  the Library bay's and the judgement was M4's, handed to that bay. Neither has a row on
  [every operation](manual/operations.html), so no exit condition read either and the bay closed
  without them; the visual half of dual embeddings above waits on the same absence. From M5.3, and
  from M4 before it.

- **What the Library bay's filter fields should be.** ADR-0262 refused to invent a text-entry flow
  there rather than on the merits and deferred it to a console-wide decision about focus; ADR-0292
  took that decision, so the deferral is spent and the question is open rather than closed. It is
  not the same question the two built flows answer: both are bounded to one path component, and a
  filter is an unbounded fragment matched against what a node is called. ADR-0262's stepping stands
  as the first cut until somebody answers it. From M5.3.

- **`written(TapBeat)` and `written(ScaleGrid)` answer `Owed(NotSettled)`.** A gap in
  `karakuri-operation-record` rather than in the panel: it cost one early return in `App::performed`
  and the record that argues for it (ADR-0278), and both controls are built. Closing it means saying
  what a `Current` carries about a beat lock. From M5.4.

- **Which number the transport's frame readout carries.** The numerator is `Cost::whole`, the CPU's
  three stretches, while `Cost::period` is the frame (ADR-0303) — so a frame the GPU is holding up
  draws as headroom, which is the one case the readout exists for. The period in place of the CPU
  figure, both numbers side by side, or the tooltip's promise rewritten: all three are page changes
  before they are code and none has been chosen. From M5.4.

- **The transport's own notes, read against the tips its controls now carry.** Every control in that
  row carries a `data-tip`, the last of them written the day the bay's controls landed. What is left
  is the reading rather than a drawing — that bay's notes against what the tips now say. From M5.4.

- **What `over_budget` says once `committed_ms` is the deck's total.** `committed_ms` and
  `headroom_ms` sum only Live slots, so since ADR-0269 they understate the deck by three slots of
  step and draw and a four-slot frame registers as no violation at all; making `committed_ms` the
  total would leave `over_budget` permanently on for any four-slot deck of heavy material. ADR-0313
  put `Report::deck_over_period` beside it as a **second** flag rather than as an answer, and made a
  slot's own number what decides its verdict — so a deck can be over its period with every candidate
  kept on its own merits, and that is the state the ruling has to speak to. From M5.14.

- **Caching a bay to a texture.** ADR-0283 is its precondition and it is built; ADR-0303's frame
  instrument is what would measure it. From M5.14.

- **Whether the risk badge marks an estimated number differently from a measured one.** The dot is
  the band of the number the governor spent, and whether that number was estimated or measured
  crosses the seam on `Decision::basis` and is drawn nowhere, because the mock does not distinguish
  them and the page moves first (ADR-0298). P-0095 rules out handing an inferred number out as a
  measured one, so what is open is what mark the page gains, and that is the maintainer's. From
  M5.14.

- **The extrapolation is wrong on a concentrated Set.** `estimate` measures how much coverage a Set
  has and not how concentrated it is, so it is right at the sizes it measures and wrong
  extrapolating a concentrated Set to full size. The case is *What a machine's size is allowed to
  decide*'s, which names it and adds no limit for it, so what is owed here is what the estimate says
  about a Set it cannot extrapolate: a refusal on ADR-0266's terms, or a fit that reads
  concentration as well as coverage. That section pointed it at M5.14, which closed without it ever
  having been one of that sub-milestone's items. From M5.14.

- **What a pattern or a master chain saved under a name looks like.**
  [ADR-0227](adr/0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md) puts
  the two in the library as two tiers, and both were built without touching that half: ADR-0317 adds
  no store directory for the chain, and `crates/karakuri-pattern` ships with no serialiser, ADR-0320
  fixing banks that are positions in the session rather than saved names. Neither tier carries a row
  on [every operation](manual/operations.html), so no exit condition in this file reads either, and
  the shape of a saved one is for the record that has something to serialise — which both now have.
  From M5.8 and M5.9.

- **Fullscreen on a chosen display.** The projector window is a second `winit` window with a surface
  and a `WindowSink`, and it is neither made fullscreen nor put on a named monitor: that is one line
  of `winit` and one list of monitors this program does not read, and a fullscreen with no way to say
  *where* is worse than none. It closes no badge — the row it belongs to reads `has` — so nothing
  reads it. From M5.6.

- **[`docs/manual.md`](manual.md)'s *The canvas is fixed for a run* is false for the panel.** The
  render size belongs to the output and the frame follows the largest enabled one (ADR-0325), so the
  sentence holds only for `karakuri-cli`, whose `--canvas` is untouched. ADR-0246 predicted it would
  go false the day this was built and ADR-0325 names it without closing it. It is a page change on
  the CLI's own page. From M5.6.

- **A slot's estimate is dropped on every frame of a drag**, because `HotSwap::resize` drops it and
  the frame is re-derived per changed output size. Nothing sets an estimate today —
  `set_estimated_cost` has no caller — so it costs nothing now, and whoever wires the estimate meets
  it here rather than finding it later. From M5.6.

- **The watcher's other two dead ends go unreported.** `Source::poll` can answer *I refused* and the
  Staging lane draws it, but only for the checker's refusal: the two other paths in `Watch::rebuild`
  that answer nothing — a file that will not read, and a stack that will not sort into a Set — still
  print and return `None`.
  [ADR-0310](adr/0310-a-source-can-say-it-refused-and-the-lane-draws-it.md) names them and declines
  to give them a word, because `did not compile` is the wrong one for either and naming them is a
  further decision. Neither has a row on [every operation](manual/operations.html). From M5.7.

- **The Staging lane omits `origin`, the timestamp and the head's count.** Each is named at the code
  and none of them is a row on the page. The first has no producer anywhere; the second waits on the
  spelling the Library bay's own time column waits on, which is one decision that bay declined to
  take twice. From M5.7.

- **ADR-0323's refusal is built and uncalled, so a live lane still kills a fade onto its deck.** A
  lane reaches `Deck::set_opacity`, which cancels, and
  [ADR-0323](adr/0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md) decides that the
  schedule is refused instead and that the refusal names the lane. The lookup exists and the caller
  does not, on any deck the mixer draws rather than only the one the run seeds. It is engine-facing
  rather than a badge, which is why no exit condition reads it. From M5.9.

- **A channel fader does not say who is holding it.** The mock draws `seq 1` as a source on a
  parameter row and nothing of the kind on a strip, so a fader lane drives a control that says
  nothing about what is driving it, which is rule 02 of [the seven](manual/index.html). It has no row
  of its own, and [the console page](manual/console.html) carries a purposeful note until the readout
  is drawn. From M5.9.

- **Removing a lane has no control, no row and no operation.** `+ lane` adds one and nothing takes
  one away; the tip beside it says so rather than the bay pretending otherwise. From M5.9.

- **The `Param` lane arm reaches only the decks the Inspector holds a pane for.** The published rows
  come from `View::inspector`, which is pointed at two decks, so a target deck no pane is showing
  offers its fader and no parameters. What the console has not been handed it does not offer, which
  is the right behaviour and the wrong reach. From M5.9.

- **The window's minimum is a measurement, and a bay that grows a control invalidates it.**
  `MINIMUM_VIEWPORT` is the declared minima summed along each axis (ADR-0272), a test recomputes it
  from the tree, and what the test cannot check is whether a region's *declared* minimum is still a
  reading of its content. `centre`'s first number was a CSS track rather than a reading, and at it
  the parameter faders were not drawn (ADR-0279) — so every bay still growing controls, M5.5 above
  all, owes that reading again. From *The console's own shape*.

---

## The handover

**Read this if you are picking the work up.** It says where to start, what that piece needs, and
what nobody has decided. Everything else is in the sub-milestone above, in the code, or in a record.

### Where to start: M5.5 — Inspector

**The work starts at M5.5 — Inspector.** The order is this file's own, top to bottom, and the
Inspector is the first sub-milestone above still open — **and the only bay still open**, every one
below it having closed on or before 2026-09-09, *The console's own shape* with them. What follows it
is the cross-cutting run, M5.10 to M5.13, which is a column of [every
operation](manual/operations.html) apiece rather than a bay. Each entry names its rows, its exit
condition and what it is blocked on, and is the one place those are written down; what a closed
sub-milestone left owed is under *Mx — TODO* and in [history/m5.md](history/m5.md).

**The panel is a program.** `cargo run -p karakuri` opens the console over a real deck: the
picture, four deck previews, the transport, the mixer, the Library bay, the Inspector, the
Staging lane and the Master fader. It is [`crates/karakuri`](../crates/karakuri), a binary of its
own since ADR-0214; what it needs that is not a surface is
[`karakuri-environment`](../crates/karakuri-environment)'s (ADR-0215), and
`crates/karakuri-cli/src/` is `main.rs` alone. `--presets DIR` and `--store DIR` name where the
data lives, and with neither given a four-candidate search runs (ADR-0230). `examples/` ships the
`.kset` files a fresh clone opens on; `my sets` is empty until something is saved.

**M5.1 was the current sub-milestone until it closed on 2026-09-03, and what it settled is below.**
The Program bay carries no `plan` panel or key badges on
[every operation](manual/operations.html). The preview sizing and arrangement
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
which also retired P-0070 and re-recorded the requirement as P-0080,
*An operator can see a slot's own material without putting it on air* — itself retired on
2026-09-05, because every question it decided was inside auditioning and it had to name the crate it
bound. The requirement is stated in
[ADR-0258](adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md).

**Residency does not gate a cell, and the two gaps that paragraph used to name are gone.** `Deck`
draws every slot into its own target on every frame, whatever its residency, and the field that
chose one slot is deleted (`crates/karakuri-engine/src/deck.rs`). `Engine::aim` asks whether there
is a slot behind the cell — `slot_bind_groups[slot]`, which is `None` past `slot_count` — and never
what that slot's residency is (`crates/karakuri/src/main.rs`). That closes the second gap. The
first, that `karakuri-cli` cannot audition a slot, is not a gap:
[ADR-0242](adr/0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)
makes the command line test tooling and scopes it out of the instrument's principles, so a rule
written for the person playing the instrument does not reach it. **The prose that narrated the old behaviour is corrected**, on 2026-09-02: the *Three of the four
preview cells are off at a time* paragraph is gone, the doc on `ON_AIR` reads *"every cell draws its
own slot whatever its residency"*, and the deck's fullness doc says each resting slot is *"still
drawn into its own cell"*. `docs/contributing.md` §4 is met here, and this paragraph said otherwise until 2026-09-03.
**The clause
[ADR-0258](adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
left owed is written.** It was the rejected build's cell — black, or the sentence saying what went
wrong — which [the console page](manual/console.html) specified under *What a deck preview cell
shows, and when* and said of *"neither can happen yet, so nothing draws either word"*. **Half of
that reason was wrong and the conclusion was not.** The panel watches every slot's `.kir` pair, so a
build can be refused while a slot is running and the Staging lane draws that verdict — on the word
`did not compile`, carrying the first diagnostic
([ADR-0310](adr/0310-a-source-can-say-it-refused-and-the-lane-draws-it.md)). What cannot happen is
the *cell* that page described: a refused build installs nothing, a failed compile makes no
`Request`, and the honest picture after a rejection is the material that is still there, which
`crates/karakuri-engine/tests/deck.rs`'s `a_rejected_build_shows_what_is_still_running` asserts.
**What a cell does say is `overloaded`**, and that is the state that arrived in place of the one the
page described: an over-budget candidate stays in the slot and the slot stops updating, so the image
is the last frame it drew and the caption marks it, because a still nothing marks is a preview that
lies
([ADR-0316](adr/0316-an-over-budget-candidate-stays-in-the-slot-and-the-slot-stops-updating.md)).
The page is written to that, and the state it still names as unable to happen is `empty`. **The
question under the owed sentence went the way ADR-0258 offered**: a cell carries no diagnostic, and
saying what went wrong stays the Staging lane's.

**A cell now shows material *running*, which is the other half of the same requirement and was
missing until 2026-09-07.** Every slot was drawn and only some were stepped, so a cell for an
off-air slot showed the still it stopped at — or, for a slot that had been off air since the window
opened, near-black. **A still is not what the operator is deciding from**, and a cell off the room's
tempo is not a preview of what putting that slot on air would look like, so every drawn slot is now
stepped on every frame at the session's tempo:
[ADR-0269](adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md). That
retires the governor's priming rate, makes `Residency::Allocated` mean *off air and asked of
nothing* rather than *at rest*, and takes the *cold slot shows black* half of the test above with it
— no deck can show a never-stepped Set any more. **What it leaves owed is one number**, and it is
under *Performance discipline* below rather than here.

**The panel serves MCP.** `karakuri --mcp PORT` binds `127.0.0.1:PORT` and runs
`karakuri_environment::mcp::serve` with the same `Opening` the four bay-head pills write, so a pill
opens a class the server reads on the next call. Seven tools: `read_procedure`, `write_procedure`,
`wire_input`, `swap_outcome`, `save_set`, `read_set` and `list_sets`. The last three came with a
**save path this program did not have** — `Live::save_set`, which is the one place a Set is written
here and where the `k` key, the MCP tool and whatever control the Library bay grows all end. **What
that one path now differs on is one argument**: a save asked for over MCP lands in
`<store>/sandbox/` and the operator's own act writes `<store>/sets/`
([ADR-0261](adr/0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md)),
which leaves the Library bay's planned *keep* pill an operator's act and owing nothing new, and
leaves a model unable to read back what it saved — `read_set` and `list_sets` read the library. The
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

**The order is the sub-milestone list above.** Work them top to bottom; each one names its own
rows, its exit condition and its blockers. Run the fourth command beside them: a `plan` badge does
not distinguish a drawing that is owed from a machine that is missing, and a row has already been
picked up as the first while being the second. **Master and Sequencer were last for machinery rather
than for drawing, and both have closed** — the chain is three fixed passes in the engine (ADR-0317)
and the sequencer's producer is the frame loop polled against `beats` (ADR-0322), which is what that
ordering was waiting on.

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

**What it did not touch is the canvas, and it is what unblocked it.** Looking for what the canvas
was waiting on found that the model was not missing but four weeks old, and that its one restrictive
clause — *the window gets no vote* — rested on the picture depending on the render size, which this
work and `point_rate` between them removed. The size now belongs to the output
([ADR-0246](adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)),
and what made that safe is a rule:
[P-0086](principles/0086-a-procedure-knows-only-what-it-declares.md) — a procedure does not know
how large its target is, so the same Set at two sizes is one picture at two resolutions.

### The decisions nobody has taken

Each is listed with what it blocks, and every one was found by building the thing next to it.

**Which cut of the previous frame a feedback effect reads has left this list by being decided.**
[ADR-0317](adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md) takes
it, on the maintainer's ruling: the master chain is fixed built-in passes rather than a writable
L5, and feedback reads either cut — `mix`, the frame as the mixer wrote it, or `exit`, this chain's
own output — with the parameter choosing and **only the chosen cut retained**, which is the holding
cost this entry argued, honoured rather than dodged. What it blocked was M5.8 and M5 itself, and
M5.8 has closed with the chain ADR-0317 specifies built — the account of it is in
[history/m5.md](history/m5.md).

**Others left this list before it, and none of those by being decided here.** *What the deck A
preview cell is showing* is answered in the M5.1 paragraph above. *The instrument has no resolution
model* was not a decision at all: the model was
[ADR-0077](adr/0077-the-canvas-belongs-to-the-session-and-the-window-gets-no-vote.md)'s, four weeks
old and never cited from this file, and its one restrictive clause held only while the picture
depended on the render size — which `point_rate` and the sub-pixel floor removed.
[ADR-0246](adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)
gives the size to the output,
[ADR-0247](adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md) says one render scaled into
each, and [P-0086](principles/0086-a-procedure-knows-only-what-it-declares.md) is the rule that
keeps both safe. What was left of it — **what names an output** — was M5.6's implementation item
rather than a standing question, and it is answered:
[ADR-0324](adr/0324-an-output-is-a-named-destination-with-a-size-and-an-on-off.md) names one with a
word from a closed list and never a label, and that bay closed on 2026-09-09. So the risk badge's
bands and the whole-frame budget below are not waiting on anybody's decision, and an output has a
size.

**`crates/karakuri/src/main.rs` no longer composites at `const CANVAS`**, and **every cost figure in
this file was measured while it did**. That constant was a measuring harness's default standing in
for an instrument's; what replaced it is the largest enabled output's size
([ADR-0325](adr/0325-the-frame-follows-the-largest-enabled-output-and-a-resize-costs-0-145-ms.md)),
leaving `CANVAS` the shape every output is fitted to and the size a run starts at. **This was never
a question and it was never this section's** — ADR-0247 decided it and ADR-0246 named M5.6 as the
owner of implementing it, which that bay has done — so it stays here only as the reason every number
below carries the same caveat, each having been taken at a fixed pair rather than at whatever an
output now asks for. This paragraph also said the constant's *"own doc says why: it is the workspace's reference
workload"*; that doc was rewritten under ADR-0270 and now disclaims the material half outright. **What the reference workload now is** — `examples/drift_cloud.kset` at 1280x720, named
rather than taken from whatever the default pair happens to be — is
[ADR-0270](adr/0270-the-reference-workload-is-a-named-set-rather-than-whatever-the-default-pair-is.md),
which also says what it leaves undone here: a panel figure is a deck of four stepped and drawn
slots and a headless figure is one Set, so the sentence above about a panel number sitting beside
every other number in this repository holds only against other panel numbers.

- **Whether loading a shipped preset should write into the operator's own library.** ADR-0229
  makes a load a packaging step that stores the bundle, so opening a shipped Set adds an entry to
  `<store>/sets/` — a consequence of two decisions rather than a decision anybody took. It is
  probably right, being copy-on-load exactly as `scratch.rs` performs it for material and what
  keeps `examples/` unwritten (P-0096). **The first symptom if it is wrong** is a `my sets` filling
  up with things the operator did not make.

- **What `expand` under a solo should do.** `Layout::soloed()` can lie: the solve never reads it
  and `check_structure` only checks that it addresses a node, so a solo's exclusivity lives in
  collapsed flags any later `expand` may contradict, and the next `unsolo` restores its snapshot
  and throws the expand away with nothing said. Nothing reaches that state today.

**A question this clause carried has left it by being decided.** *What the Staging lane draws when
one build changes two nodes* was M5.7's item 4, and the maintainer took it on 2026-09-09 — 変更ノー
ドごとに 1 行, one row per changed node, so a row is a node a build changed and the build's one
verdict is repeated on each of its rows
([ADR-0326](adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md)). The account of it
is in [history/m5.md](history/m5.md) under M5.7.

Two more questions are named where they are met rather than here: who owns the pointer (M5.11), and
whether an addressed write by an agent onto a node the operator kept is refused, which cannot be
decided until something writes a parameter on an agent's behalf (M5.5). **What an agent is one *of***
is M6's and is open there.

---

## The instrumentation

**Nothing in the test suite opens a window, and on 2026-09-08 that cost a program that would not
start.** `cargo test --workspace` was green over 1953 tests while `cargo run -p karakuri` aborted on
its first frame: a startup line asserted `View::select_scope`'s return value, which answers whether
the mark *moved*, and the scope it selects is the console's own default — so the console agreeing
read as the console refusing. Nothing in the suite reaches `resumed`, so nothing ran the line.
`mod gpu`'s tests take a device and still open no window.

**The same shape was caught the same day by a different route.** A `#[cfg(test)]` placed nine
thousand lines above the test modules silenced five source-scanning tests in
`crates/karakuri/src/main.rs`, because each bounds its scan at the first `#[cfg(test)]` it finds.
That one was found by verifying in a throwaway `git worktree` at HEAD plus one change. **The pair is
worth keeping together**: a green suite proves that what ran passed, and neither of these ran.
What closes the first is a test that reaches the startup path; what closes the second is nothing
written down yet.

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
ADR-0164's schedulability conditions and `tests/parked.rs` holds the still panel's zero. Take any
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
- Watchdog on new pipelines, which **stops the slot rather than rolling back**. Both halves of it
  moved on 2026-09-09. **What it judges was wrong**: it compared the deck's whole frame interval —
  every slot's step and draw, the composite, the cell presents, the picture, the `egui` pass and the
  vsync wait — so one slot's candidate was thrown out for the other three slots' cost plus the
  console's, and a Set whose own frame costs 9.58 ms was rolled back on a number that read 33 ms. A
  candidate is judged on **its own** measured cost now, against one frame of the display, with
  nothing about the other slots on either side; the deck's period is still measured and is a
  deck-level alarm that warns and never acts
  ([ADR-0313](adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)).
  **And what it does about it was unreadable**: a rollback restored the previous *Set* and could not
  restore the previous *file*, so the disk went on holding the refused version and the next
  unrelated save reinstalled it — *"事情を知らないとバグってるようにしか見えない"*. The verdict against
  is now `overloaded`: the version stays in the slot, the slot stops updating — no step, no draw, its
  target holding the last frame it made — and the lane, the health capsule, that deck's cell caption,
  the CLI status line and `swap_outcome` all say so. Nothing is put back and nothing ends it on its
  own; the three ways out are the operator's fader, an earlier version landed out of the history, and
  the next build
  ([ADR-0316](adr/0316-an-over-budget-candidate-stays-in-the-slot-and-the-slot-stops-updating.md)).
  This is `P-0094`'s *be loud* in place of its *undo*, taken because the undo was not one — and
  **that principle's own delete-and-re-record pass is owed and is the maintainer's**, its *Undo*
  example being `swap.rs`, which no longer rolls anything back; ADR-0313 left a pass owed on it too
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

**The frame *is* measured now, and it is a reading rather than a budget.**
[ADR-0313](adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)
took the deck's frame period off the swap watchdog's verdict — where it had been judging individual
candidates on their neighbours' cost — and put it on `Deck::frame_period_ms` and
`Report::deck_over_period`, a rolling median on a host clock that says the deck as a whole is or is
not keeping up and acts on nothing. **That does not close this item.** The period is one number for
four slots and cannot be divided among them, which is exactly what this heading asks for; what it
supplies is the measurement *What `over_budget` says once `committed_ms` is the deck's total*
needs — the *Mx — TODO* entry that item became — and an alarm where there was silence.

**The failure is silence rather than slowness.** Four slots at 262144 elements measured about 10 ms
a frame and registered as no violation at all. The maintainer's point: the problem is not that it is
slow, it is that it is not detected. **Half of that is answered**: the deck's own period is measured
and `Report::deck_over_period` says when it is over, so the silence is gone from the *deck*. What is
still not detected is which slot, because a frame interval cannot be divided — that is the per-slot
question and it is `committed_ms`'s, below.

**What must not be done about an over-budget slot.** Forcing it to keep drawing, or lowering the
frame rate to fit it, confuses the means with the end — the frame rate is what the show is played at,
not a dial to spend. **A slot over budget is stopped.** That is the last of the risk badge's five
bands and the one that is also a behaviour ([the console page](manual/console.html), *What a deck
preview cell shows, and when*).

**What is thin.** What the whole-frame measurement is taken *with* is not settled — the probe
measures a Set, and a frame is passes this project has never timed together. The instrument question
[P-0095](principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md) asks of any number applies to
this one and has no answer yet.

**The gap widened on 2026-09-07 and the record says so rather than closing it.**
[ADR-0269](adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md) steps
every drawn slot on every frame, so an off-air slot now costs its L1 as well as its L4 and neither is
in `Report::committed_ms`, which sums the **Live** slots. That field says what it means and an
admission decision needs exactly it; what nothing computes is the deck's total, which is the
per-slot `Decision::cost_ms` summed. Making `committed_ms` that total would leave `over_budget` — the
governor's one warning — permanently on for any four-slot deck of heavy material, which is why it was
not done in passing. **There is a second warning beside it since 2026-09-09** and it is deliberately
not the same flag: `Report::deck_over_period` is one measured frame of the whole deck against one
frame's deadline, where `over_budget` is a sum of per-Set costs against the compute budget
([ADR-0313](adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md),
which supplies that measurement and takes none of this ruling). **The same change also measured the thing this item is about**: the four-slot
frame that *"measured about 10 ms and registered as no violation at all"* is 21 ms with every slot
warm and was 209 ms with three of them cold, and it still registers as no violation at all.

#### The preparation slot is the measurement

**Built, and not wired — it is what would produce the risk badge's estimate.** Today a reference
measurement is the hint that admits a Set into a slot at all. The slot is then **drawn small while it
prepares**, and that small draw is a second measurement — a higher-confidence estimate of what the
same material costs at full size, on this machine, in this environment, rather than a number carried
from elsewhere.

**`Topology` looked like the discriminator and is not**, which took a measurement to settle. This
item said *a `Fullscreen` procedure's cost is the pixel count so it scales with the target's area; a
`Points` procedure is dominated by primitive work so it does not; `Lines` is unmeasured*. The
fragment stage tracks the area for **all three**, and what decides whether a procedure looks
invariant is its coverage rather than its enum — see *The rule that follows from the language* below,
and [ADR-0266](adr/0266-two-rungs-and-a-fit-because-a-frame-is-an-invariant-part-plus-a-fragment-part.md),
which rejects the per-topology branch and says what it would have got wrong.

**The small draw has a floor under it now**, which cuts both ways for this.
[ADR-0245](adr/0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md)
holds a sub-pixel primitive to one fragment rather than dropping it, so a small target's picture is
the same picture and the measurement is of the same material — which is the whole premise of
measuring there. It also puts a floor under the *cost*: below the size at which sprites reach a
pixel, a `Points` procedure costs one fragment per element whatever the target is, so a draw made
smaller past that point stops getting cheaper and stops saying anything new. `tests/deck.rs`
measures the pair: a 112x63 render of the panel's own material went from 0.6x the whole 720p frame
to 1.1x of it.

**Built on 2026-09-06, and the area-ratio extrapolation it shipped with was replaced the same day**
by [ADR-0266](adr/0266-two-rungs-and-a-fit-because-a-frame-is-an-invariant-part-plus-a-fragment-part.md).
`crates/karakuri-engine/src/estimate.rs` now draws a Set at **two** rungs — half and a quarter of the
target's height — solves `a + b·area` through them, rewinds and restores the viewport, and returns
either the fit or a named refusal. It is still exported from `lib.rs` and wired to nothing; wiring it
to `Deck::govern` is undecided and is where to resume.

**The rule that follows from the language rather than from a machine.** A frame is three parts: the
simulation over `capacity`, the vertex stage per primitive, and the fragment stage over covered
pixels. The first two do not depend on the target's size. The third does, and it does so for *every*
topology, because `point_rate` is a **fraction of the target** rather than pixels — a fullscreen pass
covers the target's area, a sprite covers `capacity × (rate × height)²`, a stroke covers its length
times its width and both are fractions. So the cost is `a + b·area`, and **`Topology` is not the
discriminator**: what decides whether a procedure looks invariant or looks area-proportional is
coverage, `capacity × rate²`, which a param can move. The same procedure sits on either side of any
rule keyed on the enum.

**Two terms need two rungs, and that is what it does now.** One draw at one size gives `a + b·A` and
no way to separate them, and scaling that sum by the area ratio scales `a` with it — which for
per-element material is most of the frame. Measured on one machine, the shipped corpus overshot by
very nearly the whole area ratio and every `Points` and `Lines` slot landed in the badge's last band.
ADR-0266 takes the second rung, fits, and extrapolates the fragment term alone.

**What that decision leaves undone**, because a consequence recorded only in the record is one the
next person meets rather than reads:

- **`estimate` refuses every `Points` and `Lines` Set** with `Unfit::FloorUnknown` and draws nothing,
  because `point_rate` is a per-element vertex expression the CPU side never reads and the floor
  below is therefore not a number it can compute. `estimate_above_floor` takes the floor from a
  caller that knows it. **Closing this is a static analysis of the `point_rate` expression against
  its params' declared ranges, and it is not written.**
- **The fit is noisier per call than the number it replaced**, by about fivefold on the difference
  between the two rungs, and on a host clock that is the clock rather than the fit: driven with the
  full-size truth interleaved, one unchanged configuration measured 8.85 to 28.00 ms **at the same
  size**, which `examples/small_draw.rs` would discard on its own spread limit. What the fit does
  with that is refuse — about half the attempts on dense material returned
  `Unfit::FragmentTermNegative` rather than a wrong number — which is the design working and is the
  second reason nothing is wired to the governor yet.

**Numbers taken here are not a basis for a decision.** Development is one unified-memory Metal
machine; a tile-based renderer's binning behaviour and its thermal drift are that machine's and not
the instrument's. Measurement is for confirming a rule that logic already argued, and for finding a
floor. The floors themselves are the language's: ADR-0245 stops a sprite's coverage falling below one
pixel, so both rungs must sit above `1/rate` rows — and `speed_lines` ships a stroke one pixel wide
at 720 rows, so the reference size is already at its floor and no smaller draw says anything new
about it.

**What the ceilings turned out to be.** `MAX_OPS_PER_ELEMENT`, `MAX_OPS_PER_SPAWN`,
`MAX_OPS_PER_FRAGMENT` and the fullscreen one all arrived in one commit whose message does not
mention them, and **none has ever been measured against a frame time**. 4096 was chosen as an order
of magnitude above two spec examples; 16384 is that times four with nothing arguing the factor; 512
stands in for a quantity — `capacity × sprite area × overdraw` — that is nowhere counted. The doc on
`MAX_OPS_PER_ELEMENT` says it *"will need retuning once real probe data (stage 7) exists"*; that data
exists and nothing was retuned. **They are a backstop, not a budget**, and the governor is what
enforces: a refusal before the show costs nothing, which is why a bound stays, but it belongs where
it catches absurdity rather than where it negotiates a surface's normal.

**Round the estimate toward refusing.** A show is cheaper to protect before it starts than to rescue
during it.

**`Priming` gains a second meaning.** It already means warming buffers
([ADR-0053](adr/0053-priming-runs-the-simulation-and-skips-rendering.md)); this makes it also mean
measuring.

**Two slots live and two preparing is the standard way of working.** That is why drawing a small
target cheaply matters, and it is the premise the risk badge's bands are read against — *blue* is the
ordinary state of good material precisely because two at 60 Hz is the ordinary case.

**What is thin.** How small *small* is now follows from the target — half and a quarter of its
height, both above ADR-0245's floor — and `Lines` is measured: at the declared maximum stroke width
the fragment term is 67 ms of an 87 ms frame, so it tracks the area like everything else. **How the
extrapolation's confidence is expressed is still unstated**, and it is the live question: the fit
either answers or names a refusal, and there is nothing between those two. A third rung's residual is
what would say whether the cost is affine at all, and it is deferred in ADR-0266 on what it costs and
on this machine's spread. And every one of these numbers was taken at a resolution nobody chose — the
panel's `CANVAS` constant. What replaces it is the output's own size
([ADR-0246](adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)),
which makes an extrapolation a per-output question rather than a global one.

#### What a machine's size is allowed to decide

**No hard limit, and no ceiling chosen here.** A rich machine should be allowed to spend
what it has: more capacity, more slots resident, longer chains. Capping that to make a small
machine's experience the only experience would be this project choosing against its own
operator.

**The case that tested this rule is not named anywhere else, so it is named here.** A Set whose
elements land on one destination texel costs about 74 ms where the same Set scattered costs 2.6 —
the raster back-end serialising order-dependent blending at one address, and the cost tracking the
number of distinct texels rather than the number of fragments. Nothing about the arithmetic is
wrong, an operator can reach it with a picture anybody would write, and **no limit is being added**:
somebody with the hardware who wants that picture is allowed it (ADR-0269). What it does break is
the extrapolation, which measures how much coverage there is and not how concentrated — so
`estimate` is right at the sizes it measures and wrong extrapolating a concentrated Set to full
size. This file pointed that at M5.14, which closed without it ever being one of that
sub-milestone's items; it is under *Mx — TODO*, and it is a gap in the extrapolation rather than a
limit here.

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
