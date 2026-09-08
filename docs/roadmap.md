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

#### M5.1 — Program — **closed**

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

**Closed on 2026-09-03**, and the exit is a grep rather than a judgement: the board names no control
of this bay, and no row still carrying a `plan` key badge is one of its. See
[docs/history/m5.md](history/m5.md) for what met it. The bay draws one thing less than the mock does,
and it is not a badge and does not block the exit — see the risk badge, below.

**A cell's letter and state are outside the image.** Each cell is an image with a caption band under
it: the deck's letter, one word for what the cell is showing — `material`, or `no slot` — and a
place kept for the risk badge. `karakuri-console`'s `caption_of` derives the band from the image
rectangle rather than taking it beside one, so `preview_rects`, which is what the engine sizes a
texture from, stays the image and never the image plus its label.

**The risk badge is not drawn, and it is not a drawing that is owed.** Its five bands are specified
in [the console page](manual/console.html), under *What a deck preview cell shows, and when* and on
deck A's caption tooltip; the panel draws no dot because nothing a deck can reach estimates what a
slot costs. What would produce that estimate is `karakuri-engine`'s `estimate` module, which is built
and wired to nothing — see *The preparation slot is the measurement* under *Performance discipline*,
below.

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
here once and the other three bays take it as it stands. **Done.** The size pill and the picture were
the two controls in the bay with no `data-tip`, and both carry one now: the pill says the size is an
output's rather than this bay's ([ADR-0246](adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md),
[ADR-0247](adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md)), and the picture carries
*Program, sized by height* whole.

#### M5.2 — Mixer — **closed**

**Rows.** None carries a `plan` panel badge any more. *Choose the wipe shape, the quantum, the
length* and *Wipe the next deck in* are built, and ***Fade a deck out or in* is a `gap`**, decided on
2026-09-05: a fade is an opacity moved over a few seconds and the fader is already the control for
moving one, so a capsule asking for the same move would take permanent room on a strip to save the
gesture the strip is built around. What it would have bought is the one thing a hand cannot do —
landing the move on the beat rather than near it — and that is not worth a control here. The
argument an operator reads is *Nothing on a strip starts a fade* on
[the console page](manual/console.html), beside *The mixer has no crossfader*, which is the same
shape of decision and the precedent for the `gap`. *Gain*, *Opacity* and *Blend mode* reach the
instrument's keyboard as well, bound on 2026-09-05. **The four rows still carrying a `plan` key
badge are M5.13's and not this bay's** — the letters they name (`f g`, `x`, `c`, `z n j`) are a
scheme ADR-0259 retired, and binding them here would have been work thrown away.

***Choose which renderer of a deck is live* was counted here and is M5.5's**, moved on 2026-09-05.
It was never a control of this bay: the mock draws `rend-row` in the **Inspector** and nowhere in
the Mixer, and [the console page](manual/console.html) describes it and *Composite a deck's
renderers* as one thing — *"Composite is the deck's, and the renderer chips below are what it turns
into a choice"* — while that other half was already counted under M5.5. Split across two
sub-milestones, the chips would have been built first and meant nothing until the toggle above them
existed. Its `plan` key badge (`r`) goes with it, and is
[ADR-0259](adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)'s
rather than that bay's.

**Exit.** No `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html).

**Closed on 2026-09-05**, and the exit is a grep rather than a judgement: no row of this bay carries
a `plan` panel badge. What it drew on the way is the transition row, which was painted by nothing;
what it stopped drawing is the mock's fourth strip, which a record had rejected; and what it decided
is that a fade has no control on a strip. The key column it used to owe left with ADR-0259 and is
M5.13's.

**Order inside the bay.** *Choose the wipe shape, the quantum, the length* went first and **is
drawn**: `.xfade` was painted by nothing, and the console now owns the quantum, the length and the
wipe shape as a pointer of its own and emits the operation from three pills. The other three rows
convert only once a surface holds what that row sets: *Fade a deck out or in* answers `Owed::NotRead` until the transition settings
are handed over, and *Choose which renderer of a deck is live* and *Wipe the next deck in* read the
same way — the wipe reading the row's third setting, its front shape, as well as the other two.

**Blocked on — the panel column, nothing; the key column, four rows.** *Fade a deck out or in*
(`f g`), *Choose which renderer of a deck is live* (`r`) and *Choose the wipe shape, the quantum, the
length* (`z n j`) name letters this binary already binds to folding, resetting and unfolding — the
collision ADR-0220 recorded and did not resolve — and
[ADR-0259](adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
now dissolves it rather than picking a winner: the keyboard is addressed to the bay that has focus,
and those rows' badges stop naming letters at all. **So this sub-milestone closes short of its stated
exit on purpose**, and the four `plan` key badges it leaves are ADR-0259's to clear rather than
work anybody skipped. What is still this bay's own is the `go` pill and *Crossfade to the next deck*,
both of which wait on the binary's `reading()`, which answers `None` for the transition settings and
the mix. *Wipe the next deck in* was the one row of the eight that was blocked, and it
waited on who says the front shape and the soft edge are its. Both have an owner now: the shape is
`Operation::SetTransition`'s third setting and reaches the conversion inside `Current::transition`
beside the quantum and the length, which are that operation's other two, and the soft edge is read
off the mask already running on the deck being wiped in. `written(Wipe)` writes its six records —
four or five where the deck arriving is already under `over` or already live, because the mode is
the operator's and a wipe does not take it back — and `karakuri-cli`'s `c` is one `operate` call, so
what the panel's row needs is the control, not a decision.

*Set a deck's mask position* carries no `plan` badge in either column and is not in this exit
condition. It is the mixer's under-draw, named in
[ADR-0206](adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md), and whatever
surface gives the mask's front a position closes it.

**The bay's prose, as tooltips. Done on 2026-09-03, re-opened the day the transition row was built,
and done again on 2026-09-05.** Two notes and half of a third — *Mixer*, *The mixer has no
crossfader*, and the selection half of *Two focuses, and they do not look alike* — are on the
controls they describe. Eleven tips on 2026-09-03: the strip-count readout, deck A's strip (the
selection, which is why the strip has no capsule of its own), three trims, three faders, three
meters, deck B's pending-fade number, and the transition row, which carried the crossfader argument
whole and said where the wipe's shape and soft edge come from — the row itself and the arriving
deck's mask. (That tip said they were open questions while they were; it was rewritten when they
were answered.)

**It re-opened on 2026-09-05**, because `74719c3` and `094804c` drew four capsules the mock had no
tip for — the shape pill, the quantum pill, the length pill and `go` — and a bay whose prose is on
its controls is a claim about the controls it has and not about the ones it had on the day it was
closed. Four tips written the same day, sourced rather than invented: the three cycles and the words
in them are `karakuri-console`'s `WIPE_SHAPES`, `QUANTA` and `FADE_BEATS`, and `go`'s two refusals
are `TransitionRow::go`'s. The row's own tip keeps the argument — the crossfader, the wipe this bay
cannot draw, and the soft edge, which is no control's anywhere — and what each setting *means* moved
down onto the pill that sets it, as did the two decks a wipe names and the two ways it is refused.
Moved and not copied: the row's tip is 705 characters shorter and no sentence it lost is still on
it. The capsules carry more than that, because the cycles' own words and what each refusal names are
read off the code rather than off the row.

The meters are the only ones not sourced from the notes, which say nothing about a
meter: they come from
[ADR-0043](adr/0043-the-meter-never-waits-and-the-deck-owns-it.md) and
[ADR-0178](adr/0178-the-mixer-draws-four-tracks-and-as-many-strips-as-the-deck-has.md).

**And it found the mock drawing what ADR-0178 rejected.** The fourth track carried a whole empty
strip — a name, a `tally off`, a trim, a fader and a meter — and that record's *Alternatives
rejected* is precisely *"a well per track, empty where no deck fills it … A track nothing fills
draws nothing at all"*, on the grounds that an empty strip asserts *there is a deck here and it is
at zero*. `karakuri-console`'s `tests/mixer.rs` already holds the panel to the record and says so
against the mock. **Four tips for that strip were written and then removed rather than kept**: a
tooltip is what an operator reads, and one on a control the panel is decided never to draw is a
reading nobody can take. **The mock lost that strip on 2026-09-05**, which was a change to a
designed page and this bay's rather than a tidy-up; `.tally.off` went with it, the one rule in
`docs/manual/style.css` nothing else used.

#### M5.3 — Library

**Rows.** Counted in the panel column of [every operation](manual/operations.html)'s *The library*,
where the badges are. **Neither the count nor the list belongs here, and both have been tried.** The
count read four while it was three, was corrected on 2026-09-05, and read three while it was two by
the next commit; the list that replaced it named *Load material into a deck*, which went `has` on
2026-09-06, and missed two rows that had not. Each went stale in silence inside the paragraph
explaining the previous correction, which is *The instrumentation*'s lesson one sub-milestone down
and now has two witnesses instead of one.

**What is owed, on 2026-09-07, and one of the three is not what its badge said it was.** *Keep what
a deck is playing* is a control to draw and nothing behind it: the operation is `has` from the key
and over MCP, and what this bay does not have is a press for it. *Send a Set to somebody* is what
this milestone is blocked on, below. ***Walk the edit history* is the third, and it left *Rows the
manual has not given a home* to get here** — that list holds it on the reading that it is undo, a
surface over the store's history files and a later milestone. It is not undo. It is a listing of the
versions a Set has had and a load of the one you pick, which is this bay's shape and nothing else:
the Library already lists, already reads, and already loads. What that costs is below.

***Read what one Set holds and declares* is `has`**, on 2026-09-06. The bay opens a reading under
the cursor's row and it follows the cursor: every published key with its range and default, the
capacity and what the geometry emits, read off the metadata cards the store keeps beside the
artifacts rather than off a built Set — so what it draws costs a directory read and nothing else. **A reading is one question and only the opening asks it**: closing emits
nothing and a cursor move re-reads without emitting, which would otherwise reach from the keyboard
a row whose key column is a `gap`
([ADR-0264](adr/0264-a-reading-is-one-question-and-only-the-opening-asks-it.md)).

**What that row still owes is a home and a sentence, and neither is a decision.** The merge of
several nodes' declarations into one row per key — the range intersected, the default the first
declarer's, the emitted attributes unioned in file order — is done in
`crates/karakuri/src/main.rs`'s `declared`, which is a second copy of what
`karakuri_engine::set::Set::published` and `Set::declared_range` already decide, and it is a second
copy of what `karakuri_environment::mcp::read_set` walks. **The right home is a structured reading
in `karakuri-environment`** that both the tool and the panel render; it is in the host because that
crate was not writable in the pass that drew the panel. `declared()`'s own head says so, and it is
said here as well because nothing under `karakuri-environment` says it is owed anything.
**It is M5.10's item and not this bay's**, and it was floating because it is written here: what a
model reads and what the panel draws being one structured reading is the model-facing surface,
where this bay's stake in it closed the day the row went `has`. **The sentence is no longer owed**: the tip gave way on 2026-09-06. It had promised a reading
*"without fetching a source or compiling it"*, which is exact for the three blocks read off each
artifact's own card and untrue of the fourth `read_set` volunteers — `element_storage_block` runs
`compile::check` over every source before `Set::validate` can size anything. Both the row and
`Operation::ReadSet` now say which figure costs what. The two options not taken were stopping the
tool offering it, which removes a capability to fix a sentence, and making it a question of its own,
which adds an operation; neither is foreclosed by this.

***List what the store holds* is `has` too**, from 2026-09-05. The two fields the mock has always
drawn under the scope row are controls: they emit `Operation::ListSets` and the row reads
`panel has library filters`. The badge had been honest
rather than stale — the drawing existed and the route did not, because the bay took its listing as
names off a directory read and constructed the operation nowhere. Why the fields **step** rather
than take letters, and what that costs, is
[ADR-0262](adr/0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md);
the bay's rows changed order in the same work and that is
[ADR-0263](adr/0263-the-library-bay-lists-most-recent-first-because-the-listing-is-the-operations-and-not-the-surfaces.md).
**What is left open is the console's text entry**: ADR-0262 refuses to invent one for this bay and
says the control is a first cut, so the row being `has` is not the same as the fields being
finished.

**Exit.** No `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html).

**Blocked on. One thing, as of 2026-09-07, and it is the task below.** *Send a Set to somebody, and take
one in* waited on a destination the vocabulary could carry, and
[ADR-0260](adr/0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)
answers it by refusing the premise: sending is a **read**, and a read's answer goes where the surface
that asked puts answers, so the operation names no destination and none is owed. `SetTransfer` does
not change. The vocabulary already holds three reads — `ReadSet`, `ListSets` and `ReadProcedure` —
and names a place for none of them; the command line prints a package to stdout for the same reason
it prints a listing there.

**What was left of that row was a control rather than a decision, and the form is now taken.**
ADR-0260 set out two candidates and chose neither — a save sheet, and the clipboard.
[ADR-0267](adr/0267-the-panel-sends-into-the-folder-the-library-bay-is-pointed-at-and-the-destination-is-drawn-before-the-press.md)
takes neither of them either: **the bay is already a file browser, so the directory it is pointed at
is the destination**, and sending writes `<id>.kbset` there. The mock has been drawing that
destination the whole time — `docs/manual/console.html:108` is a `.path` row reading
`~/sets/tour-2026/night-b › opening` — and `view::LibraryBay` lists it as one of the mock's elements
this crate has not built. It closes the loop the other two could not: the file lands where the
listing can point at it, so `SetTransfer::Take`, which names a **file**, can consume what a send
produced.

**And it is what puts this milestone back on a blocker.** ADR-0267 needs `Scope::Folder` to have a
directory, and no operation can carry one — `ListSets { holds, layer }` has nowhere to put it, so
the chip *"waits on a row of the page and not on a decision"* (`crates/karakuri-console/src/view.rs`).
The bay owes that directory whatever is decided about sending, because a folder scope that cannot be
asked for a listing is a chip with no answer; what is new is that **this row is now the only `plan`
panel badge left in this bay**, so the exit above runs through work that appears nowhere in this file
as a task. Give it one, or it is an unfinished item under a milestone that will close without it.

**One sentence on the page has to turn around**, and until it does this record is ahead of the page
(`docs/contributing.md` §5 step 3): `docs/manual/console.html:1715` reads *"a folder is a way **in**,
and **my sets** is where things are"*, and `:1690` *"So a folder row is a take"*. The taking-in half is built and reached, a `presets` row taken in and loaded on the one
press. **The claim this paragraph used to make about a letter-taking control in this bay was
wrong**: the Library bay draws none, and the naming flow it was pointing at is the arrangement
pill's, in the transport row. *Load material into a deck* is not
blocked: `l` performs the operation
([ADR-0228](adr/0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)), and
the control the page names is the drag from a row onto a strip, which is built and reached: a row
is picked up, carried and dropped on a strip, and the page's panel badge for this operation reads
`has` (`docs/manual/operations.html:202`). **The deck is named at the release rather than by the
selection, and nothing is refused** — that, and what a drop over nothing does, is
[ADR-0265](adr/0265-a-carried-set-names-its-deck-at-the-release-and-the-panel-refuses-no-drop.md).
**The carry is drawn now, and that half is closed.** The rectangle under the pointer is ringed
while a Set is in hand, the four deck preview cells take the drop as well as the strips, and the
pointer is a grab for as long as the gesture runs
([ADR-0273](adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)) — the mock
specified all three in `4ec14db` and none of them is a route, so none holds the operation's badge.
**What is still not drawn is the Set**: there is no ghost under the pointer and no second mark on
the row in hand, and both were declined in the design pass rather than left open.
The pill beside it is a readout on purpose, so a press on it would be a third route
nobody specified (`view::LibraryBay`).

**The two pieces of work this milestone is waiting on, as tasks rather than as prose.**

1. **A folder scope with a directory in it — and it does not wait on an operation.** Three places
   say it does: `view::Scope`'s doc bullet, `why_nothing`'s `Folder` arm, and the tip on *List what
   the store holds*. All three read a correctly shaped payload as a defect. **`ListSets { holds,
   layer }` is not missing a field**: both its fields are filters over what the store already holds,
   a directory is *which store is being asked at all*, and that is already the host's — `listing()`
   picks the store or the presets root outside the operation. What decides it is the rule the one
   path in the vocabulary already obeys: **a path may sit in a payload only where every route that
   fills it derives it from something the program itself produced.** `SetTransfer::Take { file }`
   obeys — the panel's route re-asks the presets listing and finds the row by the word that was
   pressed, so no surface spells a path. A directory on `ListSets` would be spelled, and `mcp.rs`'s
   *"paths never cross the protocol"* is the same wall on the other side. There is no per-surface
   exemption to reach for either: `gate` is one classification and one sentence for every route.

   So what the chip waits on is **a host mechanism for choosing a directory, and a sentence on the
   page** — `console.html`'s `.path` row is drawn and carries no `data-tip`, alone among that bay's
   elements. The decision is a folder dragged onto the bay: no dependency, no modal over a live
   instrument, and the `.path` row is already drawn as a readout of where you are, which is what a
   dropped folder sets. It rests on one unknown — whether a dropped *directory* reaches
   `egui`'s `dropped_files` on this platform — and a dialog with a dependency is the fallback if it
   does not. **Sending is the only `plan` panel badge this blocks**, and the page turning `a folder
   is a way in` around is ADR-0267's other half.
2. **The bay walking the edit history. The lister is built** —
   `karakuri_environment::history::list`, most recent first, foreign entries skipped and counted,
   capped on days opened before a day is read and saying when it stopped short. What is left is the
   manual naming the control and the bay drawing it.

   **Building it turned up a gap, and the gap is closed.** `Snapshots::record` addresses a version
   by `(slot, layer, index)` and **the layout carries no Set id anywhere**, so as the files stand
   the two sides of a library load on one slot are indistinguishable and *a Set's history* is not
   answerable from them. **The id is written from now on, and it costs an argument rather than a
   design.** Every route that edits is addressed by slot — `mcp::write_procedure` resolves
   `Slots::path(slot, layer, index)` and refuses a node the slot does not hold, and an operator's
   own editor is pointed at that slot's scratch copy — so at the moment of a write the program knows
   which Set the slot is running. **It is already kept**: `Gfx::material` is one name per slot,
   rewritten in `played` on the load, and its doc says why a name per deck was not enough. What is
   left open is only what to write where there is no Set: a run launched with a pair on the command
   line has no id, and `k` writes a new one mid-chain. **Both are answered and the writer is
   built** ([ADR-0276](adr/0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md)):
   the id goes in the snapshot's *name*, behind `@`, because a row is a name and `list` opens no
   file; a run with no Set writes none and `Version::set` is an `Option`; and a save does not move
   the slot, so only a load re-files a chain. **A narrowing must treat a `None` row as matching no
   Set rather than as a wildcard** — which is this bay's to get right, the filter being answered
   where ADR-0262 says. Rows are a Set's versions. The page owes the
   rest: the row
   moves into *The library*, the control is a fifth scope chip beside `my sets`, `favourites`,
   `presets` and `folder`, the order is stated in the row's own sentence as *List what the store
   holds* states its, landing on a row is a **load**, and `WalkHistory`'s payload is *a revision to
   land on*. `Operation::WalkHistory`'s doc gave *"the store has no reader"* as half its reason for
   `Undecided`; that half was false and the doc is corrected. The payload is unchanged, and what it
   now waits on is only the control.

   The paragraph this replaces read: `karakuri_environment::history` has `Snapshots::record`, `seed`
   and `stamped_id` and **no lister**: nothing opens `<store>/history/`
   to answer *what versions has this had*. Once it can, *Walk the edit history* is a scope in this
   bay — the rows are versions, the reading is the one already built, and landing on one is the load
   already built. That also settles `WalkHistory { step: Undecided }`, whose own doc offers three
   shapes and whose answer is the second: **a revision to land on**, because a row is what an
   operator picked. The manual names the control first (`docs/contributing.md` §5 step 3), and until
   it does this row's panel cell reads `—`.

**What is *not* here, and it moved rather than being dropped.** Nothing records a history in this
program at all — only `karakuri-cli` builds a `Snapshots`, so a panel run keeps none and an MCP
write keeps none. That is M5.10's, under *Every write that compiles is a version*.

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
`<store>/scratch/`. The mock tips four of the five scope chips, the two filter fields, two stars
and the `load → A` pill; `my sets`, the path and the rows carry none.

#### M5.4 — Transport

**Rows.** Six carry a `plan` panel badge: *Tap the beat*, *Halve or double the grid*, *Nudge the
latency offset*, *Set the free-run tempo*, *Find out what a write did*, *Record the session*.

***Tone map* and *Exposure* were counted here and are M5.13's**, dropped on 2026-09-05. They name
the transport as their panel home and this row is where the mock draws them, but both carry a `has`
panel badge — the tone capsule cycles the four operators and the exposure track is twelve stops you
click into, and both have carried a `data-tip` since the mock was written. So this bay never owed
them a control; what it owed was the letters on their key badges — `t` for the tone map, and `-`,
`=` and the backtick for the exposure track — which is the scheme
[ADR-0259](adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
retired. Their prose stays this bay's, in the tooltip item below, because the controls are still
drawn here.

**Exit.** No `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html).

**Three of the six are built, on 2026-09-08** — *Tap the beat*, *Halve or double the grid* and
*Nudge the latency offset* — and they are one group and one control row: `view::tracker_group`
finishes the four things `.tracker` puts round the audio-in pill, one row of `input::PROBES` and one
entry of `press_handler::ASKED`. Two records came out of it. The offset is drawn as a **track and
not the capsule the mock had**, because a press has to name a value and a capsule can only ask for
the next one ([ADR-0277](adr/0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md)) — the
mock and both manual pages moved first. And the tap and the octave **leave `App::performed` before
`written` is reached**, which is what lets one press and one key be one route
([ADR-0278](adr/0278-an-operation-no-record-can-be-written-for-leaves-the-window-before-it-is-written.md));
`b`, `,` and `.` emit now instead of performing the operation themselves.

**Blocked on.** *"Nothing"* was true of three of the six and is not true of the other three. This
said *"the cheapest bay on the page: every table row is a control beside a readout the row already
draws"*, and the three that are built were exactly that. Each of the remaining three wants a decision
before it wants code, and none of the three decisions is this bay's to take alone.

- ***Set the free-run tempo* has no shape and no meaning while a tracker is running.** The row's
  panel home is `transport`, and the only thing the mock draws there for it is the `128.0` figure —
  which `view::transport` states is one of four **readouts** and `karakuri-console/tests/transport.rs`
  asserts is not a control. So the shape is undecided in the same way the offset's was, and unlike
  the offset the manual has not already argued it. Worse, the *meaning* is undecided: with an input
  open `BeatLock::update` returns a `Trim` on every frame and `apply_tempo` pulls the oscillator
  toward the estimate, so a tempo set from the panel is trimmed away within a second — and
  `--bpm` does a second job the panel would have to decide about, seeding the tracker's octave window
  (`Audio::open(selector, …, session_bpm)`). **The plumbing is not the problem**: `Signals::correct`
  and `karakuri_environment::audio::apply_tempo` are both public and both reachable from
  `crates/karakuri`, and `written(SetFreeRunTempo)` already answers a `Record::Tempo` — what is
  missing is an arm for it in that file's `apply`, which is one match arm. **And this program has no
  launch route either**: `crates/karakuri` has no `--bpm` flag at all and runs at
  `Signals::default()`'s 120.0, so the row's `CLI has` badge is `karakuri-cli`'s alone.
- ***Find out what a write did* is a readout, and whether a readout may carry a `has` panel badge is
  not settled.** The panel already holds every verdict, on every run and not only under `--mcp`:
  `Deck::events` is drained once a frame in `staging`, and `view::Stage` already spells the mock's
  own three words — `landed`, `rolled back`, `refused`. What is missing is the pill in the transport
  row, and behind it a rule: `karakuri-console/tests/panel_column.rs` requires that a control
  **emit** the operation before the page may mark the panel column `has`, and a readout emits
  nothing. ADR-0264 settled which *gesture* of a reading emits; it did not settle a reading with no
  gesture. That rule governs six rows and not one, so it is a decision above this bay.
- ***Record the session* is machinery and not a rectangle.** `crates/karakuri` constructs no
  `session::Recorder`, has no `--record-session` flag and writes no record stream at all — the three
  greps return nothing. And `Recorder` cannot be started or stopped on a press as it stands:
  `Recorder::open` creates a file and spawns a writer thread, which `karakuri-cli` says out loud is
  done *"before the first frame and never on one"*, and `finish` consumes the recorder.
  `Recording::Stop`'s own documentation says the same from the vocabulary's side — *"Nothing does
  this today"*. A `rec` pill is that whole lifecycle, and it moves the row's `when` from **launch**
  to **immediate**.

**A `Closed` class does not refuse a hand.** Worth writing down beside the last of those, because it
reads the other way at first: `gate::audit` is called in exactly one place in the workspace,
`karakuri-environment`'s MCP server. The panel's press path never consults it. So *Record the
session* being `Closed(InputsAndOutputs)` constrains a model and says nothing about a pill.

One thing to know before drawing the first two, and it still holds. *Tap the beat* and *Halve or
double the grid* reach `karakuri_environment::audio` directly rather than going through `written`,
because `written(TapBeat)` and `written(ScaleGrid)` both answer `Owed(NotSettled)`. That is a gap in
`karakuri-operation-record` and not in the panel, and it did not stop either control: what it cost
was one early return in `App::performed` and the record that argues for it (ADR-0278). **The gap is
still open**, and closing it means saying what a `Current` carries about a beat lock.

**The bay's prose, as tooltips.** Six notes: *The beat moves, always*, *The octave is a person's,
and only one half of it is ever live*, *Health, in the transport*, *The latency offset, and which
offset it is*, *The arrangement is a file, and the reset is one of them*, and *The look is two
controls, and they sit where a frame leaves* — the tone map and the exposure track, drawn in this
row and belonging to the master chain. This is the bay the mock has already tipped: every control
here but `tap` and the tempo figure carried a `data-tip`. **`tap` has one now**, written with the
control on 2026-09-08, so the item is the tempo figure and reading the six notes against what the
tips already say.

#### M5.5 — Inspector

**Rows.** The rows whose panel badge names a control this bay draws, which is **not** the page's
*Inside a Set* section and is where the count has to be derived from: they are spread over five
sections, and the section holds two rows that are not this bay's. Ten of them —
*Composite a deck's renderers*, ***Choose which renderer of a deck is live*** (moved here from M5.2
on 2026-09-05, because the mock draws its chips directly under *Composite* and the console page
describes the two as one control and the choice it turns into), *Write a parameter*, *Attach a
signal to a parameter*, *Take a parameter back*, *Element capacity, seeds, the camera*, *Set a
node's authority*, *Keep what a deck is playing*, *Read one node's source* and *Check and write one
node's source*. The two the section sweeps in and this bay does not own are *Wire a procedure's
input to a node* and *Narrow the published interface*, which carry a `plan` badge over a panel cell
reading `—` and are in *Rows the manual has not given a home*.

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
[every operation](manual/operations.html).

**Blocked on. Five rows, and one mechanism covers three of them** — this paragraph said two, and
the sentence under it said the other eight waited on nothing, which was the claim that hid it.

**There is no public route from a `&mut Deck` to a live `Set`.** Every control this panel has built
lands operation → `Record` → `apply` → a `Deck` setter, and `Deck`'s public surface has no param,
binding or authority writer. `Deck::slot` hands back a `&HotSwap` and says why — *"Deliberately not
mutable"* — and `HotSwap::live_mut` is `pub(crate)` with the same argument written out. The engine's
writers all exist and are all reachable only at a build: `Set::write_param`, `Set::set_published`,
`Set::bind`, `Set::set_authority`. **`Set::write_param` no longer is**:
[ADR-0280](adr/0280-a-parameter-written-to-a-live-set-is-a-session-record.md) put `Deck::write_param`
over it and `Record::Ride` under it, so *Write a parameter* has a route through the engine and what
the panel owes is the control. The hole still stops *Attach a signal to a parameter* and *Set a
node's authority*, both the same one-line shape once somebody decides what each records — and it is
what made this the bay where an operator turns a knob and the bay with no way to turn one.

- ***Take a parameter back* has nothing to call even once that route exists**: `Set::bind` has no
  inverse anywhere in the workspace and `Binding` carries no suspended state. The row's own tip
  already says it — *"Nothing does this today; it is the second rule's other half."*
- ***Composite a deck's renderers* waits on a setter.** `Set::merge` is `Some` only where
  `layering == Layering::Composite` at `Set::build`, nothing writes it afterwards, and neither `Set`
  nor `Deck` offers one, so the operation names a state the engine cannot be moved into while
  running.
- ***Element capacity, seeds, the camera* is two-thirds blocked and one-third undecided.** A capacity
  sizes buffers at build and a salt is assigned there; `Set::source_capacities` and
  `Set::source_salts` are readers with no writers. The camera arm is `Property::Camera(Undecided)` —
  the vocabulary has no numbers for a control to send.
- **The two *node's source* rows have nothing drawn and nowhere to type.** The mock's Inspector is a
  half-head, a deck head and node groups; no source area is drawn anywhere on it. And the console has
  one letter-taking flow, bounded to naming an arrangement, which is why the Library's filter fields
  step rather than take letters. A source editor is a second one, and that is a console-wide
  decision nobody has taken.

**Three more this bay owes, and none of them is a row.** They were recorded, verified and left in
prose, which is how a bay's exit — a grep over one column — could be met while its pane drew
nothing an operator could reach. **One of the three is now done** — the centre's declared minimum,
below — so what is owed here is two.

- **A pane draws a node group whole or not at all, and there is no scroll position anywhere in the
  crate.** With the pair a bare run opens on, the first group is the L1's at 26.5 + 19 x 22.5 =
  454px against a bay minimum of 151.5, so below roughly 534px of Inspector bay **not one parameter
  row is drawn**. `view.rs` says outright that this is the first region of the console that
  genuinely wants a scroll. The decision in front of it — scroll, or a group that can be part-drawn
  — is this bay's rather than the arrangement's.
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

**Two of the ten wait on nothing, and both are short.** *Keep what a deck is playing* — the save
path is built and `k` runs it, so the pill is a rectangle, a claim and the same call. *Choose which
renderer of a deck is live* — `Deck::schedule_selection` is public and reaches the swap on the beat,
`written(SelectRenderer)` already answers a `Record::Select` given the transition reading this file
already takes, and what is missing is that arm in `apply` and a claim on the chips.

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

**And there may be no row on screen to press.** A pane draws a node group whole or not at all and
there is no scroll position anywhere in the crate, so with the pair a bare run opens on the first
group is the L1's at 26.5 + 19 x 22.5 = 454px against a bay minimum of 151.5. Below roughly 534px of
Inspector bay, not one parameter row is drawn. Whether that is a blocker or the first thing this
bay's own work has to fix is a decision about scrolling that no record here takes.

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
the spelling is first felt, and because **no surface can write a parameter live today**, which is
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
camera is a node, and it is the one with nothing to turn*. Only the focus half of *Two focuses* is
this bay's; the selection half is the mixer's, above. This is the largest body of prose on the page
and the bay the mock has tipped most: the deck head, the wildcard group, the authority chips, the
renderer row and *take back* each carry one, and the parameter rows and their faders do not.

#### M5.6 — Outputs

**Rows.** One carries a `plan` panel badge: *Choose where the frame goes*.

**Exit.** No `plan` badge in the panel column of this bay's rows on
[every operation](manual/operations.html).

**What an output is, which is this bay's to build.** `RouteFrame { output: Undecided }`, and no
output has an identity anywhere in this workspace. It was in *The decisions nobody has taken* and it
is here instead: it blocks nothing but this bay, so it is an implementation item of it rather than a
standing question. Three constraints are on record and the rest is this bay's work.

- **The picture is in the set that needs naming and a preview cell is not**
  ([ADR-0243](adr/0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md)).
- **An output holds the size it is rendered at**, which the operator or the destination window sets
  ([ADR-0246](adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)).
- **One render, scaled into each output**, at the largest enabled output's size, so no output is
  ever upscaled ([ADR-0247](adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md)). There
  is no second `Deck`, no second mix and no second present pipeline to design.

What is left to decide is the identity itself — the destination, and what tells a sink this
repository owns from one a plugin brings. The mock already draws four of them: *program view*,
*projector · DELL U2720Q*, *Syphon*, *NDI · no plugin*.

**Blocked on.** Nothing outside this bay. The bay draws its sinks and hit-tests the dot; what it owes is
the switchable list, which is the blocked part.

**A second `Sink`, and it is an item rather than an aside.** The fan-out is built
([ADR-0171](adr/0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md)) and
nothing has been written to put in the slice, so *Choose where the frame goes* is a control over a
list of one. A projector window is the one this repository owns. It carries no row on the
operations page, which is how it sat in *Also here* while this bay's exit — one badge — could be
met without it.

**The bay's prose, as tooltips.** One note, *Outputs*: every place a frame goes as one switchable
list, which sinks live in this repository and which appear only with a plugin, that the Program
picture is the first row of the list, and that all of them may be off. The mock tips the four sinks
and the `mcp` pill, and `+ add output` carries none. One short note is all the prose this bay has,
so this is the smallest of the nine items.

#### M5.7 — Staging

**Rows.** Two carry a `plan` panel badge: *Keep a candidate*, *Put a node's previous version back*.

**Exit.** No `plan` badge in the panel column of this bay's rows on
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
5. **A reader for the edit history — built, and it is M5.3's now.** `history::list` answers
   *what versions has this had*, most recent first, and ADR-0276 puts the Set a version belongs to
   in its name. The row it serves is *Walk the edit history*, which left *Rows the manual has not
   given a home* because it is a listing and a load rather than undo. What this bay still waits on
   is item 6: nothing in this program writes a history for it to read.
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

**Both are blockers 7 and 8 rather than prose, and the first is a *Live safety* item as well.**
Neither has a row, so this bay's exit — the two rows' badges — can be met with both still true, and
a picture that disagrees with the disk while nothing says so is the one thing that section refuses.
The lane exists to say exactly these two things.

**The bay's prose, as tooltips.** The staging half of *Library, and staging under it* — a candidate
waits whether it came from you or from an agent, and a rejected one costs nothing — and *The staging
lane, and the sense in which a candidate waits*, which is the longest note on the page and is where
the absence of a *regenerate* is argued. The mock draws the lane with no `data-tip` anywhere in it,
so every tip in this bay is written from nothing.

#### M5.8 — Master

**Rows.** Three carry a `plan` panel badge: *Feedback*, *Bloom*, *RGB shift*.

**Exit.** No `plan` badge in the panel column of this bay's rows on
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

**Exit.** No `plan` badge in the panel column of this bay's rows on
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

**Every write that compiles is a version, and none of them is kept.** `mcp::write_procedure` runs
`compile::check` and writes the file, and touches nothing else;
`crates/karakuri/src/main.rs` never constructs `history::Snapshots` and never calls `record`. Only
`karakuri-cli` does, so a `--watch` run accumulates versions under `<store>/history/` and a panel run
— which is where MCP is served from — accumulates none. **It belongs here rather than in the bay that
would read them**: what makes a version worth keeping is that something other than the operator's
own hands wrote it, and this is where that something is. M5.3's *Walk the edit history* reads what
this keeps, and has a lister to build either way. **The Set id goes in with the record** — the
slot names it, so recording without it would file a version under a node and lose which Set it
belonged to, which is the one thing the bay reading it needs. **The writer takes it now**
([ADR-0276](adr/0276-a-versions-set-id-goes-in-the-snapshots-name-and-a-run-without-one-writes-none.md)):
`Snapshots::record` takes the Set the slot is running and writes it into the name, and
`watch::Watch::snapshotting_to` carries it per slot. **What this program owes it is the answer, and
`Gfx::material` is not it**: that field is a *readout* — one name per slot, and at launch it is the
pair `"coil_vortex + star_flares"` rather than any id — so a slot that has never been loaded onto
has no Set and writes `None`, and only `played`'s `LoadSet` arm turns it into a real id. So this
program owes a per-slot `Option<String>` beside the deck, written where `played` writes the readout,
handed to `snapshotting_to` at construction, and **moved with every `watch::Aim` a library load
sends** — `Aim` carries no id today, and without one every version after a load is filed under the
Set before it.

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
tooltip. That position is why a vector parameter publishes as `x`, `y`, `z` in that order and never
as one row — one row would be a position no control change can set
([ADR-0268](adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)). The last two are M2's, and they are the surface itself rather than a route into it:
**MIDI *out***, so a surface's LEDs and motorised faders follow the deck, which matters the moment
two things can move a fader; and **14-bit control changes**, so a fader is more than 128 positions.

**Blocked on.** M5.11, and only for learn: there is no tooltip to learn from until the pointer
decision is taken. MIDI out and 14-bit control changes do not wait on it.

#### M5.13 — The keyboard

**Rows.** The key column of [every operation](manual/operations.html), which is not this bay's list
or any bay's — `grep -c 'rt plan">key' docs/manual/operations.html` is the count and it is not
written down here.

**Exit.** No `plan` badge in the key column of [every operation](manual/operations.html).

**What it is.**
[ADR-0259](adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md):
a key press is addressed to the bay that has focus, `Tab` walks the arrangement down a column and
then across, and the keys that act inside a bay are the same six everywhere because they are rules
about kinds of thing rather than about bays. A global letter survives only where the operation it
names has no operand, or where its only operand is the choice the key itself spells — seven keys of
the thirty-one this file binds today.

**Blocked on. Nothing.** The one uncertainty the record carried was whether `egui` swallows `Tab`
before the window loop's `match` sees it, which would have been the failure that looks like nothing
at all. It does not, and the reason is that `App::to_egui` reads `repaint` and nothing else —
`egui-winit` reports `consumed` for every `Tab` and this program never asks. That is now an
invariant a test holds.

**What it owes beyond the badges**, and none of it is drawing:

1. **The manual.** ADR-0259 edits none of it on purpose. `operations.html`'s key column stops naming
   letters and needs a spelling that says what it does name; `console.html` gains a mark for a
   folded bay holding focus, and its *Two focuses, and they do not look alike* note is rewritten by
   item 2; `concepts.html` has never mentioned focus and a reader needs it before the panel makes
   sense. `index.html`'s seven rules are capped and this fits inside rule 01 rather than adding one.
2. **The three pointers collapse.** `View::selection`, `View::cursor_row` and `View::scope` become
   instances of one mechanism — a bay's remembered address — which is the record's strongest
   structural result and the reason the deck selection persisting *"while your hands are in the
   library"* stops being a special property of the mixer.
3. **`map.rs` moves to `karakuri-map`.** A global key is a complete line exactly as a `cc` is, so it
   is a third `Key` beside `Cc` and `Note`; the crate holding the layer between a surface and the
   vocabulary then stops being named for a wire. Keys reached under focus are deliberately not
   customisable, which is a property rather than a gap.
4. **`key_column`'s machinery changes shape.** `KEYS`, `ROWS` and `NO_ROW` in
   `crates/karakuri/src/main.rs` are built around one letter reaching a fixed set of rows; under
   focus a digit reaches a different row in every bay. The check reads this file's own `match` arms
   as text, so a keyboard that becomes a per-bay dispatch table is invisible to it — which is the
   thing to solve rather than to discover.

**And it takes `z` with it.** *Bring back what is folded* unfolds everything rather than one region
because *"a region a pointer cannot reach is a region a key has no way to name either"*, and focus
can name a folded bay. The key may still earn its place as a bulk act; its recorded reason is gone.

#### Rows the manual has not given a home

Two rows carry a `plan` panel badge over a panel cell reading `—`: *Wire a procedure's input to a
node* and *Narrow the published interface*. Nothing on the console is specified to reach either, so
no bay holds them and none of M5.1 to M5.9 can close them. **The manual has to name a home before
they can be scheduled.** There is no drawing to owe until it does.

*Narrow the published interface* has a second blocker behind the missing control: it is upstream of
the Inspector, and until something publishes, every control that bay will ever draw is a wildcard.

**This list held three, and the third left it by being read again rather than by being decided.**
*Walk the edit history* was here because the operations page calls the surface over those files
*undo* and *a later milestone*, and undo is nobody's bay. It is not undo. A Set's history is a list
of the versions it has had, walking it is a listing, and landing on one is a load — all three of
which the Library bay already does — so the row is M5.3's, and what it needs is a lister rather than
a home. **A blocker that names a milestone rather than a mechanism is a blocker nobody can check**,
and this one survived two readings of this file before the question *what actually stops it* was
asked of it.

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

**Sizing has a third owed item, and it is one line.** The window has no minimum inner size, so it
can be dragged under the arrangement's own minima — where the solve scales everything down together
(ADR-0250) and the Mixer stops drawing strips while the deck previews carry on, so the pointer loses
the deck selection and the keys keep it.
[ADR-0272](adr/0272-the-window-has-a-minimum-and-only-one-of-adr-0250s-three-cases-is-real.md) takes
the decision and `karakuri-console` publishes the number as `MINIMUM_VIEWPORT` — 777 x 658.5, the
declared minima summed along each axis, with a test recomputing both from the tree, and
`crates/karakuri/src/main.rs`'s window attributes set it — so the constant is enforced rather than
claimed.

**And it left one thing undone that a window minimum could not reach, which is now done**:
`centre`'s declared minimum of 340 was `.body-grid`'s CSS track rather than a reading of its
content, and the inspector's `.param` grid wants 207 in a pane before its fader has any width — so
at a 340 centre the parameter faders were not drawn, and a divider drag reaches that centre at *any*
window width.
[ADR-0279](adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)
moved the arrangement instead of the window: a pane declares 208, the centre 2 x 208 + 9 = 425, and
`MINIMUM_VIEWPORT` is **777 x 658.5**. The 423 and 775 this paragraph used to name are one pixel per
pane short — the fader is the leftover track and wants *more* than the row's 207, where the mixer's
threshold is met *at* its 172 — and the record carries both measurements.

Nothing in this section is blocked.

#### M5.14 — The frame's cost

**Its subject is what a frame costs and who is told, and it is a sub-milestone because nothing else
here can hold it.** Everything below was recorded, verified and hooked into *Performance
discipline* — which `Continuous concerns` opens by saying are **not milestones**. So it was prose
under a heading that closes nothing, which is how four items came to float free of every exit
condition in this file.

**What it is for is drawn in the mock and drawn nowhere else.** The risk badge's five bands are
specified in [the console page](manual/console.html), under *What a deck preview cell shows, and
when* and on deck A's caption tooltip. **M5.1 closed on 2026-09-03 with the badge undrawn**, on the
reading that it is *"not a drawing that is owed"* because nothing could produce the estimate — and
nothing was scheduled to. That is this repository's own named failure, realised: an unfinished task
under a closed milestone is never picked up.

**Items, in the order they unblock each other.**

1. **A floor a caller does not have to supply.** `estimate` answers `Unfit::FloorUnknown` for every
   `Points` and `Lines` Set, because ADR-0245's sub-pixel floor needs `1/rate` and `point_rate` is a
   per-element vertex expression the CPU never reads. Closing it is a static analysis of that
   expression against its params' declared ranges. **Until it closes the estimate answers for almost
   nothing this instrument ships**, which is the precondition of item 4 being worth doing.
2. **An instrument that measures a frame rather than a pass.** Nothing has ever timed the frame's
   passes together — the panel's three medians are CPU time by construction and the wait beside them
   is excluded on a reading that holds only while the GPU is not the bottleneck. P-0095 asks what a
   number carries about how it was taken, and *the whole frame* has no answer yet.
3. **A deck total beside `committed_ms`.** Both it and `headroom_ms` sum only Live slots, so since
   ADR-0269 they understate the deck by three slots of step **and** draw. The measured case is in
   this file: a four-slot frame is 21 ms warm and was 209 ms cold, and **it registers as no violation
   at all**. Making `committed_ms` the deck's total would leave `over_budget` permanently on for any
   four-slot deck of heavy material, so what `over_budget` should then say is part of the item.
4. **`estimate` wired to `Deck::govern`.** Built on 2026-09-06 and exported from `lib.rs` to no
   caller. It is also the other half of the ceilings' sentence — *they are a backstop, not a budget,
   and the governor is what enforces* — so nothing is owed on the four ceilings themselves beyond
   this.
5. **The badge drawn**, which is what the four above are for.
6. **A region declares that it is moving, and not merely that something is pending.** **Done on
   2026-09-08** ([ADR-0283](adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)),
   and taken ahead of the five above because every control that lands writes the same unconditional
   read-back and the sites only multiply. A live region named two numbers — what its update costs
   and how stale it may get — and **nothing said whether it had changed**, so the mixer bay's roll
   was serviced at its declared thirty a second through the 600 ms of every second its curve is flat
   at zero: 31 frames asked for over a period where the motion needs 14. `Declared::moves_in` is the
   third number, `moves_in >= staleness` is why it can only ever remove a frame, and the beat is
   untouched because P-0094 forbids a panel that stops moving when nothing has changed.
   **What is left of it**: the mixer's level meter moves every frame and declares nothing, which is
   an under-declaration older than that record and named in it; and caching a bay to a texture,
   which this is the precondition for and item 2 above is the measurement for.

**Exit.** Not a badge grep, and that is the point: **the panel draws a dot whose band a test can
predict from a measurement.** Every other sub-milestone's exit is a column of
[every operation](manual/operations.html); this one's consumer is a readout rather than an
operation, which is exactly why no column could hold it.

**Blocked on.** Item 1 blocks item 4. Nothing else here waits on another bay, and item 6 waited on
nothing at all — which is why it was taken first.

**What ADR-0226 says, and what it does not.** M5 closes when M5.1 to M5.9 close and the two sections
at the end of this list close with them. **M5.14 is not in that list**, and neither are M5.10 to
M5.13 — the four columns and this are cross-cutting where M5.1 to M5.9 are bays. So M5 can close
with the badge undrawn exactly as M5.1 did, and whether it should is ADR-0226's question rather than
this section's.

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
**One clause is still owed, and
[ADR-0258](adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
carries it**: the rejected build's cell — black, or the sentence saying what went wrong. [The console
page](manual/console.html) specifies it under *What a deck preview cell shows, and when* and says of
it *"neither can happen yet, so nothing draws either word"*. **Half of that reason is wrong and the
conclusion is not.** The panel watches every slot's `.kir` pair, so a build can be refused while a
slot is running and the Staging lane draws that verdict; what cannot happen is the *cell* the page
describes, because a refused build installs nothing — a failed compile makes no `Request` and a
rolled-back trial leaves the previous Set running — so the honest picture after a rejection is the
material that is still there. `crates/karakuri-engine/tests/deck.rs`'s
`a_rejected_build_shows_what_is_still_running` asserts it.
**What is owed is the page's sentence**, and the open question under it is whether a cell ever
carries a diagnostic or whether saying what went wrong stays the Staging lane's.

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

**What it did not touch is the canvas, and it is what unblocked it.** Looking for what the canvas
was waiting on found that the model was not missing but four weeks old, and that its one restrictive
clause — *the window gets no vote* — rested on the picture depending on the render size, which this
work and `point_rate` between them removed. The size now belongs to the output
([ADR-0246](adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)),
and what made that safe is a rule:
[P-0086](principles/0086-a-procedure-knows-only-what-it-declares.md) — a procedure does not know
how large its target is, so the same Set at two sizes is one picture at two resolutions.

### The decisions nobody has taken

Three, each with what it blocks. Every one was found by building the thing next to it.

**Two have left this list since it was written, and neither by being decided here.** *What the deck A
preview cell is showing* is answered in the M5.1 paragraph above. *The instrument has no resolution
model* was not a decision at all: the model was
[ADR-0077](adr/0077-the-canvas-belongs-to-the-session-and-the-window-gets-no-vote.md)'s, four weeks
old and never cited from this file, and its one restrictive clause held only while the picture
depended on the render size — which `point_rate` and the sub-pixel floor removed.
[ADR-0246](adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)
gives the size to the output,
[ADR-0247](adr/0247-one-frame-is-rendered-and-scaled-into-each-output.md) says one render scaled into
each, and [P-0086](principles/0086-a-procedure-knows-only-what-it-declares.md) is the rule that
keeps both safe. What was left of it — **what names an output** — blocks only M5.6 and is an
implementation item of that bay rather than a standing question. So the risk badge's five bands and
the whole-frame budget below are not waiting on anybody's decision; they are waiting on an output to
have a size.

**`crates/karakuri/src/main.rs` still renders at `const CANVAS: (u32, u32) = (1280, 720)`** with no
flag, and its own doc says why: it is the workspace's reference workload, so a number taken in the
panel sits beside every other number in this repository. **Every cost figure in this file was
measured at that constant.** That is a measuring harness's default standing in for an instrument's,
and what replaces it is the largest enabled output's size, with whoever wants the reference workload
typing it. **What the reference workload now is** — `examples/drift_cloud.kset` at 1280x720, named
rather than taken from whatever the default pair happens to be — is
[ADR-0270](adr/0270-the-reference-workload-is-a-named-set-rather-than-whatever-the-default-pair-is.md),
which also says what it leaves undone here: a panel figure is a deck of four stepped and drawn
slots and a headless figure is one Set, so the sentence above about a panel number sitting beside
every other number in this repository holds only against other panel numbers.

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
  keeps `examples/` unwritten (P-0096). **The first symptom if it is wrong** is a `my sets` filling
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
[P-0095](principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md) asks of any number applies to
this one and has no answer yet.

**The gap widened on 2026-09-07 and the record says so rather than closing it.**
[ADR-0269](adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md) steps
every drawn slot on every frame, so an off-air slot now costs its L1 as well as its L4 and neither is
in `Report::committed_ms`, which sums the **Live** slots. That field says what it means and an
admission decision needs exactly it; what nothing computes is the deck's total, which is the
per-slot `Decision::cost_ms` summed. Making `committed_ms` that total would leave `over_budget` — the
governor's one warning — permanently on for any four-slot deck of heavy material, which is why it was
not done in passing. **The same change also measured the thing this item is about**: the four-slot
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
size. That belongs to M5.14 rather than to a limit here.

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
