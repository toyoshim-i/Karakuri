---
id: 0393
title: MCP capture_frame image tool with non-blocking readback
status: accepted
date: 2026-10-13
supersedes: []
superseded_by: []
principles: [0091, 0094, 0095]
tags: [mcp, capture, png, image, readback, webgpu]
---

# MCP capture_frame image tool with non-blocking readback

## Context

AI agent pair programming in Karakuri previously operated strictly on code and structured diagnostics: agents could inspect procedure source code, wire inputs, and query swap outcomes, but had no mechanism to visually perceive the resulting rendering of their changes.

Visual feedback is essential for creative shader development, generative art, and debugging visual defects (e.g. black screens, nan explosions, clipping, or incorrect camera projections).

However, adding frame capture to a live real-time VJ performance engine introduces significant architectural risks:
1. **GPU Stalls:** Reading pixels back from the GPU to the CPU (`map_async` and waiting for device completion) typically stalls the render thread or introduces pipeline hiccups.
2. **Memory & Bandwidth Overhead:** Unbounded image captures or frequent full-resolution 4K/8K frame downloads would exhaust memory and frame budgets, violating [Principle 0091](../principles/0091-cost-is-known-before-it-is-paid.md).
3. **Frame Drop / Audio Degradation:** Stalling the render loop interrupts the 60fps clock and causes audio dropouts, directly violating [Principle 0094](../principles/0094-the-show-does-not-stop.md).

## Decision

1. **Publish `capture_frame` MCP Tool:**
   - Add `capture_frame` tool to the MCP server adhering to the Model Context Protocol standard `ToolContent::Image` representation (`image/png` format, base64 encoded).
   - Parameters:
     - `target`: `"program"` (default) to capture the master mixed Program output, or `"slot"` to capture an individual deck slot preview.
     - `slot`: Deck slot index (required when `target: "slot"`).
     - `max_dimension`: Maximum bounding pixel dimension (integer between 128 and 1024, default 512). Aspect ratio is derived proportionally from the canvas resolution.

2. **Cost is Known Before It is Paid (P-0091):**
   - The bounding dimension is strictly bounded (`max_dimension <= 1024`). At default 512px, readback size is limited to ~512x288 texels (~589 KB raw uncompressed RGBA).
   - Allocation and transfer costs are predictable and bounded on every invocation.

3. **The Show Does Not Stop (P-0094):**
   - Frame capture never stalls or blocks the render thread:
     - In the render loop, pending `capture_frame` requests are drained before `compose`.
     - In `compose`'s single submission command encoder, the target (Program or slot) is rendered to an offscreen target texture via `Present::draw` or `Present::draw_with_bind_group`, and copied to a staging readback buffer via `copy_texture_to_buffer`.
     - Drawing and copy commands execute synchronously within the normal frame render pass in one submission, without pipeline barriers.
     - Buffer mapping, row unpadding, color space normalization (BGRA to RGBA swap if required by native display surface formats), and PNG compression run strictly asynchronously:
       - On native/desktop: offloaded to an asynchronous background worker thread (`std::thread::spawn`).
       - On browser/WebAssembly: mapped via non-blocking WebGPU callback on `map_async`.
     - When encoding completes, the worker signals `CaptureReply::settled(Ok(CapturedImage))` which fulfills the pending MCP response.

4. **Structured and Multi-Modal Output:**
   - Returns both explanatory text and base64 PNG image content (`ToolResult::text_and_image`), augmented with structured metadata (`width`, `height`, `format`).

## Rejected Alternatives

- **Blocking `device.poll` in the Render Loop:**
  Rejected because waiting for GPU readback on the render thread causes frame stutters and violates Principle 0094.
- **Returning Raw Pixel Buffers or URLs:**
  Rejected in favor of standard MCP inline base64 `image/png` content supported natively by modern multimodal LLMs.
- **Uncapped Resolutions:**
  Rejected to prevent sudden out-of-memory spikes and performance degradation during live sets (Principle 0091).
