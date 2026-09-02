# The render size is not part of the picture

The same Set drawn at two sizes is the same picture at two resolutions, and never two pictures. A
procedure is authored once and drawn wherever it is drawn — a projector, a capture at four times the
canvas, a preview cell — and none of those sizes is in the file or knowable from it.

**What it rules out.** A unit in pixels. A procedure that can ask how large its target is. Material
that thins out, fattens or reframes because the target changed. A reference resolution in the
specification, which is the same mistake written once instead of per file: an anchor only invites a
procedure to be authored against it and then be wrong everywhere else.

**Why it is a rule rather than a nicety.** It is what lets the render size be somebody else's to
choose. An output holds its own size and the destination window may set it
([ADR-0246](../adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md));
that is only safe while changing the size does not change what is drawn. Every mechanism below is
the price of that, and none of them is optional.

## Where it holds

- **`point_rate` is a fraction of the render target's height, not a count of pixels** — a sprite's
  extent and a stroke's width, both. `docs/ir-spec.md` specifies it under *`point_rate` is a
  fraction of the target's height, and not a count of pixels*, and the old spelling `point_size` is
  refused by name with the new one. The height rather than the width or the diagonal, so a change of
  aspect ratio moves the frame's edges rather than the sprite's.
- **A procedure has no way to ask.** `karakuri_ir::ast::Ambient` is `Seed, Copy, Point, Capacity, T,
  Beats, Dt, Camera, PointCoord, Eye, Ray, Source`. The generated uniform carries `viewport` because
  the lowering needs it to turn a fraction into a clip-space offset; it is engine state and is
  deliberately not an ambient, so nothing a procedure computes can depend on it.
- **A primitive below a pixel is drawn at one pixel and dimmed**, rather than dropped by the
  rasteriser wherever it failed to cover a pixel centre
  ([ADR-0245](../adr/0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md)).
  This is the clause that most nearly failed: the sample grid is a size the picture *would* have
  depended on, and arbitrarily — which sprites survived was decided by where they landed.
- **The picture letterboxes rather than stretches.** `Present::draw` fits the canvas into whatever
  it is drawn into, and the console gives the picture the canvas's shape rather than the region's
  ([ADR-0181](../adr/0181-the-picture-is-the-canvass-shape-and-the-leftover-is-the-consoles.md)), so
  framing survives a window nobody sized.

## Where it is not met

**Stated because a rule in force says where it is not yet true**
([P-0036](0036-an-invariant-that-is-not-yet-true-says-so.md)).

**Occlusion still moves with the size.** A primitive the one-pixel floor rounded up claims the
coverage a whole pixel would have claimed, so under an `over` mix it hides what is behind it as
though it filled the texel. Its colour is right and its occlusion is a texel's worth; ADR-0245
records the price and names what would remove it.

**"As far as it can be" is the honest reading of the rule, and a rasteriser is where it stops.** A
sample grid is a size, and everything above is the work of keeping the picture off it. A future
mechanism that quietly reintroduces a dependence — an effect measured in texels, a mask in pixels, a
threshold tuned at one resolution — is what this rule exists to catch, because none of them will
look like a pixel unit when it is proposed.
