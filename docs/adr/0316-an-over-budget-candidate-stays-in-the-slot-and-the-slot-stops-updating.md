---
id: 0316
title: An over-budget candidate stays in the slot, and the slot stops updating
status: accepted
date: 2026-09-09
supersedes: [0256]
superseded_by: []
principles: [0082, 0083, 0087, 0091, 0094, 0096]
tags: [engine, live, console, cli, environment, manual, m5]
---

# An over-budget candidate stays in the slot, and the slot stops updating

## Context

The maintainer, on 2026-09-09:

> rolled backが分かりにくい。事情を知らないとバグってるようにしか見えないんだよね。バジェット越えてた
> 時、スロットには入れたままで更新を停止、勝手にデフォルトにフォールバックしない方が異常事態を分かり
> やすく伝えられると思う。なのでrolled backという名前も変えたほうが良いのかな、overloadedとか？

*A rollback is hard to read; without knowing the machinery it looks like nothing but a bug. When the
budget is exceeded, leaving it in the slot and stopping the updates — rather than falling back to the
default on its own — would say "something is wrong here" more clearly. So perhaps the name `rolled
back` should change too. `overloaded`?*

**What a rollback actually did.** `HotSwap::judge` put the displaced Set back on air at the `t` it
was parked at and retired the candidate. The operator saw their save not take, twice over: the
picture went back to the material they had just replaced, and **the file on disk still held the
version that had been refused** — the watcher re-reads every file on every rebuild, so the next
unrelated save in that slot built the over-budget version again and it was thrown out again. Nothing
between the two states said which was which. `roadmap.md`'s M5.7 named that as one of *"two findings
this lane exists for"*, and the lane's `rolled back` row said the verdict without ever being able to
say the disagreement it left behind.

**The freeze was already the maintainer's option (b) and was already deferred once.**
[ADR-0313](0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)
records the sequencing — *"ゲート (a) だけ先に"*, the gate first and only the gate — and says why the
order was not negotiable: under the old gate a candidate was judged against **the deck's whole frame
interval**, so a freeze would have stopped slots for their neighbours' cost, and *"a frozen slot is a
hole in the picture where a rollback at least leaves the previous material running"*. That objection
is what the gate fix removed. **Measured on 2026-09-09**, headless at 1280x720 on an M4 Pro, host
clock and biased high: the reference Set in one slot of four read **33 ms** on the panel under Fifo —
mostly the other three slots, the four cell presents and the `egui` pass — while one frame of that
Set's own material costs **9.58 ms**. Against the first number a freeze would have stopped material
that fits; against the second it stops exactly the material that does not. The freeze is safe now and
was not before, and that is the whole of why this record comes second.

`P-0094` admits three answers to *what does this do at its worst*: **it cannot happen**, **it undoes
itself**, and **it is loud and the operator's hands still work**. The rule says take the earliest
available. What this record finds is that the second answer was **not available** here — the undo was
not one. A rollback restored the picture and not the file, so it did not put the show back where it
was; it put half of it back and left the other half disagreeing silently. Loudness is not being
preferred to a working undo. It is being taken because there was none.

## Decision

**A candidate whose own frame costs more than the budget stays live, and its slot stops updating.
`swap::Event::Overloaded` is the word for it, on every surface.** Five points.

**1. *Stops updating* means the slot skips `prepare` and `render`.** `HotSwap::overloaded` is a flag
on the slot's swap, and `Frame::render` reads it before the residency branches: a stopped slot takes
no step and records no draw, at any residency. **Its target is persistent and still holds the last
image it made**, which the composite and the deck's preview cells already read every frame — so a
stopped slot goes on contributing exactly the frame it stopped at, and costs **zero step and zero
draw**. The transport is not advanced either: a clock that ran on while the material it drives stood
still would put the slot somewhere it never played the moment a build cleared the freeze.

**2. The displaced Set is freed.** It is retired to the graveyard at the install, on both sides of the
verdict, and `judge` no longer takes it. **There is no rollback target anywhere** — not the three
fields on `HotSwap` that ADR-0313 removed, not the one-call `Parked` local that replaced them.
`HotSwap::previous` was already gone; what goes here is the last thing that stood in for it. Putting a
version back is *Put a node's previous version back* with `Revision::Previous`
([ADR-0308](0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)),
which reads the store's history — every version that compiled is filed there,
[ADR-0089](0089-history-is-gated-on-compiling-not-on-landing.md) — and lands a build like any other.
The put-back becomes an operator's act instead of the engine's, which is where `P-0096` and `P-0082`
already put every other write.

**3. Three doors out, and all three reach a stopped Live slot.**

- **The fader to zero.** `Deck::set_gain` / `set_opacity` reach a stopped slot exactly as they reach
  any other, and the composite skips a slot at zero rather than multiplying it
  ([ADR-0040](0040-a-gain-of-zero-means-no-contribution-so-the-slot-is-skipped.md)). This is
  `P-0094`'s last resort and it has to work on the material that has gone wrong, which is what a
  stopped slot is holding.
- **An earlier version**, landed from the Library bay's `history` scope — a build, which clears the
  freeze by point 4 below.
- **The next save.** The freeze belongs to the **installed version**, so any build landing in that
  slot clears it before the new candidate is judged on its own number.

A residency change is deliberately **not** a door: a stopped slot taken off air and put back is still
stopped, because what stopped is the version and not the placement.

**4. The word is `overloaded`, everywhere the verdict is said.** `swap::Event::Overloaded { id,
label, cost_ms, basis, budget_ms }` replaces `RolledBack` with the same fields and the same
asymmetry — `cost_ms` is an `f32` and not an `Option`, so a slot cannot be stopped without a number
by construction. `view::Stage::Overloaded` carries it to the Staging lane row and the transport's
health capsule, which are one enum and one set of words; the CLI's status line prints `overloaded`
beside the slot while it is true; `swap_outcome` says it to a model, with what to do about it.
**And the deck's preview cell caption says it**: `view::PREVIEW_OVERLOADED` in place of `material`,
because the image is a still and *a still is not a preview*
([ADR-0269](0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)) — an
unmarked frozen cell is a working preview that lies. It is the third word that cell distinguishes,
under `no slot`, which is [ADR-0258](0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)'s
rule: a cell says which nothing it is.

**5. It supersedes ADR-0256's rollback clause by a new record rather than by an edit.** ADR-0256
decided two things and keeps one: a swap still lands only at the top of `HotSwap::begin_frame`, and
pipelines are still double-buffered and exchanged between frames. What it decided about the verdict —
*"a candidate that does not hold up is taken back out by the engine"*, *"the outgoing Set is parked
rather than dropped"* — is what this replaces.

## Alternatives rejected

**Keep drawing without stepping.** The slot would `render` its last state every frame and skip
`prepare`. It removes the L1 compute passes and keeps the L4 draw, which is the fill-rate half and
the expensive one on any slot big enough to be stopped — so it saves almost nothing while looking
like a remedy. A persistent per-slot target means there is nothing to gain by re-drawing an image
that is already there.

**A reduced rate** — step and draw the stopped slot every *n*th frame. ADR-0269 deleted exactly this
for the preview path and its argument holds here twice over: a picture on a clock of its own is a
picture of nothing an operator can judge, and the deck would be paying an unbudgeted cost on a
schedule nobody declared. It also makes the frame time depend on which frame you measure, which is
the property `governor.rs` and `swap.rs` both need to be free of.

**A blank cell**, or a slot that stops contributing to the mix. Indistinguishable from an empty slot
and from a deck that is off air — `console.html` says a cell never shows nothing at all, *"because an
operator cannot tell that apart from a slot with nothing in it and would go looking for the material
rather than for the fault"*. Blanking also takes the picture away from the operator, which is the
half of `P-0094` that is never bought with authority: the material they asked for is still what they
asked for, and the fader is theirs.

**Keep the previous Set in memory as a rollback target, unused.** A Set's worth of VRAM per slot held
for a recovery nothing can trigger. [ADR-0071](0071-there-is-no-panic-key.md) refuses a permanent
rollback target for its own reasons, and ADR-0256 refused it too; with no rollback there is not even
a window to qualify it with.

**An automatic put-back of the *file*** — the engine rewriting the operator's `.kir` to the previous
version so that the picture and the disk agree again. This is the only thing that would have made a
rollback a real undo, and it is refused outright. `P-0096` says the operator's library is written by
the operator's own act; `P-0082` says looking never writes back; and a program that edits a file
under an editor that has it open is a lost afternoon. **The disagreement is repaired by removing the
rollback, not by extending it.**

**A fourth `Stage`, or a second enum for the capsule.** Neither: the lane and the health capsule read
one enum and the page gives them one set of words, so this is a rename of one variant and not a fifth
state (ADR-0310's own argument, taken in the same direction).

## Consequences

- **A stopped slot costs nothing per frame and is marked in four places.** No `prepare`, no `render`,
  no transport advance; the composite folds in the target it already had and the meter goes on
  reading it, because a held image really is what that slot is contributing. What says so: the
  Staging lane row, the transport's health capsule, the deck's preview caption, and the CLI status
  line — plus `swap_outcome` for a reader who is not at the panel.
- **`Event::RolledBack` is gone and every consumer had to say what it does now.** `karakuri`'s
  `verdict` maps `Overloaded` to `Stage::Overloaded`; its `staging` no longer pushes a *took* entry
  for it and no longer counts it as the live Set changing, because the swap in the same drain is what
  changed it. `Deck::begin_frame` retires a slot's meter on `Swapped` alone. `karakuri-cli` does the
  same with `procedures` and `set_changed`.
- **`Running::previous` and `Playing::previous` are deleted**, with `Running::rolled_back` and
  `Playing::rolled_back`. They existed to name what a rollback brought back, and nothing is brought
  back. Four tests went with them: three that staged a rollback to check the restored addresses were
  recorded, and one that checked a save after a rollback wrote what was on screen. **The rule the
  last of those held is unchanged** — a save records the version in the slot, whatever the path holds
  — and the two tests that reach it through the states that still separate the two (a run with no
  watcher; an edit between the compile and the first frame) are untouched.
- **A replay does not know a slot was stopped, and will run material the performance froze.**
  `Record::Procedure` is written where the swap lands and names the version, which is right — it is
  what the slot holds. Nothing in the record vocabulary says *and it was not running it*, and
  `HotSwap::install`, the replay path, judges nothing. So a replay of a night that stopped a slot
  draws that slot at full rate. **This is a gap in the vocabulary and is left open**: adding a record
  is a decision about what a session stream is for, and it is the maintainer's.
- **`seed_store_for_replay` has lost its stated reader and is left standing.** A recorded run seeds
  every slot's launch sources into the store because *"a rollback onto the launch version writes
  `procedure` records naming those hashes"*. No record names a launch hash that a save has not also
  stored now. Removing the seeding on that reasoning alone would be a replay that resolves nothing,
  found out later; the function says so at its head and the decision is the maintainer's.
- **The governor still counts a stopped slot's cost in its committed sum.** A stopped slot spends
  nothing, so the sum overstates what the deck is paying and a neighbour's prime request can be
  refused for budget a stopped slot is not using. It errs conservative and it errs in the direction
  that costs a warm-up rather than a picture, and the governor never pulls a Live slot down
  (ADR-0054), so nothing is taken away for it. **Not decided here**: what the governor should make of
  a slot that holds material it is not running belongs with M5.14 item 3's ruling on what
  `over_budget` says as a deck total.
- **`P-0094`'s *Undo* example is now false, and the principle is not edited.** It reads
  *"[swap.rs](../../crates/karakuri-engine/src/swap.rs) builds on a worker, polls with `try_recv` and
  never `recv`, replaces `live` only at the top of `begin_frame`, … and leaves the outgoing Set
  unstepped, so a rollback resumes it exactly where it was parked"* — there is no rollback and the
  outgoing Set is retired. **`swap.rs` moves from *Undo* to *Be loud*.** The replacement example is
  here, on ADR-0000's rule that a rule which stops being true is deleted and re-recorded rather than
  amended: *`swap.rs` judges a candidate on its own measured cost in the call the swap lands in, and
  a candidate over the budget stays in the slot with the slot stopped — no step, no draw, its target
  holding the last frame it made — said on the lane, the health capsule, the deck's caption, the
  status line and the MCP surface, and ended only by the operator's fader, an earlier version, or the
  next build.* What survives on the *Undo* side is the parked-Set property itself, which is now what
  makes a **stop** free rather than what makes a rollback resume. **P-0094's delete-and-re-record
  pass is the maintainer's and is owed**, and it is the second one owed on that principle — ADR-0313
  left one too.
- **The tests, each watched to fail against the shape it replaced.** In
  `karakuri-engine/tests/hot_swap.rs`,
  `an_over_budget_candidate_stays_and_the_slot_is_marked_stopped` asserts the candidate is the live
  Set, the flag is set, the sentence says both, and a build that fits clears it;
  `an_over_budget_candidate_is_stopped_on_a_budget_derived_from_its_own_measurement` holds the same
  claim against half the candidate's **own** measured cost, which is `docs/contributing.md` §1's
  rule. In `tests/deck.rs`, `a_stopped_slot_takes_no_step_and_keeps_the_image_it_stopped_at` reads
  the slot's target back and compares the bits across eight frames, then lands a build that fits and
  watches the slot run again; `a_fader_to_zero_takes_a_stopped_slot_out_of_the_picture` is
  `P-0094`'s last resort against a stopped slot, and carries the residency half — off air and back
  leaves it stopped and unstepped; and `an_off_air_slots_candidate_is_judged_at_the_install` now
  asserts the parked slot keeps the candidate and stops.
- **The draw's absence is not asserted anywhere, and the test says so.** A stopped slot's buffers do
  not change and `points.rs` clears its target before redrawing from them, so a slot that was still
  being drawn would produce **the same bits** and the readback above would pass. What an outside
  observer can see of that alternative is a *cost*, and this suite asserts no costs. *Keep drawing
  without stepping* is ruled out by the argument in *Alternatives* — the draw is the fill-rate half,
  so it removes almost nothing — and by nothing in the test files. The step count and the kept image
  are what is held. In `karakuri-console`,
  `the_state_word_is_what_the_cell_actually_distinguishes` reads the third word off the painted
  caption and holds the two rules that shape it: the cells beside a stopped one do not move, and a
  mark against a cell with no slot behind it still says `no slot`.
