---
id: 0244
title: A sub-pixel primitive is drawn at one pixel and compensated in the alpha
status: superseded
date: 2026-09-02
supersedes: []
superseded_by: [0245]
principles: []
tags: [renderer, codegen, ir]
---

# A sub-pixel primitive is drawn at one pixel and compensated in the alpha

> **Superseded 2026-09-02, later the same day, by
> [ADR-0245](0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md).**
> **One clause below reversed and the rest of it stands.** The floor at one pixel, `s²` for a
> sprite and `s` for a stroke, the flat varying, the non-positive rule, and every alternative
> rejected under *Leave it*, *MSAA* and *Clamp in the check pass* are all carried forward
> unchanged. What reversed is the channel: this record put the factor in the alpha and rejected
> the colour, and the maintainer's objection is that a render target's alpha is **coverage** —
> it survives the mix into the master chain
> ([ADR-0224](0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)) and is
> the one thing anything outside the present pass keys on, so a size that had to be rounded up
> does not belong in it. The reasoning below for preferring alpha is wrong in one step, which
> ADR-0245 names: `additive` blends with `SrcAlpha, One`, so the light is exact from either
> channel and there was never a factor applied twice to weigh against the coverage. Read
> ADR-0245 for the decision in force; this record is kept for the argument it got wrong.

## Context

A sprite of side `s` pixels — `point_rate` times the render target's height — produces no
fragment at all when `s < 1`, unless its quad happens to cover a pixel centre. There is no MSAA
and no hardware minimum point size to fall back on: `topology points` lowers to a quad pair
(`docs/ir-spec.md`, *How an L4 says what it draws*), so the rasterizer's coverage rule is the
whole of it.

Measured on one element: `examples/soft_points.kir`'s shipped rate of 4/720 covers sixteen texels
at 1280x720 and none at 128x72.

**The failure is selection rather than dimming**, and that is why it needed a decision. Which
sprites survive is decided by where each one landed against the sample grid, so a small target
does not show a fainter version of the picture — it shows a sparser and arbitrary one. Material
authored and judged at one size is a different artifact at another.

**It was found as a preview-cell problem and it is not one.** `tests/deck.rs` measured two ways
of filling a 112x63 cell and concluded that re-rendering into the cell is "cheaper and wrong",
because the sprites are about a third of a pixel across there and vanish. The same sprites vanish
on any low-resolution *output*, and what a low-resolution output even is has not been decided
(`docs/roadmap.md`, *The instrument has no resolution model*). The cell is where it was noticed,
not where it lives.

## Decision

**A primitive smaller than a pixel is drawn at one pixel, and the fragment's alpha is multiplied
by the coverage the floor took away.** One pixel is the smallest thing that can be drawn; scaling
by the coverage it should have had preserves the *integrated* contribution rather than the peak.

- **`s²` for a sprite and `s` for a stroke.** A sprite is short of coverage in both axes; a
  segment is short across its width and along none of its length.
- **The alpha, not the colour.** `blend additive` lowers to `SrcAlpha, One` and adds
  `color.rgb * color.a`; `blend weighted` weights its accumulation by `color.a`. The alpha is the
  one channel both modes scale a contribution by, and it is also the coverage L5's `over` reads.
- **The factor travels as a flat varying.** The size is known in the vertex stage and the colour
  is written in the fragment stage, so this is a shape change to what codegen emits rather than a
  constant folded in.
- **Inert at or above a pixel.** The floor and the factor are both no-ops once `s >= 1`, so
  nothing authored at a size where it was never sub-pixel renders differently. The reference
  workload is one of those: `soft_points` at 1280x720 draws sprites between 1.4 and 4 pixels
  across, so no host-clock figure in this repository was taken on material this touches.

**Exact under `additive`, approximate under `over`.** Adding light commutes with scaling it, so a
dimmed one-pixel sprite adds exactly what the sub-pixel one would have. Compositing does not: two
sub-pixel sprites in one pixel are composed as though each covered the whole pixel at a reduced
alpha, which is the ordinary coverage-as-alpha assumption and is wrong by however much they
actually overlap.

**A `point_rate` of zero or less draws nothing**, stated rather than left incidental. Zero already
did — a zero-extent quad has no area. A negative rate used to draw a sprite of `|s|` with its
`point_coord` mirrored; the floor would otherwise have clamped it up to a full pixel, and it is a
size, so below zero there is nothing to draw.

## Alternatives rejected

**Leave it, and hand it to whoever settles the resolution model.** It is the same defect either
way, and the floor does not depend on which resolution is chosen — it is right at every one.
Waiting means every low-resolution output silently loses material until a question nobody is
working on is answered, and the question it blocks on is the largest open one in the project.

**MSAA, or supersample the small target.** It buys real coverage rather than an estimate of it,
and it buys it on the frame path: a multiple of the fragment work plus a resolve, in exactly the
case whose premise is that the target is small because something has to be cheap. It also only
moves the threshold — below the sample count the same drop-out returns.

**Clamp `point_rate` in the check pass, or give the language a minimum.** There is no value a
checker could clamp it to. The rate is a fraction of a target the procedure never sees, and the
same `.kir` is a four-pixel sprite at 1280x720 and a sub-pixel one at 128x72; the unit was chosen
precisely so that a procedure does not know the size it will be drawn at (`docs/ir-spec.md`,
*`point_rate` is a fraction of the target's height, and not a count of pixels*). A minimum would
be a reference resolution in disguise, which that section refuses.

**Multiply the colour rather than the alpha.** Multiplying *as well as* the alpha applies the
factor twice and dims by `s⁴`. Multiplying *instead of* the alpha leaves the coverage the mix's
`over` reads untouched, so a sprite covering a hundredth of a pixel would still claim a whole
pixel of it — dark, and opaque.

**Compensate a stroke by `s²` as well, for one rule instead of two.** A one-pixel-wide stroke that
should have been a fifth of a pixel wide is short of coverage across its width only; squaring
would dim it by a factor it never lost, and the two topologies genuinely differ here.

## Consequences

- **`crates/karakuri-codegen/src/l4.rs` emits three new lines in the points expansion and two in
  `SEGMENT_EXPANSION`**: the side or width in pixels, `max(…, 1.0)` as what is drawn, and
  `out.coverage`. `_rate_ndc` is now `_drawn_px / u.viewport`, which is the same arithmetic the
  aspect-ratio term used to spell.
- **`VsOut` gains a `@interpolate(flat) coverage: f32`**, after `view_depth`, so every other
  varying's location is where it was. `fragment_entry` emits
  `_color.a = _color.a * in.coverage;` between the lowered body and the return.
- **The fullscreen path is untouched.** It has no `vertex` block, no `point_rate` and its own
  `VsOut`.
- **`tests/lines.rs` asserted the absence and now asserts the compensation.**
  `a_sprite_below_a_texel_drops_out_rather_than_being_drawn_small` is replaced by
  `a_sprite_below_a_texel_is_drawn_at_one_texel_and_dimmed_to_compensate` and by
  `a_stroke_thinner_than_a_texel_is_compensated_by_its_width_rather_than_its_square`, which is
  what holds the two exponents apart. Both were run against the defect with the floor and the
  factor removed, and again with the two exponents swapped.
- **What fills a preview cell is open again**, and this record does not settle it. `tests/deck.rs`
  said re-rendering into the cell is cheaper and wrong; the *wrong* was this defect, and it is
  gone. The panel downsamples today and nothing here changes that.
- **`tests/deck.rs`'s sweep was measured with the defect in it and its shape is not the shape it
  reports.** The paragraph claiming a 112x63 render rasterises `(63/720)²` of the fragments a
  1280x720 one does was true only because most sprites produced none at all; under the floor a
  sub-pixel sprite produces exactly one fragment, so a small target's fragment count for `Points`
  material bottoms out at the element count rather than falling with the area. The figures are
  re-measured and re-recorded in that file.
- **The roadmap's *Decided and not built* section is spent**, and what it named as still open —
  that the instrument has no resolution model — stays open under *The decisions nobody has taken*.
