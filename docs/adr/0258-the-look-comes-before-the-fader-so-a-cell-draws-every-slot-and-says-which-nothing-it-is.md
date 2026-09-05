---
id: 0258
title: The look comes before the fader, so a cell draws every slot and says which nothing it is
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: []
tags: [ui, engine, operations]
---

# The look comes before the fader, so a cell draws every slot and says which nothing it is

## Context

P-0080, *An operator can see a slot's own material without putting it on air*, is retired. Every
question it decides is inside auditioning, and it names the crate it binds — `crates/karakuri` and
not `karakuri-cli`
([ADR-0242](0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)).
A rule that has to state its own scope is a product requirement for one bay rather than a premise
somebody stuck on an unrelated question can hold a problem up against, which is
[ADR-0249](0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)'s gate. Its general
clauses were never its own: *looking never writes back* is
[P-0082](../principles/0082-looking-never-writes-back.md), *pay the bill now* is
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md), *name the
property, never the shape* is [P-0087](../principles/0087-name-the-property-never-the-shape.md), and
an instrument that has nothing to report saying which nothing it is, rather than drawing a reading
nobody took, is
[P-0095](../principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md).

**The requirement does not move, which is the same distinction
[ADR-0241](0241-auditioning-survives-the-control-that-was-retired-and-is-re-recorded-as-a-property.md)
drew when it retired P-0070**: choosing between candidates is the basic workflow of this instrument
and it cannot be done blind, so the instrument owes the operator a look at what a slot is making
before that slot reaches the audience. It is a decision, and it is recorded as one.

**Five records carry parts of this and none carries the whole.**
[ADR-0072](0072-auditioning-adds-a-draw-and-never-a-step.md) has *faders are deliberately ignored*
and the cost clause, both argued about the mechanism that is gone — the mix run once with the target
slot at unity, on the one output there was.
[ADR-0240](0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) retired that
control, [ADR-0243](0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md) settled
that the picture is an output and a cell is a monitor welded to its letter, ADR-0241 re-recorded the
requirement as a property and named the console's `Residency::Live` gate as the gap, and ADR-0242
took the command line out of scope. **None of them states the order the instrument is played in, that
the gate is gone, or what a cell draws when it has nothing to draw.**

## Decision

**Three steps, and they happen in this order: the operator looks at the slot's cell, decides from
what they see, and then raises the fader.** The look is what the decision is made on and the fader is
what carries the decision out.

**The fader is therefore not in the monitor.** It is the step *after* the decision, so a cell with it
applied would show the decision back rather than the material it was made on — and a slot held at
opacity zero would be a black cell for exactly the moment it is being judged in. Nor is the gain
trim, the blend or the mask. **A slot's own target is fader-free by construction**: `gain`,
`opacity`, `blend` and `mask` are applied in the composite and nowhere else
(`crates/karakuri-engine/src/deck.rs`), so what a slot renders into its target is the input to
setting a fader rather than the output of having set one. A surface that samples `Deck::slot_view`
gets that; a surface that samples the mix does not.

**What is judged is the colour and the motion the material arrives at**, so a cell is that slot's
texels, moving, through the transfer curve the audience will see them through — one tone-mapping pass
per cell off that slot's own target, through the same `Present` the picture goes through
(`crates/karakuri/src/main.rs`).

**Residency does not gate a cell.** An off-air slot is exactly the one worth looking at: it is the
candidate, and residency is the thing the operator has not decided yet. An Allocated slot shows the
still it stopped at, a Priming one shows what it is warming into. `Deck` draws every slot into its
own target on every frame and the field that chose one is gone; `Engine::aim` asks whether there is a
slot behind the cell — `slot_bind_groups[slot]`, which is `None` past `slot_count` — and never what
that slot's residency is.

**Looking adds a draw and never a step** (ADR-0072). A slot that is watched and then put on air
resumes where it stopped, or the audition would have moved the thing it was opened to judge
([P-0082](../principles/0082-looking-never-writes-back.md)).

**Never nothing: a cell that has nothing to show says which nothing it is, and none of the nothings
is being off air.** An absence an operator cannot tell apart from a slot with nothing in it sends
them looking for the material rather than for the fault. The word is what the program can
distinguish and no more — `state_word` in `crates/karakuri-console/src/view.rs` answers `material`
where there is a slot behind the cell and `no slot` where there is not. It is deliberately not `off`,
which was residency and has not gated a cell since ADR-0240, and deliberately not the page's *empty*,
which is a slot that exists with nothing loaded into it: `HotSwap::new` takes a live `Set` and
`Deck::new` takes one `HotSwap` per slot, so the engine cannot be in that state.

## What it costs, and that the bill is paid rather than argued

An audition adds a draw that is not in the budget and falls inside the window `swap.rs`'s watchdog
judges candidates in, so watching a heavy slot can roll back an unrelated slot's build (ADR-0072,
[ADR-0256](0256-a-swap-lands-on-a-frame-boundary-and-an-over-budget-set-rolls-back-without-being-asked.md)).
**Every loaded slot is watched here, so it is four draws rather than one**, and none of them reaches
the governor, which reads a per-Set cost. The figure `crates/karakuri/src/main.rs` carries for it is
four slots at 262144 elements against one slot, about 10 ms a frame against 5.9 — a host clock, so
biased high — and `crates/karakuri-engine/tests/deck.rs` prints the pair the bill actually is, *deck
of one* against *four, one Live*, rather than arguing it in a comment.

It is unbudgeted risk on the live path, added knowingly. It is the case
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
loses, and it loses for the reason written there: a rule protecting a performance may not be used to
remove what the performance is played with. The answer is to pay the bill and write it down
([P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)), not to remove
the look or hide it behind a flag. **An operator may still turn a cell off by hand and nothing else
may** — the Program bay head's *previews 3 of 4* is that press, and it is the only thing in the
instrument that takes a loaded slot's cell dark.

## Alternatives rejected

**Show the slot as it appears in the mix.** The cheapest monitor there is, since the composite is
already being drawn. It answers what a slot *contributes* and not what it *is*, and it is wrong in
the one direction that matters: at the fader positions an operator is deciding between, including
zero, the cell goes dark exactly when it is being read.

**A still, a thumbnail or a scaled-down copy.** Cheaper than a pass per cell, and it drops the two
things being judged. Motion is most of what distinguishes candidates, and a copy that is not through
the transfer curve is not the colour the room gets.

**One audition at a time, chosen.** This is what `Deck::set_preview` was, and it is the shape the
retired operation put on the one output there was — ADR-0240, corrected by ADR-0243. The requirement
is *every* slot's own material, always, on a surface of its own; a chooser is a control an operator
has to work before they can look, which is the blind choice this exists to remove.

**Draw only the slots that are already drawing.** The gate that was here, in `Engine::aim`, with the
reason *an off-air slot has nothing new in its view* — true only because the engine refused to draw
one. It offers the slots that are easy to offer and removes the requirement from the one moment it is
needed, which is why it read as an optimisation rather than as a defect for as long as it did.

**Leave a cell with nothing to show blank.** It costs no drawing decision and it produces the one
state an operator cannot read: a cell showing nothing looks the same whether the slot is empty, the
deck is short, or the surface is broken.

**Drop the audition, or put it behind a flag, to keep the frame path budgeted.** The frame budget is
real and this spends it. Refusing here would be P-0094 used to delete the control the show is played
with, which is the argument ADR-0234 said that rule had to answer the first time it was cited to
remove something.

## What this leaves standing that a reader will notice

**The rejected build's cell.** [The console page](../manual/console.html) specifies, under *What a
deck preview cell shows, and when*, that a build that was rejected — over budget, or failed to
compile — shows black with the sentence saying what went wrong, and says of that and of *empty* that
*"neither can happen yet, so nothing draws either word"*. **Half of that reason is now wrong and the
conclusion is still right.** A rejection can happen: every slot in this program is built over a
`watch::Watch`, so a save that fails to compile or a candidate the watchdog rolls back is an ordinary
event on a running slot, and the Staging lane's rows are the verdicts. What cannot happen is the
*cell* the page describes. A refused build installs nothing — a failed compile returns no `Request`
at all, and a rolled-back trial leaves the previous Set running with its `t` untouched — so the
honest picture on the frame after a rejection is the material that is still there, drawn exactly as
on the frame before, and anything else would be the cell inventing a state the deck is not in.
`crates/karakuri-engine/tests/deck.rs`'s
`a_rejected_build_shows_what_is_still_running_and_a_cold_slot_shows_black` asserts both ends of that,
including the cold end: a Set that has never stepped draws its zeroed element state, which is
near-black, and priming is what exists to warm it.

**So what is owed is the page's sentence and not a drawing.** Whether a cell ever carries a
diagnostic, or whether saying what went wrong stays the Staging lane's job with the cell keeping the
picture, is open and is the decision that sentence is waiting on.

## Consequences

- **`docs/principles/0080-…` is deleted and P-0080 is never reused.** `INDEX.md`'s *Retired numbers*
  table carries the row and is where the number resolves.
- **`docs/principles/` is fifteen files**, and this was the last retirement of the pass.
- **The console page and this record now say the same thing about the cells** — the three steps, the
  fader, residency, and which nothing a cell is in — with the page as the specification and this as
  why. The page's *neither can happen yet* clause is the one place they disagree, and it is named
  above rather than corrected here.
- **ADR-0234's citation of the cost clause is left naming P-0080 unlinked.** The sentence is true of
  this record clause for clause, which is why `885aa24` re-pointed it at P-0080 in the first place;
  what it may not do is make a record written on 2026-08-31 read as having consulted one written five
  days later in the commit that moved the link, which is the fault that commit was fixing elsewhere.
  The number resolves through `INDEX.md`
  ([ADR-0257](0257-a-pointer-inside-a-records-prose-is-metadata-too.md)).
- **Fourteen files cited the retired number.** The six in `crates/` cite this record — the panel's
  module documentation and its printed legend, `Engine::aim` and the cell loop, the console's
  `View::previews` and `state_word`, the deck's module documentation and `Deck::level`, the offscreen
  renderer, and the engine tests whose subject it is — and `docs/roadmap.md` names it beside the
  clause that is still owed. The records that describe the file keep the number, unlinked.
- **`Operation::RouteFrame`'s open naming question is untouched.** A cell is not in the set that
  needs naming and the picture is (ADR-0243); nothing here adds to that.
