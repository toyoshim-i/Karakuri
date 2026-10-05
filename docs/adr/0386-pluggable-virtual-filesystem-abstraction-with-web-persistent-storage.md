---
id: 0386
title: Pluggable Virtual FileSystem abstraction with Web persistent storage
status: accepted
date: 2026-10-05
principles: [0084, 0090, 0092, 0094]
tags: [vfs, filesystem, store, web, wasm, indexeddb]
---

# Pluggable Virtual FileSystem abstraction with Web persistent storage

## Context

Karakuri relies on filesystem operations for persisting state, saving sessions, sets, and procedures, managing scratch files, reading/writing MIDI and keyboard mapping files, and discovering external binaries. Previously, this was split between `std::fs` and an incomplete `karakuri_store::vfs` abstraction.

In WebAssembly (`wasm32-unknown-unknown`), there is no native synchronous filesystem. Past workarounds relied on static bundled presets and an ephemeral in-memory store. However, ad-hoc patching for individual use cases (such as supporting only store or set saving) introduces platform divergence and breaks whenever new file-backed subsystems are introduced (such as user-customized MIDI maps, keymaps, bridge transfers, or external plugins).

Furthermore, Karakuri's core engine relies on synchronous, infallible/`io::Result` signatures for disk I/O (`fs::read`, `fs::write`, `fs::create_dir_all`). Web persistent storage engines (IndexedDB) are inherently asynchronous and event-driven. Converting synchronous core calls across all crates into `async` would introduce massive ecosystem churn and destroy audio/real-time frame performance.

A comprehensive solution must satisfy two properties:
1. **Universal abstraction**: All filesystem demands across the entire codebase—not just stores or sets—must route through a unified, backend-agnostic abstraction.
2. **Zero-overhead synchronous API with Web persistence**: The synchronous `fs::*` interface must be preserved on all platforms, while mutations on the Web are seamlessly persisted across browser sessions via IndexedDB.

## Decision

1. **Pluggable `FileSystem` Trait & Dynamic Dispatch (`karakuri-store::vfs`)**:
   - Define a comprehensive `FileSystem` trait modeling standard filesystem operations: `read`, `write`, `create_dir`, `create_dir_all`, `remove_file`, `remove_dir_all`, `copy`, `rename`, `metadata`, `canonicalize`, `read_dir`, and `open_file`.
   - Provide platform-neutral VFS types: `Metadata`, `FileType`, `DirEntry`, `ReadDir`, `File`, `OpenOptions`.
   - Manage the active backend through a globally accessible `CURRENT_FS` (`RwLock<Arc<dyn FileSystem>>`) supporting dynamic replacement (`set_filesystem`) and scoped overrides (`with_filesystem`).
   - Expose drop-in `std::fs`-compatible functions in `karakuri_store::fs::*`.

2. **Native and In-Memory Backends**:
   - On native targets (`not(target_arch = "wasm32")`), `CURRENT_FS` defaults to `NativeFs`, delegating directly to `std::fs`.
   - On WebAssembly targets (`wasm32`), `CURRENT_FS` defaults to `MemoryFs`. `MemoryFs` manages a thread-safe in-memory virtual tree (`Arc<RwLock<BTreeMap<PathBuf, Vec<u8>>>>`) supporting hierarchical directory traversal, file metadata queries, and fallback to compiled presets (`BUNDLED_PRESETS`).

3. **Non-blocking Write-Through & Hydration via `StorageProvider` (`karakuri-store::vfs::web`)**:
   - Introduce the `StorageProvider` trait (`persist_write`, `persist_remove`, `persist_rename`) plugged into `MemoryFs`.
   - Implement `IndexedDbStorageProvider` targeting browser IndexedDB (`karakuri_vfs` database, `files` object store). Mutations (`fs::write`, `fs::remove_file`) update `MemoryFs` synchronously for immediate consistency and trigger non-blocking write-through promises into IndexedDB.
   - At startup (`WebApp::start` / `hydrate_web_fs`), read all persisted records from IndexedDB in a single batch query and hydrate `MemoryFs` before launching the application handler.

4. **Workspace-Wide Direct `std::fs` Elimination**:
   - Replace all remaining direct `std::fs` calls across `karakuri-environment` (MIDI maps, binary discovery) and `karakuri` (bridge transfers) with `karakuri_store::fs`.

## Consequences

- All subsystems (Store, Sets, Presets, MIDI mappings, Keymaps, Scratch, Bridge transfers) now share a single, unified filesystem abstraction that works identically on Desktop and Web.
- WebAssembly instances gain durable persistence across page reloads without compromising synchronous execution or audio/frame timing.
- Testing and tooling can easily inject custom virtual filesystems (e.g. mock or in-memory fixtures) via `with_filesystem` without touching physical disk.
