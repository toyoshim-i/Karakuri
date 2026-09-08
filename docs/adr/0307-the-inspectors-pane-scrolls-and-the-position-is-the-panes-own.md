---
id: 0307
title: The Inspector's pane scrolls, and the position is the pane's own
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0082, 0090, 0091]
tags: [console, ui, inspector, layout]
---

# The Inspector's pane scrolls, and the position is the pane's own

## Context

An Inspector pane drew a node group **whole or not at all**. `view.rs` said why, and the argument
was a good one: *"A half group is worse than a missing one — a node head with two of its five
parameters under it reads as a node with two parameters, where a group that is simply not there
reads as a pane that has run out, which is what it is."* It was the `floor` `LibraryBay::rows`
takes, on rows of unequal height, and it was honest because there was **nowhere to scroll to**: the
Library bay's own foot says so — *"nothing scrolls — which is honest, because there is no scroll
position anywhere in this crate and inventing one would be a control"*.

**The price was that the bay whose whole content is parameter rows could draw none of them.** With
the pair a bare `cargo run -p karakuri` opens on, the first group is the L1's at
26.5 + 19 x 22.5 = **454 px** against an Inspector bay minimum of 151.5 — so below roughly **534 px**
of bay, not one parameter row is drawn. [roadmap.md](../roadmap.md)'s M5.5 carried it twice, and
carried the decision as open both times: *"The decision in front of it — scroll, or a group that can
be part-drawn — is this bay's rather than the arrangement's"*, and *"Whether that is a blocker or
the first thing this bay's own work has to fix is a decision about scrolling that no record here
takes."*

**It is not a minimum this can be fixed by.** [ADR-0279](0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)
closed the same shape of hole on the other axis — a pane declares 208 across, because a pane that
cannot draw a fader is not a minimum — and said outright that the column axis is different: *"the
Inspector's declared minimum does not fit the Inspector's contents down the column, and no minimum
can fix that one."* A minimum tall enough for one *Set* is a minimum that depends on what somebody
loaded, and [ADR-0250](0250-below-the-minima-the-arrangement-scales-rather-than-being-rewritten.md)
already says a minimum is a preference and not a guarantee.

**The maintainer answered it on 2026-09-08**, on the fourth of the questions put to him:

> 4はスクロール

## Decision

**The pane's body scrolls under its two heads, and the position is the pane's own state.** Five
parts, and each of them is a smaller decision that could have gone the other way.

**1. What scrolls is the body.** `.half-head` — which deck this pane is showing — and `.deck-head`
— what that deck's clock is doing — stay where they are, and the node groups move under them.
Everything under those two rows is only legible once you know whose it is, so a head that scrolled
away would take the meaning of what is left with it.

**2. The wheel is the way in, and it is the only one.** `input::wheeled` answers *which pane is the
pointer over*, `Readout::pointer`'s wheel arm turns that into `View::scroll_by`, and the pointer is
what decides which of the two panes moves. **The keyboard's route is not bound here.**
[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
makes the arrows walk a bay's items, and this bay's are its panes and a pane's are its node groups —
so a walk that scrolled would follow from that grammar and not from this record. It is M5.13's, and
a key invented here would be the panel deciding a question
[operations.html](../manual/operations.html) reserves.

**3. A part-drawn group is now the right answer, and the count is what makes it one.** The rule this
replaces was right about the failure — a node head with two of its five rows under it reads as a
node with two parameters — and wrong about the remedy, because it answered a lie with a blank. What
removes the lie is that the head says how many groups are **whole**: `n of m`, the Library foot's
`5 of 27` counted on this bay's items, and `m of m` cannot be read off a pane with a group hanging
over an edge. That is rule 04 — *"A list that showed you part of itself says so and says how
much"* — and the rest of the group is one notch of the wheel away.

**4. There is no scrollbar, and the count is the mark instead.** A bar you can drag is a control,
which owes a row on [operations.html](../manual/operations.html) and a key and a map line beside it
under rule 01; a bar you cannot drag is a shape that looks like a control and refuses the hand,
which is rule 03 broken by a thing that cannot say what a click will do because a click does
nothing. The console draws no scrollbar anywhere and this does not add the first one.

**5. The position is clamped at the draw and stored unclamped against the pane.** `View::scroll`
holds one `f32` per pane; `inspector` clamps it to `0 ..= content - body height` and hands the
clamped value back as `InspectorPane::scroll`, and **nothing is written back**. Both ends of that
range move when a divider moves, so a clamp stored there would be a resize rewriting what an
operator scrolled to —
[P-0082](../principles/0082-looking-never-writes-back.md), and ADR-0250's rejected *clamp the stored
size during the solve* one region in. What the **store** clamps against is the content, which is a
reading of the deck rather than a viewport: a wheel spun over a short Set cannot put the number in
the thousands, and no pane's height reaches the stored value.

### It is pointer-state, so it has no row on operations.html

**The test the records give is not *is it visible* but *could anything else be the model of
record*.** `View::cursor_row` is refused a row because *"nothing downstream can be the model of
record for it, and a host that kept a copy would be keeping the console's state on its behalf"* —
the argument [ADR-0264](0264-a-reading-is-one-question-and-only-the-opening-asks-it.md) makes for
the reading's close and [ADR-0265](0265-a-carried-set-names-its-deck-at-the-release-and-the-panel-refuses-no-drop.md)
makes for the cursor a press moves. **A fold passes that test where this fails it**: a fold is a
state of the arrangement, the arrangement is a file an operator saves and restores
(`Operation::SaveArrangement`, `Panel::restore`), and *Fold a bay away* is a row because there is
something outside the console that is the record of what is folded. `karakuri-layout`'s tree holds
sizes and folds and holds no scroll position; nothing saves one, nothing restores one, and no
surface but this pointer could name one. So there is **no row**, and none of the nine places
`docs/contributing.md` §5 lists is touched — this is a §6 change and not a §5 one.

**The mirror of that is what a row would have cost.** A row needs a key, a MIDI target and an MCP
call under rule 01, and a map line names a slot, a range or a word from a closed list — *how far
down deck A's pane in the left window is* is none of the three, and a model asking for it would be
asking about a rectangle in somebody's window rather than about the instrument.

### It declares nothing

**A scroll is not a live region and gets no `Declared`.**
[ADR-0283](0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
made a declaration a claim about **motion** — *when this region's picture is next different from the
one on screen* — and a scrolled pane's picture is next different when a hand turns the wheel, which
is not a time this region knows. `budget.rs` already excludes it by name: *"What the operator does.
A drag on a divider, a fold, a room toggle: these are the operator's own load, they are transient,
and they are not budgeted."* A `Declared` for the Inspector would put a rate into
`Σ (cost / staleness)` for a region that is still between wheels, which is the over-declaration
ADR-0283 was written to remove. What the frames a scroll changes the picture on cost is one
`Change::Wheeled(Claim::Panel, true)` per notch, answered `Repaint::Now` — the
`Change::Pointer(Claim::Panel)` arm's own argument, one event along.

## Alternatives rejected

**Let a group be part-drawn and draw no more of it — no position at all.** The other half of the
question M5.5 wrote down, and the cheapest thing on this list: `pane_box` stops taking a `floor`,
`inspector_into` clips what it already clips, and nothing new is stored anywhere. It fails on the
case it was proposed for. A 454 px group in a 200 px pane draws its head and the first seven rows,
and the eighth to the nineteenth are unreachable **by any means** — no key, no pointer, no widening
short of a taller window. The bay's own exit condition is that no row of its ten carries a `plan`
badge, and *Write a parameter* would be `has` for a control an operator cannot get to. Part-drawing
is what this record does; what it refuses is part-drawing **without** a way to reach the rest.

**Scroll by whole groups — a first-group index rather than a distance.** Tempting, because it keeps
*whole or not at all* intact for whatever is on screen and the stored state is a `usize` that no
resize can invalidate. It does not solve the problem: the pane that draws nothing draws nothing
because its **first** group does not fit, so stepping to the second changes which group is missing
and not that one is. It also makes the last group unreachable in its lower half at every pane
height short of that group's own.

**Grow the Inspector's declared minimum until a group fits.** ADR-0279's move on the other axis, and
ADR-0272's before it. It cannot be made to work here and ADR-0279 says so: the tall dimension of a
group is `26.5 + 22.5 x rows`, and `rows` is what somebody's `.kir` published — 19 in the demo, and
nothing bounds it. A minimum derived from the loaded Set is a minimum that changes when a deck is
loaded, which is a window that resizes itself; a minimum derived from the worst case is a bay that
cannot be made small on a machine running a Set with three knobs.

**Put a scrollbar down the side of the pane.** The obvious drawing, and it loses on rule 01 before
it loses on anything else — see Decision 4. There is a weaker version, a bar that is only a mark,
and it loses on rule 03: *"Anything compact enough to be a symbol says three things when you point
at it: what it is, what state it is in, and what a click will do"*, and the third has no answer for
a bar nothing may drag. The count answers all three, because a readout's third answer is *nothing,
and here is why*, which the readouts on this console already say in as many words — the mixer head's
`3 of 3 · page 1` among them.

**Say how much with a percentage, or with pixels.** `62%` is exact and is a number about a
rectangle. The Library foot counts rows and the Mixer head counts strips, because what an operator
counts is items — and ADR-0259 already fixes what this bay's items are. A percentage would also make
the readout change on every notch of a continuous scroll, where a count changes when something
crosses an edge, which is the moment worth drawing attention to.

**Keep the position in `View::inspector`, beside the pane it is about.** Where it looks like it
belongs. It is refused by the same sentence that put `View::naming` in a field of its own: *"a pane
is rewritten whenever a Set lands, so a buffer kept in one would be a name that vanished mid-word"*
— and a position kept there would send the pane back to the top every time a build landed, taking a
knob out from under an operator's hand on a frame nobody touched. **One per pane and not one per
deck**, for the reason a second pane exists at all: two panes pointed at one deck are two places in
one list, and a position filed under the deck would move the pane an operator is not looking at.

**Clamp the stored position at the wheel, against the pane it is over.** One line, and at the moment
it runs it is indistinguishable from the right thing — the pane draws exactly the same picture.
ADR-0250 has the whole argument and this is its case: **it fails only across time**, because every
excursion into a short pane permanently rewrites where the operator was, and there is nothing to
restore it from. `tests/scroll.rs`'s `a_position_survives_the_pane_growing_and_shrinking` is what
holds it, and it is a defect a picture cannot show.

**Add a row to `PROBES` for the pane's body, so `claim` routes the wheel.** The smallest change that
would have routed the event, and it is wrong twice. `CONTROLS` is printed to an operator as *what
the pointer reaches here* and a pane's body is not something a press reaches — nothing happens when
you click it. And the probes are asked on a **press** as well, so the row would claim every press on
a pane's ground and hand it to an arm with nothing to do with it. `input::wheeled` is a second
question asked of the same derivations instead, and it asks `claim`'s rules 1 and 2 itself.

## Consequences

- **`karakuri_console::view::inspector` takes a fourth argument**, the position that pane is
  scrolled to, and every caller passes `View::scroll_in(index)` — the five probes in `input.rs`,
  `View::draw`, and the five press arms in `crates/karakuri/src/main.rs`. Tests that are not about
  scrolling pass `0.0`, which is the pane every one of them already had.
- **`InspectorPane` has three fields where it had one.** `scroll` is the position in force,
  `content` is how tall everything comes to, and `shown` has changed meaning: it counted how many
  groups were **drawn** and counts how many are **whole**. The two are the same number at rest, so
  every existing assertion on it holds; what walks the drawing and the hit tests now is
  `InspectorPane::drawn`, which is a range.
- **`InspectorPane::group` answers for every index in `nodes`** where it used to refuse anything
  past `shown`, because a scrolled pane has groups off both edges and `drawn` cannot be asked until
  the rectangle exists.
- **`InspectorPane::grip` refuses a press outside the body**, which is new and is the one thing here
  that fails silently: a knob scrolled up under the deck head still has a rectangle, and without the
  check the pane would claim a press on a control nobody can see, under a control that is drawn
  there. `select_renderer` beside it already asked.
- **`Change::Wheeled` carries a second field**, whether the console moved anything — `Change::Pointed`'s
  own shape — and `Repaint::Never` for a wheel the panel claimed is now `Repaint::Now` where it
  scrolled. A wheel withheld mid-drag and one spun against the top of a list are both still
  `Never`.
- **`Pointer::Wheel` carries a distance.** `crates/karakuri/src/main.rs` pulls the vertical half out
  of `MouseScrollDelta` — a `LineDelta` multiplied by `room::size::WHEEL_STEP` and a `PixelDelta`
  divided by the scale — and the sign is flipped, because `winit`'s positive `y` is a wheel pushed
  away and a scroll position is how far down the content the pane has come. The horizontal half
  reaches nothing here and is dropped where the two are pulled apart.
- **`room::size::WHEEL_STEP` is three parameter rows**, written as `PARAM_H * 3.0` because that is
  what `docs/manual/console.html` says a notch is worth.
- **Two sentences in `view.rs` were false and are narrowed rather than deleted.** The Library bay's
  foot said *"there is no scroll position anywhere in this crate"* and now says this bay has none;
  `View::walk`'s doc says the same thing about its own cursor and points at the pane that does. The
  Library's list still does not scroll and the reason is unchanged.
- **`docs/manual/console.html` gained a note and two readouts**, and `style.css` one rule —
  `.half-head .shown`, the Library foot's ink and measure. **No row on
  [operations.html](../manual/operations.html)**, and none of §5's nine places is touched.
- **Nothing declares.** `budget.rs` is unchanged, `tests/schedulable.rs`'s sums are unchanged at
  0.0889 against 1.0, and the Inspector is still not a live region — see *It declares nothing*.
- **M5.5's two scroll paragraphs are rewritten** and its *three more this bay owes* is two rather
  than three. What is left owed there is adding and removing a node, which is an engine item.
