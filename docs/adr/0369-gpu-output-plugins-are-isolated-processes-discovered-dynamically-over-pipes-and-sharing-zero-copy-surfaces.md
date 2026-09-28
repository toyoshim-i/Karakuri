---
id: 0369
title: GPU output plugins are isolated processes discovered dynamically over pipes and sharing zero-copy surfaces
status: accepted
date: 2026-09-26
supersedes: []
superseded_by: []
principles: [0092, 0094]
tags: [runtime, plugins, outputs, gpu, syphon, spout, m8]
---

# GPU output plugins are isolated processes discovered dynamically over pipes and sharing zero-copy surfaces

## Context

Karakuri routes composited video frames to external applications and display servers. On macOS,
the platform standard is Syphon; on Windows, the standard is Spout2; across networks, protocols
such as NDI are common.

Interfacing with these external systems directly inside the host render engine introduces severe
architectural hazards:
1. **Foreign toolchains and non-Rust dependencies**: Syphon requires Objective-C runtime interop;
   Spout2 requires Direct3D 11/12 and Windows SDK C++ headers; NDI requires proprietary dynamic
   libraries. Embedding these into the host binary would compromise a clean, closed Rust workspace
   and impose foreign toolchains on all contributors, even on platforms where those targets do not
   exist.
2. **Show-stopping crashes and undefined behavior ([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md))**:
   Loading foreign C/C++ or Objective-C code into the host render process means any segmentation fault,
   uncaught exception, or driver-level crash inside a plugin terminates the entire visual performance.
   Crossing foreign FFI boundaries with panics or exceptions is undefined behavior and cannot be
   safely caught.
3. **PCIe and CPU memory bandwidth bottlenecks**: Copying uncompressed 4K or 60fps framebuffers back
   to CPU host memory to stream across shared memory rings saturates memory buses (e.g. 1080p60 at
   ~500 MB/s, 4K60 at ~2 GB/s), inducing cache eviction, frame jitter, and dropped frames in the
   real-time engine.

[ADR-0358](0358-the-projector-is-fullscreened-by-the-operating-system-on-the-display-it-is-on-and-another-application-is-reached-through-a-plugin.md)
established that inter-application video sharing must be handled via out-of-process plugin sinks.
This record documents the dynamic discovery protocol, zero-copy GPU surface sharing, and decoupled
fault-isolation architecture implemented across M8 Deliverable 2 (`ab4b97c`, `d4bab71`, `6264ad8`,
`454004b`, `79b2ac5`, `c6d9d90`, and [docs/plugins.md](../plugins.md)).

## Decision

**1. Out-of-process execution and process isolation:**
- Output plugins run strictly as isolated child processes spawned by the host engine.
- Communication occurs over standard I/O pipes using newline-delimited JSON (ndjson).
- If a plugin crashes, freezes, or exhausts memory, its child process terminates cleanly while
  Karakuri's engine continues rendering uninterrupted ([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).

**2. Dynamic discovery via Hello handshake:**
- The engine scans the plugins directory resolved by `karakuri_environment::places::plugins`
  (evaluating `--plugins <DIR>`, `KARAKURI_PLUGINS_DIR`, macOS app bundle `PlugIns`, sibling
  `plugins/`, and workspace root).
- Candidate executables are probed at startup via piped standard I/O.
- The child executable immediately emits an ndjson greeting on stdout:
  ```json
  {"t":"hello","v":1,"kind":"output","name":"syphon","surfaces":["iosurface"]}
  ```
- The host validates:
  - Protocol version `v == 1` matching `PROTOCOL_VERSION`.
  - `kind == "output"`.
  - The plugin's reported `surfaces` list contains the platform's required surface protocol
    (`iosurface` on macOS, `dxgi` on Windows).
- Discovered plugins are registered on the console GUI. The Outputs bay dynamically presents the
  plugin's self-reported name (e.g., `Syphon`, `Spout`) on the output sink chip.
- The host terminates probe candidates within 1.5 seconds with `{"t":"close"}`; non-output binaries
  (e.g., tempo sources) and unresponsive processes are discarded without error.

**3. Hardware zero-copy GPU texture sharing:**
- The host bridge (`crates/karakuri/src/bridge/plugin_sink.rs`) negotiates native hardware surface
  handles across the process boundary:
  - **macOS (`iosurface`)**: Passes 64-bit `IOSurfaceID` handles backed by Metal textures.
  - **Windows (`dxgi`)**: Passes 64-bit NT shared handles (`HANDLE`) backed by Direct3D 11 /
    Direct3D 12 committed resources. The host Vulkan backend imports these via
    `VK_KHR_external_memory_win32` and `texture_from_d3d11_shared_handle`, matching DXGI adapters
    by LUID.
- Frame updates are signaled across the pipe with the surface identifier:
  ```json
  {"t":"frame","seq":1024,"surface":12345678,"width":1920,"height":1080,"format":"rgba8unorm"}
  ```
- No CPU pixel readback is ever performed.

**4. Non-blocking asynchronous frame drops:**
- The main window is the primary sink; output plugins are decoupled secondary sinks.
- If a plugin's IPC pipe is full or its rendering loop stalls, the host drops that output frame
  immediately without blocking or throttling the main engine render loop.

**5. Determinism boundary ([P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)):**
- Output plugins sit strictly downstream of compositing. They consume completed frame surfaces and
  record no journal events. A performance replays identically whether an output plugin was attached
  or omitted.

## Alternatives rejected

- **In-process dynamic libraries (`.dylib` / `.dll` via `libloading` / C FFI):**
  Loading foreign plugins directly into the host process exposes the render thread to fatal crashes,
  memory corruption, and driver crashes caused by buggy third-party plugins. Furthermore, it drags
  C/C++ compilers and foreign SDKs into the host build pipeline.
- **CPU shared memory transfers (POSIX shm / Windows named shared memory):**
  Streaming uncompressed RGBA pixel buffers over CPU memory introduces significant memory bandwidth
  penalties and latency, failing real-time 60fps budgets at high resolutions.
- **Static build-time plugin configuration:**
  Hardcoding plugin binaries or compile-time features defeats modularity and prevents deploying
  updates to output servers independently of the core Karakuri binary.

## Consequences

- Plugin implementations live in standalone sibling repositories (`Karakuri-syphon`, `Karakuri-spout`).
- Developer script `scripts/plugins.sh` provides standard commands to check out, build, and link
  plugins into `plugins/`.
- The core engine remains 100% pure Rust with zero foreign C/C++ build dependencies.
