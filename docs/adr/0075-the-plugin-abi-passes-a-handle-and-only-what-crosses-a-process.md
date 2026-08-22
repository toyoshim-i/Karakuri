---
id: 0075
title: The plugin ABI passes a native handle, and only what can cross a process
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0043]
tags: [engine, process]
---

# The plugin ABI passes a native handle, and only what can cross a process

## Context

What crosses the output ABI is the whole design. Syphon and Spout are both **zero-copy GPU texture
sharing** — IOSurface on macOS, a DXGI shared handle on Windows. NDI encodes, so it wants CPU
pixels.

## Decision

**Pass a native handle, not pixels.**

An ABI that passes pixels makes NDI happy and **makes Syphon and Spout pointless**: it inserts a
full-frame readback every frame purely to hand the result back to the GPU — about a gigabyte a
second at 4K60, plus the pipeline stall. So the handle type is platform-specific, and opening a
plugin **negotiates surface kinds**: what the plugin can accept against what the host can produce,
and an empty intersection means it does not load.

The cost is `unsafe` in the host to reach a Metal texture or a D3D12 resource through `wgpu-hal`'s
`as_hal`. **But no Objective-C and no cmake** — that is the exact size of the win, and it is worth
writing down as such.

**And the ABI is restricted to what can cross a process boundary**: a shareable handle plus a frame
number. This is nearly free here, because IOSurface and DXGI handles are cross-process already —
Syphon is built on exactly that. Passing a `wgpu::Texture` pointer or a callback into the host would
close the door.

The door is worth keeping open because `dlopen`ing third-party code into the render process during a
live show is frightening on its own, and a panic or a C++ exception crossing an FFI boundary is
undefined behaviour that cannot be caught there.

## Consequences

- **Rot moves; it does not disappear.** This corrects something I had said earlier — that a
  default-off integration rots. A feature flag rots **loudly, at compile time**; a cdylib against a
  C ABI rots **silently, at runtime**, and a struct layout mismatch is a segfault mid-performance.
  So the protocol's first call is a **version handshake**, and a mismatch refuses to load and says
  both versions. Cheap, and the whole difference.

## Evidence

Session 2026-08-11T08:53Z. Standing rule:
[P-0043](../principles/0043-nothing-external-enters-the-render-process.md).
