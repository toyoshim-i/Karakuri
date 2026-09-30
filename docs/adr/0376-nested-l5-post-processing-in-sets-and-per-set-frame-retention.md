---
id: 0376
title: Nested L5 post-processing procedures in Sets and per-Set frame retention
status: accepted
date: 2026-09-29
supersedes: []
superseded_by: []
principles: [0086, 0090, 0091, 0092, 0094]
tags: [engine, environment, set, l5, post-processing, feedback, retains, m10]
---

# Nested L5 post-processing procedures in Sets and per-Set frame retention

## Context

Originally, `kind L5` (image-processing) procedures were confined exclusively to the global Master chain ([ADR-0340](0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)). Decks (A, B, C, D) ran Sets composed strictly of generator sources (L1), transformation nodes (L2), cameras (L3), and surface renderers (L4).

Under that model, any screen-space post-processing effect—such as lens distortion, glitch slicing, CRT phosphor bloom, or temporal feedback—could only be applied globally after deck compositing. This imposed two severe limitations on live performance:
1. **Lack of deck isolation:** A performer could not apply heavy analog distortion or feedback trailing to Deck A while blending it against crisp geometry on Deck B.
2. **Missing localized state:** Self-contained visual styles that rely on temporal recursion (such as reaction-diffusion, slime mold simulation, or decaying motion trails) could not be packaged into reusable Sets (`.kset`).

Furthermore, [ADR-0238](0238-retains-in-an-l5-reads-the-frames-own-picture-and-the-pass-allocates-two-textures.md) left an explicit open question: if an L5 were nested inside a Set, which texture constitutes `held`?

## Decision

**1. Lift Compiler Rejection of `kind L5` in Sets:**
- Update `karakuri-environment::compile::sort_compiled` to recognize and accept `kind L5` procedures within Set declarations.
- Extend Set `Plan`, `Wiring`, and `Names` to represent nested L5 image passes.

**2. Intermediate Render Target and Execution Pipeline:**
- In `Set::draw`, when a Set declares a nested L5 procedure, allocate an off-screen HDR render target (`Rgba16Float`) matching the deck output size.
- Preceding L4 surface renderers rasterize into this intermediate target.
- The L5 pipeline compiles into an `ImagePass` that executes as the final stage of the Set's rendering graph, reading the intermediate target as `src` and outputting to the deck's primary render surface.

**3. Per-Set Frame Retention (`retains`):**
- When a nested L5 declares `retains`, allocate a secondary persistent ping-pong texture (`held_texture`) scoped exclusively to that deck slot.
- `held` samples from the Set's own previous frame output.
- Frame retention is completely isolated per deck slot: scrubbing, hot-swapping, or muting Deck A has zero impact on Deck B's retained history, satisfying [P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md) and [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).

**4. Parameter Routing and Signal Modulation:**
- Nested L5 parameters resolve through the same `effective` and `effective_vector` pipelines used by L1–L4 nodes.
- Nested L5 parameters can be bound to real-time audio and musical signals (`onset`, `beat`, `spectrum_0`..`spectrum_7`) via standard Set `bind` declarations.
- Uniform packing unifies around `node::write_params`, correctly handling scalar `f32` parameters as well as `vec2` and `vec3` vectors through component sub-keys (`glaze.x`, `glaze.y`).

## Consequences

- Reusable visual instruments with intrinsic feedback and screen-space stylization can be authored, saved, and shared as self-contained `.kset` files.
- The deck mixer operates on fully post-processed deck layers, enabling wipes and blends between radically different visual aesthetics.
- VRAM overhead for intermediate targets (~16 MB @ 1080p HDR) is paid only when a Set declares an L5 procedure, upholding [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md).
