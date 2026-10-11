# Karakuri Web / WebAssembly Architecture & Implementation Plan

---

## 1. Overview & Vision

The objective of the Web version of Karakuri is to deliver a **100% client-side, zero-backend, serverless static web application** running directly in any modern browser supporting WebGPU (Chrome, Edge, etc.).

Unlike native desktop Karakuri, which relies on host operating system facilities (POSIX/Windows PTYs, TCP sockets, and shared-memory GPU output handles like Syphon and Spout), the Web port reimagines these subsystems using browser-native standards:

1. **WebGPU-Native Procedural Shader Pipeline**: Karakuri IR (`.kir`) compiles to WGSL via `karakuri-codegen`, matching WebGPU's native shader language without any transpile step.
2. **In-Process Agent Harness & Direct Tool Execution**: Eliminates the PTY process manager and TCP-based MCP server in favor of an internal, in-memory agent runner connecting directly to OpenAI-compatible LLM endpoints.
3. **Web Audio & Web MIDI Buses**: Retains real-time audio FFT spectral analysis and hardware controller mappings via Web Audio API and Web MIDI API.
4. **Zero-Copy Multi-Window Projector**: Replaces OS texture sharing with direct same-origin multi-window WebGPU surface attachment on a shared GPU adapter/device, featuring transient Watchdog grace handling.

---

## 2. Core Architecture

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Karakuri Web (Browser)                          │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   ┌────────────────────────────────────────────────────────────────┐   │
│   │                      Console UI (egui)                         │   │
│   │  Program  ·  Staging  ·  Mixer  ·  Library  ·  Prompt Bay      │   │
│   └───────────────┬────────────────────────────────┬───────────────┘   │
│                   │                                │                   │
│         [In-Process Dispatch]              [In-Memory Tools]           │
│                   ▼                                ▼                   │
│   ┌────────────────────────────────┐   ┌───────────────────────────┐   │
│   │   WebGPU Engine (karakuri-     │   │ In-Process Agent Harness  │   │
│   │   engine + wgpu WebGPU target) │   │ (wasm-bindgen-futures)    │   │
│   └───────┬──────────────┬─────────┘   └─────────────┬─────────────┘   │
│           │              │                           │                 │
│   [Direct Surface]  [Frame Sink]              [Fetch Stream]           │
│           │              │                           │                 │
│           ▼              ▼                           ▼                 │
│   ┌──────────────┐ ┌──────────────┐    ┌───────────────────────────┐   │
│   │ Projector    │ │ Plugin Sinks │    │ OpenAI-Compatible API     │   │
│   │ Secondary Win│ │ ├─ WebRTC    │    │ (Ollama, LMStudio, Cloud) │   │
│   │(Shared GpuCtx)││ └─ WebXR XR  │    │                           │   │
│   └──────────────┘ └──────────────┘    └───────────────────────────┘   │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Subsystem Breakdown

### 3.1 Prompt Bay: In-Process Agent Harness (Replacing PTY)

#### The Problem on Web
Desktop Karakuri spawns local CLI processes (`agy`, `claude`, `ollama`, `sh`) via `portable-pty`. Browsers cannot spawn OS processes or create pseudo-terminals due to sandbox security boundaries.

#### The Solution: In-Process Agent Runner
Instead of delegating to an external binary, the Web build embeds an in-process agent execution loop:

- **Asynchronous Loop (`wasm-bindgen-futures`)**:
  - The Prompt bay acts as the UI frontend, capturing multiline prompts (including Japanese IME) and rendering streaming ANSI terminal output.
  - Submitting a prompt initiates an asynchronous task that streams directly to an OpenAI-compatible HTTP/HTTPS endpoint.
- **OpenAI-Compatible Client**:
  - Uses browser `fetch` (or `reqwest` with `wasm` feature) to call `POST /v1/chat/completions` with `"stream": true`.
  - Fully compatible with local LLM runners (Ollama with CORS enabled, LM Studio, vLLM) as well as cloud providers (OpenAI, Gemini via proxy, OpenRouter, DeepSeek).
  - Streamed Server-Sent Events (SSE) tokens are written directly into the Prompt bay's scrollback ring buffer.
- **In-Terminal CUI Configuration**:
  - Avoids adding complex visual settings forms by supporting slash commands directly inside the Prompt bay:
    - `/config endpoint <url>` (e.g. `http://localhost:11434/v1` or `https://api.openai.com/v1`)
    - `/config model <name>` (e.g. `llama3.2`, `gemini-2.0-flash`, `gpt-4o`)
    - `/config key <api-key>`
  - Configuration is persisted in the browser's `localStorage`.

---

### 3.2 In-Memory Tool Dispatching & WebMCP Bridge

#### The Problem on Web
Desktop Karakuri runs a TCP listener on `127.0.0.1:3000` via `std::net::TcpListener`, exchanging JSON-RPC messages with external agents. Browsers cannot open raw TCP listener sockets.

#### The Dual-Path Solution: In-Memory Dispatch & WebMCP Standard Bridge
Instead of a TCP loopback socket, Karakuri Web uses a unified in-process dispatch engine (`karakuri_mcp::in_process`) that serves two concurrent pathways:

```
               ┌───────────────────────────────┐
               │    14-tool Schema &           │
               │  In-Process MCP Dispatcher    │
               │   (karakuri-mcp::in_process)  │
               └──────────────┬────────────────┘
                              │
             ┌────────────────┴────────────────┐
             ▼                                 ▼
   【Path 1: In-Process Bay】           【Path 2: WebMCP & Browser Bridge】
 OpenAI format in-memory tools       document.modelContext.registerTool()
 (Direct LLM streaming dispatch)     window.__karakuri_mcp.callTool()
                                     (External AI, extensions, MCP-B)
```

1. **Path 1: In-Process Agent Harness (Internal CLI)**:
   - The 14-tool Karakuri MCP matrix (`read_slot`, `copy_slot`, `operate`, `get_permissions`, `check_set`, `build_procedure`, etc.) is declared as OpenAI-format tool definitions in the chat completion request.
   - When the LLM emits a `tool_calls` response chunk:
     1. The agent harness intercepts the function name and arguments.
     2. Calls `InProcessMcp::call_tool` directly in Rust memory.
     3. Awaits completion asynchronously via `Pending::poll_settled` without blocking the main event loop.
     4. The result JSON is appended as a `tool` role message, and streaming resumes.

2. **Path 2: WebMCP (W3C `document.modelContext`) & `window.__karakuri_mcp`**:
   - On startup, `karakuri-web` inspects the DOM for `document.modelContext` (or `navigator.modelContext`).
   - If present (e.g. Chrome with WebMCP enabled or via polyfills), all 14 tools are registered via `modelContext.registerTool(...)`.
   - External browser agents (Chrome built-in AI, side panel assistants, MCP-B bridges connecting Claude Desktop) can discover and call Karakuri tools directly over the web standard.
   - Additionally, `window.__karakuri_mcp = { listTools, callTool }` is exposed on the global object for testing and extension consumption.
   - Tool execution wakes up the `winit` event loop via `EventLoopProxy`, ensuring zero-latency frame processing and non-blocking Promise resolution.

---

### 3.3 WebGPU Rendering & Shader Pipeline

- **WGSL Native Compatibility**:
  - Karakuri's shading compiler (`karakuri-codegen`) emits WGSL.
  - WebGPU uses WGSL natively, meaning no shader transpilation, SPIR-V conversion, or feature loss occurs in the browser.
- **Egui & Wgpu Presentation**:
  - `wgpu` 30.0 natively compiles to the WebGPU browser backend.
  - `egui-wgpu` and `egui-winit` bind to the DOM HTML `<canvas>` element.
- **Timing & Platform Differences**:
  - `std::time::Instant` is abstracted using `web-time` (which maps to `window.performance.now()`).
- **Shader Compilation Workers**:
  - Master Chain compilation (ADR-0354) is scheduled via async tasks (`wasm-bindgen-futures`) or Web Workers to ensure 60fps presentation never stutters.

---

### 3.4 Audio & MIDI Buses

- **Web Audio (`karakuri-audio`)**:
  - `cpal` compiles to WebAssembly using the Web Audio API.
  - Incorporates user-gesture activation (`AudioContext.resume()` on first mouse click).
  - Preserves the 8 semantic spectral bands (Sub, Bass, Low-Mid, Mid, High-Mid, Presence, Brilliance, Air) computed via in-browser FFT (`rustfft`).
- **Web MIDI (`karakuri-midi`)**:
  - `midir` compiles with the `web-midi` feature, connecting directly to the browser Web MIDI API (`navigator.requestMIDIAccess`).
  - Physical MIDI controllers connected via USB work out-of-the-box for fader sweeps, parameter riding, and key learning.

---

### 3.5 External Outputs: Projector, WebRTC Streaming, and Immersive WebXR

#### The Challenge on Web
Desktop Karakuri manages external video distribution via OS-level facilities:
- Secondary full-screen monitors via native `winit` windows ([ADR-0358](adr/0358-the-projector-is-fullscreened-by-the-operating-system-on-the-display-it-is-on-and-another-application-is-reached-through-a-plugin.md)).
- Inter-application texture sharing via out-of-process GPU handles (macOS Syphon `IOSurface`, Windows Spout `DXGI`, [ADR-0369](adr/0369-gpu-output-plugins-are-isolated-processes-discovered-dynamically-over-pipes-and-sharing-zero-copy-surfaces.md)).

Web browsers run inside a sandboxed multi-process security model and do not expose raw OS shared-memory GPU texture handles. Karakuri Web provides three distinct browser-native output modalities corresponding to physical venue screens, broadcasting, and spatial performance:

---

#### 3.5.1 Projector: Direct Zero-Copy WebGPU Multi-Window Surface Sharing
Rather than running duplicate state machines via `BroadcastChannel` or copying heavy uncompressed 4K video framebuffers across browser windows (which induces PCIe/RAM bus saturation and framerate throttling):

1. **Same-Origin Popout Window (`projector.html`)**:
   - The operator activates `[ Projector ]` in the Outputs bay.
   - The browser opens a clean popout window via `window.open('projector.html', 'KarakuriProjector', 'width=1920,height=1080,...')`.
   - Loading a dedicated same-origin target (`projector.html`) ensures that the `window.opener` relationship remains intact without cross-origin isolation breakage.
2. **Direct WebGPU Surface Attachment (`Gpu::instance().create_surface`)**:
   - The main thread creates a secondary `wgpu::Surface` bound directly to the popout canvas (`#projector-canvas`).
   - Both the primary console window and the projector popout window share the exact same `wgpu::Instance`, `wgpu::Device`, and `wgpu::Queue`.
   - In every render frame, `App::render` acquires a swapchain texture from the projector surface and encodes a zero-copy blit/render pass directly into the main command encoder.
3. **Transient Watchdog Grace (`grant_watchdog_grace`)**:
   - Browser window creation, destruction, and resize operations incur transient compositor hitches (often 50–200ms) while the browser allocates new display swapchains.
   - Without compensation, Karakuri's 60fps overload watchdog (`BudgetWatchdog`) would register these transient hitches as shader pipeline stalls and gate/stop visual execution.
   - Karakuri Web invokes `engine.grant_watchdog_grace(Duration::from_millis(500))`, which resets the rolling median frame duration filter (`Period::reset`) and exempts the transition window from overload tripping.
4. **Key Advantages**:
   - **Zero IPC & Zero Copy**: Zero message serialization, zero state divergence, and zero duplicate GPU memory allocations.
   - **True Zero-Latency Frame Parity**: Projector output is 100% synchronized with console mixer faders, audio FFT analysis, and prompt actions down to the exact WebGPU presentation frame.
   - **Resolution Independence**: The console preview can run on a 1080p laptop while the projector window renders at native 4K (3840×2160) on the venue screen without overhead.
   - **Refresh Rate Decoupling**: The venue projector and laptop display swapchains are driven seamlessly by the unified WebGPU pipeline.

---

#### 3.5.2 Plugin Sink 1: Low-Latency Wireless Cast Streaming (Presentation API + WebRTC)
Corresponding to native desktop Syphon/Spout/NDI plugin sinks (ADR-0369), Web Karakuri provides serverless, ultra-low-latency wireless video distribution to smart TVs, Chromecast dongles, and remote screens:

1. **Clean Offscreen 1080p Canvas (`#karakuri-cast-canvas`)**:
   - The engine binds a secondary `wgpu::Surface` to an offscreen full-HD canvas (`#karakuri-cast-canvas`, 1920×1080).
   - In every render frame, `compose` writes the uncompressed clean program video (no egui console UI overlays) to the cast surface.
2. **In-Band Signaling over Presentation API (`PresentationRequest`)**:
   - The operator activates `[ Cast ]` (the 4th chip in the Outputs bay).
   - The browser opens the native device selector dialog for W3C **Presentation API** (`new PresentationRequest(['receiver.html'])`).
   - The receiver screen (TV, Chromecast, secondary browser) loads `receiver.html`—a zero-dependency lightweight HTML5 shell containing a single fullscreen `<video>` element with hardware acceleration.
   - The controller and receiver use the established `PresentationConnection` message channel to exchange SDP offer/answer descriptors and ICE candidates in-band. **No external WebSocket signaling server or cloud relay is required.**
3. **Hardware-Accelerated WebRTC P2P Stream (`canvas.captureStream(60)`)**:
   - Captured frames stream directly over local Wi-Fi via `RTCPeerConnection` to the receiver with glass-to-glass latency under 50 ms.
   - The receiver plays the stream on its native `<video>` tag, fully bypassing the need for WebGPU support on the TV hardware.

---

#### 3.5.3 Plugin Sink 2: WebXR Immersive Spatial Mode (Virtual Cockpit)
Corresponding to spatial XR headsets (Meta Quest, Apple Vision Pro, PCVR), Karakuri Web integrates a dedicated spatial performance mode:

```
Outputs Bay: [ Monitor ] [ Projector ] [ WebRTC ] [ WebXR ]
```

When `WebXR` is toggled:
1. **Three-Tier Spatial Topology**:
   - **Tier 1 (Personal Deck HUD)**: A floating 30-inch virtual control desk positioned 70 cm in front of the performer at a 35° incline, rendering the full egui 2D console with controller raycast pointer interaction.
   - **Tier 2 (3D World Geometry)**: Karakuri's procedural 3D vertex pipelines (`topology triangles`, `grid`, `ribbon`) are rendered directly into the 6DoF stereo eye buffers, surrounding the performer.
   - **Tier 3 (Celestial Dome)**: Fragment shaders and L5 Master Chain post-effects are mapped onto an encompassing 180°–220° geodesic sky dome behind the spatial geometry via dome-master fisheye projection.
2. **Specification & Implementation Plan**:
   - Full technical specifications, coordinate mathematics, and interaction mappings are documented in **[docs/immersive.md](immersive.md)**.

---

### 3.6 Storage & Persistence

- **Embedded Starter Library**:
  - Built-in `.kir` procedures and demonstration `.kbset` bundles are embedded into the WASM binary using `include_bytes!`, enabling immediate offline usability upon page load.
- **User Library Storage**:
  - Replaces `std::fs` with browser **IndexedDB** or **Origin Private File System (OPFS)**.
  - Saved procedures, learned MIDI maps, and session history persist across browser reloads.
- **Import / Export**:
  - Standard browser file upload (`<input type="file">` / drag-and-drop) to import external sets.
  - Download trigger (`Blob` URL) to export `.kbset` bundles for sharing or moving to desktop Karakuri.

---

## 4. Implementation Roadmap & Phases

```
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 1: WebGPU Foundation & Standalone Player (~1 week)               │
│ - wasm32-unknown-unknown build setup                                   │
│ - Abstraction of std::time::Instant with web-time                      │
│ - Conditionalize portable-pty out for wasm32 target                    │
│ - In-memory embedded starter library (include_bytes!)                  │
│ - WebGPU canvas rendering + egui console presentation                  │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 2: Web Audio & Web MIDI Integration (~3-5 days)                  │
│ - cpal Web Audio backend with user gesture activation                  │
│ - midir Web MIDI backend (navigator.requestMIDIAccess)                 │
│ - Verification of 8-band FFT reactivity and hardware MIDI control      │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 3: In-Process Agent Harness & OpenAI API Client (Completed)
- wasm-bindgen-futures async agent loop in Prompt bay
- Streaming fetch client for /v1/chat/completions with SSE token decoding
- In-terminal /config parser & localStorage persistence
- /model local LLM auto-detection probing localhost ports (Ollama, LM Studio, vLLM)
- Interactive TUI menu selection with arrow keys and direct number selection
- Direct in-process tool dispatcher (14-tool MCP matrix bridge) and WebMCP standard bridge
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 4: Storage Persistence & Virtual FileSystem (Completed)
- Pluggable FileSystem abstraction with zero-latency synchronous fs::* API (ADR-0386)
- IndexedDB non-blocking background write-through and startup hydration
- Full persistence across browser sessions for sets, procedures, and MIDI/key mappings
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 5: Advanced Web Outputs: Projector, WebRTC & WebXR (In Progress)
- Multi-window WebGPU Projector popout with shared GPU device & zero-copy surface (Completed)
- Transient watchdog grace & rolling median reset on window transitions (Completed)
- WebXR Spatial World & Side-by-Side Stereo Canvas Projection (Completed)
- Wireless Presentation Cast with WebRTC P2P direct streaming (Completed)
- Document Picture-in-Picture [ Float ] Preview Monitor (Upcoming)
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 6: UX Polish, Preset Bundling & Standalone Hardening (Upcoming)
- Bundle import/export UI (.kbset file upload & Blob download)
- Offline PWA manifest & service worker caching
- Headset / touch & pointer interaction audit
- Long-session memory and WebGPU resource profiling
└────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Architectural Decision Summary

| Component | Desktop Implementation | Web / WASM Implementation | Rationale |
| :--- | :--- | :--- | :--- |
| **Agent Terminal** | PTY child processes (`portable-pty`) | In-process agent loop (`wasm-bindgen-futures`) | Browser sandbox cannot spawn OS processes |
| **LLM Connection** | Local CLI binaries (`agy`, `claude`, etc.) | Direct HTTP streaming to OpenAI-compatible API | Eliminates external process dependency |
| **Tool Execution** | Loopback TCP socket (`karakuri-mcp`) | In-memory direct Rust function dispatch | No TCP sockets in browser; zero latency |
| **Config UI** | OS environment variables / CLI flags | In-terminal `/config` commands (`localStorage`) | Clean, zero UI overhead, persistent |
| **Shaders** | Naga WGSL -> Metal/Vulkan/DX12 | Native WGSL -> WebGPU | WebGPU speaks WGSL natively |
| **UI Framework** | egui + winit + egui-wgpu (Native) | egui + winit + egui-wgpu (WASM Canvas) | Native wasm32-unknown-unknown support |
| **Audio Input** | cpal (CoreAudio / WASAPI / ALSA) | cpal (Web Audio API) | Native browser microphone & audio support |
| **MIDI Input** | midir (CoreMIDI / WinMM / ALSA) | midir (Web MIDI API) | Native browser USB MIDI device support |
| **File Storage** | `std::fs` under `~/.karakuri/store` | `karakuri_store::fs` + IndexedDB write-through | Sandboxed persistent VFS (ADR-0386) |
| **Projector Output**| `winit` secondary fullscreen window  | Same-origin popout (`projector.html`) with direct WebGPU surface sharing & Watchdog grace | Zero-copy multi-window GPU rendering, zero IPC latency |
| **Plugin Sinks**   | Syphon (macOS) / Spout (Windows) / NDI | WebXR (VR/MR) & Wireless Cast (Presentation API + WebRTC P2P) | Browser-native spatial 6DoF and zero-backend wireless projection |
