# A pending transition shows where it is, where it is going, and that it has not arrived

A control whose request has been made and not yet granted has three things to say and must say all
three: **the state it is actually in**, **the state it was asked for**, and **that the second has
not happened**. Any presentation that says all three is admissible; none of them is the rule.

**What it says that a colour cannot.** A control drawn from what is true *now* cannot separate
*nobody asked* from *asked, and waiting*: those are the same value and they are opposite
situations. The failure is silent and it is the operator's — a request that has been deferred reads
as a request that was discarded, and the only way to find out which is to press again.

## The rule is the property, and the animation is one instance of it

This began as a sentence about lamps: *on request the state it is in stays lit, the state it was
asked for blinks until it lands, and when it lands the new one is lit and the old goes dark.* That
is a good presentation and it is **an example**, not the requirement.

Written into a principle it would fix a shape — two indicators, and a blink. A surface with room for
only one would then be outside the rule rather than covered by it, and would invent its own idiom
off the books; a surface where a blink is wrong for some other reason would do the same. That is
[P-0060](0060-name-the-property-not-the-shape.md) exactly: **a shape is one instance of a property**,
and naming the shape leaves the property to be reinvented wherever the shape does not fit — in a
form that is harder to find afterwards, because nothing advertises it.

So the clauses below are what a presentation is *checked against*, and the choice of presentation
belongs to whoever draws the surface.

## What any presentation must do

- **The state the thing is actually in stays legible and unambiguous.** A pending transition never
  overwrites the truth with a wish. Whatever moves, the operator can still read where the deck *is*
  at any moment, because that is the thing they are about to act on.
- **The destination is identifiable from the surface itself.** Not from a tooltip alone, and not
  from a log. The request is half of what is being said, and half of it delivered only on hover is a
  compact control that has to be interrogated to be read.
- **It reads as unsettled in a single frame.** A screenshot, a test, or a compositor that has
  stopped servicing the window catches the surface at one instant. Every instant of it must say
  *not yet*; a presentation with a phase indistinguishable from the settled appearance is a
  presentation that lies half the time, and it lies exactly where it is most likely to be captured.
- **It declares a price and a staleness**, in milliseconds, and is scheduled like anything else that
  moves — [P-0072](0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md). The rule
  is that the number is declared, not what the number is: presentations differ, and a roll once a
  second is a different deadline from a blink twice a second.

## Two worked examples, with what each buys and costs

### A pair of lamps, where there is room for two indicators

Old lit, new blinking, swap on arrival — the sentence above. It satisfies the first two clauses
directly and cheaply: both states are drawn, one of them is the truth and steady, the other is the
request and moving.

**Its weak clause is the third**, and that is a real constraint rather than a caveat. A blink caught
at its bright phase looks like a lamp that is simply on, so the blink must swing between **two
appearances that are both distinct from the settled one** — otherwise half the still frames of it
are a picture of a transition that has not happened. That is a condition on the animation, and it
has to be stated because the obvious way to write a blink violates it.

### A roll, where there is one indicator and no room for a second

Where a second indicator cannot be drawn, the one that is there **rolls half a turn toward the value
it was asked for and falls back**, once a second — a slot machine that never quite lands. The word
in the chip is the current state at rest; the word coming up behind it is the destination.

The constraint is real and was measured rather than argued about: a mixer strip is **53 wide inside
`.strip`'s `padding: 7px 4px`**, and three chips side by side do not fit — the blend row's three
words are **49.75 of the 53 available as bare glyphs** and **53.75** with the thinnest separators
anyone would draw, before the chip that shares the row is placed
([ADR-0187](../adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md) has
the numbers and the alternatives that were tried against them; the tally's own words are not
shorter).

**What it buys over a blinking mark is the second clause**: the destination is named on the surface,
in the same 53 pixels, rather than deferred to a hover the panel does not currently have. **And it
satisfies the third clause better than a blink does** — a half-rolled word cannot be mistaken for a
settled one, so *every* frame of the animation reads as unsettled, where a blink relies on a
condition being honoured to make *most* of them do.

**What it costs**: a taller clip region than a static chip needs, a second galley for a word the box
was not measured against, and a rate of its own to declare.

Neither example is the rule. A third presentation that meets the four clauses is admissible without
amending this file, which is the point of writing the clauses rather than the pictures.

## One phase for the panel, and it arrives as a value

**Everything pending moves together.** Two controls moving out of step looks broken rather than
informative, and it is not a smaller cost either: N independent animations are N deadlines for a
scheduler to service where one phase is one.

The console owns that — *one* phase, panel-wide — but it does not own the clock that advances it.
`crates/karakuri-console/src/view.rs` reads no clock at all, deliberately: the transport row's doc
says an `Instant` there *"would put a clock in it, and then the row would be reading wall time in a
repository whose first principle is that nothing does"*
([P-0002](0002-simulation-time-comes-from-a-record-never-from-a-clock.md)). Every `Instant::now` in
that crate is in `examples/panel.rs`, which is whoever owns the window and the device.

So the phase arrives the way `Transport` and `View::picture` arrive — **a plain value written per
frame by whoever has the clock** — and `src/` stays a pure derivation with no device, no window and
no clock ([ADR-0156](../adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)). A
presentation driven by a value is also a presentation a test can assert at a phase it chose, which
one reading wall time is not.

## The price is declared, and P-0072 is the rule for it

Whatever moves is a live region: it declares what its update costs and how stale it may get, and it
is scheduled into what is left. A pending transition's staleness is set by the presentation itself —
a half-roll a second, a blink at 1 Hz — which makes it one of the few tolerances on this panel that
is not a guess.

**State the cost honestly.** `egui` is immediate mode, so what repaints is **the panel**, not the
chip: the panel is never still while anything is pending, for as long as it is pending. The
mechanism to pay it exists and needs nothing new —
`crates/karakuri-console/src/repaint.rs`'s `Repaint::After(Duration)`, whose doc says `egui`
*"asked to be drawn again after this long, and it is naming a **deadline** rather than asking for a
frame: draw then, and not before."* A deadline is exactly what a once-a-second animation wants;
asking for a frame is what turns one into a spin.

## Whether a transition is pending is derived every frame, never stored

The engine holds both halves already. `Deck::residency` is the effective one,
`Deck::requested_residency` is the request, and `Deck::is_parked` is exactly
`requested == Priming && effective == Allocated`. A surface asks; it does not remember.

**A copy in the surface would be a second place the truth lives, and it goes stale silently.** A
MIDI map, a key or an MCP call moves the request without the console seeing it — that is the whole
premise of *one vocabulary, four ways in* — and a panel holding its own idea of what is pending
would then animate a request that has been withdrawn, or sit still over one that stands. There is no
assertion to write against a stale pixel.

## Where it holds

**Nowhere yet.** Nothing on the panel moves for this reason, and nothing declares a cost or a
staleness under P-0072 either. The first user is the mixer strip's residency chip, where a deck
asked to prime with no room sits **parked** — the request stands and the engine has not granted it —
and the rule is written for the relation rather than for that chip, because the console is about to
have other things that move.

Decided in
[ADR-0188](../adr/0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md), which carries
the alternatives, the number, and the fact that the published mock has no animation of any kind yet.

**What this file deliberately does not settle** is what a control with a pending transition may
*forbid* — the answer is nothing, and it is
[P-0076](0076-a-surface-owns-the-affordance-never-the-authority.md).
