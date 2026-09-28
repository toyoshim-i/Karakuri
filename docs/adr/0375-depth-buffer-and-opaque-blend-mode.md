---
id: 0375
title: Depth buffer, blend opaque, and draw-order verification within a Set
status: accepted
date: 2026-09-28
supersedes: []
superseded_by: []
principles: [0083, 0084, 0086, 0091, 0092, 0094]
tags: [engine, ir, codegen, depth, blend, l4, m10]
---

# Depth buffer, blend opaque, and draw-order verification within a Set

## Context

Prior to Milestone 10, all rasterization in Karakuri operated entirely without a depth buffer (`depth_stencil_attachment: None` across every render pass). Two blend modes existed for L4 renderers:
1. `blend additive`: Emissive accumulation into an HDR target with `over` alpha tracking.
2. `blend weighted`: Two-pass Weighted Blended Order-Independent Transparency (WBOIT) resolving into HDR.

As procedural meshes, shared-vertex topologies (`triangles`, `grid`, `ribbon`), and non-emissive surface shading models (MatCap, Toon, faceted low-poly) enter the system in M10, the lack of depth buffering presents critical visual failures:
- In `blend additive`, overlapping polygon faces blow out to white rather than depicting solid volume.
- In `blend weighted`, surface color is averaged across depth. Under the WBOIT weighting function $w(z, \alpha) = \alpha \cdot \max(10^{-2}, (1 - d)^3)$, a 1-unit-thick solid mesh located 3 units away from a camera with near plane 0.1 and far plane 52 receives roughly 52/48 front/back color blending. For reflective MatCap materials and quantized cel shading, this produces severe visual wash-out and transparency artifacts on supposedly opaque geometry.

Furthermore, unlike OpenGL contexts which historically created a depth attachment implicitly, WebGPU/`wgpu` requires explicit allocation of depth textures, explicit configuration of render pipeline depth-stencil states, and explicit pass attachments.

## Decision

**1. Introduce `blend opaque` as the Third L4 Blend Mode:**
- Add `Blend::Opaque` to `karakuri-ir::ast::types::Blend`.
- In L4 WGSL lowering, `blend opaque` generates a standard color write (`write_mask: ALL`, replace blending) that writes both depth and color, with coverage locked to 1.0.

**2. On-Demand Per-Set Depth Target Allocation:**
- A `Depth32Float` target is allocated strictly on demand: only when a Set contains at least one renderer declaring `blend opaque` (or requesting depth writes).
- Sets consisting purely of `additive` or `weighted` renderers incur zero depth buffer VRAM allocation, defending [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) (~8 MB @ 1080p, ~33 MB @ 4K).
- Depth targets remain internal to the Set's rendering passes and never leak into the deck mixer; deck mixing continues to operate on 2D compositing layers.

**3. Depth Testing for Non-Opaque Renderers (Early-Z and Occlusion):**
- When a depth target is present:
  - `blend opaque` tests and writes depth (`depth_write_enabled: true`, `depth_compare: LessEqual`).
  - `blend additive` and `blend weighted` test depth without writing (`depth_write_enabled: false`, `depth_compare: LessEqual`).
- This allows opaque geometry to occlude downstream additive particles and transparent surfaces, enabling early-Z fragment rejection before shading.

**4. Strict Set Draw-Order Rule and Rejection:**
- Renderers within a Set draw strictly in declaration order.
- To prevent non-deterministic sorting and respect artist control ([P-0084](../principles/0084-the-artist-controls-the-composition.md)), a Set that declares an `opaque` renderer after a non-opaque renderer is rejected at build time with `SetError::OpaqueAfterNonOpaque`.
- In accordance with [P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md), the refusal explicitly names the offending renderer and gives the actionable fix ("move renderer 'X' before non-opaque renderers"). Silent reordering is strictly prohibited.

**5. Placement-Level Depth Testing (`test` / `no test`):**
- Whether a non-opaque renderer tests depth is a Set wiring property (`depth_test: bool`, default `true`), not declared in `.kir`.
- Setting `depth_test: false` allows a renderer to draw unconditionally over prior geometry (e.g. UI overlay, HUD lines, or unfettered particle overlays) without allocating extra render targets, adhering to [P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md).
- `blend opaque` always tests and writes depth regardless of placement flags.

**6. Fullscreen Fields at the Far Plane:**
- A fullscreen L4 procedure (e.g. procedural skybox or raymarched background) without custom depth output is rasterized at the far plane ($z = 1.0$).
- When depth test is enabled, the skybox naturally appears behind all rasterized opaque meshes in the Set, eliminating the need for manual depth-clearing workarounds.

## Consequences

- Opaque procedural meshes, MatCaps, and cel shaders render with solid occlusion and accurate surface reflections.
- Transparent and additive effects properly occlude behind solid meshes with zero sorting overhead.
- VRAM footprint is preserved when depth is unused.
- The Set authoring contract remains deterministic, explicit, and informative upon refusal.
