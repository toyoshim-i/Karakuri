---
id: 0394
title: Render resolution is settled solely by the Program bay selection and scaled to outputs
status: accepted
date: 2026-10-14
supersedes: [0247, 0325]
superseded_by: []
principles: [0086, 0091, 0094]
tags: [engine, ui, outputs, performance]
---

# Render resolution is settled solely by the Program bay selection and scaled to outputs

## Context

[ADR-0246](0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md) established that the render size belongs to an output rather than the session, but left open the first question under *What this does not settle*:

> Whether two outputs at two sizes cost two renders or one render and a scale. Nothing here answers it. Today the deck's slot targets, the mix and the present pass are one canvas, and `Present::draw` already fits that canvas into targets of different sizes a frame — so *one render, scaled per output* is what the code would do if nothing changed, and *render per output* is what the words most plainly say.

[ADR-0247](0247-one-frame-is-rendered-and-scaled-into-each-output.md) and [ADR-0325](0325-the-frame-follows-the-largest-enabled-output-and-a-resize-costs-0-145-ms.md) resolved the scaling question by choosing to composite once at the *largest enabled output's size*, including the Program bay preview rectangle and external projector windows.

In practice, that derivation contradicted operator intent and created unwanted coupling:
1. **Retina and high-DPI displays**: The console's Program bay preview rectangle alone frequently exceeds 1280×720 on high-DPI screens. Taking the maximum between the preview rectangle and the dropdown selection meant that choosing HD (1280×720) or FHD (1920×1080) had no effect on actual rendering dimensions.
2. **Thermal budgeting on large displays**: As recognized in ADR-0246, "a thermal budget is a reason to send 1080p into a 4K window" ([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)). Forcing the render size to the 4K projector window prevented operators from downscaling to preserve frame rates under thermal load.
3. **Divider drag reallocations**: Sizing from the preview rectangle meant every pixel of an interactive divider drag resized render targets and dropped shader cost estimates on the render thread.

## Decision

1. **One render, scaled per output**: Answering ADR-0246's question, the frame is composited exactly once per frame, and each enabled output (the Program bay monitor view, projector windows, Syphon/plugin sinks) scales that rendered composite into its destination bounds.
2. **Settled solely by Program bay selection**: Outside WebXR, the composite rendering resolution is determined solely by the operator's selection in the Program bay dropdown menu (`View::selected_resolution`).
3. **The preview frame never dictates render resolution**: The Program bay preview frame is an inspection monitor, not an output destination. It scales the composite frame and never dictates rendering dimensions.
4. **WebXR native eye resolution**: In WebXR mode, the native eye dimensions (configured via the VR dropdown menu) decide composite dimensions.
5. **Deterministic resizing & logging**: Target resizing occurs only when the operator changes the resolution selection (or enters WebXR), emitting a diagnostic log line (`render size: WxH (Program bay selection)`). Divider drags and window sizing do not trigger GPU reallocations.

## Consequences

- The unused `output_resolution_changed` flag in `karakuri-console` is eliminated.
- Selecting a resolution in the Program bay dropdown immediately dictates GPU texture dimensions across both desktop and browser builds.
- Supersedes ADR-0247 and ADR-0325.

## Alternatives rejected

**Deriving render size from the maximum active output (ADR-0325)**. Overrides explicit operator intent on high-DPI screens and prevents thermal downscaling when outputting to 4K projectors.

**Rendering separately per output at native size**. Re-rendering complex compute passes and fragment shaders for each output violates [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) by introducing uncontrolled GPU budget multiplications.
