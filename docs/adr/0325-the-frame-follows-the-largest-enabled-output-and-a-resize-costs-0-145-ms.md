---
id: 0325
title: The frame follows the largest enabled output, and a resize costs 0.145 ms
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0086, 0091, 0095]
tags: [engine, ui, performance, outputs, m5]
---

# The frame follows the largest enabled output, and a resize costs 0.145 ms

## Context

[ADR-0247](0247-one-frame-is-rendered-and-scaled-into-each-output.md) decided that the frame is
composited once and scaled into every output, **at the largest enabled output's size**, and closed
with *"the frame's single size is a derived number now, not a setting … Nothing is built by this
record."* [ADR-0246](0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)
names M5.6 as the owner of building it, and says of what stood there instead:

> `crates/karakuri/src/main.rs`'s `const CANVAS: (u32, u32) = (1280, 720)` is wrong twice over now:
> it is a session-wide constant where the size belongs to an output, and it is a measurement default
> — the workspace's reference workload — standing in for an instrument's.

[ADR-0324](0324-an-output-is-a-named-destination-with-a-size-and-an-on-off.md) gave outputs an
identity. Two of them now have sizes: the program view's is the rectangle the Program bay gives the
picture, and a projector window's is the window the operating system gave it. So the derivation can
be written — and the moment it is, **it moves on every frame of a divider drag**, because the Program
bay's rectangle does.

That is a reallocation on the render thread, which is
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)'s territory: `Present::resize`
remakes the HDR target and the master chain's four, and `Deck::resize` remakes every slot target,
rebinds the composite and the meters, and forwards to each slot's `HotSwap::resize` — which resizes
the Set's renderers and **drops its estimate**, because `Estimate::target` says which size it
answered for.

## The number, because a decision here needs one and not an argument

**Measured 2026-09-09 on this machine (Apple M-series, Metal, `cargo test` profile — optimised with
debuginfo), host clock, biased high.** A four-slot deck of `drift_shell`/`soft_points` at 10240
elements, resized sixty times each way after four warm-up resizes, `Deck::resize` and
`Present::resize` together:

| render size | per size change |
|---|---|
| ~466 x 262 — a Program bay at the narrowest console | **0.145 ms** |
| ~1920 x 1080 | **0.149 ms** |

The two being the same within noise is the finding, not the absolute: what the call pays for is
about nine objects being made, not their texels. Against a 16.7 ms frame it is under 1%, and against
`budget::PANEL_PASS` — 1.26 ms, the panel's own declared cost — it is about a ninth. It is paid on
the frames a size changed and on no others, and those are the same frames `Presented::fit` was
already remaking the picture's texture on.

**This machine is evidence about this machine** (ADR-0110). What generalises is the shape — a fixed
cost per resize rather than one that scales with the target — and the reason to re-measure is a
platform where texture creation is not cheap.

## Decision

**The frame is composited at the largest enabled output's size, derived once a frame, and the
Program bay's rectangle is the program view's size.** A divider drag moves what the frame is
rendered at, and that costs 0.145 ms on the frames it moves.

- **Largest by pixel count, and the answer is always some output's own size.** A componentwise
  maximum is refused: 1920x1080 beside 1024x1280 gives 1920x1280, which is a size no output is and
  which upscales both of them in one axis while claiming to prevent upscaling. The tie goes to the
  earlier entry, which is the program view — the output an operator is looking at while they decide.
- **Every output off leaves the size where it was.** An output going off is not a statement about
  what the frame should be rendered at, and re-deriving there would reallocate everything in order to
  change a picture nobody is looking at, and again on the way back. ADR-0247's *"the session's
  `canvas` record is what it falls back to when no output says anything"* is honoured as the run's
  **starting** value: `Deck::new` and `Present::new` open at `CANVAS`, and the derivation says
  *nothing to say* rather than guessing. `render_size` returns `Option` for exactly that.
- **`CANVAS` stays, with a smaller and a different job**: the shape every output's rectangle is
  fitted to, and the size the run starts at. Both halves ADR-0246 called wrong are answered — the
  size is an output's, and what is left is a shape and a starting value.
- **The picture is fitted to the session canvas and not to what is rendered.** `aims` hands
  `CANVAS` to `picture_rect` where it used to hand `Present::size()`. The two were the same number
  by two routes while the frame was composited at `CANVAS`; they are not any more, and reading it
  off the `Present` would fit the picture to itself one frame late. The fixed point happens to be
  the same rectangle, which is exactly what would have made the mistake invisible: the reason would
  be circular and the picture would have no shape of its own, only whatever rounding it converged
  on. The console page's *"It is the canvas's shape rather than the region's"* is what this keeps
  true.
- **The reading names the size it was taken at.** `Costs::say` prints `Present::size()` rather than
  `CANVAS`, which is [ADR-0303](0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md)
  and [P-0095](../principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md):
  a panel figure is at *whatever the window was* and always was, and pinning the constant only made
  that unsayable.

## Alternatives rejected

### The picture's size is the session canvas, and its rectangle is only where that render is drawn

**The strongest one, and it is nearly right.** It costs nothing: the render size then changes only
when a projector window is resized, so a divider drag reallocates the picture's texture and nothing
else. It reads well against ADR-0246 — the picture has no window of its own, so *nothing else says*
and the canvas is the default. And it keeps every panel figure at one number.

**It loses on the case the manual names.** The Program bay's `solo` pill *"folds the panel away and
only the picture is left, which is also how you capture this window"*. Soloed on a 4K display the
picture's rectangle is about 3840x2160, and under this alternative the frame would be composited at
1280x720 and magnified into it — an upscale, which is the one thing ADR-0247 exists to prevent, in
the one place an operator is capturing the output. A rule that is right until somebody uses the
control the page recommends is not a rule.

**And the cost it saves turned out to be 0.145 ms.** The alternative was written to avoid a hitch,
and the hitch was measured and is not one. Refusing on an unmeasured fear is exactly what
P-0091 is against — *cost is known before it is paid* cuts both ways.

### Re-derive only when the size settles, and hold the last one during a drag

It removes the per-drag-frame reallocation without giving up the picture's own size. It loses on
having to invent *settled*: a timer or an idle count, which is a staleness nobody declared and a
second answer to *what is the frame's size* that is right at rest and wrong in the hand. With the
cost measured there is nothing left for it to buy.

### Quantise the render size to a ladder of steps

Fewer reallocations, and the picture is then a small resample instead of a blit — which is the one
thing `picture_rect`'s own documentation says the picture may not be, at length, and for a preview of
what is being captured.

### Render per output

Refused by ADR-0247 and not reopened. Two outputs would double the frame for a picture P-0086 says is
the same picture.

## Consequences

- **`render_size(&[Option<(u32, u32)>]) -> Option<(u32, u32)>`** in `crates/karakuri/src/main.rs`, a
  free function with the tie rule and the empty case written on it, tested on the CPU.
- **`Engine::aim` takes the projector's size and resizes the deck and the present pass together**
  when the derived size changes. Both, always: `Frame::render` checks its sizes and panics at the
  call site, so a deck resized without the present pass is a loud failure a frame later — the right
  failure in the wrong place to find out.
- **Nothing is printed when the size changes.** Every frame of a divider drag changes it, so a line
  would be sixty a second and sixty formats a second with it. The size is a readout: `Costs::say`
  prints it beside the numbers it is about.
- **A slot's estimate is dropped on every frame of a drag**, because `HotSwap::resize` drops it and
  the governor then budgets on the measurement (ADR-0296). **Nothing sets an estimate today** —
  `HotSwap::set_estimated_cost` has no caller in this workspace — so this costs nothing now and is
  recorded because it will not stay that way. Whoever wires the estimate meets it here.
- **A panel figure is no longer at 1280x720**, and it never really was: the reference workload is a
  named Set at that canvas measured on a harness (ADR-0270), and a bare `cargo run -p karakuri` was
  already not one. What changes is that the reading says the size instead of implying it.
- **`docs/manual.md`'s *The canvas is fixed for a run* is now false for the panel and still true for
  `karakuri-cli`**, whose `--canvas` is untouched. ADR-0246 predicted this sentence would go false
  *"when this is built"*. It is built; the sentence is `karakuri-cli`'s page and is not edited here,
  which is a gap this record names rather than closes.
- **The size pill on `docs/manual/console.html` reads a real number now**, and the mock's 3840x2160
  is a console with a projector on a 4K display — which is what the four chips beside it draw.
