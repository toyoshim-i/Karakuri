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

#### 3.5.3 Document Picture-in-Picture: Always-On-Top Floating Monitor
For multitasking, coding, and live control where the operator works across different browser tabs or external applications, Karakuri Web integrates the W3C **Document Picture-in-Picture API**:

1. **Always-On-Top Floating Window (`window.documentPictureInPicture`)**:
   - The operator toggles `[ Picture-in-Picture ]` (the 3rd chip in the Outputs bay).
   - In Chromium browsers (Chrome/Edge 111+), the browser opens an OS-level always-on-top window (`requestWindow({ width: 640, height: 360 })`) that stays floating above all other desktop applications.
   - An offscreen canvas (`#karakuri-pip-canvas`) inside the floating window document binds directly to a secondary `wgpu::Surface` using the parent window's shared `Gpu::instance()`.
2. **Dynamic DPR & Resizing**:
   - The PiP canvas dynamically tracks window drag-resizing and Retina/DPR changes.
   - `WindowSink` cleanly letterboxes the composed clean program video (no egui UI overlays) into the floating monitor with zero memory copying or CPU overhead.
3. **Lifecycle Synchronization**:
   - Window closure (via OS titlebar close button, navigation, or un-toggling `[ Picture-in-Picture ]`) triggers the `pagehide` event and automatically synchronizes the Outputs chip state.

---

#### 3.5.4 Plugin Sink 2: WebXR Immersive Spatial Mode (Virtual Cockpit)
Corresponding to spatial XR headsets (Meta Quest, Apple Vision Pro, PCVR), Karakuri Web integrates a dedicated spatial performance mode:

```
Outputs Bay: [ Program View ] [ Projector ] [ Picture-in-Picture ] [ WebXR ] [ Cast ]
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

### 3.7 Social Sharing, Stateless Permalinks & Media Capture (ADR-0389)

Sharing live procedural shader sets on social platforms (e.g. 𝕏 / Twitter) traditionally requires either a cloud database to store user state or manual screen recording with third-party software. Karakuri Web implements a **100% serverless, zero-backend sharing pipeline** combining stateless URL hashes, remote cloud storage fallback, in-browser offscreen video recording, and a two-step social composition workflow:

#### 3.7.1 Dual-Tier Sharing Model (Inline Deflate & Remote Storage)
1. **Tier 1 (Inline Hash `#set=...`)**:
   - The active Set (slot assignments, `.kir` shader code, parameter maps, and topology) is serialized into standard `.kbset` ndjson via `App::export_active_set_bundle`.
   - The payload is compressed in-memory using **raw Deflate** (`CompressionStream('deflate-raw')`) and encoded as **URL-safe Base64** (`#set=<base64url>`).
   - Compact sets easily fit within browser URL hash limits while remaining completely transparent to static web hosting servers.
2. **Tier 2 (Remote Storage `#url=...`)**:
   - Social platforms such as 𝕏 employ strict URL parsing thresholds (`twitter-text` / Netty limits at ~4,096 bytes; Web Intent limits at ~3,800 characters). Because complex `.kbset` bundles repeat inlined source records (`Record::Src`) for every line of WGSL shaders, compressed bundles can exceed 3,800 characters.
   - Exceeding 3,800 characters causes 𝕏 to fail t.co link shortening, dumping the literal URL into the post body and triggering massive character overflow.
   - When a bundle exceeds 3,800 characters (or when creators prefer short permalinks), the UI offers two external storage options:
     - **Cloudflare KV (Karakuri Public Storage)**: Uploads the `.kbset` bundle to a Cloudflare Worker backed by Workers KV (`https://karakuri-share.toyoshim.workers.dev/set`), protected by non-interactive Cloudflare Turnstile tokens.
     - **User GitHub Gist**: Saves the Set as a public Gist using the performer's Personal Access Token (`gist` scope), generating an immutable GitHub raw URL.
3. **Unified Startup Hydration**:
   - On page load, `karakuri-web` inspects `window.location.hash`.
   - The loader transparently resolves `#set=...`, `#url=...`, and `#load=...`, fetching and decompressing the raw bundle text and hydrating directly onto Deck A.
   - **Idempotent Hydration**: If the Set ID already exists in local storage (e.g. author testing their own shared link), `import_and_load_bundle` caches missing inlined artifacts into the store and loads the set onto Deck A without raising collision errors.

#### 3.7.2 Pure Visual Offscreen Recording & Two-Step Social UX
1. **Offscreen Canvas Recording (`#karakuri-recorder-canvas`)**:
   - Attaches a secondary offscreen `wgpu::Surface` (dynamically matched to 1080p `1920×1080` or 2K `2560×1440`) to `Gfx.recorder_sink`.
   - The engine tone-maps and renders clean program video to the recorder canvas before egui UI chrome (bays, faders, knobs) is drawn.
   - During recording, `selected_res` is overridden to match the target canvas resolution so that internal HDR composites (`Present` and `Deck`) render without upscaling blur, and tone mapping applies subtle screen-space Interleaved Gradient Noise (IGN) TPDF dithering to protect fine dark gradients against social platform re-encoding banding.
   - **Quality Toggle (1080p FHD vs 2K 1440p)**: The Share modal provides resolution pills persisted via `localStorage` (`karakuri_share_video_res`). Selecting a resolution dynamically resizes the offscreen canvas, reconfigures the recorder sink, and starts a fresh recording run with an asynchronous cancellation token (`captureGeneration`).
   - **Primary WebCodecs Pipeline**: Captures 480 frames over 8.0 seconds at an exact 60.0 fps CFR via `VideoEncoder` with hardware acceleration preference (`prefer-hardware`). Probes 2K profiles (`avc1.640033` High Profile Level 5.1 / `avc1.640032` Level 5.0, 28 Mbps) and 1080p profiles (`avc1.64002A` Level 4.2, 16 Mbps) with 1-second Closed GOP, automatically falling back to 1080p if 2K is unsupported.
   - **X-Optimized Faststart MP4 Muxer**:
     - Produces non-fragmented ISO BMFF MP4 (`ftyp` -> `moov` -> `mdat`).
     - Includes QuickTime/VideoToolbox-compatible `colr` (`nclx` Rec.709 color primaries and transfer characteristics with full range) and `pasp` (1:1 square pixel aspect ratio) visual sample entry extensions within `avc1`.
     - Synthesizes 375 distinct, sequential 7-byte silent AAC-LC audio frames (`mp4a.40.2`, 2,625 bytes total in `mdat`) with strictly monotonic increasing `stco` chunk offsets (X's ingestion engine drops audio tracks when duplicate chunk offsets point to overlapping byte ranges).
     - 8.0-second duration (480 video frames, 375 audio frames) ensures 𝕏 transcodes multi-rendition HLS ladders with the high-quality resolution gear enabled (𝕏 treats videos $\le 6$s as GIF-like loops and disables quality selection).
   - **MediaRecorder Fallback**: For platforms lacking WebCodecs, falls back to `canvas.captureStream(60)` and `MediaRecorder` with an `AudioContext` silent track and 8-second capture duration.
2. **Explicit Two-Step Social Workflow**:
   - **Step 1 (`[ ⬇ Step 1: Download Loop Video (8s) ]`)**: Downloads the generated clean 8-second video loop (`.mp4` / `.webm`).
   - **Step 2 (`[ 𝕏 Step 2: Open 𝕏 Post Composer ]`)**: Opens Twitter Web Intent pre-filled with `Generated by #KarakuriVJ\n` and the share URL.
   - The user attaches the downloaded video in the composer, achieving video fidelity and executable reproducibility without friction.
3. **Native Console HUD Integration**:
   - Both `share` and `logs` are first-class pills in the console `OutputsRow` (painted directly in WebGPU with HUD tooltips cited from `docs/manual/console.html`), replacing floating HTML DOM buttons.

---

## 4. Implementation Roadmap & Phases

```
┌────────────────────────────────────────────────────────────────────────┐
│ Phase 1: WebGPU Foundation & Standalone Player (Completed)             │
│ - wasm32-unknown-unknown build setup                                   │
│ - Abstraction of std::time::Instant with web-time                      │
│ - Conditionalize portable-pty out for wasm32 target                    │
│ - In-memory embedded starter library (include_bytes!)                  │
│ - WebGPU canvas rendering + egui console presentation                  │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 2: Web Audio & Web MIDI Integration (Completed)                  │
│ - cpal Web Audio backend with user gesture activation                  │
│ - midir Web MIDI backend (navigator.requestMIDIAccess)                 │
│ - Verification of 8-band FFT reactivity and hardware MIDI control      │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 3: In-Process Agent Harness & OpenAI API Client (Completed)      │
│ - wasm-bindgen-futures async agent loop in Prompt bay                  │
│ - Streaming fetch client for /v1/chat/completions with SSE token decoding
│ - In-terminal /config parser & localStorage persistence                │
│ - /model local LLM auto-detection probing localhost ports              │
│ - Interactive TUI menu selection with arrow keys and direct selection │
│ - Direct in-process tool dispatcher & WebMCP standard bridge           │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 4: Storage Persistence & Virtual FileSystem (Completed)          │
│ - Pluggable FileSystem abstraction with synchronous fs::* API (ADR-0386)
│ - IndexedDB non-blocking background write-through and startup hydration│
│ - Full persistence across browser sessions for sets and mappings       │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 5: Advanced Web Outputs: Projector, WebRTC & WebXR (Completed)   │
│ - Multi-window WebGPU Projector popout with shared GPU surface (ADR-0358)
│ - Transient watchdog grace & rolling median reset on window transitions│
│ - WebXR Spatial World, Stereo eye buffers & Celestial Dome (ADR-0388) │
│ - Wireless Presentation Cast with WebRTC P2P direct streaming          │
│ - Document Picture-in-Picture [ Picture-in-Picture ] floating monitor │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 6: UX Polish, Preset Bundling & Standalone Hardening (Completed) │
│ - Native console Outputs row pills (share, logs) & HUD tips            │
│ - Full GUI tooltip audit across all console outputs                    │
│ - Universal component unification and browser IO feature gaps deferred │
│   to global roadmap (docs/roadmap.md § M11)                            │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 7: Social Sharing, Permalinks & Media Capture (Completed, ADR-0389)
│ - Stateless Set permalink generation with raw Deflate & Base64URL      │
│ - Remote cloud storage fallback (Cloudflare KV + Turnstile, GitHub Gist)
│ - Startup URL hash hydration (#set=..., #url=...) with idempotent store│
│ - In-browser clean offscreen video loop recording (1280x720, 5s)       │
│ - Two-step social sharing workflow (Download loop video -> Open 𝕏)    │
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
| **PiP Monitor**    | N/A (Desktop window management)     | Document Picture-in-Picture API (`window.documentPictureInPicture`) | Always-on-top OS-level floating monitor across applications |
| **Plugin Sinks**   | Syphon (macOS) / Spout (Windows) / NDI | WebXR (VR/MR) & Wireless Cast (Presentation API + WebRTC P2P) | Browser-native spatial 6DoF and zero-backend wireless projection |
| **Social & Permalinks** | Manual file save / CLI export | Dual-Tier URL Hash (`#set=...` / `#url=...`) + Offscreen 5s loop capture | 100% serverless stateless sharing + remote storage fallback for complex shaders |
| **Console GUI**    | egui immediate-mode native widgets  | WebGPU-rendered native console pills + hover HUD cards | Unified multi-modal pointer invariance and zero-DOM chrome (ADR-0387) |

---

## 6. Milestone Completion & Roadmap Integration

With the completion of **Phase 1 through Phase 7**, all primary WebAssembly / WebGPU engine foundations, input buses, virtual filesystems, multi-window outputs, in-process agent harnesses, and stateless social sharing capabilities are fully operational.

To maintain a single authoritative operational ledger and avoid split backlogs, cross-platform component refactorings and future Web platform feature enhancements are tracked centrally in **[docs/roadmap.md](roadmap.md)**:
- **Master Bay Component Unification ([ADR-0387](adr/0387-universal-console-gui-component-architecture.md))**: Tracked under M11 item 9.
- **WebAssembly Platform Feature Gaps & Offline Hardening**: Tracked under M11 item 10, covering browser canvas file drag-and-drop (`.kbset` / `.kir`), local `.kbset` bundle export, offline PWA manifest & Service Worker (`sw.js`), touchscreen & tablet fader optimizations, and WebGPU context loss recovery.

