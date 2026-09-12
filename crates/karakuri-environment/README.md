# `karakuri-environment`

System coordination environment bridging runtime engine, content storage, external sync, and authoring workflows.

---

## 1. Role in Architecture

`karakuri-environment` sits at **Layer 4 (Environment & Services)** in the Karakuri workspace:

- **Coordination Layer**: Assembles and orchestrates lower-layer subsystems (`karakuri-engine`, `karakuri-store`, `karakuri-audio`, `karakuri-midi`, `karakuri-ir`) into a cohesive live performance environment.
- **Setfile Management**: Resolves human-authored `.kset` files, packages self-contained `.kbset` bundles, and serializes live deck configurations to content-addressed storage.
- **Hot-Reload & Watcher Pipeline**: Dispatches filesystem file modifications to background compilation threads and safely coordinates hot-swaps at frame boundaries.
- **Session & Preset Management**: Maintains pristine asset libraries, isolated scratch directories for live edits, and recorded session journals.
- **External Synchronization**: Parses and tracks external tempo sources with network jitter filtering and latency compensation.

---

## 2. Key Subsystems

1. **Setfile Packaging (`setfile/`)**:
   - Manages text-based authoring sets (`.kset`) and serialized binary bundles (`.kbset`).
   - Content-addresses shader sources and node bindings into immutable storage hashes.
   - Enforces strict path containment rules preventing directory traversal attacks.
2. **HotSwap Watcher (`watch.rs`, `compile.rs`)**:
   - Asynchronously monitors shader files and set configurations for modifications.
   - Debounces file events and sends source payloads to background worker threads.
   - Verifies AST diagnostics before generating pipeline swap commands.
3. **Transition & Mix Engine (`mix/`)**:
   - Coordinates transition states, parameter interpolation, and crossfade timing between decks.
   - Applies tonemapping and compositing rules across multiple active render slots.
4. **Session Journaling & Scratchpad (`session.rs`, `scratch.rs`)**:
   - Records frame metadata, parameter events, and audio analysis frames to append-only journals.
   - Isolates live edits into scratch paths, preserving pristine presets in the library.
5. **Tempo Synchronization (`tempo_source.rs`, `clock.rs`)**:
   - Ingests clock anchors from external sync daemons.
   - Estimates clock drift and latency offsets to align visual oscillators with audio tempo.

---

## 3. Testing & Verification

Run the test suite:

```sh
cargo test -p karakuri-environment
```
