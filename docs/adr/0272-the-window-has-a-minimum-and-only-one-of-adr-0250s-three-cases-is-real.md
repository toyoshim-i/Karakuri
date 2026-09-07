---
id: 0272
title: The window has a minimum, and only one of ADR-0250's three cases is real
status: accepted
date: 2026-09-07
supersedes: []
superseded_by: []
principles: []
tags: [ui, layout]
---

# The window has a minimum, and only one of ADR-0250's three cases is real

## Context

[ADR-0250](0250-below-the-minima-the-arrangement-scales-rather-than-being-rewritten.md) says what
the solve does below the minima — every child is scaled by `avail / total`, nothing stored is
touched — and it justifies needing that at all by naming three ways a window comes to be narrower
than the minima:

> A window can be narrower than that — **dragged there**, **restored there from a session saved on
> a larger screen**, or **arrived at when a projector disconnects and the desktop reflows**

**Two of the three do not exist**, and the code says so in as many words.

**A restore does not carry a viewport.** `Panel::restore` — *"The viewport in `layout` is
discarded, and that is the decision rather than an omission — an arrangement carries the window it
was saved at, and a console arranged on a laptop would otherwise come back on a projector with the
laptop's margin round it."* `docs/manual/console.html` promises the same thing to the operator:
*"What it does not save is the window. An arrangement carries the viewport it was written at and
comes back at the one in front of you."* A saved arrangement's viewport is read off the file and
thrown away before the first solve, so a restore sets whatever viewport the window already has —
which is the case above it or the case below it, and never a third one.

**A display change is a resize.** Nothing per-screen is stored anywhere: `Layout` is built once
from one `Spec`, every `min` in it is fixed at that moment, and a projector disconnecting reaches
the panel as `WindowEvent::Resized` and one `set_viewport` — the same two lines a drag reaches it
through. **The minimum is a property of the arrangement and not of the screen.**

So one case is real, and a minimum inner size on the window removes it. **There is none today** —
no `min_inner_size` anywhere in the workspace; `crates/karakuri/src/main.rs` and
`crates/karakuri-cli/src/main.rs` each set an inner size and no bound on it.

What is left after that is a screen narrower than the window's minimum, and the answer there is
that the window is simply larger than the screen. That is an ordinary state of an ordinary
operating system, not a failure the layout has to have an arm for.

### Three figures, and all three are true of different things

The panel's startup legend prints the body row as `min 556` with `left-pane` 160, `centre` 340 and
`right-pane` 172 — which sum to 672 and not to 556. ADR-0250 says the console's row of three
declares *"minima summing to 700 plus 8 of divider"*, and its column *"272 plus 8"*. None of the
three is stale, and none is a rounding of another:

- **`min 556` is a height.** A node's `min` is stated along its *parent's* axis
  (`Spec::min`), and the body row's parent is the root column, so the number the legend prints
  for it is the 556.5 `arrangement()` declares as a height — the transport 48, this 556.5, the
  outputs 34. `{:.0}` renders 556.5 as `556`.
- **160, 340 and 172 are widths**, stated along the body row's axis. They sum to 672, and with the
  two 10px `COLUMN_DIVIDER`s to **692**.
- **700 + 8 and 272 + 8 are not the console at all.** They are
  `crates/karakuri-layout/tests/common/mod.rs`'s hand-built `console()` fixture: a row of three at
  160 + 320 + 220 = 700 with two 4px dividers, inside a column of 48 + 200 + 24 = 272 with two
  more. ADR-0250's *"stored 240 and 320"* are that fixture's stored sizes too. The fixture's own
  header says *"Nothing in the crate knows this"*; the record read a fixture named `console` as the
  console.

**[ADR-0174](0174-a-node-claims-only-what-its-visible-content-can-use.md) is not the reason, and
could not have been.** Capping a declared minimum by what a node's content can use is real, but on
the width axis it never touches these three: each pane is a split laid out **across** its parent's
axis, so `measure` answers `f32::INFINITY` for it, and `min(160, ∞)` is 160. The cap bites there
only when a pane's whole content is folded away — which is exactly why it cannot be the source of a
window minimum.

### What that costs today

`view::strip_box` returns `None` once a track is too narrow for the fader column — 17 of `.vfader`,
a 6px gap and 6 of `.vmeter`, 29 — so a mixer under it draws no strips at all. `view::preview_cells`
asks only for positive area, so the four deck previews are still drawn. **Between the two
thresholds the pointer cannot select a deck and the keyboard still can**: a press reaches a deck
only through `Mixer::select`, which reads the strip boxes, while `0`..`3` in
`crates/karakuri/src/main.rs` emit `Operation::SelectDeck` unconditionally. Swept on the console's
own arrangement, one axis at a time with the other at the arrangement's own minimum: at 658.5 high
the strips go below **692** wide and the cells hold to **99**; at 1244 wide the strips go below
**503** high and the cells hold to **285**. Everything between the two thresholds on either axis is
a panel with four deck previews on it that no press can reach a deck through.

## Decision

**The window gets a minimum inner size, and it is the arrangement's declared minima summed along
each axis: `karakuri_console::MINIMUM_VIEWPORT`, 692 x 658.5 logical pixels.**

- **692 wide** — `left-pane` 160, `centre` 340, `right-pane` 172, plus the two 10px
  `COLUMN_DIVIDER`s.
- **658.5 high** — `transport` 48, the body row 556.5, `outputs` 34, plus the two 10px
  `ROOT_DIVIDER`s.

The panel's viewport is the window's inner size in logical pixels (`main.rs` divides the physical
size by the scale factor before handing it to `Panel::set_viewport`), so the two are the same units
and the constant goes to `WindowAttributes::with_min_inner_size` unchanged.

**The declared minima, not the capped ones.** ADR-0174's cap moves with what is folded: a pane with
both its bays folded can use nothing and claims nothing, so a window minimum tracking the capped
sum would shrink as an operator folds and would leave a window that cannot be grown back to what
unfolding needs — the operator would fold a bay, drag the window in, unfold, and be below the
minima with no way out but the window manager. The declared minima are fixed when the `Spec` is
built and no drag, fold, restore, session or screen changes them, which is the only property a
window minimum can be built on.

**692 is exactly where the Mixer's last strip fits.** At a right pane of 172 a track is 37 and the
fader column is 29 inside 4 + 4 of padding, which is `strip_box`'s condition met with nothing to
spare — the arrangement's minimum and the bay's threshold are one number, as `right_pane`'s comment
already claimed. One tenth of a logical pixel narrower and the bay draws nothing.
`tests/mixer.rs::under_the_minimum_viewport_the_bay_draws_no_strips_and_the_previews_remain`
asserts the pair — four strips at the minimum, none just under it, the four preview cells drawn
either way, and no press anywhere on the bay selecting a deck.

**ADR-0250's decision stands whole and keeps its job.** What is wrong with that record is the
argument for why the scaling was needed, not the scaling. A viewport can still be `0 x 0` — a
window being created before its first `Resized`, and a window minimised — and both arrive at
`solve_split` as extents a minimum cannot be honoured in. `karakuri-layout`'s
`shrinking_below_the_minima_and_growing_back_reproduces_the_arrangement` and
`a_drag_survives_the_viewport_collapsing_and_coming_back` sweep to `0 x 0` and back and compare
every rectangle in the arena for equality; a window minimum does not reach either of them, and
nothing in this record proposes removing anything.

**It is a floor under the solve, not a promise about every control.** Two of the three width terms
are read off content — `right-pane`'s 172 from four strips side by side, `left-pane`'s 160 from a
library row that still reads as a name and a time. The third is not: `centre`'s 340 is
`.body-grid`'s CSS track, and `inspector` says in as many words that its `.param` grid wants 207 in
a pane before the fader has any width. At a centre of 340 a pane is 165.5 and the parameter faders
are not drawn. **No window minimum can close that**, because a divider drag reaches a 340 centre at
any window width at all; what would close it is the centre's declared minimum becoming 2 x 207 + 9,
which is a decision about the arrangement and belongs in its own record.

## Alternatives rejected

**No window minimum, which is the status quo.** It leaves the one case that is real. The drag is
also the worst of the three that were named, because it is the only one an operator performs on
purpose and gets no signal back from: the window narrows, the strips vanish, the preview cells stay,
and nothing anywhere says the panel has stopped being the panel its own numbers describe. ADR-0250's
*"visible and recoverable by making the window bigger"* is true of the picture and not of the
controls — a bay that has quietly stopped hit-testing looks like a bay.

**A minimum that tracks the capped minima**, so the window may be as small as the arrangement
*currently* needs. It is the more accurate number at every instant and it is the wrong invariant: it
moves as an operator folds bays, and it moves *downward* just when the operator is making room, so
the sequence *fold, shrink, unfold* ends below the minima with the window already at the smallest
size the window manager will allow. A bound that the thing it bounds can lower from under it is not
a bound.

**Refuse to draw below the minima rather than scaling.** ADR-0250 disposes of the neighbouring
alternative — *enforce the minima and let the last child go negative or overflow* — and not this
one, so it is carried here. A solve that answers *no rectangle* has to be a solve that can fail, and
every caller of `Layout::rect` then acquires an arm for a panel that has no geometry: hit testing,
clipping, the boundary a drag reads, the four bays that ask for their own region every frame. That
is a fallible answer threaded through the whole console to describe a state that lasts one frame
while a window is created. Scaling gives all of them a rectangle that is small, sane and
non-negative, and this record's window minimum is what makes the state rare rather than what makes
it representable.

**The console's claimed smallest window, 1244 x 658.5**, which `tests/common/mod.rs`'s `SMALLEST`
already names and which is the tempting answer to *the width at which every function still works*.
It is a larger number that buys no additional invariant: it holds the inspector's panes at 237.5
only in the default arrangement, and a drag on the body row's first boundary starves the centre to
340 at any window width — verified. So it would forbid window sizes the panel is fine at in order to
prevent a state it does not prevent, and would put the console's *content* claim where a *solve*
floor belongs. The two numbers stay two numbers, and `tests/common/mod.rs` now says which is which.

## Consequences

- **`karakuri_console::MINIMUM_VIEWPORT` is new**, and
  `tests/arrangement.rs::the_minimum_viewport_is_the_sum_of_the_declared_minima` recomputes both
  figures from the tree — the same guard the body row's 556.5 is under, because the model still does
  not derive a split's minimum from its children's.
  `one_pixel_under_the_minimum_no_region_holds_its_minimum` asserts the other half: at the constant
  every track is exactly its minimum, and a tenth of a pixel under it none of them is.
- **One line in `crates/karakuri/src/main.rs` is owed** — `.with_min_inner_size(...)` on the window
  attributes — and until it lands the constant is a claim nothing enforces. It is reported rather
  than made because that file belongs to another session today.
- **`crates/karakuri-cli/src/main.rs` takes no minimum.** Its window draws no console — it is a
  preview surface whose size is `--size` and whose contents are the fitted canvas — so there are no
  minima for one to come from.
- **The centre's declared minimum is under what its content needs, and is now written down as
  that.** 340 against 2 x 207 + 9 = 423. It is reachable by a drag with or without a window minimum,
  and closing it is a change to `centre()` and `inspector()` that moves the declared width minimum
  to 775 — a decision about the arrangement, owed, and hooked from `roadmap.md`.
- **ADR-0250 is not edited.** Its decision about what the solve does is unchanged and still has a
  job; what this record replaces is two sentences of its context, and an argument that would have to
  change is a new record (ADR-0151).
- **Two stale figures in the console's own tests were corrected while checking these.**
  `tests/arrangement.rs` said the tree's implied width sum was 846 — the number it was when the
  outer tracks' minima were 218 and 268, before ADR-0239 — and `tests/common/mod.rs` derived
  `SMALLEST` from a lede of *990 wide* while the constant beside it read 1244. Both were
  descriptions of an arrangement that had moved under them, which is what made the legend's figures
  look like a disagreement in the first place.
