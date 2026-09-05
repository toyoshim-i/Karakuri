---
id: 0233
title: The console's mixer has no crossfader, and the page had already argued it away from the other end
status: accepted
date: 2026-08-31
supersedes: []
superseded_by: []
principles: [0085, 0090, 0093]
tags: [ui, console, mixing]
---

# The console's mixer has no crossfader, and the page had already argued it away from the other end

## Context

**The control was specified, drawn in the mock, costed in the layout, and never built.** It is
`.xfade`'s first row on [the console page](../manual/console.html) — an `A … B` track with a mark on
it — and `crates/karakuri-console/src/lib.rs` already carries its height in the arithmetic that
gives the mixer its 316:

> Bay head 27, `.mixer-strips` … = 227.5, and `.xfade` (1px rule, 8 + 10 padding, a 16.5 row, a 7px
> gap and an 18.5 row) = 61.

`view.rs`'s enumeration of *what is in the mock's bay and is deliberately not here* names the whole
block and says what it is: *"an A/B track with a handle on it, and `wipe`, `iris`, `next bar`,
`8 beats` and `go`. That is a transition being armed and then fired, and every one of them is a
control over machinery the console cannot reach … **It is 61 of the bay's 316 and it stays empty.**"

**Two records had already decided things about it.**
[ADR-0219](0219-the-crossfader-spans-the-selection-and-the-one-after-it.md) fixed which two decks it
spans — *the deck that is selected and the one after it, which is exactly what `x` means on the
keyboard* — on [P-0090](../principles/0090-a-surface-offers-it-never-decides.md)'s
grounds, *a surface owns the affordance, never the authority*: which two decks a control spans is an
affordance over an operation that names both outright, so the page could settle it and the
vocabulary was untouched. And it recorded, separately and on
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md)'s terms, that **the mark is a
readout**: one number derived from two recorded ones, where *"a draggable mark would have to invert
a projection that is not invertible — inventing the split between two `SetOpacity` records by a law
no record names."*

### What the throw asked for

The note the page wrote around it, *The crossfader is the bay's, never a strip's*, is where the
gesture is stated:

> **It is a readout and a throw, and it is not a blend you hold.** The mark says where the two
> channel faders have already put the pair — A at 1.00 against B at 0.30 puts it just under a
> quarter of the way across — so it is a reading of two numbers rather than a third number of its
> own. Throwing it to an end asks for a crossfade: one gesture and four records, the incoming deck
> silenced and put on air at once and the two fades landing on the grid, at the quantum and length
> the transition row beneath it sets.

**So the control had exactly one control function left after the drag was refused, and that function
was the throw.** Everything else about it is a picture of two faders that are drawn six inches away.

### And the note had already argued the rest of it against itself

The same paragraph closes on the argument this record is mostly made of, and it reaches the
conclusion from the other end — from redundancy rather than from what a crossfader is for:

> Dragging it would be two opacities written at once, and both those faders are on the strips
> already; **a third control over the same two numbers is a second way to say one thing.**

That is [P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) written about a
fader: *before a feature gets a subsystem, check what it is made of*. A crossfader is made of the
two channel faders, and both of them are built — `SetGain` and `SetOpacity` are two of the panel
column's seventeen `has` badges, emitted by `panel.rs`, while the crossfader's own badge has never
left `plan`.

### What ADR-0232 settled the day before this was asked

[ADR-0232](0232-a-control-is-thrown-because-the-request-is-asynchronous-and-a-curve-is-a-helper-on-the-control-map.md)
is the record to read beside this one, and its first part is the frame: **the panel is real-time
control and it authors no envelopes**, and *thrown* means **asynchronous** rather than flung — *"the
render path forbids synchronous work, so a control dispatches an operation ASAP and it lands
later."* Curve control, if it is ever wanted, is automation: a helper standing where the control map
stands, emitting operations on the slower clock, never a feature of the engine.

**That is what leaves the crossfader with nothing of its own to do.** What a hand on a real
crossfader is *for* is riding a blend — a shape a person draws with an arm over a length they
choose. The panel does not author that and, after ADR-0232, has decided not to. What remains of the
gesture is a request dispatched at the moment of the throw, whose entire shape — the instant, the
length, the curve — comes from `Operation::SetTransition`'s three settings and not from the hand.
**Strip the scheduled fade out of the picture and the throw is a cut**, and a cut is not what a
crossfader is for; it is what a key is for, and `x` is already that key.

## Decision

The maintainer's, on 2026-08-31, in two moves.

**The console's mixer has no crossfader.** The `A … B` track leaves the mock, the panel column of
[the operations page](../manual/operations.html) stops claiming it as a home, and nothing is drawn
in its place.

**1. Its only control function was the throw, and the throw is not a crossfader.** The drag was
refused when the note was written; what was left was a mark that reports two faders and an end you
could click. Under ADR-0232 the panel authors no envelopes, so the throw carries none of the shape
of the move it asks for — the quantum and the length are the transition row's, the curve is a
constant on no surface at all, and the gesture's whole content is *ask for the thing `x` asks for*.
**A control whose only remaining function is a trigger is a button drawn as a fader**, which
misstates what a hand can do with it, and the one thing a fader promises — that the position under
your fingers is the value — is the promise this one was already documented as not keeping.

**2. The gesture is not the one modern practice reaches for**, and this half is the maintainer's own
judgement rather than a derivation:

> DJでも最近はビドル系の人しか使ってなくて、現代の複雑なミックス作業は基本縦フェーダー命だな

— *even among DJs it is mostly the battle players who use one now; modern, complicated mixing lives
on the channel faders.* The crossfader is the signature control of a technique — cutting, chopping,
scratch work — and the mixing this instrument is built around is the other kind: several sources
held at once, ridden independently, on the vertical faders. **The panel has four of those and they
are built.**

**And the manual had already argued most of this against itself, which is the strongest evidence in
the record.** The page reached the same place from redundancy — *a third control over the same two
numbers is a second way to say one thing* — the day it was writing the control's own prose, and
still kept the control, because the throw was a function the two faders could not perform. Take the
throw's reason away and nothing on that side of the argument is left standing. **A page that argues
against a control it is documenting has usually found the answer and stopped one step short**, and
this record is that step.

**What is given up is real and is named here rather than waved past.** A crossfader **constrains the
sum**: two channel faders can be left both down (black) or both up (both sources at full), and a
single hand on a crossfader cannot reach either of those states — the constraint is the instrument.
That is a genuine property, it is lost, and it is the only argument for bringing one back. **It is
also an argument for the control the note rejected** — one you *hold* — rather than for the throw,
so bringing it back would mean revisiting the redundancy paragraph on **one hand and a constrained
sum**, not on the number of values a drag writes at once. That is the ground a future proposal has
to win on, and it is written down here so the proposal does not have to be reconstructed
(`docs/contributing.md` §4: the forced part and
the chosen part are marked, so this is a one-clause revision rather than a reopened decision).

**And a two-way control was a partial view of this mixer in any case.** A deck is one to four slots
— `MAX_SLOTS` is 4, the bay draws *4 of 4*, and ADR-0219 had to invent a rule (*the selection and
the one after it*) precisely because *"the mock hardcodes `A … B`, which says nothing at all about a
mixer with four."* Whatever a crossfader is worth on a two-channel mixer, the version that fits this
one was never drawn and the version that was drawn covers half the deck.

**Nothing in the vocabulary changes.** `Operation::Crossfade` keeps everything it has, and that is
P-0090 read in the direction it is usually read the other way: removing an affordance is the
surface's to do, and the operation is not the surface's to touch.

## Alternatives

### a. Keep the mark and drop the throw

A readout with no gesture: the `A … B` track stays, showing where the two faders have put the pair,
and nothing happens when you touch it. It is the honest remainder of what the note decided — the
mark was *"a readout rather than a number of its own"* all along — and it costs nothing to leave a
picture on the screen.

**It loses because the picture is of two controls already on the screen**, six inches up and in the
same bay, each drawn with its own value and its own meter. The derived number answers *how far
across is the pair* for a pair the operator chose, and no gesture anywhere reads it. That is
P-0085's test failed rather than passed: the mechanism that already exists is the two faders, and
this is a third rendering of them.

**And an inert fader is worse than no fader.** A track with a handle on it is the strongest *drag
me* signal on the whole console; drawing one that refuses is the manual's own complaint about
controls that teach by experiment, paid for with 16.5px of a bay whose minimum height is its size.

### b. Keep it as a blend you hold

The control the note rejected, argued from the property this record admits losing: a crossfader
constrains the sum, one hand cannot put both decks at black or both at full, and that constraint is
the whole reason the shape exists on hardware. **It is the strongest of the four** and it is the one
a future proposal should be built from.

**It loses today on P-0090 and on the record it would have to invent.** A drag is *"two opacities
written at once"*, and the pair a mark's position implies is not recoverable — one position is
infinitely many pairs, so the control would have to pick one by a law no record names and no operator
chose. Making it real means a *record* for a constrained pair, which is a vocabulary change and not
an affordance; the console does not get to make one by drawing a control (P-0090), and the
operations page would owe it a row before anything drew it
([ADR-0226](0226-m5-closes-when-the-manual-is-implemented-and-the-meter-is-progress-rather-than-completion.md)).

**And it is the control the maintainer's second sentence is about.** *Modern, complicated mixing
lives on the channel faders* is a judgement about exactly this — the held blend, not the throw — so
this alternative loses on the decision's own terms and is marked as the preference it is rather than
dressed as a derivation.

### c. Keep the gesture and drop the fader — a button that fires a crossfade

`go`-shaped: one press in the transition row asking for `Crossfade { from: selection, to: next }`,
which is what `x` does and what the throw asked for. Nothing is lost from the operation's reach and
the misleading affordance is gone.

**It loses on being a home nobody has drawn.** The panel column of the operations page is a
specification of *where a control lives*, and ADR-0219 quoted the standing that makes that binding —
*"naming one is a change to this specification rather than a note on it."* A badge reading
`panel transition-row button` would be a plan written on the day the previous plan was deleted, for
a control nobody has designed, on a row whose own future this record leaves open. **`gap` is the
true statement today**: the panel cannot reach a crossfade at all, which is exactly what that badge
means ([ADR-0213](0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)).
Nothing here forbids such a button; it forbids claiming one before it exists.

### d. Widen it instead — a four-way crossfader, or assignable ends

If the fault is that two ends are a partial view of a four-slot deck, the fix could have been more
control rather than less: assignable ends (ADR-0219's own rejected alternative, the convention on
every hardware DJ mixer) or a two-dimensional blend over four decks.

**It loses twice.** Assignable ends lost in ADR-0219 on cost against need — new console state that
P-0090 wants ending in a record nothing else asks for, to serve what two `SelectDeck` presses already
do — and nothing in this record improves that trade. A four-way blend is alternative b made
four times harder: one position standing for four opacities, with three more degrees of freedom for
the law that would have to invent them. **Widening a control whose narrow form had no function does
not give the wide form one.**

## Consequences

- **`Operation::Crossfade` is untouched, and all three of its routes were checked rather than
  assumed.** The `x` key stands — `crates/karakuri-cli/src/main.rs` binds `'x' => self.crossfade()`,
  and `Live::crossfade` is now *one `operate` call*, the promise ADR-0232 kept. The conversion
  stands: `karakuri-operation-record`'s `Operation::Crossfade` arm answers four records against
  `current.transition` — the incoming deck silenced, put on air, and the two fades sharing a start
  and a length — and `Written::Records` still says of itself that `Crossfade` *"is what it was built
  for"*. **What goes is a pointer control the panel never drew.** No record type, no operation, no
  conversion and no test of any of them is affected.
- **The milestone's meter loses a row from its denominator and gains nothing built.** *Crossfade to
  the next deck* had `panel crossfader` as a `plan` badge — *designed: this surface is meant to reach
  it and does not yet* — and it becomes `gap`: *nothing — this surface cannot reach it at all*. The
  panel column reads 17 `has`, 44 `plan`, 3 `gap` today and becomes 17, 43, 4, so the meter goes from
  17 of 61 to 17 of 60. **Nothing transcribes that**, which is ADR-0213's whole design — the meter is
  the column and the count is a `grep` — so no number anywhere needs correcting. The edit itself is
  another session's; this record states what it means.
- **The console page's note is describing a control that will not exist**, and rewriting or removing
  it is part of this change rather than housekeeping after it
  (`docs/contributing.md` §4).
  The tooltip on `.xtrack` and the note *The crossfader is the bay's, never a strip's* are both
  confident and specific, which is the expensive kind of stale. That edit is being made in another
  session in parallel with this record.
- **[The roadmap](../roadmap.md) carries the same claim in the present tense and is named here rather
  than left to be discovered.** Its account of the manual pass says *"the crossfader spans the
  selection and the one after it, so the panel and the `x` key are one gesture"* and *"the
  crossfader's knob has no record for a half-done throw, so it is written as a readout of the two
  channel faders plus a throw rather than as a drag"* — true when written, and now a description of
  a control that is gone. Its Transitions row (*"`x` crossfades to the next slot"*) is about the
  engine and the keyboard and stays true.
- **ADR-0219 keeps its reasoning and gains one sentence.** Its subject was removed rather than its
  decision reversed, so it is **annotated and not superseded**: `superseded_by` points a reader at
  the record that answered the same question differently, and nothing answers *which two decks does
  the crossfader span* any more — the question is void, not re-decided. That is
  `docs/contributing.md` §4's
  test applied as written — the sentence changes what a reader can find out, not what the record says
  happened — and it is the form ADR-0229's open item took when ADR-0231 closed it.
- **The one sentence ADR-0219 said was still owed by the page is not owed any more.** *"Which
  direction a click on the near end means"* was a real gap, unanswerable from the keyboard, and it
  dissolves rather than being answered: there is no near end. A question about a control that does
  not exist is not a debt.
- **`karakuri-console` loses nothing and its arithmetic is not touched here.** The crossfader was
  never drawn, so no code is deleted; `.xfade`'s 61 is *two* rows and only the first of them is
  decided by this record, so the bay's 316, `STRIP_H`'s 215.5 and the `tests/mixer.rs` assertion that
  ties them together stand until the second row's question is answered.

### What this leaves undone

- **Whether anything on the panel still stands on the transition row.** The mock's second `.xfade`
  row is `wipe · iris · next bar · 8 beats · go`, and the operations page gives that home to two
  rows: *Choose the wipe shape, the quantum, the length* (the settings themselves, keys `z n j`) and
  ***Wipe the next deck in*** (keys `c`) — which is now the **only** remaining candidate for a
  scheduled gesture living there, and is itself still owed. `Operation::Wipe` stays
  `Owed::NotSettled` for the reason ADR-0232 recorded: *"what a wipe still owes is who says"* the
  front's shape and the softness of its edge are its. **The settings half keeps its readers either
  way** — under ADR-0232 the quantum and the length feed `Current::transition`, which the strip's
  fade and the renderer chips both read — but whether the row keeps a *gesture*, and therefore
  whether the `go` pill has anything to fire, is not settled here and is not this record's to settle.
  ADR-0232 left the adjacent question open in the same words: *"whose control the mock's transition
  row is, is not settled here."*
- **Whether the bay is still 316 tall.** `.xfade` is a 1px rule, 8 + 10 of padding, a 16.5 row, a 7px
  gap and an 18.5 row; this record deletes the 16.5. Whether the block shrinks, keeps its height for
  the row beneath, or goes entirely depends on the item above, and the layout has a test that will
  say so out loud when somebody decides.
- **Whether the crossfade gesture ever gets a panel home of another kind.** Alternative c is refused
  as a badge, not as a design: the day somebody draws a control for it, naming its home is a change
  to the operations page and the badge moves back. Until then `gap` is the honest reading and the
  operation is reached by `x`.
- **MIDI is unaffected and unassigned.** The tooltip's `⊕ MIDI: unassigned` was true and stays true;
  the row's MIDI badge was `gap` before this record and is `gap` after it, for its own reasons.
