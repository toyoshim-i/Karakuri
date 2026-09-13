# Karakuri

A GPU-native, AI-native real-time visual performance system for VJ work.

Karakuri combines procedural visual generation on the GPU with AI-assisted shader authoring and live operator control. The system enables manual human performance and autonomous AI agents to operate simultaneously through a shared control architecture.

## Core Architectural Pillars

- **GPU-Native Pipeline**: Shaders and visual procedures are written in the `.kir` procedural DSL ([docs/ir-spec.md](docs/ir-spec.md)). The pipeline parses, type-checks, estimates GPU costs, compiles to WGSL, and submits GPU pipelines dynamically with zero hand-written WGSL shaders.
- **AI-Native Interface**: Native Model Context Protocol (MCP) server support on loopback (`--mcp`) allows LLM agents to inspect active slots, edit procedural code, rewire graph nodes, and verify compilation budgets in real time.
- **Deterministic Real-Time Execution**: Procedures compile asynchronously on worker threads and swap atomically at frame boundaries. Unsafe or over-budget procedures automatically roll back without stalling the render thread.
- **Live Performance Architecture**: Multi-deck arrangement supporting up to four independent Sets per deck with individual gain, opacity, blend modes, audio reactive modulation, and external clock synchronization.

---

## Quickstart

### Prerequisites
- Stable Rust toolchain
- Vulkan, Metal, or DirectX 12 compatible GPU

### Running the System

```sh
# Launch the desktop VJ console
cargo run -p karakuri

# Launch the console with specific geometry and renderer procedures
cargo run -p karakuri -- geometry.kir renderer.kir

# Specify explicit store and presets directories
cargo run -p karakuri -- --presets /path/to/presets --store /path/to/store

# Launch the headless CLI runner
cargo run -p karakuri-cli

# Run headless with live file watching for shader hot-reloading
cargo run -p karakuri-cli -- --watch

# Render offline frames to PNG
cargo run -p karakuri-cli -- --render out.png --frames 240
```

### Key Components

- **Console Application (`karakuri`)**: Complete visual interface containing Program output, Deck previews, Transport, Mixer, Library browser, and Node Inspector.
- **Headless CLI (`karakuri-cli`)**: Lightweight runner for headless rendering, offline PNG export, automated benchmarks, and replay verification.
- **Audio & Clock Reactivity**: Real-time FFT analysis (spectrum, energy, onset) feeding the internal signal bus, plus Ableton Link synchronization via `--tempo-source`.
- **Session & Arrangement Persistence**: Live arrangements save to `arrangements/<name>.arrangement.json`, working copies are managed in `<store>/scratch/`, and session event streams record to ndjson via the Transport `rec` control.

For complete operator controls, keybindings, and CLI flags, refer to [`docs/manual.md`](docs/manual.md) and the published manual at [https://toyoshim-i.github.io/Karakuri/manual/](https://toyoshim-i.github.io/Karakuri/manual/).

---

## Technology Stack & Architecture

- **Graphics Core**: Rust, `wgpu` (WGSL compute and render pipelines), `winit`.
- **UI Framework**: `egui` with custom pixel-aligned widgets and arithmetic layout tree (`karakuri-layout`).
- **Unified Operation Bus**: [`karakuri-operation`](crates/karakuri-operation) defines the single unified command vocabulary shared across GUI pointer gestures, keyboard shortcuts, MIDI controller mappings, and MCP agent calls.
- **Storage & Artifacts**: Content-addressed artifact store (`karakuri-store`) for compiled IR blobs, packaged Set bundles (`.kbset`), and append-only event journals.

---

## Documentation Index

- **[docs/contributing.md](docs/contributing.md)**: **Mandatory reading for all contributors and AI agents**. Engineering standards, design-first priority, verification checklists, and ADR guidelines.
- **[docs/architecture.md](docs/architecture.md)** & **[docs/architecture/](docs/architecture/)**: Comprehensive 3-tier architecture guide covering system topology, subsystem internals, and crate boundaries.
- **[docs/roadmap.md](docs/roadmap.md)**: Active milestones, delivery status, and technical backlog.
- **[docs/ir-spec.md](docs/ir-spec.md)**: Formal grammar and specification for the `.kir` procedural shading language.
- **[docs/principles/](docs/principles/)**: Active, non-negotiable architectural invariants.
- **[docs/adr/](docs/adr/)**: Historical records of architectural decisions and trade-offs.
