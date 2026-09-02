---
id: 0245
title: The sub-pixel compensation is paid in the colour, because alpha is coverage
status: accepted
date: 2026-09-02
supersedes: [0244]
superseded_by: []
principles: []
tags: [renderer, codegen, ir]
---

# The sub-pixel compensation is paid in the colour, because alpha is coverage

## Context

[ADR-0244](0244-a-sub-pixel-primitive-is-drawn-at-one-pixel-and-compensated-in-the-alpha.md)
settled the defect and the mechanism: a primitive of side `s` pixels with `s < 1` produces no
fragment unless its quad happens to cover a pixel centre, so it is drawn at one pixel with a
factor of `s²` — `s` for a stroke — carried from the vertex stage as a flat varying. All of that
stands. It also put the factor in the **alpha**, and rejected the colour on the grounds that
scaling both would dim by `s⁴`.

**That grounds was a step short.** `blend additive` lowers to `src_factor: SrcAlpha,
dst_factor: One` (`node/renderer.rs`), so what a slot target receives is `color.rgb * color.a`
whichever of the two the factor sits in: the *light* is exact from either channel, and there was
never a doubled factor to weigh against putting it in the alpha. Scaling both would have doubled
it; scaling one or the other does not, and the choice therefore turns entirely on what else each
channel means.

**A render target's alpha means coverage**, and it means it everywhere downstream:

- `composite.wgsl` reads a slot target as *"colour premultiplied by coverage, and coverage in
  alpha"*, and `MODE_OVER` hides the accumulator behind `covered = opacity * min(src.a, 1.0)`.
- The mix writes coverage back out — `covered + acc.a * (1 - covered)` — so the composited frame
  carries it.
- The master chain sits between the mix and the present pass and has not been built yet
  ([ADR-0224](0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md) names
  feedback, bloom and an rgb shift). `composite.wgsl` says why the channel exists at all: *"it
  exists because `over` needs it, and because coverage is what anything handed this frame outside
  the present pass would key on."*
- `oit_resolve.wgsl` reconstructs the same pair for `blend weighted`, so both L4 modes hand the
  mix one meaning.

So the alpha is a signal with named future consumers, and none of them is asking about a size.

## Decision

**The factor multiplies the colour. The alpha is left exactly as the fragment block wrote it.**

```wgsl
_color = vec4<f32>(_color.rgb * in.coverage, _color.a);
```

**Because a rounded-up size is not a coverage claim.** Paying the compensation in the alpha would
put a rasteriser workaround into the channel that means *there is material at this texel*, and
every later reader of it — the mix's `over`, the master chain, a key or a matte, an external sink
— would inherit that silently and have no way to tell it from real coverage.

**Under `weighted` it would not even stay a coverage claim.** The generated epilogue builds the
depth weight from the same number, `_w = _a * max(1e-2, pow(1.0 - _depth01, 3.0))`, so a
rounded-up sprite would lose weight against its neighbours at the same depth. That is not an
approximation of anything; it is a second, unrelated effect.

**Both modes stay consistent.** Under `additive` the target receives `rgb * s² * a` and the
coverage accumulates from the untouched `a`. Under `weighted` the accumulation carries the
dimmed colour, `oit_resolve` divides it back through the untouched `a * w`, and the revealage is
untouched — so the resolve hands the mix the same pair with the same meaning.

**The price, stated:** a rounded-up primitive claims the coverage a whole pixel would have
claimed, so under `over` it hides what is behind it as though it filled the texel. Its own colour
is right and its occlusion is a texel's worth. **This is what makes *approximate under `over`*
true under this record**, where under ADR-0244 the approximation was in the composition of
overlapping sub-pixel alphas. The error moved channel with the factor; it did not appear or
disappear.

## Alternatives rejected

**The alpha, which is ADR-0244.** Rejected on the reasoning above: it is exact for the light and
exact for the occlusion, and it buys both by writing a size correction into the one channel this
pipeline reserves for a different question. The cost lands on code that does not exist yet — the
master chain — which is the kind of cost that is cheapest to refuse now and expensive to find
later, because nothing in the frame would look wrong while it was being got wrong.

**Both channels, so that the occlusion is right too.** This is the `s⁴` that ADR-0244 named, and
it is genuinely wrong: `SrcAlpha, One` already multiplies them together.

**Split it by blend mode — colour under `additive`, alpha under `weighted`.** It would be exact
in each, and it makes the two modes disagree about what a slot target's alpha means, which is
precisely the property `oit_resolve.wgsl` was written to preserve: *"What leaves here is exactly
what the additive path leaves … `composite.wgsl` reads it that way whatever produced it, so
`blend weighted` reaches the mix without the mix knowing the mode exists."* One meaning per
channel is worth more than exactness in one of two modes.

**Carry the true coverage in a second channel, and let the mix read that.** It is the shape that
would be right, and it is a frame-sized target and a change to every consumer of a slot target
for a defect measured in one texel of occlusion. If sub-pixel occlusion ever reads wrong on
screen, this is the record to reopen.

## Consequences

- **`crates/karakuri-codegen/src/l4.rs`'s `fragment_entry` emits a whole-vector assignment**
  rather than a component one, WGSL having no assignable multi-component swizzle. Everything the
  vertex stages emit is unchanged from ADR-0244.
- **`tests/lines.rs` asserts both channels off one texel now.** A 0.4-pixel sprite carries 0.16
  in red and **1.0 in alpha**; a 0.4-wide stroke carries 0.4 and 1.0. The alpha assertion is what
  holds this record: it was run against ADR-0244's line as the injected defect and reports
  `alpha is 0.15991211 rather than 1.0`.
- **No cost figure moves.** The fragment count is identical either way, so `tests/deck.rs`'s
  measured pair stands as recorded.
- **`docs/ir-spec.md` states the occlusion price** beside the rule, under *A sprite smaller than
  a pixel is drawn at one pixel and dimmed to compensate*, and `docs/manual.md` says the same
  thing to an operator: what got rounded up hides a little more of the deck beneath it than it
  should.
