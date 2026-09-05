---
id: 0188
title: A pending transition says it is pending, and no surface holds the rule
status: accepted
date: 2026-08-26
supersedes: []
superseded_by: [0189]
principles: [0087, 0090]
tags: [ui, vocabulary, decks]
---

# A pending transition says it is pending, and no surface holds the rule

## Context

The mixer's controls arrived one at a time — the two faders
([ADR-0185](0185-a-fader-translates-a-drag-into-an-operation-and-applies-nothing.md)), then the
blend chip ([ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md))
— and each left the same bullet behind: **the tally is next, and its question is not about the
pointer.** A fader's readback is exact, and the blend chip's is a word the deck already agrees with.
The tally is the first control on this panel whose readback can **disagree with what was asked for**.

The disagreement is the engine's design rather than an accident. `Deck::residency` is the effective
residency, `Deck::requested_residency` is what the operator asked for, the governor may hold a slot
below the request and never above it, and the state that names the gap already exists:
`Deck::is_parked` is `requested == Residency::Priming && effective == Residency::Allocated` —
*"asked to prime, and not priming: the request stands and the budget has not allowed it yet."*
`karakuri-cli`'s status line draws it, in four fixed columns as **`park`** and spelled out as
**“parked (asked to prime, waiting for room)”**, with the reason beside the function: *"a surface
that names them alike tells the operator their request was discarded when it is being reconsidered
every pass."*

So the console has to draw two values where every other control draws one, and the general question
underneath is not the deck's: **how does this panel say that a request has been made and has not
landed?** The maintainer settled it in one sentence — *on request the state it is in stays lit, the
state it was asked for blinks until it lands, and when it lands the new one is lit and the old goes
dark* — and settled it as a **GUI rule**, with the tally as its first user rather than its subject.

A second question arrived with it, and it is the one this record exists to close properly: while a
request is pending, should the panel refuse everything except the withdrawal?

## Decision

Two principles and one number.

**What must be said is
[P-0087](../principles/0087-name-the-property-never-the-shape.md),
and the presentation is not.** A pending control says where it is, where it is going, and that it
has not arrived: the actual state stays legible and unambiguous, the destination is identifiable
from the surface itself, it reads as unsettled in a single frame, and it declares a price and a
staleness like anything else that moves. One phase for the whole panel, arriving as a value because
`karakuri-console/src/` reads no clock; the pending-ness derived every frame from the two values the
engine already holds.

**Where a constraint lives is
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md).** A surface owns
what a press asks for and never what may be asked. The withdrawal the maintainer wanted is available
as an **affordance** — a press on a pending control asks for the withdrawal, which is P-0090's
division applied one control along — and no authority moves to get it.

### The blink is an example, not the rule, and that was a correction

**The first draft of P-0075 was the maintainer's sentence written as the requirement**: two
indicators, old lit, new blinking, swap on arrival, with a degenerate one-indicator form —
a blinking pending mark and the destination in a tooltip — prescribed underneath it.

**That fixes a shape**, and this repository has a rule against exactly that habit:
[P-0087](../principles/0087-name-the-property-never-the-shape.md) — *a shape is one instance of a
property*, and naming the shape leaves the property to be reinvented wherever the shape does not
fit, in a form that is harder to find afterwards because nothing advertises it. A surface with one
indicator, or one where a blink is wrong for some other reason, would have been outside the rule
rather than covered by it.

What made it concrete was a second presentation the maintainer offered for the no-space case: **the
chip rolls half a turn toward the value it was asked for and falls back, once a second** — a slot
machine that never lands. It is not a worse answer than the blink and on one clause it is a better
one:

- **It names the destination on the surface**, in the same 53 pixels, rather than deferring half of
  what is being said to a hover the console does not currently draw at all.
- **It reads as unsettled in every frame.** A half-rolled word cannot be mistaken for a settled one.
  A blink can: caught at its bright phase it looks like a lamp that is simply on, so the blink only
  meets the still-frame clause if its two appearances are *both* kept distinct from the settled
  one — a condition on the animation that the obvious implementation violates. **That is an argument
  about the rule rather than about taste**, and it is the reason the clause is stated as a property
  and the two animations are stated as examples measured against it.

Both are written into P-0075 as worked examples with their trade-offs. Neither is prescribed. A
third presentation meeting the four clauses is admissible without amending the file.

### A word instead of a motion

**This is the strongest alternative and the one worth writing down carefully, because it is
currently cheaper on every axis.**

The word exists. `park` is already the name for this disagreement, already drawn by another surface,
already spelled out in a sentence written for an operator. A word costs **no animation, no phase, no
scheduler and no repaint at all** — the strip is redrawn when the deck's value changes, and P-0072's
first clause holds untouched. And today it carries **the same one bit** any motion would: the only
disagreement the engine can produce is prime-requested over allocated-effective, because effective
Live and requested Live are the same set of slots and nothing is ever held above its request. There
is exactly one pending state to name, and it has a name.

It loses for two reasons, neither of them about the tally.

- **It is a name for one pair where the rule is about the relation.** `park` says
  *Priming-over-Allocated* and nothing else. It does not extend to a slot asked to go live during a
  transition, to a wipe waiting on the grid, to a mask whose travel is scheduled, or to any of the
  bays that are still empty — each of those would get its own word, and a panel of six coined words
  is six things to learn where the rule is one. **A rule that covers the second case has to be
  written before the second case, or it is written differently.**
- **The console is about to have other things that move.** The order of work already has the rest of
  the mixer, the other surfaces and four more bays; the picture and the beat grid already move. A
  vocabulary of static words is a decision to have no idiom for *in flight*, taken by default.

The word does not go away, and should not: it stays as what the **status line** says and as what any
tooltip will say. It is the destination named in prose, which is the second clause satisfied by a
different means — not a replacement for saying, on the surface, that nothing has landed.

### The console holding the pending state

Rejected. A `bool` or an enum kept in the panel would go stale the moment a MIDI map, a key or an
MCP call moved the request without the console seeing it — which is not an edge case, it is the
premise of *one vocabulary, four ways in*. The console would then animate a request that had been
withdrawn, or sit still over one that stands, and **there is no assertion to write against a stale
pixel**: it fails silently and looks like a slow panel.

Deriving it costs two accessor calls per strip per frame against the deck the harness already holds,
which is less than the two galley lookups `claim` already pays per strip per pointer event
(ADR-0185). It is also the shape every other reading in this crate has: `Strip` is what the deck
says this frame, and a value the console kept would be the first exception.

### The console holding the constraint

Rejected, on the two arguments now in P-0090.

**A rule that binds one surface binds none.** A lock in the panel makes a pad do what a chip
refuses, and an operator finds that out by experiment, mid-set — the failure
`docs/manual/console.html` names in its own words about a different control: *"a control that
quietly declines the last of something is a rule an operator can only find by experiment."*

**And it would refuse the one operation that is never refused.** A console that blocked *go on air*
while a prime request was parked would contradict the engine outright: the governor never takes a
Live slot off air ([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) —
which, read exactly, forbids automatic demotion and does not speak to granting requests), and
`deck.rs`'s module doc supplies the other half: *effective Live and requested Live are the same set
of slots.* A Live request lands. A surface is not entitled to a rule the instrument does not have.

Where such a constraint is genuinely wanted it goes where the record is applied, and the surfaces
draw the refusal — which the vocabulary already does elsewhere: `docs/manual/operations.html` refuses
*Wipe the next deck in* with **“Mask, opacity, blend and residency land immediately; the mask's
travel is scheduled. Refused with no shape chosen.”**

### What it costs, as a number

**The panel repaints for as long as anything is pending.** That is the whole of the cost, and it
should be read as a rate rather than as a feeling. The rate is the presentation's, which is why
P-0075 requires that it be *declared* rather than fixing it:

- A blink slow enough to read as *waiting* rather than as a fault is about **1 Hz** — two phase
  changes a second — so the panel asks for `Repaint::After(500 ms)`.
- A half-roll **once a second** is a continuous movement rather than two phases, so it asks for
  whatever step the roll is drawn in: fifteen steps through the half-turn over a third of a second
  is `Repaint::After(22 ms)` while it is moving and `Repaint::After` out to the next roll for the
  two thirds of a second it is not — **fifteen frames a second, not sixty**, which is the whole
  reason a deadline is the right shape for this.

Either way, `Repaint::After` is how it is paid and it already exists: it names a **deadline** rather
than asking for a frame, which is exactly the distinction any of these want, and `Repaint::soonest`
can only bring a frame forward, so adding one cannot lose `egui`'s own deadline or the picture's.

Against [ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s measured 184
allocations and 226.2 kB per drawn frame, the blink is roughly **370 allocations and 450 kB a
second** and the roll about **2800 and 3.4 MB** — nothing beside a picture at 59.7 fps, and not
nothing beside the still panel this crate measures at **0 frames, 0 allocations, 0 bytes**. **`egui`
is immediate mode: what repaints is the panel, not the chip.** So the honest statement of the cost
is that a parked deck ends the still panel, and a parked deck can stand for the length of a set. It
is also why the two presentations differ by nearly an order of magnitude on one axis and by almost
nothing on the operator's — which is the trade a scheduler exists to make.

**This is what makes P-0072's scheduler load-bearing rather than optional.** The roadmap has it as
worth doing *"when the mixer's controls make it move rather than before"*, and this is that. The
argument is not the megabyte; it is that the panel is acquiring **live regions at different rates**,
and each one taken as a special case is a rate somebody hard-codes. The beat grid is drawn today as
a row of dots with exactly one lit — `TransportRow::on` is `Transport::beat`, and `.beat-grid i` is
`var(--c-line)` where `.on` is `var(--c-pink)` with a glow — so it is a **0/1 switch per dot**,
changing two to four times a second. The maintainer wants it analogue eventually. **A continuous
glow at the tempo and a pending animation at another rate cannot both be special cases**: two rates
and two staleness tolerances is the scheduler's job description, and writing either one by hand is
writing the first half of it badly.

### The mock has no animation at all

Verified: `docs/manual/style.css` — 570 lines — contains no `@keyframes`, no `animation` and no
`transition` property. Nothing in the published design language moves.

**So this introduces a kind of thing the design language does not have**, and the record says so
plainly rather than letting it arrive as an implementation detail. It is a real cost: the mock is
the reference implementation is checked against, and a behaviour that exists only in `egui` is a
behaviour the two cannot be compared on.

**The mock gains it when the first user lands, not now.** Writing `@keyframes` into `style.css`
ahead of a control that moves would put a behaviour in the reference that nothing has been decided
about — the mock is written ahead of the interface on purpose, but ahead of the *decision* is how it
stopped agreeing with the code before (`console.html` listing four blend modes against the engine's
three, ADR-0187). It also means the mock is where the choice between the two presentations gets
made, which is the cheapest place to make it.

## Consequences

- **The tally's open question is closed in part.** What it has to *say* while the two values
  disagree is decided; **which presentation it uses is not**, and neither is the phase's units, its
  carrier on `View`, or which residency a press on the chip asks for. `docs/roadmap.md` keeps what
  is still open and points here.
- **P-0072's scheduler moved from *eventually* to *next*.** Item 2 of the order of work said it was
  worth doing when the mixer's controls make something move; this is the record that makes something
  move, and it adds a third rate to the picture's and the beat grid's.
- **The console will need a phase, and it must not be an `Instant`.** `karakuri-console/src/` reads
  no clock — every `Instant::now` in the crate is in `examples/panel.rs` — and the transport row's
  doc already argues the general case: *"an `Instant` here would put a clock in it, and then the row
  would be reading wall time in a repository whose first principle is that nothing does."* The phase
  arrives as a value on `View`, beside `transport` and `strips`.
- **A phase passed in is testable and an animation is not**, which is a consequence of that seam
  rather than a bonus: a test writes the phase it wants and asserts the appearance, with no window,
  no clock and no sampling. The still-frame clause is then an assertion rather than an intention.
- **The console still draws no tooltips**, and the manual's third rule wants one — *hover says three
  things: what the control is, what state it is in, and what a click will do*. A tooltip needs
  `egui` to own a widget where this console paints, and that is a decision about who owns the
  pointer (`view.rs`'s `outputs` writes the sentence). P-0075 no longer *depends* on it — the roll
  names the destination on the surface, which is why the second clause is written as *from the
  surface itself* — but the gap is still owed.
- **`karakuri-cli`'s `park` is unaffected.** A status line is not a panel; four fixed columns with
  no clock behind them is the right answer there, and P-0075 is about a surface that already
  redraws.
- **Nothing in `crates/` changed in this record**, and neither did `docs/manual/`. This is a decision
  taken before the work, which is what §4 asks for and the reason the transcripts were not needed to
  reconstruct it.
