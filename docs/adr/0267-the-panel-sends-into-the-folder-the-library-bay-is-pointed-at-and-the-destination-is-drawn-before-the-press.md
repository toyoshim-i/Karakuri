---
id: 0267
title: The panel sends into the folder the Library bay is pointed at, and the destination is drawn before the press
status: superseded
date: 2026-09-06
supersedes: []
superseded_by: [0311]
principles: []
tags: [ui, console, operations, surfaces, library, docs]
---

# The panel sends into the folder the Library bay is pointed at, and the destination is drawn before the press

> **Superseded 2026-09-09 by
> [ADR-0311](0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md).**
> The maintainer chose the system's save dialog: a send is asked for from a menu on a Library row
> and the destination is **the file the operator names in that dialog**, not the folder the bay is
> pointed at. The decision below — *the panel's control for Send is the `folder` scope, and sending
> writes `<id>.kbset` into the directory the `.path` row names* — is what that replaces, and it is
> the whole of this record's decision.
>
> **Three things here are not superseded and are still in force.** The `.path` row stays: it is the
> folder scope's own readout and what `folder` lists, which is ADR-0275's rather than this
> record's, and it is where the save dialog opens. ADR-0260 stands whole and is only quoted here.
> And **the page edit this record said was owed is withdrawn rather than done** — the sentence
> *"a folder is a way **in**"* did not have to turn round after all, because the folder scope is
> not bidirectional any more. A reader who comes here looking for that edit should find this note
> rather than a gap.
>
> **Annotated 2026-09-08: the directory this record waits on is settled, and by neither of the
> shapes named below.**
> [ADR-0275](0275-a-folder-is-chosen-by-dropping-one-on-the-window-and-the-drop-is-the-windows-rather-than-a-bays.md)
> (2026-09-07) decides that **a folder is chosen by dropping one on the window**, window-global
> rather than aimed at a bay. `Operation::ListSets { holds, layer }` is not missing a field — both
> are filters over what a store already holds, and a directory is *which store is asked at all*,
> which is the host's outside the operation entirely. That answers *What this does not decide →
> When the folder scope gets its directory, or what asks for it*, where this record left
> `ListSets` growing a field, a new operation and *something else* as the open list: it is the
> third. (What is owed for it is `crates/karakuri/src/main.rs` reading `dropped_files`, which that
> file names at its own site.)
>
> **This record's decision is untouched, and reason 1 is discharged rather than reversed.** Reason
> 1 reads *"the folder scope has no directory"* as an **ordering** and not an objection — *not yet*,
> not *not this* — and the ordering has since run out.
>
> **The `view.rs` quotation in reason 1 was that file's wording on 2026-09-06 and was exact then;
> the file now says the opposite.** `Scope::Folder`'s doc reads *"it waits on a **directory**, and
> not on an operation"* and cites ADR-0275, where the passage quoted below said *"it waits on an
> operation … so the chip waits on a row of the page and not on a decision."* The consequence that
> predicted that sentence *"gains a second thing waiting on the same directory"* is what moved
> instead: the row of the page it was waiting on turned out not to be the answer, and a drop on the
> window is.

## Context

[ADR-0260](0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)
settled what *Send a Set to somebody, and take one in* asks for: **sending is a read, its answer is
the package, and a read's answer goes where the surface that asked puts answers.** The operation
names no destination and none is owed. That decision stands, unchanged, and it is what makes this
record possible rather than what this record replaces.

What it deliberately left open is one section, headed *What this does not decide → The form of the
panel's control, and the two candidates*: *"The panel's destination is the surface's; **what the
surface asks with is open**, and the two candidates cost different things."* The two it set out were
a save sheet and the system clipboard, and it closed with *"Neither is chosen here. The console
page's, under `docs/contributing.md` §5 step 3."*

**Neither is taken.** What is taken is an alternative ADR-0260 had already written down and
rejected — (d), *The Library's `folder` scope as the outbox*, the one that record calls
*"the closest loser"*. So the argument here is not a new case for (d); it is what happened to the
three reasons it lost by.

### What (d) had going for it, in ADR-0260's own words

> The scope already exists as a drawing and a chip; a directory the operator pointed the bay at is
> a destination the operator named; writing `<id>.kbset` there would let the listing point at the
> file the moment it exists, which answers `view.rs`'s asymmetry — *taking in names a file that
> exists and sending names one that does not yet* — head-on; and it needs no dialog.

Every clause of that is still true, and the asymmetry it answers is
`crates/karakuri-console/src/view.rs:9182-9184`'s: *"The asymmetry is that taking in names a file
that exists and sending names one that does not yet — and no listing can point at a file nobody has
written."*

### The three reasons it lost, and what became of each

**1. *"The folder scope has no directory"* — an ordering rather than a reason.**

The fact is exact. `Scope::Folder` is *"A directory somebody names during the run"*
(`crates/karakuri-console/src/view.rs:8955`) and its doc says **it waits on an operation**:

> - [`Scope::Folder`] — **it waits on an operation.** *A folder scope reads Sets, and a bundle is
>   not a third thing*: *"no operation in the vocabulary can ask a folder for its listing"*, because
>   `Operation::ListSets` carries what a Set holds and has nowhere to put a directory. So the chip
>   waits on a row of the page and not on a decision.
>
> — `crates/karakuri-console/src/view.rs:8932-8936`

`crates/karakuri-operation/src/lib.rs:1313` is the payload: `ListSets { holds: Option<String>,
layer: Option<Layer> }`. `docs/manual/operations.html:624` says the same from the page's side —
*"It has nowhere to put a directory, which is what the library's folder scope waits on."* The panel
says it out loud at the chip: `why_nothing(Scope::Folder)` in `crates/karakuri/src/main.rs:5821-5826`
prints *"no operation in the vocabulary can ask a directory for its listing — `ListSets` carries what
a Set holds and has nowhere to put a folder"*.

**But that is a fact about what is built, not an objection to the shape.** The bay cannot be a file
browser without a directory *either way*: the chip is drawn, the listing under it is
`Scope::Favourites | Scope::Folder => Vec::new()` (`crates/karakuri/src/main.rs:5896`), and the same
work is owed whether or not a send is ever drawn there. A reason that is discharged by work already
owed for another purpose is an **ordering** — it says *not yet*, not *not this*. What it costs is
below, stated as plainly as what it buys.

*(ADR-0260 attributes those words to `crates/karakuri/src/main.rs`. They are
`karakuri-console/src/view.rs`'s; what the host file has is `why_nothing`, quoted above, which says
the same thing in different words. The argument is unaffected.)*

**2. *"It makes the destination a property of console state … that a map cannot see"* — the state
is drawn, and the surfaces that cannot see it do not take this route.**

The premise is right and the conclusion does not follow, for two reasons that have to be checked
separately.

*The state is on the screen.* In a file browser, where a file goes **is** where you are browsing.
The console page already draws that: `docs/manual/console.html:108` is a `.path` row in the Library
bay reading `~/sets/tour-2026/night-b › opening`, and `view.rs` names it as one of the mock's
elements this crate has not built yet — *"**The `.path` row**, `~/sets/tour-2026/night-b ›
opening`. It is the walk *inside* a folder scope, so it says nothing until that scope can be asked
for a listing at all"* (`crates/karakuri-console/src/view.rs:9082-9084`). So the destination is not
hidden console state; it is a line of text above the list, in the specification, waiting on the same
directory the chip is.

That is what a control owes here. [P-0090](../principles/0090-a-surface-offers-it-never-decides.md)
is *"A control decides **what a press asks for**"*, and the phrasing the panel has settled on for
that is one bay over, on the load pill in this same foot: *"what may be asked for is the
instrument's to decide, so the letter is what says where it lands before you press it"*
(`docs/manual/console.html:137`), which
[ADR-0265](0265-a-carried-set-names-its-deck-at-the-release-and-the-panel-refuses-no-drop.md)
generalised at lines 140-141 — *"What the control owes instead is to say where it lands before the
press, and on this route that is the strip under the pointer."* The `load → A` pill names the deck
before the press; the `.path` row names the directory before the press. Same bay, same rule, one row
apart.

*The surfaces that cannot see it do not take this route.* **MIDI** is `gap` on this row
(`docs/manual/operations.html:670`), and `gap` in that page's legend is *"nothing — this surface
cannot reach it at all"* (`docs/manual/operations.html:60`). ADR-0260 chose that rather than
inheriting it: *"MIDI is `gap`, and chosen: a read's answer has nowhere to land on a controller"*,
and `crates/karakuri-midi/src/map.rs:53-54` is why — a line can say *"a slot number, a value word out
of a list, or a trailing `[lo, hi]`"*. A map cannot see the path because a map is not asking.
**MCP** is handed the bytes: ADR-0260's consequence is *"the `.kbset` text back as the tool result,
no path in either direction"*. That badge is `plan` and no such tool exists yet —
`crates/karakuri-environment/src/mcp.rs` constructs no bundle and names no `TransferSet` — so this is
the decided shape rather than the built one. **The CLI** is `has` and prints to stdout. Four
surfaces, four answers, and only the one that draws a file browser reads the file browser.

**3. *"`console.html` settled the scope's direction"* — this one survives, and it is what the page
owes.**

`docs/manual/console.html:1715-1716`, inside the note *A folder scope reads Sets, and a bundle is not
a third thing* (`console.html:1670`):

> it is the same order the scope list is drawn in — a folder is a way <em>in</em>, and <em>my
> sets</em> is where things are.

And one paragraph up, at `console.html:1690`: *"**So a folder row is a *take*, and the reader for it
already draws the line the scope was waiting on.**"* The note is one-directional on purpose and by
argument.

**This record does not talk its way past that. The page has to change**, and under
`docs/contributing.md` §5 the page moves first — step 2 is the operations row, step 3 is the
control on the console page, and only then the crates. **So this record is ahead of the page until
that edit lands**, and it says so rather than reading as a description of a tree that matches it.

## Decision

**The panel's control for *Send a Set to somebody* is the Library bay's `folder` scope. The bay is
already a file browser; the directory the operator has the bay pointed at is the destination, and
sending writes `<id>.kbset` there.**

`<id>.kbset` is the store's own naming rule and nothing is invented — `<store>/sets/<id>.kbset` is
the row for the operator's library in
[P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md), and
`crates/karakuri-environment/src/mcp.rs:2810` spells it `<dir>/<id>.kbset`.

Neither of ADR-0260's two candidates is taken. That record's decision is untouched: the operation
still names no destination, `SetTransfer::Send { id }` gains no field, and what is decided here is
only *what the surface asks with*, which is the question ADR-0260 left to the console page.

**The bay is already a file browser, and that is a fact about the built program rather than an
aspiration.** `presets_listing` in `crates/karakuri/src/main.rs:5753-5764` reads a told directory and
answers `{ id, path }` per file; the panel prints at startup that the bay *"lists the {} `.kset`
file{} in it … and `l` on one takes it into the store and then loads it"*
(`crates/karakuri/src/main.rs:2472-2478`). The `folder` scope is that same listing over a directory
the operator names instead of one the program was told. A browser that can open a file from a
directory is a browser that can put one there.

### What the shape buys

- **It closes the loop, which is the thing the asymmetry asked for.** `view.rs:9182-9184` says no
  listing can point at a file nobody has written. Write it where the listing is looking and the
  listing points at it on its next read — and the receiving half is right there:
  `SetTransfer::Take { file: PathBuf }` (`crates/karakuri-operation/src/lib.rs:659`), whose panel
  route is a press on a row of exactly this kind of listing. Sending and taking in become the two
  directions of one bay, which is what the row's own heading has always claimed they were.
- **No dialog, so no dependency and nothing to cost.** `Cargo.lock` contains no `rfd` — checked, it
  is absent — and the chip, the list and the `.path` row are all things the console page already
  draws.
- **No modal window over a live instrument**, so
  [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
  never has to be argued out. ADR-0260 stated the save sheet's P-0094 problem fairly and stated the
  counter-reading fairly too — *"Nobody has argued that out."* Nobody now has to: this control opens
  no window, and the question of whether a rule about mechanisms that run *during* a performance
  reaches a control nobody presses during one stays unasked rather than answered badly. P-0094's
  *"a window that opens and cannot be touched"* is in that rule's own list of what it rules out.
- **It completes on a key press.** Under
  [ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
  the Library's items are the listed Sets, digits name the first nine, and *"`enter` on a row is the
  load onto the selected deck"* (lines 247-248) — with *"**Where an item has two acts they are two
  controls and a digit chooses between them**"* (lines 157-158) for a row that gains a second. A row
  in a `folder` scope with a destination already on screen needs no letters typed, which matters
  because rule 01 of `docs/manual/index.html:45-56` is *"Every operation is reachable from the
  panel, from the keyboard alone, from a mapped MIDI control, and from a model over MCP"* and this
  row's `key` badge is `gap` today. Whether that badge moves is the page's under §5 step 2; what
  this record contributes is that the control it describes does not stand in the way.

## What it costs

**`Scope::Folder` needs a directory, and no operation can carry one.** `ListSets { holds, layer }`
has nowhere to put it, `karakuri-console` cannot do a directory read at all
(`view.rs:9071-9072`: *"a listing is a directory read, a frame path does not do those (P-0091), and
this crate could not do it anyway (ADR-0156)"*), and the host answers the folder scope with
`Vec::new()`. **So the send now waits on that work**, and the wait is real rather than nominal: this
is the same dependency the chip has been carrying, and this record adds a second thing hanging off
it.

**That dependency has no home in the schedule.** `docs/roadmap.md` mentions the folder scope exactly
twice — line 495, *"the list already carries **presets** beside favourites and a folder, and says the
scope list is itself extensible"*, and line 498, the tooltip inventory naming the note *A folder
scope reads Sets, and a bundle is not a third thing*. Neither is a task. There is no milestone row
for *give the folder scope a directory*, and no *Blocked on* entry naming it: M5.3's reads
**"Blocked on. Nothing, as of 2026-09-05."**

**And M5.3's exit condition now rests on this row alone.** That exit is *"No `plan` badge in the
panel column of this bay's rows on [every operation](manual/operations.html)."* *Load material into a
deck* carries `panel has library → deck` (`docs/manual/operations.html:202`); *Read what one Set
holds and declares* and *List what the store holds* are `has`; **the one remaining `plan` panel badge
in this bay is this row**. So the folder directory is on the critical path to closing M5.3 and is
scheduled nowhere. Naming it is the honest half of taking this decision, and the roadmap pointer
`docs/contributing.md` §4 asks for is where it stops being only this record's problem.

## Alternatives rejected

### a. A save sheet — the platform's own ask for a place

ADR-0260 set this out in full and chose neither candidate. Its reason for losing **now** is not that
record's list; it is this one: **it buys a dependency and a modal window to reach a place the bay can
already name.** `Cargo.lock` has no `rfd`, and
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) says that bill is
paid now rather than designed around — so the question is what the bill is *for*. A sheet's whole
value is naming a directory the program does not know, and the bay's `.path` row is a directory the
program does know, drawn on screen, one row above the list the file will appear in. Paying a
dependency and opening an operating-system window to ask a question the panel is already displaying
the answer to is the wrong trade, whatever P-0094 turns out to say about the modal — and this way
that argument is not needed at all.

**What it had going for it, kept honest**: it names a file that does not exist yet, which is the one
thing ADR-0260 correctly says no listing can do, and it takes no rule from ADR-0221 because it is the
operator's filesystem asked by the operator's own tool. The answer is that the listing does not need
to name the file — the *scope* names the directory, and the file's name is `<id>.kbset`, which the
store already decides.

### b. The clipboard

**It loses because nothing in this system takes text.** `SetTransfer::Take { file: PathBuf }` names
a file; `--take-in FILE` reads one (`crates/karakuri-cli/src/main.rs:564`, `:1587`); the panel's
built receiving route is a press on a row of a listing. So `Take` cannot consume what a send would
produce, and a send onto the clipboard is a read whose answer no route in this program can receive —
the send half would work and the pair would not. ADR-0260 said the same from the other end: *"the one
built receiving route, `--take-in FILE` and a `folder` row, takes a **file**, and bytes on a
clipboard are not one."*

It is worth saying what this does *not* rest on. `karakuri_environment::setfile::unbundle` takes
`&[Line]` rather than a path (`crates/karakuri-environment/src/setfile.rs:1824`), so the *mechanism*
could read text; what cannot is the operation and every route to it. And `arboard` 3.6.1 is in
`Cargo.lock` — transitively, through `egui-winit`, named as a direct dependency in no manifest in
this workspace, checked — so the cost objection is small and is not the objection. `SetTransfer::Take`'s
own doc leaves the bytes question open — *"whether a route that has no filesystem — a model handing
over the text — takes bytes instead is open"* — and this record does not close it; it observes that
today the answer is no, and a control cannot be built on an answer nobody has given.

### c. A place in the store — `<store>/out/<id>.kbset`, or `my sets` itself

ADR-0260's (b), and it loses now for the reason it lost then, which has not weakened: *"a second copy
of a Set sitting beside the Set is a second answer to which of them is the file"*, and *"it does not
answer the question: the file still has to leave the machine, so the panel would have to show the
operator a path to fetch it from, which is the control it does not have wearing a different hat."*
The folder scope **is** that control, which is the whole of this record — so (b) is now not merely
losing but redundant.

**Sending into `my sets` is not a fifth candidate.** `my sets` is `<store>/sets/<id>.kbset`, which is
the file `bundle` read; writing the bundle back there is ADR-0260's alternative (e), *the store holds
bundles*, reached from the other end, and that record disposed of it on the store's own layout — the
Set file is a projection and the artifacts are the material
([ADR-0007](0007-a-set-file-is-a-projection-and-a-session-stream-is-the-timeline.md), ADR-0229
part 5). It is also the wrong direction by `console.html`'s own surviving sentence: *my sets* is where
things are, not a way out.

### d. Wait for the larger folder question rather than decide the form against it

The status quo, and the most respectable of these: the send is blocked on a directory either way, so
deciding its form now is deciding against a dependency nobody has scheduled, and a form decided early
is a form decided with less information.

**It loses because the wait is the same length and buys nothing.** The directory is owed for the chip
regardless; waiting does not make it arrive sooner, and it leaves the *form* open for exactly as
long. What an open form costs is on the record already: `view::LibraryBay` wrote five paragraphs
explaining why it draws nothing (`view.rs:9168-9212`), the roadmap carried a *Blocked on* in the same
words, and ADR-0260 was written to end that — it moved the row from a decision to a build. Leaving
the control undecided keeps a second decision alive in its place, to be re-argued by whoever reads
that paragraph next. And it inverts §5: the page moves first, so a form nobody has chosen is a page
nobody can write, and a page nobody can write is a build nobody can start.

## Consequences

- **`docs/manual/console.html` moves first, and the sentence that has to change is named.**
  Line 1715-1716's *"a folder is a way **in**, and **my sets** is where things are"* and line 1690's
  *"So a folder row is a *take*"* are one-directional by argument, and this decision makes the folder
  scope bidirectional. The note is *A folder scope reads Sets, and a bundle is not a third thing*
  (`console.html:1670`). The `.path` row at line 108 carries no `data-tip` today and is where the
  destination is said out loud; the `folder` chip's tip at line 105 says what a folder row is and
  would gain the other direction. **Until that edit lands this record is ahead of the page**, which
  is `docs/contributing.md` §5's order and not an omission here.
- **`docs/manual/operations.html` step 2.** The row's tip at line 663 ends *"and what nothing draws
  is the sending"*, which stops being true when the control is drawn. Whether the panel badge moves
  from `plan` to `has`, and whether the `key` badge moves off `gap` given ADR-0259's walk and rule
  01, are that page's calls under §5 step 2 — this record names them rather than making them.
- **Nothing in `crates/` changes on this record.** `SetTransfer` keeps both arms and their fields,
  `Operation::TransferSet` is untouched, `gate.rs`'s `Standing::Open` stands for ADR-0260's reason,
  and `karakuri-operation-record` still answers `Silent(NoRecord)`. The four tests §5 names —
  `karakuri-console/tests/panel_column.rs`, `karakuri-operation/tests/the_manual_and_the_vocabulary_agree.rs`,
  `karakuri-console/tests/vocabulary.rs` and `karakuri/src/main.rs`'s `key_column` — read the pages,
  so they are what will hold the page edit and the control together when they land.
- **`view::LibraryBay`'s *"Nothing here writes a Set out"* section acquires an answer to its second
  half.** ADR-0260 answered *where does a package go* by refusing the premise; this answers *what the
  panel asks with*, so the paragraph at `view.rs:9205-9212` should point at both records rather than
  restate the wait.
- **`Scope::Folder`'s doc gains a second thing waiting on the same directory.** Today it says the
  chip *"waits on a row of the page and not on a decision"* — after the page edit that is still true
  and the row it waits on has grown a second column.
- **M5.3 owes a *Blocked on* entry it does not have.** It reads *"Blocked on. Nothing, as of
  2026-09-05."* The vocabulary question is genuinely gone, which is what that line was about; the
  build item that replaced it now has a named dependency — `ListSets` has nowhere to put a directory
  — with no milestone row anywhere in `docs/roadmap.md`. Under §4's *hook it from where the work is*,
  the roadmap gets the pointer, beside the item, including the sentence naming what is left undone.
- **The two candidates ADR-0260 named are closed rather than deferred.** Anyone reading that record's
  *What this does not decide* should arrive here. The save sheet and the clipboard each have a reason
  of their own above, so re-proposing either means answering those rather than re-running ADR-0260's
  cost comparison.
- **This is a decision and not a principle.** Two plausible alternatives lost — both were written
  down as live candidates by the previous record, which is as plausible as an alternative gets — and
  the rule decides one control in one bay. Under
  [ADR-0249](0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)'s gate there is no
  question outside this bay that reading it settles; the general rule it leans on, *a control says
  where it lands before the press*, is P-0090's and ADR-0265's and is not restated as a new file.

## What this does not decide

- **When the folder scope gets its directory, or what asks for it.** `ListSets` growing a field, a
  new operation, or something else is `docs/manual/operations.html`'s and `karakuri-operation`'s, for
  the reason `view.rs:9007-9008` gives about what a scope is named *by* — *"a folder scope has a
  path, `presets` has a root the program was told"* — and that sentence is still unwritten.
- **What the `.path` row's walk is.** Whether it browses, whether it can leave the directory it was
  given, and how a directory is chosen in the first place are the console page's.
- **Whether a send refuses an existing file, and in what words.** A `<id>.kbset` already in the
  destination folder is a collision, and taking a Set in already refuses an id the store holds
  (`crates/karakuri-environment/src/setfile.rs` and the `--take-in` tests). What the send does is the
  page's, and a refusal is
  [P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)'s shape either way.
- **The Library walk's second act under ADR-0259.** Named by ADR-0260 already: a row's load and send
  become two controls a digit chooses between, or that record grows a rule that a row's first act
  keeps `enter`. That record's and the page's.
- **Whether `Take` takes text over MCP.** Unchanged from ADR-0260. This record's clipboard argument
  observes that nothing takes text *today*; it does not close the question.
- **The row's `key` and panel badges.** §5 step 2, the page's, named above.
