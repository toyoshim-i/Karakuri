---
id: 0166
title: The engine's frame and the panel's are one submission
status: accepted
date: 2026-08-24
supersedes: []
superseded_by: []
principles: []
tags: [ui, colour]
---

# The engine's frame and the panel's are one submission

## Context

[ADR-0155](0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md) chose `egui` and paid a `wgpu`
major version on one claim: the panel and the engine share a `Device`. Half of that settled when
the versions resolved to one `wgpu`. The other half is whether a texture the engine rendered can
actually reach the panel, and it is settled here — the picture in the Program bay is
`Present::draw`'s output, letterboxed into the picture region and sampled by `egui` as an image.

Two things had to be decided to get there, and both have a plausible other answer.

## Decision

**One encoder, one submission, and the order inside it is engine, present, panel.**

`Deck::begin_frame` owns an encoder and `Frame::finish` submits it. An encoder of the panel's own
would be a second submission over a texture that is a colour attachment in one and a sampled
resource in the other — correct only by whatever ordering the queue happened to give. So the
panel's pass is recorded into the engine's encoder. The proof is a device test that reads the
picture's texels back: **recording the present pass after the panel's, in that same encoder, fails
it**, which is what a test for an ordering has to be able to say.

**The picture is one texture with two views: `Rgba8UnormSrgb` to draw into, `Rgba8Unorm` to
sample.**

The two shaders want opposite things from the same bytes. `Present`'s relies on the target format
carrying the sRGB transfer — that is where
[P-0064](../principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)'s
one encode happens. `egui`'s says outright that it expects textures which are **not** sRGB-aware,
and [ADR-0162](0162-the-panels-surface-is-not-srgb.md) is the panel's whole surface arranged around
that. A single format is wrong for one of them whichever is chosen: sRGB and `egui` decodes an
already-encoded picture; linear and the present pass never encodes.

Two views of one texture answers both without a conversion pass. The encode still happens exactly
once, in the present pass, and `egui` copies the bytes through untouched.

## What this cost, and it is stated rather than mitigated

**A live picture means the panel is never still.** The property that landed the same day — 0
frames, 0 allocations on an untouched window — stops applying the moment the engine runs, because
the picture is something changing. Measured on this window: **60.3 frames a second, a median 1.777
ms a frame, 10.7 % of a second spent drawing**, where what one frame costs did not change at all
and only how many frames pay it did.

Nothing here schedules, caches the panel to a texture, or declares a cost. That is
[P-0072](../principles/0072-a-still-panel-costs-nothing-and-what-moves-declares-its-price.md)'s
remaining clauses, and the number above is what they get decided on.

## Alternatives

**A second encoder and a second submission for the panel.** The obvious separation, and it makes
the dependency between the two passes implicit — the picture is written in one submission and read
in another, and nothing in the code says which comes first.

**One format for the picture.** Rejected above: it is wrong for one of the two shaders whichever
one is picked, and the failure is a picture that is subtly too dark or too pale rather than an
error.

**A conversion pass between the present pass and `egui`.** A whole fullscreen pass per frame to
undo an encode we asked for, and a second place for P-0064's *once* to become twice.
