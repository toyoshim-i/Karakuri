---
id: 0162
title: The panel's surface is not sRGB
status: accepted
date: 2026-08-24
supersedes: []
superseded_by: []
principles: []
tags: [ui, colour]
---

# The panel's surface is not sRGB

## Context

[P-0064](../principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)
says the pipeline is linear HDR and sRGB is encoded once, at final output. Every rule of thumb that
follows from it points the same way: when in doubt, let the hardware do the encoding, which means an
sRGB surface format.

**For the panel's window that is wrong, and wrong in a way nothing reports.** `egui`'s shader is
told its target is gamma space and writes gamma-encoded texels itself. Hand those to an sRGB
surface and the hardware encodes a second time: the whole panel washes out — pale, low contrast,
the night room greyed — with no validation error, no warning, and nothing in any log.

## Decision

**The panel's surface is a non-sRGB format, and `egui` does the encoding.**

This is not an exception to P-0064; it is P-0064 applied. The principle says sRGB is encoded **once,
at final output**, and for this window the toolkit *is* the final output — the panel's colours are
authored in sRGB, in `docs/manual/style.css`, and never enter the linear HDR pipeline at all. The
engine's own path is untouched: it composites in linear HDR and tone maps at the end exactly as
before.

**It is asserted rather than trusted.** `gpu::egui_paints_the_console_onto_a_device` renders the
console onto a device and reads the texels back: an empty bay's body must be exactly `--c-panel`,
and a divider must be nearer the ground than the bay. An sRGB target fails the first of those by a
couple of counts per channel — which is the point, because a couple of counts per channel is
precisely what nobody notices in a screenshot and everybody notices in a dark room.

That test began as *it rendered without complaining*, and **the first defect injected against it
passed** — Metal tolerated a missing font-atlas upload. Reading the pixels back is what made it a
test.

## Alternatives

**An sRGB surface, following P-0064 literally.** The one that will be re-proposed, by someone
citing exactly that principle and being right about everything except which pipeline this is.
Rejected on the double encode, and the reason it needs a record rather than a comment is that the
failure is silent: nothing crashes, nothing warns, and the panel merely looks a bit flat.

**Tell `egui` its target is linear.** It has no such switch, and writing one would mean carrying a
patched shader.

**Convert in a pass of our own between `egui` and the surface.** A whole extra fullscreen pass, on
every frame of the panel, to undo an encoding we asked for.
