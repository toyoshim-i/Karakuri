# `karakuri-cli`

Command-line interface, headless runner, and offline batch rendering tool for the Karakuri system.

---

## 1. Role in Architecture

`karakuri-cli` sits at **Layer 5 (Applications & Presentation)** in the Karakuri workspace:

- **Command-Line & Headless Runner**: Serves as the standalone terminal interface for live performance, automated benchmarking, and non-interactive execution.
- **Offline Batch Rendering**: Renders sets and animations offscreen directly to image sequences without opening a graphical window.
- **Deterministic Session Replay**: Reads recorded session journals from `karakuri-store` and replays frames deterministically at fixed time steps.
- **Setfile Maintenance**: Ingests, inspects, and packages `.kset` authoring files into distributable `.kbset` bundles.

---

## 2. Key Subsystems

1. **Argument Parsing & Dispatch (`args.rs`)**:
   - Parses CLI flags, geometry configurations, shader source paths, and audio/MIDI mappings.
   - Dispatches standalone commands such as `--list-sets`, `--package`, and `--take-in`.
2. **Interactive Live Runner (`live.rs`, `app.rs`)**:
   - Manages real-time render loops, hot-swapping watchers, and keyboard shortcuts.
   - Integrates audio input analysis, MIDI controllers, and MCP background threads.
3. **Compilation & Aiming (`aiming.rs`)**:
   - Tracks active and staging node slots, coordinating background compiler threads.
   - Manages HotSwap aim targets and recompilation triggers.
4. **Session Replay (`replay.rs`)**:
   - Reconstructs pipeline state from CAS session journals.
   - Replays control operations, parameter attachments, and timing tracks frame-accurately.
5. **Persistence Management (`save.rs`)**:
   - Commits running deck configurations and procedural graphs to Set files.
   - Drains pending background writes to avoid stalling the render loop.

---

## 3. Testing & Verification

Run the test suite:

```sh
cargo test -p karakuri-cli
```
