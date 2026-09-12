# `karakuri`

Main desktop application entry point, window management, and bridge subsystem for Karakuri.

---

## 1. Role in Architecture

`karakuri` sits at **Layer 5 (Applications & Presentation)** as the primary desktop application:

- **Desktop GUI Application**: Orchestrates window creation (`winit`), GPU device initialization (`wgpu 30`), and immediate-mode UI rendering (`egui` via `karakuri-console`).
- **Bridge Architecture**: Coordinates data and control flow between the GPU runtime engine (`karakuri-engine`), content storage (`karakuri-store`), environment services (`karakuri-environment`), and operator UI.
- **Embedded MCP Server**: Hosts the background Model Context Protocol service (`karakuri-mcp`) enabling real-time inspection and manipulation by AI coding agents.

---

## 2. Key Subsystems

1. **Window Lifecycle & Application Loop (`app.rs`, `main.rs`)**:
   - Implements `winit::application::ApplicationHandler` to manage event processing, redraw scheduling, and DPI scaling.
   - Manages graceful shutdown and ensures pending CAS writes are drained to disk.
2. **Runtime Engine Bridge (`bridge/engine.rs`)**:
   - Updates Deck playback states, compositing arrangements, and transition animations at frame boundaries.
   - Forwards GPU Governor load estimates to console status readouts.
3. **Presentation Sinks (`bridge/sinks.rs`)**:
   - Allocates and samples WGPU textures into egui texture IDs.
   - Renders live Program and Staging preview cells with aspect ratio letterboxing and scale matching.
4. **Operation & Input Dispatch (`bridge/handlers.rs`, `keymap.rs`)**:
   - Translates raw mouse and keyboard events into strongly typed `Operation` values.
   - Evaluates console safety gates, routing permitted operations to the runtime engine.
5. **Filesystem & Preset Management (`bridge/filesystem.rs`)**:
   - Resolves library directories, user-saved Sets, and pristine presets.
   - Ingests drag-and-dropped directories into isolated scratch workdirs for live editing.

---

## 3. Testing & Verification

Run the test suite:

```sh
cargo test -p karakuri
```
