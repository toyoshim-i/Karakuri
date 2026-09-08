---
id: 0279
title: The centre is two parameter rows wide, because a pane that cannot draw a fader is not a minimum
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: []
tags: [ui, layout]
---

# The centre is two parameter rows wide, because a pane that cannot draw a fader is not a minimum

## Context

[ADR-0272](0272-the-window-has-a-minimum-and-only-one-of-adr-0250s-three-cases-is-real.md) gave the
window a minimum inner size and, in the same breath, wrote down the one thing that minimum could not
reach:

> **It is a floor under the solve, not a promise about every control.** Two of the three width terms
> are read off content — `right-pane`'s 172 from four strips side by side, `left-pane`'s 160 from a
> library row that still reads as a name and a time. The third is not: `centre`'s 340 is
> `.body-grid`'s CSS track, and `inspector` says in as many words that its `.param` grid wants 207 in
> a pane before the fader has any width. At a centre of 340 a pane is 165.5 and the parameter faders
> are not drawn. **No window minimum can close that**, because a divider drag reaches a 340 centre at
> any window width at all; what would close it is the centre's declared minimum becoming 2 x 207 + 9,
> which is a decision about the arrangement and belongs in its own record.

**All of that stands except the figure.** This is that record, and the number it arrives at is not
423.

### What 207 is, and that it is a reading rather than a choice

`.param` is `grid-template-columns: 15px 88px 1fr 58px` with `gap: 8px` and
`padding: 3px 10px 3px 12px`, and every one of those is already in `room::size` with its citation.
Read across: 12 of left padding, the ordinal's 15, the name's 88, the value's 58, the three 8px gaps
between the four tracks and 10 of right padding — **207**, with the fader's `1fr` at nothing.

**A pane is exactly that wide.** `view::inspector` takes the pane's own rectangle out of the layout
and insets it only along the top; `pane_box` takes the two heads off the top; `node_into` gives each
row `rect.min.x` to `rect.max.x`. Nothing indents a row inside a pane, so a parameter row's width
*is* the inspector pane's width and 207 is a number about the arrangement rather than about a box
inside it.

### Why 2 x 207 + 9 is one pixel per pane short

The fader is the leftover track, and `param_into` draws it only where the leftover is positive:

```rust
if positive(track) { fader_into(…) }        // positive is width > 0 && height > 0
```

So at a pane of exactly 207 the leftover is exactly zero and **the fader is not drawn**. 2 x 207 + 9
= 423 is the width at which the defect is still there, one pixel per pane away from being fixed. It
was watched:
`tests/arrangement.rs::at_the_minimum_an_inspector_pane_draws_a_parameter_fader` fails at 423 with
*"at the minimum viewport: inspector-1 is 207 wide, and a `.param` row's fixed tracks want 207 before
the fader has any width"*.

**This is where the mixer's threshold and the fader's differ, and the difference is the direction of
the comparison.** `strip_box` returns `None` where `inner.width() < column_w`, so the right pane's
172 is a width at which the strips *are* drawn — the arrangement's minimum and the bay's threshold
are one number, as ADR-0272 says. A fader's condition is the other way round: it wants *more* than
its fixed tracks, so 207 is a width at which the fader is *not* drawn, and the minimum is the first
whole pixel above it. Every derived number in `lib.rs` is rounded to the whole pixel, so that is
**208** a pane.

### What is measured, at the arrangement's own numbers

Solved on the console's own tree, one width at a time, at 658.5 high — pane width, the fader's
leftover, and the mixer strip's inner width against `strip_box`'s 29:

| viewport | left | centre | right | pane | fader | strip inner |
| --- | --- | --- | --- | --- | --- | --- |
| 692 | 142.03 | 377.28 | 152.69 | 184.14 | **none** | 24.17 — **no strips** |
| 775 | 159.58 | 423.88 | 171.55 | 207.44 | 0.44 | 28.89 — **no strips** |
| 777 | 160 | 425 | 172 | 208 | 1 | 29 — four strips, nothing to spare |
| 1244 | 340 | 484 | 400 | 237.5 | 30.5 | 86 |
| 1440 | 340 | 680 | 400 | 335.5 | 128.5 | 86 |
| 1920 | 340 | 1160 | 400 | 575.5 | 368.5 | 86 |

The 1244 row is the mock's own console — `.console`'s `min-width: 1010px` gives a centre of 484 and
panes of 237.5 — and it is what a comfortable parameter row looks like: 30.5 of fader. The 777 row is
a floor and reads like one.

## Decision

**An inspector pane's declared minimum is one parameter row with a fader in it — 208 — and the
centre's is two of those over the pane divider: 208 + 9 + 208 = 425. `MINIMUM_VIEWPORT` becomes
777 x 658.5.**

- **`INSPECTOR_PANE_MIN` is a new private constant in `lib.rs`**, written as the arithmetic over
  `room::size`'s `.param` terms plus one pixel, so the row the pane is measured by and the row
  `view.rs` draws are one derivation. The one pixel is the console's own and is argued at the
  constant: it is the least leftover that is still a whole pixel.
- **The direction of the derivation is inverted, and that is the substance of this record.** The
  panes used to read their minimum off their parent — *(340 - 9) / 2, rounded down* — with the
  comment saying outright that this was deliberate *"because the content does not fit"*. Now the pane
  reads its own content and the centre sums the panes, which is what `right_pane` already does with
  four mixer strips and what `left_pane` does with a library row.
- **The height axis is untouched.** A node's `min` is stated along its parent's axis, so the centre's
  is a width and the body row's 556.5 is a height; `MINIMUM_VIEWPORT.1` is the same 658.5.
- **`tests/arrangement.rs` recomputes 777 from the tree**, as it did 692 — `implied_min` takes the
  larger of what a child declares and what its children imply, so the inspector's two panes and their
  divider have to come to the centre's declared 425 or the test names both numbers.
- **The new test is the property and not the number**: it recomputes 207 from `room::size`, asserts
  the solved pane is over it at `MINIMUM_VIEWPORT`, asserts it is over it by no more than a pixel —
  which is what makes 425 a threshold rather than a comfortable pick — and asserts the same after
  dragging the body row's first boundary to the far right at 1920, because the drag is what reaches
  the declared minimum at any window width and is therefore the half a viewport test cannot see.

**[P-0073](../principles/0073-a-node-claims-only-what-its-visible-content-can-use.md) is what says
this was a defect and not a preference**: *"a layout node's stored size and its declared minimum are
both claims about a node whose content is whole."* A minimum of 340 was a claim about a CSS grid
track, and the node's content could not be drawn in it. **No principle forces the floor from below** —
P-0073 caps a claim from above — and this record says so rather than borrowing it.

### ADR-0239's crossover is not touched, and that was checked rather than assumed

ADR-0239 flips the Program bay from *below* to *beside* by area, at a bay body **706** wide. The body
is the centre less `.program-body`'s 9 either side, so the crossover is a centre of 724 — reached by
widening the window to 1484, which is what `tests/rearrange.rs` pins as the pair 1483 / 1484 and what
still passes untouched. A declared minimum only binds from underneath, and 425 is under 706 as 340
was: at `MINIMUM_VIEWPORT` the body is 407 and the bay is *below*, before and after. Starving the
centre with a drag makes it narrower, which is the same direction. **Nothing in the placement decider
reads the centre's minimum**, and the default window's outer tracks — the 340 and 400 ADR-0239 chose
so a standard window opens *below* — are `Fixed` sizes and not minima; they are unchanged. (In
passing: ADR-0239's *"~644px"* for the default 1440 window measures 662 today.)

## Alternatives rejected

**Leave 340, which is the status quo.** It is the mock's `.body-grid` track transcribed, and it
describes a web page whose `.console` never goes under 1010 and whose panes are therefore never under
237. Read against *this* panel's tracks the same stylesheet gives a centre of 230, so the two CSS
numbers do not agree and the arrangement was holding the one that suits it. What it costs is the
Inspector bay's own pane being unable to draw the control the bay exists for, at a width the operator
reaches by dragging a divider at any window size — a bay that draws parameter rows with no parameter
in them and says nothing anywhere.

**2 x 207 + 9 = 423, which ADR-0272 and `roadmap.md` both name.** Rejected on the measurement above:
at 423 a pane is exactly 207, the leftover is exactly zero and `positive` refuses the fader, so the
figure closes nothing. It is the right derivation with the wrong end of an inequality.

**A scroll instead**, which is the sibling item under M5.5 — *"a pane draws a node group whole or not
at all, and there is no scroll position anywhere in the crate"*. It is the better answer to **the
other axis** and not to this one. That item is about height: with the pair a bare run opens on, the
first node group is 454px against an Inspector bay minimum of 151.5, so below roughly 534px of bay
not one row is drawn, and no minimum anybody would accept fixes that — a scroll, or a group that can
be part-drawn, has to. Width is not the same shape of problem: the rows are all one width, that width
is 207, and it is 22% of a 1440-pixel window rather than three times the bay. A horizontal scroll
would put the fader and the value column behind a gesture — a control an operator has to scroll into
view before grabbing, on the surface `docs/manual/console.html` describes as *nothing that scrolls out
of reach mid transition* for the bay next door. So the scroll stays owed for the column axis, this
axis is closed by the minimum, and the two are recorded as the two things they are.

**A smaller `.param` row**, if 207 were a choice. It is not: all six terms are transcriptions with
citations in `room::size`, held against `docs/manual/style.css` by
`tests/transcribed_constants_cite_the_mock.rs`. Making the row narrower is an edit to the mock — the
specification the panel is checked against — and it would buy at most a few pixels of a number that is
already small beside the window it lives in. The value column is the one place a parameter's number is
read and the name column already elides; neither is padding.

**Take the mock's other number — `.console`'s `min-width: 1010px`, a pane of 237.5 — and declare
that.** It is a defensible reading and it makes the fader 30.5 wide instead of 1, which is the width
the mock actually draws. It is rejected because it is the same mistake in a nicer suit: a minimum
transcribed out of a stylesheet rather than read off what the console draws, and this record exists
because that is what 340 was. It would also put the window's floor at 836 — 59 pixels of window
forbidden to buy comfort at a size nobody works at — and the console already has a number for *the
width at which the panel is comfortable*, `tests/common/mod.rs`'s `SMALLEST` of 1244, which is
precisely the 484 centre that reading implies. The two numbers stay two numbers, as ADR-0272 left
them.

## Consequences

- **The window's minimum inner size grows by 85 logical pixels**, to 777 x 658.5.
  `crates/karakuri/src/main.rs` passes `MINIMUM_VIEWPORT` to `with_min_inner_size` and needs no edit;
  the comment beside it still spells the old *"692 x 658.5"* and is owed a word, in a file that
  belongs to another session today.
- **The band from 692 to 776 changed character**, and this is the real cost. Every width in it used
  to hold all three tracks at their declared minima; now none of them is held and the solve scales
  (ADR-0250). At 692 the right pane is 152.7 and the Mixer draws no strips at all — which is exactly
  the state ADR-0272 refused the window into, so the band is unreachable by a drag for the same reason
  and by the same line. A 775-wide window is now a panel with no mixer strips, a parameter fader under
  half a pixel wide, and a library row 0.4 under its own minimum: below the floor, scaled, sane and
  not the panel its numbers describe.
- **What the outer panes' minima are for is unchanged, and nothing was taken from them.** `left-pane`'s
  160 is a library row that still reads as a name and a time; `right-pane`'s 172 is four mixer strips
  side by side, still met to the pixel at the new width — `tests/mixer.rs`'s
  `under_the_minimum_viewport_the_bay_draws_no_strips_and_the_previews_remain` passes unchanged,
  because the right pane is 172 at 777 exactly as it was at 692. The window minimum grew by exactly
  what the centre grew.
- **A drag can no longer starve the centre under 425.** At 1920 the body row's first boundary now
  stops with the left pane at 1075, and the inspector's panes never go under 208 at any window width
  or any divider position.
- **ADR-0272 is not edited.** Its diagnosis is what this record acts on and its figure is corrected
  here rather than there — an ADR is a description of history
  ([ADR-0151](0151-an-adr-is-a-description-of-history.md)), and the sentence it wrote was true of what
  was known when it was written.
- **`roadmap.md` loses an owed item in two places** — the M5.5 bullet and the sizing paragraph that
  hooks ADR-0272 — and both now point here. The scroll item beside it is untouched and still owed.
- **`tests/common/mod.rs`'s `SMALLEST` doc said the solve honours every minimum down to 692**, which
  was a description of an arrangement that had moved under it; it says 777 and names this record.
