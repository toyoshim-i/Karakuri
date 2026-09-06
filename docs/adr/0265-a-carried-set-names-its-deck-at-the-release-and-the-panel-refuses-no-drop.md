---
id: 0265
title: A carried Set names its deck at the release, and the panel refuses no drop
status: accepted
date: 2026-09-06
supersedes: []
superseded_by: []
principles: [0090, 0096]
tags: [console, library, mixer, operations]
---

# A carried Set names its deck at the release, and the panel refuses no drop

## Context

`a109db5` built the panel's route to *Load material into a deck*. Until it, the operation had one
route an operator's hands could reach — the `l` key over the Library bay, which takes the Set from
the list cursor and the deck from the deck selection
([ADR-0228](0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md)) — and
[operations.html](../manual/operations.html) carried that row's **panel** badge as `plan`, which its
own legend defines as *"designed — this surface is meant to reach it and does not yet"*
(`docs/manual/operations.html:59`). The badge now reads `has` (`docs/manual/operations.html:202`).

**The page had already specified the gesture.** `docs/manual/console.html`'s *How a Set reaches a
deck*, lines 2020–2025:

> **Dragging a row onto a strip is a second route to the same command, and never the first.** It is
> worth having because it names both operands in the one gesture, which makes it the only way to
> load a deck without selecting it first; it is second because a route the keyboard cannot take
> would break rule 01 on its own. Two ways in, one name — which is the only reason a drag is
> safe to add later.

The mock says it a second time from the destination's side, in the `load → A` pill's own tip
(`docs/manual/console.html:137`): *"Dragging a row onto a strip is the same command with the deck
named by the strip you drop on rather than by the selection."*

**What the page does not say is how that gesture is spelled on a panel that already has two other
drags, and what it does at its edges.** A boundary
([ADR-0163](0163-a-boundary-gets-first-refusal-on-a-pointer.md)) and a fader
([ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md)) each name
**which control** the hand took hold of, resolved at the press, and every later event asks that
control what the pointer now means. A carry can name no control at the press: what it takes hold of
is a payload, and the control it is going to is in another bay. Four questions came with that, each
with a plausible answer that is not the one taken — where the `deck` operand comes from, what a
press that never moves does, what a release over no strip does, and whether a strip the room is
watching may be dropped on at all.

## Decision

**The gesture is a carry, and the destination is resolved at the release.** Four moments, and the
first three ask for nothing.

1. **The press takes the Set in hand.** `LibraryBay::take`
   (`crates/karakuri-console/src/view.rs:9539`) answers a `Taken` — a row index and the Set's name —
   and not an operation, because half a gesture names one operand. `Readout::took`
   (`crates/karakuri/src/main.rs:2335`) moves the library cursor to that row (`View::point_at`,
   `view.rs:12630`) and hands the name to `Panel::carry` (`panel.rs:1009`).
2. **A move asks for nothing.** `Panel::moved` answers `None` with a carry in hand. Nothing has
   happened, because nothing happens until the Set is let go.
3. **The release names the deck.** `Mixer::dropped` (`view.rs:6356`) answers which strip's rectangle
   the pointer is inside, laid out at the release; `Panel::released(onto)` (`panel.rs:1119`) builds
   `Operation::LoadSet { deck, set }` from the payload and that answer.
4. **A release anywhere else asks for nothing**, and says so: `Released::Nowhere { set }`
   (`panel.rs:681`).

**The load asked for is the load the key asks for.** `LoadSet` leaves by the door every other
control's operation leaves by, so the drop re-points the slot's source and installs nothing
(ADR-0228), and a drop on a `presets` row runs the key's own two helpers — `taking_in` and
`preset_press` (`crates/karakuri/src/main.rs:6012`, `:6040`) — so one gesture emits
`TransferSet { Take }` and then `LoadSet`. That is the page's *"Two ways in, one name"* met at the
operation rather than only at the badge, and taking a preset in is an operator's own act, which is
where it is allowed to write the operator's library
([P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md),
[ADR-0261](0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md)).

**Nothing is refused, and no residency is read.** A drop on a live deck replaces what the room is
watching and still asks for the load. `Strip::tally` is consulted neither in `Mixer::dropped` nor in
the release arm, *"so there is nowhere for a second rule to hide"*
(`crates/karakuri-console/src/panel.rs:663-665`). The page rules on it in as many words
(`docs/manual/console.html:2012-2015`): *"Nothing refuses it: what may be asked for is the
instrument's to decide, and a panel refusing what the key allows would be a second rule kept in a
second place."* That is
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) — *"A control decides **what a
press asks for**. It never decides **what may be asked**"* — and the gate is where the second
answer is.

## Alternatives rejected

**Name the deck from the deck selection, and let the release only confirm.** The strongest of them
and the one somebody will re-propose, because it is the smallest change: the selection is already
the `deck` operand for `l` and for every other deck-addressed keyboard translator,
`Panel::released` would keep its argument-free shape, and no bay would have to be laid out on a
button up. It loses to what the page says the drag is *for*: *"it names both operands in the one
gesture, which makes it the only way to load a deck without selecting it first"*
(`console.html:2022-2023`), and the mock's pill tip names the operand outright — *"the deck named by
the strip you drop on rather than by the selection"* (`console.html:137`). A drag that read the
selection would be a pointer spelling of `l` with the one capability the page claims for it removed,
and the drop would land on a deck the hand was not over. It is also the failure the host-side test
was written to catch: *"Deleting the `Mixer::dropped` ask from the release arm is the injection this
was watched to fail against, and moving it into the press arm is the second"*
(`crates/karakuri/src/main.rs`, `a_drop_on_a_strip_loads_the_strip_it_was_let_go_over`, `:14351`).

**Build no drag at all, and leave `l` as the only route.** Free, and defensible on the day: the
operation is reachable, ADR-0228 had already made it work, and [roadmap.md](../roadmap.md) said in
as many words that *Load material into a deck* is not blocked. What it loses to is the page and the
milestone written off it: the badge for this row's **panel** column was `plan`, M5.3's exit condition
is *"No `plan` badge in the panel column of this bay's rows"*, and a `plan` badge is a promise the
page made on the panel's behalf. The capability is not a duplicate either — with only the key, a
load is *"a cursor and a key with no pointer anywhere in it"* (`console.html:2001-2002`) and the deck
must be selected first, which is the one thing the drag exists to avoid.

**Two presses instead of a drag: press the row to pick it up, press a strip to place it.** Plausible
on its own terms — it needs no drag state, it survives a trackpad, and the panel now has a
precedent for a gesture that outlives its press
([ADR-0225](0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md), whose rule 2
holds the pointer while a menu is down). It loses because the page names a drag and the page moves
first: `docs/contributing.md` §5 step 2, *"The page is the specification, so it moves first"*, and
step 3, *"a control the panel draws that the mock does not is a defect."* It also puts a mode on the
mixer — while a Set is pending, a press on a fader knob or a blend chip would have to mean *place it
here* instead of what those five controls mean on every other press — where a drag holds the same
state in the hand and lets go of it by itself. If it is ever wanted, the edit is to
`console.html` and not to `main.rs`.

**Press a strip and open a picker of Sets under it.** The mirror route, naming the destination first;
the arrangement pill's menu is the shape it would take, and that card's own list is already a load —
*"the names already filed; picking one is the load"*
([ADR-0225](0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)). It loses
twice. The page refused the same move at the other end of the same
gesture — *"the answer that does not — a destination chosen in the library's own foot — would be a
third selection on a panel that already has to explain two"* (`console.html:2005-2007`) — and a Set
picker on a strip is a second listing of the store on a panel whose Library bay is that listing,
with the scope chips, both filter fields and the reading attached to it. The strip's controls are
enumerated by the mock and none of them is a load, so this is again a control the panel would draw
that the mock does not.

**Refuse a drop on a live deck, or make it ask first.** The most sympathetic of the rejected
answers: the drop replaces what the room is watching, the panel knows each strip's tally because it
draws it, and a guard is two lines. It loses to P-0090 and to the page's own sentence on the pill
above (`console.html:2012-2015`) — *"A rule held in one surface binds none"*, and `l` on the same row
would go on doing what the drop refused. What the control owes instead is to say where it lands
before the press, and on this route that is the strip under the pointer. The refusal is asserted
rather than assumed: `crates/karakuri-console/tests/carry.rs`'s
`a_drop_on_a_live_deck_still_asks_for_the_load` drops on the on-air deck and then re-lays the bay
out with every strip's residency turned over, *"which is what says the destination is a rectangle and
not a state."*

**Let the drop install the Set, or carry a load path of its own.** The obvious reading of *put this
Set on that deck*, and the one a pointer gesture invites — the panel has both operands in hand at
the release, so it could build and hand over. It is the decision ADR-0228 already took for the key,
and it loses to the same argument, which the page states for this row: `Deck::install` is not
reachable from a key or a surface, and *"A Set handed straight in is a Set nothing measured, on a
deck that cannot roll it back"* (`docs/manual/console.html:2073-2074`) — no trial, no budget
verdict, nothing parked to restore. The weaker version of it — a drop
with its own path to the same effect — is what the `presets` branch would have broken: a drop that
skipped the take-in *"would name a Set this store does not hold and be refused where the key
succeeds"* (`crates/karakuri/src/main.rs`, the drop's arm in the pointer handler). Two routes to one
row have to arrive at the same place.

**Start a fade instead of replacing.** The kindest-looking drop: a Set let go on a live strip could
come in over the material that is there rather than displacing it. It loses to a note the page wrote
about strips before this row existed — *Nothing on a strip starts a fade* — whose argument is that
*"A hand on the fader is the fade"* and whose conclusion is that *"**Fade a deck out or in** reaches
this instrument from everywhere except the panel"* (`docs/manual/console.html:650`, `:657-658`). A
drop that faded would put that operation on the panel through a side door, from a control that is
not the fader, and it would make one gesture ask for two operations where the page says two ways in,
one name. When the new material arrives is already answered and is not a fade: the swap *"lands at a
frame boundary and is judged for thirty frames against the budget like every other build"*
(`docs/manual/console.html:2077-2078`, and ADR-0228).

**Send a release over nothing to the nearest strip, or to the selection.** The forgiving reading of
a near miss, and it is the one arm where forgiveness is available at all — a boundary let go off its
track still lands somewhere legal, and a fader dragged past its end is still at its end. It is
refused because the operand would be invented: *"a load aimed at the nearest strip would be a deck
nobody pointed at"* (`crates/karakuri-console/src/panel.rs:674-675`). `Released::Nowhere` names the
Set instead, so the panel says what it did with the gesture rather than going quiet.

## Consequences

- **`Panel::released` takes a destination it uses for one drag in three.** `released(onto:
  Option<u8>)`; a boundary and a fader ignore it, and every caller that is not a carry passes `None`
  — including `mod gpu`'s boundary test, whose call site says why. `crates/karakuri/src/main.rs`
  lays the mixer out at a release **only** when `Panel::in_hand` answers `InHand::Carrying`, so a
  release with a boundary in hand costs no text shaping.
- **`Released` is no longer `Copy`** (`crates/karakuri-console/src/panel.rs:622`), because a Set is
  named by a `String` in `karakuri-operation` and the drop carries one. `Grab` and `Dragged` are
  unchanged.
- **`InHand` gained a third answer and the cursor gained no third shape.** `InHand::Carrying` exists
  so a window loop can tell a move that may emit from one that never can; `view::cursor` keeps the
  vocabulary *arrow, or resize over a boundary*, and a carry suppresses the resize cursor the way a
  fader does.
- **The Library bay's list is a control, and *"the one offer on this console whose press names no
  operation"*** (`crates/karakuri-console/src/input.rs:428-429`).
  `input::CLAIMS` is `[usize; 15]` and the list contributes `1` rather than a row count, because
  `CONTROLS` is printed to an operator and a figure that moved when a divider moved would answer a
  different question. `input::claim`'s rule 4 did not change to admit it.
- **Two entries were added to `press_handler::TABLE`** — `("LibraryBay", "take", …)` and `("Mixer",
  "dropped", …)`. The second is the only entry in that table asked on a button **up**, and the
  module reads text rather than pressing anything, so which arm asks it is beyond what the check can
  see.
- **The library cursor now moves on a pointer as well as on the arrow keys**, and it outlives the
  gesture: after a drop that landed and after one let go over nothing, `l` loads from the row the
  hand last touched. `View::point_at` refuses a row past the listing rather than clamping it, which
  is `View::select`'s rule and not `View::walk`'s.
- **What the drag does not draw is the carry itself.** There is no drop-target state, no ghost under
  the pointer and no mark on the row in hand beyond `.lib-row.cursor`, because the mock draws none
  of those. That is a panel affordance rather than a route, so it does not hold the operation's
  badge; it is named as open in [roadmap.md](../roadmap.md)'s M5.3, and anything richer is an edit
  to `console.html` first (`docs/contributing.md` §5 step 3).
- **A press on an open reading takes nothing in hand.** `LibraryBay::take` walks the rows through
  `LibraryBay::row` rather than dividing by the row stride, so the block a reading opens is not a
  row and the rows below it still answer their own names.
- **The reading follows the cursor on both surfaces, and the carry pays it.** `View::opened`
  (`crates/karakuri-console/src/view.rs:12655`) answers the reading only where the row under the
  cursor is still the Set it was read of, and the rule that keeps that honest is stated at
  `view.rs:12676` — *"**the reading follows the cursor**: a move with one open is a read of the row
  it arrived at"*. **That rule names no surface, and it is the cursor's rather than the
  keyboard's.** `Readout::took` discarded `View::point_at`'s `moved` and asked nothing, so taking a
  row in hand while a reading was open on another row made that block disappear and nothing brought
  it back: the only thing that moves this cursor is a hand, and the hand had already gone where it
  meant to. So the pointer now owes what the keys pay at `crates/karakuri/src/main.rs:9834` — `took`
  answers the `bool` (`main.rs:2352`) and the window loop re-reads on it (`main.rs:9571`), the same
  two conditions and the same `read_reading` one event along.
- **The page is behind that rather than against it, and half of it was already surface-neutral.**
  The prose names the keys — *"the arrow keys move the cursor and the reading moves with it"*
  (`docs/manual/console.html:2099-2100`) — and was written when they were the only thing that moved
  this cursor. The `read` pill's own tip names none, on the line this record already quotes for the
  drag (`console.html:137`): *"move the cursor and the reading moves with it, so one row is open at
  a time and never a second list."* A reading that followed one surface's cursor and not the other's
  would be two answers to *what is this a reading of* on a bay that has one cursor
  (`View::cursor_row`) — which is the sentence further down this list held to: the pointer is the
  same pointer, so the mark it moves is the same mark. A clause naming the press belongs in that
  paragraph of `console.html` and is an edit to the page rather than a second branch here.
- **The press still names no operation, and keeping that true cost a fifth `Acted`.** `read_reading`
  (`crates/karakuri/src/main.rs:5579`) reads the store and writes the answer into the view; it emits
  nothing. So Decision step 1 stands and so does everything filed under it — `input::CLAIMS` still
  counts this list as one offer whose press names no operation
  (`crates/karakuri-console/src/input.rs:428-429`), `press_handler::TABLE` gains no entry, and
  `operations.html` gains no row. What could not stand was `Acted::Nothing`: the readout holds no
  store — the division `Readout::asked_to_read` is written to, and the reason every file read in
  this program is in the window loop — so the press has to say *the cursor moved* and let the caller
  read the file. That answer is `Acted::Pointed` (`main.rs:3110`), which is `Acted::Opened`'s
  argument one control along (ADR-0236): not an operation, and not nothing either. It earns no frame
  of its own — `App::performed` answers it with `otherwise` (`main.rs:8976`), because a claimed
  press is already `Repaint::Now` and `Change::Pointed(true)` is what the arrow keys raise for the
  same move. **One standing assertion moved with it**:
  `a_drop_on_a_strip_loads_the_strip_it_was_let_go_over` read `Acted::Nothing` for the press on a
  row and now reads `Acted::Pointed`, which is that test's own point 2 — the mark follows the hand —
  said by the press rather than only by the view.
- **The two halves are held by two tests, because no one test can reach both.**
  `a_carry_that_moves_the_cursor_re_reads_the_row_it_arrived_at` (`main.rs:14509`) presses through
  `Readout::pointer` against a store on disk: the press answers `Pointed`, the block is drawn
  nowhere until the answer is acted on, `read_reading` puts it under the row the hand took, and a
  press on the row the cursor is already on answers `Nothing` and owes no read. **The window loop's
  own branch is reachable from no test** — `winit` hands out no `ActiveEventLoop` outside its loop,
  which is why `Readout::pointer` exists as a method at all — so `mod reading_follows_the_cursor`
  (`main.rs:17581`) reads *both* statements out of this file as text, in `press_handler`'s and
  `event_response`'s shape, and fails naming the one that went missing. Each was checked against its
  own injection: dropping the `bool` in `took` fails the first and not the second, and deleting the
  loop's branch fails the second and not the first.

- **The page names only the keys for that mark, and this record is what says the pointer is the
  same pointer.** `docs/manual/console.html:2044-2045` reads *"The cursor moves on the arrow keys
  and gets no row on the operations page, which is a decision and not an omission"*, and the panel
  now moves it on a press as well. Nothing new is drawn and no operation is emitted — a pointer that
  names a row instead of stepping to it reaches the same `View::cursor_row` — so the row on
  operations.html stays absent for the reason the page gives. A reader who thinks the page meant
  *only* the arrow keys is reading a sentence about which surfaces have a row, and the fix for that
  is a clause on the page rather than a branch in `main.rs`.
- **Nine tests in `crates/karakuri-console/tests/carry.rs` hold the console's half**, and the moment
  the deck is resolved at is held in the host: `crates/karakuri/src/main.rs:14351`, which is the only
  place both halves of the gesture meet a press handler.
