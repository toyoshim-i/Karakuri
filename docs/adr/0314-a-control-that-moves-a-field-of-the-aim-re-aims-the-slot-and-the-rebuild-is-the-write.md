---
id: 0314
title: A control that moves a field of the aim re-aims the slot, and the rebuild is the write
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0085, 0090, 0091, 0094]
tags: [console, engine, watch, swap, m5.5]
---

# A control that moves a field of the aim re-aims the slot, and the rebuild is the write

## Context

[roadmap.md](../roadmap.md)'s M5.5 listed four rows this bay owed and said what each was blocked
on. Two of those sentences were about the engine and both were wrong — not about the engine, which
they described correctly, but about what the description implies:

> ***Composite a deck's renderers* waits on a setter.** `Set::merge` is `Some` only where
> `layering == Layering::Composite` at `Set::build`, nothing writes it afterwards, and neither
> `Set` nor `Deck` offers one, so the operation names a state the engine cannot be moved into
> while running.

> ***Element capacity, seeds, the camera* is two-thirds blocked and one-third undecided.** A
> capacity sizes buffers at build and a salt is assigned there; `Set::source_capacities` and
> `Set::source_salts` are readers with no writers.

Every fact in those two paragraphs is true. `Set::merge` is written at `Set::build` and nowhere
else; `source_capacities` and `source_salts` have no writers; `Deck`'s public surface reaches none
of the three. What does not follow is *blocked*, and the reason it does not follow was written down
eleven days earlier, one bay over.

**A library load is the same shape and it is built.**
[ADR-0228](0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md) closed
exactly this question for a slot's *material*: the way this instrument changes what a slot is
running is not a setter on a live `Set` — it is a **re-point**, a `watch::Aim` sent down a channel
to the build worker, which recompiles the slot off the render thread, swaps it at a frame boundary
and lets the watchdog judge it against the budget for thirty frames. Nothing installs, nothing on
the render thread allocates, and a build that cannot hold the frame rolls itself back onto what the
deck was playing.

**And the layering, the capacity, the seed and the salts are fields of that aim.**
`karakuri-environment/src/watch.rs`'s `Aim` is fourteen fields, and four of them are the numbers
these two rows are about:

```rust
pub struct Aim {
    pub head: …, pub rest: …,
    pub layering: karakuri_engine::set::Layering,
    pub live: Option<u32>,
    pub capacity: Option<u32>,
    pub seed_salt: u32,
    pub salts: Vec<u32>,
    pub camera: karakuri_engine::camera::Orbit,
    …
}
```

`Watch::repointed` destructures an incoming aim with **no `..`** and re-seeds the watcher from it,
and the next `Source::poll` returns a build. So the mechanism a control over any of these fields
needs already exists, has existed since `13e27ec`, is exercised by every library load and every
`wire_procedure`, and is the one this program was already reaching for
([P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).

**What made the wrong reading easy** is that the sentence stops one clause short of its own answer,
which is the shape ADR-0228 recorded about `Deck::install`: *"the missing piece was never a way to
install a Set. It was a way to re-point a slot's source."* Read as *the engine has no writer*, the
row is blocked; read as *the engine has no writer, because a live write is not how material
changes here*, the row is a control nobody had drawn.

**The two source rows are a separate question and the maintainer answered it.** *Read one node's
source* and *Check and write one node's source* both carried a `plan` badge over a panel cell
reading `inspector`. On 2026-09-09, asked whether the panel owes them:

> モデル専用のまま (panel は gap)

## Decision

### 1. A control over a field of the aim is a re-aim, and the rebuild is the write

`Aiming::changed(|at| …)` in `crates/karakuri/src/main.rs` is the one derivation: it applies the
change to the aim this program is holding, publishes the result where the MCP server reads it
([ADR-0309](0309-a-slots-files-are-published-where-its-aim-is-sent-and-the-server-reads-them-live.md)),
and sends `restated(&self.at)` — the fourteen-field restatement written with no `..` on either
side, so a field added to `Aim` stops this compiling rather than being quietly left behind.
`Aiming::re_aim` is now that call with `edges` named, which is what `rewired` uses; it was the
whole function before, and generalising it cost one closure.

**A caller does not build an aim.** It says which field moved. The restatement is what makes a
re-aim safe — *"anything left out comes back as the outgoing slot's"* — and a second assembly of it
at a call site would be the mistake `Watch::repointed`'s missing `..` exists to stop, made from the
sending end.

### 2. *Composite a deck's renderers* is built, end to end, and it is the demonstration

The mock has drawn this control since the Inspector was drawn: `.deck-head`'s `composite` chip,
tipped *"Click to overdraw them instead."* `karakuri-console`'s `DeckHead::composite` was the one
rectangle on that row the panel laid out and claimed nothing on, and its own comment said why —
*"layering is a build decision, so a press is a rebuild rather than a write and there is no
operation in the vocabulary this chip could name."* Both halves of that were wrong: the vocabulary
has carried `Operation::SetCompositing { deck, compositing }` the whole time, and a rebuild is what
this instrument does.

- **The chip names a destination.** `DeckHead::compositing` answers `SetCompositing` carrying
  `!self.composited`, computed from the state the frame that laid the row out drew. Nothing in
  `karakuri-operation` says *toggle*
  ([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)); the affordance is the
  surface's, exactly as the blend chip's cycle is.
- **The window re-aims the slot.** `composited` in `crates/karakuri/src/main.rs` sits beside
  `played` — the library load's arm — and does the same thing with one field instead of every
  field. It touches no deck, and it is a **free function over the aims** rather than a method on
  the program, which is `rewired`'s arrangement and its reason: the field, the refusal and the
  sentence are the whole of what it decides, and none of the three needs a window, a device or a
  `Deck` to check.
- **The verdict is the Staging lane's**, because a composite press *is* a build. The chip goes on
  reading what landed rather than what was asked for, which is `Mixer::residency`'s division and
  matters here: compositing costs a frame-sized target per renderer, so the budget verdict is not a
  formality
  ([P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)).
- **A press asking for the layering the slot is already in is refused with a sentence** and nothing
  is sent. Not because asking twice is wrong — the anchor beside this chip re-anchors by naming the
  mode the deck is in — but because here it buys a recompile of the whole slot and changes no
  pixel. The chip cannot produce one; a key, a map or a model could, the day any of them names this
  operation.
- **It writes no record**, on `Operation::LoadSet`'s terms and in its company:
  `written` answers `Silent(NoRecord)` for both. `Record::Merge` is what a *Set file* says about a
  Set's layering and carries no slot; nothing in the session vocabulary says *the Set in slot 3
  composites*, and inventing a spelling here would be inventing the record stream
  ([ADR-0046](0046-a-flag-writes-into-the-record-it-does-not-invent-one.md)). A session replayed
  therefore does not come back compositing where a hand asked for it — which is the load's cost
  already, unchanged in size and now reachable from one more control.

### 3. The capacity and the salts get the record and not the control, and the reason is not the engine

The same mechanism reaches them and nothing in the engine is in the way. **Two things are, and
neither is a setter.**

- **Nothing is drawn.** `docs/manual/console.html`'s Inspector is a pane head, a deck head and node
  groups; there is no capacity row and no seed row anywhere on it. The manual is the specification
  and moves first (`docs/contributing.md` §5): *a control drawn in the mock with no operation
  behind it is a specification; a control the panel draws that the mock does not is a defect.*
- **The obvious affordance is refused by cost, and the second one lies.** A capacity is a number in
  a declared range — `capacity [min, max] = default` — which on this panel is a parameter row's
  track
  ([ADR-0286](0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)),
  and the mock says so itself of the Library's reading: *"capacity included, because a procedure
  declares one the same way it declares a knob."* But a parameter track dragged is a uniform write
  a frame and a capacity track dragged is a **rebuild** a frame, each reallocating every element
  buffer in the slot — P-0091's *nothing is discovered to be expensive while it is on air* at its
  widest. And `Aim::capacity` is one `Option<u32>` for the **whole slot** where
  `Property::Capacity` addresses a **node**, which is ADR-0228's own recorded limit — *"a Set that
  recorded two different capacities loses the second"* — so a row drawn under one geometry of a
  pairing Set would be an answer the aim cannot carry, silently and per geometry.

So the roadmap's *two-thirds blocked* becomes *nothing is blocked and nothing is drawn*, the
operations row keeps its `plan` badge, and both the row's tip and the console page say which of the
two questions is open. **The camera is the third and stays `Undecided`**: `Property::Camera` carries
no payload because there are two live answers — a built-in orbit's numbers, or an L3 procedure
whose params are written like any other node's — and no flag and no key on either side to read the
answer off. Nothing here moves it.

### 4. The two *node's source* rows are model-only, and their panel badges are `gap`

The maintainer's decision, and the argument the badge carries. A source editor on the panel is a
**third letter-taking flow** on a console that has two — an arrangement's name and a Set's id in a
pane head — each bounded to *one path component*
([ADR-0292](0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)). A
procedure is unbounded, so it is a different flow rather than the second one again, and it is
exactly the bound those two were held to that it would break.

**Reading has no separate answer.** A source worth reading on a live panel is one you are about to
change; a reader with no writer beside it is a viewer, and a viewer of one node is what the file
already is. `gap` and not `plan` is the point: the rows now say the panel is not a way in, rather
than that it is a way in nobody has built.

**What replaces it is the file, and the loop was always closed.** `--watch` and this panel's own
watchers pick a save up, the worker builds it and it swaps at a frame boundary — the same check,
the same worker and the same budget `write_procedure` gets, which is the row *Edit the file
instead*. What the panel owes is the other end, and it has it: the Staging lane's verdict and the
Library's history walk.

## Alternatives

### a. A setter on the engine — `Set::set_layering`, `Set::set_capacity`

The roadmap's own wording, and what anybody reading *`Set::merge` has no writer* will propose. It
is the one shape this whole system is built to refuse. Compositing gives every renderer its own
frame-sized attachment, and a capacity resizes every element buffer in the slot: a setter reachable
from a control does that **on the render thread, on the frame the press lands**, with nothing
measured and nothing parked to put back — the two things `HotSwap` exists to guarantee
(P-0091, and ADR-0228's alternative **a** word for word one operation along). The failure is the
worst-timed available, because the press that costs the show the frame is a press somebody made
during it.

It is also unnecessary, which is what makes it a mistake rather than a trade: the re-aim costs a
`send`, and the build it causes is one this instrument was already going to run for a save.

### b. Widen `Aim::capacity` to one entry per geometry and draw a capacity row now

The honest fix for the half of §3 that is a lie rather than a cost, and it may well be right one
day. Rejected **now** for two reasons that are not about its merits: it is a change to
`karakuri-environment`'s `Aim` and to every construction of one — `--load-set`, the panel's load,
the CLI's startup — which is a sweep rather than an edit (`docs/contributing.md` §6); and it closes
the addressing question while leaving the affordance question, so the row would still not be drawn
at the end of it. The affordance is the one that has to be answered first, because it is what
decides whether the row is a track at all.

### c. Draw the capacity row as a parameter track anyway

What ADR-0286's shape suggests on sight, and the mock's own Library reading argues for. Refused by
what a drag *is* here: `ParamGrip` turns a pointer into a value every frame it moves, which for a
parameter is a uniform write and for a capacity is a full recompile and reallocation of the slot,
dozens of them across one gesture. A track that only acted on release would be a control that
behaves differently from every other track on this console, which is a decision about the console
rather than about this row.

### d. Record a re-aim as a session record

Considered because a re-aim changes what a slot runs, which a replay must reproduce, and because
`written` answering `Silent` for something an operator can now press deserves an argument rather
than an inheritance. It loses to the shape of the record vocabulary: `Record::Merge`,
`Record::Capacity` and `Record::Seed` are **Set-file** records — what a Set *is* — and carry no
slot, on `karakuri-store`'s own stated rule that what a Set is does not depend on which deck slot
it plays in. Writing one into a session stream would need a slot-carrying twin of each, which is
three new record variants and a replay path for them; and `Operation::LoadSet`, the operation this
one is a special case of, has the same hole and has had it since ADR-0228. Closing it for one of
the two and not the other would make the gap harder to find rather than smaller.

### e. Draw a source editor in the Inspector

The panel route both source rows carried until today, and the reason they carried it is real —
*read* and *write* are the two things a model does most, and a panel that cannot do them looks
short. It loses to ADR-0292's bound, above, and to the fact that the operator already has a source
editor that is better than any pane: their own, with the file open in it and a watcher underneath.

## Consequences

- **The Inspector's deck head has four controls where it had three.**
  `karakuri_console::input::PROBES`' row is *a deck head's five* claiming 5 — five controls, four
  methods, because `DeckHead::scrub` answers for both arrows — and `crates/karakuri/src/main.rs`'s
  `every_control_in_the_table_is_asked_by_the_press_handler` names `head.compositing(` beside the
  other three.
  `DeckHead::owns` is the union of four, and `tests/deck_head.rs`'s
  `a_press_is_one_control_or_none` asserts no point on the row asks two of them.
- **`Composite a deck's renderers` carries a `has` panel badge**, and its `op-when` is *on a
  worker* where it was *launch* — the same word *Load material into a deck* carries, for the same
  reason. `tests/panel_column.rs` holds the badge to an emission in this crate's source, and
  `tests/deck_head.rs`'s `the_fold_asks_for_the_layering_the_deck_is_not_in` presses the control on
  a compositing pane and an overdrawing one.
- **`Aiming::re_aim` is one line over `Aiming::changed`** and `rewired` is untouched. Nothing else
  in the file builds a `watch::Aim` except `loading`, which still re-points wholesale.
- **`Element capacity, seeds, the camera` keeps its `plan` badge and its tip now says why**, in
  three parts: the engine is not what stands in the way, the control is not drawn and two questions
  decide what it would be, and the camera is undecided outright. The console page carries the same
  paragraph beside the fold it belongs to.
- **`Read one node's source` and `Check and write one node's source` carry `gap` panel badges**,
  and `docs/manual/console.html` has a note saying the decision and its reason. Both keep their
  `has` MCP badges; nothing about the tool surface moved.
- **A session replayed does not come back compositing where a hand asked for it.** The press writes
  no record, which is `Operation::LoadSet`'s hole and not a new one — §d above is the argument, and
  the arm in `karakuri-operation-record` says so at the match.
- **`roadmap.md`'s M5.5 loses two rows from what the bay owes and one from what it is blocked on.**
  What is left blocked there is *Attach a signal to a parameter*, *Take a parameter back* and *Set
  a node's authority*, none of which is a field of the aim: a binding, its inverse and an authority
  are things a **built Set** holds, and the route they wait on is the one ADR-0280 opened for a
  parameter and nobody has walked for them yet.

## What was watched fail

Each was run alone against its own injected defect, `docs/contributing.md` §3's terms.

- **The two tests that held the previous decision went red on their own**, which is the evidence
  worth the most here because nothing was injected to get it. `deck_head.rs` asserted the fold was
  drawn and claimed nothing — it was the only entry in
  `nothing_beside_the_three_controls_is_claimed`'s *not a control* list — so adding `hit_composite`
  to `DeckHead::owns` turned that test and `a_press_is_one_control_or_none` red before a line of
  the page or the window had been touched. The previous decision was written down where it could
  fail, and it did.
- `deck_head::the_fold_asks_for_the_layering_the_deck_is_not_in`, with `compositing` answering
  `self.composited` instead of `!self.composited`:
  `left: Some(SetCompositing { deck: 1, compositing: false })`,
  `right: Some(SetCompositing { deck: 1, compositing: true })` on the overdrawing pane — a chip
  asking for the state it is already in, which is a press that costs a recompile and moves nothing.
- `panel_column::the_scan_finds_the_page_and_the_source`, run with the page badge already `has` and
  the console emitting: it panicked with *"`crates/karakuri-console/src` constructs
  `Operation::SetCompositing` and this file has no value for it — a control started emitting an
  operation nobody accounted for"*, and the whole file went red with it. The seam named the missing
  half of the change without being asked which half.
- `tests::a_composite_press_re_aims_the_slot_and_restates_the_rest_of_its_aim` in
  `crates/karakuri/src/main.rs`, twice. With `restated` writing `live: None` instead of `live:
  *live` — `left: None right: Some(2)`, *"the fold was silently un-selected"*, which is the class
  of defect the whole restatement exists to stop and the one that shows on the next build rather
  than on the press. And with the *already in that layering* guard removed, so the second press
  sent an aim: *"the answer does not say the slot is already set that way: composite: deck A
  re-aimed to composite its renderers"*, followed by a recompile of a slot whose picture would not
  have changed.
