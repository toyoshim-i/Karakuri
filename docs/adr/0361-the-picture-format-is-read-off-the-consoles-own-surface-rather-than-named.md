---
id: 0361
title: The picture format is read off the console's own surface rather than named
status: accepted
date: 2026-09-14
supersedes: []
superseded_by: []
principles: [0064, 0083]
tags: [runtime, gpu, outputs, projector, m6]
---

# The picture format is read off the console's own surface rather than named

## Context

[ADR-0324](0324-an-output-is-a-named-destination-with-a-size-and-an-on-off.md) gave the projector
window a clause of its own:

> **A projector surface must offer `PICTURE_FORMAT`.** `Present`'s pipeline names one target format
> at construction; a surface in another would want a second present pipeline, which is what
> ADR-0247 says nothing needs. The window refuses to open and the refusal names both formats.

`PICTURE_FORMAT` was a constant, `wgpu::TextureFormat::Rgba8UnormSrgb`, in
`crates/karakuri/src/bridge/sinks.rs`. The clause's own *what is not tested* section said the
refusal was "not reachable on this machine at all: Metal offers it."

It does not. wgpu's Metal backend offers `[Bgra8UnormSrgb, Bgra8Unorm, Rgba16Float, Rgb10a2Unorm]`
and no 8-bit RGBA sRGB format at any point. So `routed`'s membership test failed on every macOS run,
the projector chip printed

    outputs: the projector window cannot be opened — the present pass draws Rgba8UnormSrgb and this
    surface offers [Bgra8UnormSrgb, Bgra8Unorm, Rgba16Float, Rgb10a2Unorm] …

and no second window ever opened. What ADR-0324 wrote as a rare mismatch was the only case.

The constant was never checked against anything. Two other places in this workspace already ask a
surface what it offers rather than telling it: `App::resumed` picks the console's own swapchain
format as the first *non*-sRGB one `caps.formats` carries, because `egui` encodes for itself
([ADR-0162](0162-egui-is-handed-a-non-srgb-surface-because-its-shader-encodes.md)); and
`karakuri-cli` picks its present format as the first sRGB one and hands that to `Present::new`
(`crates/karakuri-cli/src/app.rs`). The desktop console was the one path that named a value.

## Decision

**The picture format is the first sRGB format the console's own surface offers, read off it once, in
`App::resumed`, beside the non-sRGB one the panel is drawn in.** It is stored on `Gfx` as
`picture_format` and threaded from there — to `Engine::new`, which hands it to `Present::new` and to
each of the five `Presented`s; to `Presented::made`, which is the texture's `format`, and whose
sampled view is that format's `remove_srgb_suffix()`; and to `routed`, which asks the projector's
surface for that same format and configures the swapchain to it.

`PICTURE_FORMAT` and `PICTURE_SAMPLED_FORMAT` are deleted. There is exactly one derivation of *what
the picture is drawn in*, and it is the value read in `resumed`. On Metal that value is
`Bgra8UnormSrgb`.

**A surface that offers no sRGB format at all refuses the run**, through `no_gpu`, naming the offered
list (P-0083): the present pass writes through the hardware's sRGB encode, so a run without one
would encode twice or not at all with nothing saying so. No machine this program targets is in that
state, and the sentence is what would say so if one were.

**ADR-0324's clause survives, narrowed to what it is actually for.** `routed` still refuses a
projector surface that does not offer the console's picture format, and still names both. Two
surfaces of one adapter offer the same formats, so the refusal is now reached only where the
projector's window lands on a display the console's adapter does not drive — which is the rare case
ADR-0324 meant and did not have.

## Alternatives rejected

- **A present pipeline per surface format.** It makes the constant harmless: whatever a surface
  offers, build a pipeline for it. It is also a second present pipeline, which is exactly what
  [ADR-0247](0247-one-render-is-scaled-into-every-output-and-a-wedged-one-is-dropped.md) says
  nothing needs — one render is scaled into every output, and a second pipeline is a second thing
  that can disagree with the first about a tone map, a viewport or a letterbox. The defect was never
  that one format is too few; it was that the one format was picked without asking.
- **`Bgra8UnormSrgb` as the new constant.** It is what Metal offers and it is what this machine
  would have run on. It is still a guess about a display: it makes the program correct on the
  platform it was last debugged on and leaves the next backend to fail the same way, with the
  refusal again naming a format the machine was never going to have. A value the surface gives
  cannot be wrong about the surface.
- **Reusing `config.format`, the console's own swapchain format.** One read instead of two, and it
  is already on `Gfx`. It is the wrong quantity: that one is deliberately *non*-sRGB because `egui`
  encodes in its own shader (ADR-0162), and drawing the picture into it would put the pipeline's
  linear values into a gamma-space target — P-0064's one encode, skipped, with no error anywhere.
  The two formats are read from the same `caps.formats` and are different answers to different
  questions.

## What is not tested, and why

The projector window still cannot be opened from a test — `ActiveEventLoop::create_window` needs a
live event loop and `mod gpu` has none (ADR-0324) — and neither can `resumed`'s pick, which reads a
surface's capabilities. What is pinned instead is the source, in `mod tests`'
`a_picture_format_is_a_value_read_off_a_surface`, beside the scan ADR-0324's successor left:
no `Rgba8UnormSrgb` or `Bgra8UnormSrgb` is named anywhere in `crates/karakuri/src` outside `tests/`,
and the projector's one membership test is in `routed` and is against `gfx.picture_format`. The
device tests under `mod gpu` name a headless stand-in once, `HEADLESS_PICTURE_FORMAT`, because they
have no surface to read.
