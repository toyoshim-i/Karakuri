---
id: 0226
title: M5 closes when the manual is implemented, and the meter is progress rather than completion
status: accepted
date: 2026-08-30
supersedes: []
superseded_by: []
principles: [0030, 0087, 0093]
tags: [process, docs, console]
---

# M5 closes when the manual is implemented, and the meter is progress rather than completion

## Context

**The milestone had three descriptions of itself and no exit condition.** Each of the three is
load-bearing where it sits, and none of them answers *is it finished*.

**One — a goal sentence.** `docs/roadmap.md`'s `### M5 — Interface` opens *"Goal: the
application."* The same file says, in its own words, that no item of the milestone ever named
building one: *"This milestone opened **Goal: the application**, and nothing in it named building
one. Not the seven Adds, not the four items below, not the decisions after them."* A goal no item
names cannot be the test for whether the items are done, and this one was written to say what the
milestone is *about* — the paragraph under it is a correction of an earlier sentence that had sent
it in the wrong direction.

**Two — a seven-item *Adds* list**: the node editor, parameter surfaces with MIDI learn, the Set
browser, the staging lane, `man / sug / auto`, the sync toggles, and an arrangement saved and
restored. The roadmap disowned it as a measure with the reason attached: *"The list below is the
second half of the work, and reading it as the milestone is what makes the progress invisible."*
Not one of the seven was finished when that was written and one is now
([ADR-0221](0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md),
[ADR-0225](0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)), while
`karakuri-layout` and `karakuri-console` were written from nothing in the same window and appear on
no line of it.

**Three — a meter.**
[ADR-0213](0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
made the panel column of [every operation](../manual/operations.html) the milestone's progress
meter, deliberately derived so nothing transcribes it, and the roadmap defers to it outright:
*"How far the milestone has got is not written down anywhere in prose, on purpose."* It answers
*how far*, which is a different question from *how far is far enough*, and ADR-0213 does not claim
otherwise — it names no figure at which the milestone is over.

**Nothing said which of the three closing means M5 closed**, and the file has the shape of the
answer for a milestone that did close, so the absence is visible rather than inferred: `### M4 —
Library at scale — **closed**` carries a paragraph naming what closed (*"the machinery, tested at
the scale that exists"*), what could not be checked, and what is still owed. M5 has none of that,
and the meter's own instruction is *"Run it before believing any sentence in this file about how
much is left"* — how much is left, against no stated end.

## Decision

The maintainer's, on 2026-08-30, in two statements, the second sharpening the first:

> M5の終了はマニュアルのモックに描いたBayを全て実用的なレベルで実装すること。それを使ってみて足りない
> 機能をまた次の仕事として考えて行くので。まずは使えるアプリにする。

> 少なくともマニュアル記載の機能はすべて実装だね。その上でつかって足りないものは埋める。なので、M5
> マイルストーンとしてはマニュアルにあるものを実装がゴールで良いよ

**M5 closes when everything the manual describes is implemented.** Then he uses it and fills in
what is missing, which is the next job. First, make it an app you can use.

The first statement named the mock's bays; the second names the document they are drawn in, and it
is the same condition said precisely. **The manual is already the specification the panel is built
to**, so choosing it as the finish line adds no new artefact:
[every operation](../manual/operations.html) is the vocabulary and every way into it, and
[the console page](../manual/console.html) is the mock and what each region is for.

### What that is, concretely, and it is a `grep` rather than a judgement

**The panel column of the operations page holds no *designed* badge.** The page's own legend gives
the three words: *built — this surface reaches it today*; *designed — this surface is meant to
reach it and does not yet*; *nothing — this surface cannot reach it at all*. So the condition is
that every `designed` in that column has become `built`, and the rows marked `nothing` stay as they
are, because the page has already said the panel cannot reach them.

```sh
grep -o 'class="rt [a-z]*">panel ' docs/manual/operations.html |
  sed 's/.*rt //;s/">.*//' | sort | uniq -c
# 2026-08-30:   3 gap   14 has   38 plan
```

Fifty-five rows, one panel badge each. **Thirty-eight to go**, and the split inside them matters:
**thirty-five name the region their control lives in** — `inspector`, `library`, `staging`,
`transition row`, `deck head` and the rest, which is the board ADR-0213 derives with its second
command — and **three name none yet**: *Wire a procedure's input to a node*, *Narrow the published
interface* and *Walk the edit history*. Naming a home for those three is an edit to the
specification rather than a note taken while counting
([ADR-0197](0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md)),
and it is part of this milestone by the same reading that makes the manual the condition. The three
marked `nothing` are *Set a deck's mask position*, *Edit the file instead* and *Bring back what is
folded*.

### Why it is checkable, which is the point of choosing it

**Three tests already hold the panel against the page**, so this exit condition asks for no
instrument that does not exist —
[P-0093](../principles/0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md) is met by what is already
running rather than by a promise:

- `crates/karakuri-console/tests/panel_column.rs` — the panel column against the operations this
  crate's controls emit, in both directions, so *a control reaching past the page* and *the page
  claiming a control that does not exist* both fail. ADR-0213 said the first `has` owed exactly
  this test; it is written, and it reads the emissions out of the crate's own source rather than
  from a list somebody keeps.
- `crates/karakuri-console/tests/vocabulary.rs` — `panel::Op` against the *Arranging the console*
  rows, which is the same question one level out, and which pins the three rows that have no `Op`
  and the one `Op` that has no row.
- `crates/karakuri-console/tests/page_totals.rs` — the page's own summary against its own badges,
  written because *"three times in three days a badge moved and a sentence about how many badges
  there are did not."* **Deleted on 2026-09-01.** The page became a matrix and states no count in
  prose, so the drift this test caught cannot occur; what it guarded is now read by the commands
  under *The meter is one column*. The other two above still exist and still read the page.

So a claim that the milestone is over is a claim about a file that a suite already refuses to let
drift. That is what the alternatives below cannot offer.

### The mock's side of the manual, and how many bays it is

The console page is the other half of what *"everything the manual describes"* covers, and it is
not counted the same way. Its mock heads **seven** regions with a `.bay-head` — Library, Staging,
Program, Inspector, Mixer, Master and Sequencer, the last spelled `.seq-head` in the markup —

```sh
grep -o 'class="bay-head">.\{0,40\}' docs/manual/console.html
```

— while the console's **nine** named regions are `transport`, `library`, `staging`, `program`,
`inspector`, `mixer`, `master`, `sequencer`, `outputs` (`crates/karakuri-console/src/lib.rs`, *"the
manual's word for the region, because a name is addressable from four surfaces"*). The two that are
not bays are the transport and outputs strips, which the mock draws with no head; both are drawn
and both answer a hand today — five of the fourteen `built` panel badges name `transport`, and the
Outputs dot is drawn and pressable with its badge left `nothing` deliberately, because the dot
folds the program view and the row names choosing among sinks.

**It is P-0030's sentence taken at the width of the panel.** ADR-0213 used *"a window that opens
and cannot be touched is a demo, not a tool"*
([P-0030](../principles/0030-an-instrument-says-what-it-did.md)) to define one badge — an operation
is claimed when a person who launched the instrument can perform it. *Make it an app you can use*
is the same test asked of every badge at once.

### It is a completion condition and deliberately not a schedule

It says what has to be true, not when, and not in what order. The order stays what the survey under
the roadmap's *Where this goes next* makes it — *what decides which comes next is whether the
values behind it exist anywhere in this workspace* — and
[ADR-0200](0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md) stays the rule
for a bay's **first** pass. This record only says where the passes stop.

## Alternatives, and both are plausible enough to be re-proposed

### a. The seven *Adds* close it

The obvious reading, because the list sits under the milestone's own heading and reads like scope.

**It loses on the roadmap's own sentence about itself**, which is a finding rather than a note:
reading the list as the milestone *"is what makes the progress invisible"*, because every one of
the seven *"assumed a panel to be drawn into, a name to be routed by, and a frame to be drawn on,
and none of the three existed when the list was written."* A milestone measured by that list read
zero through the five days in which the layout crate, the console crate, the operation vocabulary
and its record crate were written from nothing.

**And it loses on a second count the roadmap does not make: a list of additions says nothing about
whether what exists is usable.** The Library bay draws a listing in every run of `cargo run -p
karakuri` — `view::library` off `Store::list_sets`, sorted, with the foot's `n of m` — and there is
no way to get a row of it onto a running deck. `Operation::LoadSet`'s panel badge is `designed`
with the home `library → deck`, and under it `Deck::install` is the one function that puts a Set in
a slot: *"deliberately not reachable from a key or a surface"*
(`crates/karakuri-engine/src/deck.rs`). Not one of the seven names that gap. Finish all seven and
the library still cannot load.

*(Closed the same day this record was written, and the closing is the argument rather than a
correction to it. The route was never `install`: a live run changes its material by handing it to
the worker and letting the budget watchdog watch it, so what was missing was a way to re-point a
slot's source — `watch::Aim`, and `l` on the operations page. The seven `Adds` still do not name
it, which is the point this section makes.)*

### b. The meter closes it at some figure

Sharper than (a), because the meter is derived, is already the thing to run before believing a
sentence, and would give a number to stop at. It loses twice.

**A bay is not a route, so the figure is blind to whole regions.** Read on 2026-08-30 with
ADR-0213's own command:

```sh
for col in panel key MIDI MCP CLI; do
  echo -n "$col: "
  grep -o "class=\"rt [a-z]*\">$col " docs/manual/operations.html |
    sed 's/.*rt //;s/">.*//' | sort | uniq -c | tr '\n' ' '; echo
done
# panel:  3 gap  14 has  38 plan
```

- **Not one of the fifty-five rows names the sequencer.** `grep -i sequencer
  docs/manual/operations.html` returns nothing, and the board has no group for it. The Sequencer
  bay can be built in full and the meter does not move; it can stay a bare head and the meter does
  not notice. Its lanes emit `SetOpacity` and `WriteParam`, and `SetOpacity`'s panel badge already
  reads `built` from the strip fader
  ([ADR-0222](0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md) is why a lane emits
  those rather than being a thing of its own).
- **The Master bay was invisible the same way until this morning.** Before `cf86ce4` the page had
  **54** rows and **13** built panel badges; it has 55 and 14 now, and the new row is *Master out*,
  drawn as the bay's whole body.
- **A drawn bay contributes nothing on its own.** Library, Staging and Inspector all draw a body,
  and the nine rows homed in them — `library` ×3, `staging` ×2, `inspector` ×4 — are `designed`,
  every one. *List what the store holds* is `designed` while the Library bay lists what the store
  holds in every run: the reading exists, the route does not, and the meter reads zero for a region
  an operator already uses.

**And the figure would be a moving target.** A threshold needs a denominator that stands still, and
this one is designed not to: the page gained a row this morning and will gain more — the Sequencer
has none at all, and the Master bay's three effects are named once, inside the *Master out* row's
prose, with no rows of their own. *N of M* with M rising is not a condition anybody can meet, and
the rise is the method working rather than failing (see the first consequence below). The condition
this record takes has no denominator in it: **no `designed` badge left in the column** is true or
false whatever the page's length.

That is [P-0087](../principles/0087-name-the-property-never-the-shape.md) one level up from where
ADR-0213 used it: a count of badges is the shape, and *nothing the specification describes is
unreachable* is the property.

## Consequences

- **The specification's own unfinished parts are inside the milestone, and that is the method
  rather than a defect.** The mock draws a Sequencer bay the operations page names no operation
  for; the Master bay's feedback, bloom and rgb shift appear once on that page, in another row's
  prose. **That is the ordinary state of a document that leads the code** — `docs/roadmap.md`'s
  *The manual is the reference, and it is written before the panel* says the manual was written
  *"ahead of the implementation on purpose"*, because *"a sentence you cannot write about a control
  is a control designed wrong."* What the exit condition changes is not what the manual is; it is
  that the method becomes **countable**. The page can gain rows and the panel can gain controls,
  and until now no single number moved in a way a reader could act on. With the manual as the
  finish line, the distance between the document and the program is what the meter measures.
  Today's controls are that order read off the page itself: *"these badges specified a control that
  did not exist, the page that owns the console's words drew it next, and the panel was built to
  that. Nothing was designed at the keyboard"* — and the Master out row *"arrived by the same order
  read from the other end"*, the console page having drawn that fader since before anything could
  move it.
- **[ADR-0213](0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)
  is not superseded, and a record that read the two as rivals would be wrong.** The meter is
  progress — how far each surface has got, one badge at a time, derived and unmaintained — and this
  is completion: whether any `designed` badge is left in the one column. The same column answers
  both, which is why they are not rivals; they are two readings of it, and the meter stays the
  thing to run before believing any sentence about how much is left.
- **The number ADR-0213 shipped with, 0 of 46, is `3 gap 14 has 38 plan` today**, read by that
  record's own command on 2026-08-30 and transcribed nowhere else; the command is the citation, and
  the figure here is dated for the same reason
  ([P-0088](../principles/0088-no-number-is-trusted-further-than-its-instrument-has-been-checked.md)).
- **Three rows naming no home are work rather than exclusions.** *Wire a procedure's input to a
  node*, *Narrow the published interface* and *Walk the edit history* each need a home named on the
  page before a control can be built to it, and ADR-0197 already established that naming one is a
  decision of its own.
- **What M6 contains is not decided here.** The maintainer's second sentence names the next job's
  *source* — he uses the finished app and fills in what is missing — and not its content. Nothing
  here moves an item between milestones; that is the maintainer's.
- **`docs/roadmap.md`'s M5 section owes a pointer to this record**, beside its goal sentence and
  beside the *Adds* list, which is `contributing.md` §4's *hook it from where the work is*: a
  milestone that now has an exit condition is exactly a record that changes what is still owed. It
  is not added here because that file is held by another session in flight.
- **`docs/adr/INDEX.md` owes a row for this record**, not added here for the same reason.
