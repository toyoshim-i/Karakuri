# Karakuri Web / WebAssembly Architecture & Implementation Plan

> **Note on Naming:** This plan is maintained as `docs/warm.md` (and symlinked from `docs/wasm.md`), capturing both the WebAssembly port specification and the "warm" (active-development backlog) design for running Karakuri entirely inside modern web browsers.

---

## 1. Overview & Vision

The objective of the Web version of Karakuri is to deliver a **100% client-side, zero-backend, serverless static web application** running directly in any modern browser supporting WebGPU (Chrome, Edge, etc.).

Unlike native desktop Karakuri, which relies on host operating system facilities (POSIX/Windows PTYs, TCP sockets, and shared-memory GPU output handles like Syphon and Spout), the Web port reimagines these subsystems using browser-native standards:

1. **WebGPU-Native Procedural Shader Pipeline**: Karakuri IR (`.kir`) compiles to WGSL via `karakuri-codegen`, matching WebGPU's native shader language without any transpile step.
2. **In-Process Agent Harness & Direct Tool Execution**: Eliminates the PTY process manager and TCP-based MCP server in favor of an internal, in-memory agent runner connecting directly to OpenAI-compatible LLM endpoints.
3. **Web Audio & Web MIDI Buses**: Retains real-time audio FFT spectral analysis and hardware controller mappings via Web Audio API and Web MIDI API.
4. **Decoupled Multi-Window Projector**: Replaces OS texture sharing with multi-window Canvas synchronization (`BroadcastChannel`) or the HTML5 Fullscreen API.

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
│   └───────────────┬────────────────┘   └─────────────┬─────────────┘   │
│                   │                                  │                 │
│      [Render]     │ [Sync State]              [Fetch Stream]           │
│                   ▼                                  ▼                 │
│   ┌───────────────────────────────┐    ┌───────────────────────────┐   │
│   │ Projector Popup Window        │    │ OpenAI-Compatible API     │   │
│   │ (window.open / BroadcastChan) │    │ (Ollama, LMStudio, Cloud) │   │
│   └───────────────────────────────┘    └───────────────────────────┘   │
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

### 3.2 In-Memory Tool Dispatching (Direct MCP Replacement)

#### The Problem on Web
Desktop Karakuri runs a TCP listener on `127.0.0.1:3000` via `std::net::TcpListener`, exchanging JSON-RPC messages with external agents. Browsers cannot open raw TCP listener sockets.

#### The Solution: In-Memory Dispatch
- The 14-tool Karakuri MCP matrix (`read_slot`, `copy_slot`, `operate`, `get_permissions`, `check_set`, `build_procedure`, etc.) is declared as OpenAI-format tool definitions in the chat completion request:
  ```json
  {
    "tools": [
      {
        "type": "function",
        "function": {
          "name": "operate",
          "description": "Execute a Karakuri operation",
          "parameters": { ... }
        }
      }
    ]
  }
  ```
- When the LLM emits a `tool_calls` response chunk:
  1. The agent harness intercepts the function name and arguments.
  2. The harness calls the internal Rust dispatch function (`Readout::operate`, `SessionManager`, or `karakuri-mcp` handlers) **in-memory**.
  3. Execution is zero-latency, thread-safe, and atomic.
  4. The result JSON is appended as a `tool` role message, and the LLM stream resumes.

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

### 3.5 Projector Display: Popout Window & Fullscreen

#### The Problem on Web
Desktop Karakuri exports frames to other VJ software using OS-level shared GPU memory (`IOSurface` on macOS, `DXGI` on Windows). Browsers do not expose raw GPU handle sharing.

#### The Solution: Browser Popout Window
External output is solved by launching a clean projector display window:

1. **Popout Projector Window**:
   - Operator clicks `[ Popout Projector ]` or uses a shortcut.
   - Executes `window.open("projector.html", "projector", "popup=true,width=1920,height=1080")`.
2. **State Synchronization (`BroadcastChannel`)**:
   - The main console window broadcasts frame state, active deck transitions, and master chain parameters via a `BroadcastChannel("karakuri-frame-sync")`.
   - The projector window initializes its own lightweight WebGPU pipeline, rendering the exact output scene in real time at full projector resolution.
3. **Single-Window Fullscreen Alternative**:
   - The program preview canvas can be enlarged to native fullscreen using the browser Fullscreen API (`canvas.requestFullscreen()`).

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
│ Phase 3: In-Process Agent Harness & OpenAI API Client (~1 week)        │
│ - wasm-bindgen-futures async agent loop in Prompt bay                  │
│ - Streaming fetch client for /v1/chat/completions                      │
│ - In-terminal /config parser & localStorage persistence                │
│ - Direct in-memory tool dispatcher (14-tool MCP matrix bridge)         │
├────────────────────────────────────┬───────────────────────────────────┤
│                                    ▼                                   │
│ Phase 4: Multi-Window Projector & Storage Persistence (~3-5 days)      │
│ - BroadcastChannel synchronization for projector.html popup           │
│ - IndexedDB / OPFS store backend for saved procedures & sets           │
│ - File export / import drag-and-drop support                           │
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
| **File Storage** | `std::fs` under `~/.karakuri/store` | IndexedDB / OPFS + memory bundles | Sandboxed web persistence |
| **Video Output** | Syphon (macOS) / Spout (Windows) | Popout Window (`BroadcastChannel`) / Fullscreen | Eliminates OS shared-memory requirement |
