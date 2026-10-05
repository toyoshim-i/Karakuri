---
id: 0381
title: wasm32 Set compilation offloading and build debouncing
status: accepted
date: 2026-10-03
principles: [0091, 0094]
tags: [wasm, engine, shader-compilation, async, debounce, web]
---

# wasm32 Set compilation offloading and build debouncing

## Context

On desktop platforms, pipeline compilation and Set preparation run asynchronously on worker threads ([ADR-0354](0354-the-master-chain-compiles-on-a-worker.md)), ensuring that shader compilation never occurs on the render thread ([P-0091](../principles/0091-cost-is-known-before-it-is-paid.md)).

On the `wasm32-unknown-unknown` target (WebAssembly in the browser), multithreading (`std::thread`) is not natively available without SharedArrayBuffer and specialized web worker infrastructure. Previously, `crates/karakuri-engine/src/swap/mod.rs` executed `Set::build_many` synchronously inside `install_if_ready` on the main browser thread.

When an AI agent (via MCP) or an operator made rapid, successive edits to procedures across multiple deck slots, synchronous shader compilation blocked the browser event loop for hundreds of milliseconds to several seconds. This triggered browser watchdog timeouts (such as macOS Metal's `ImpactingInteractivity` violation), causing WebGPU context loss and terminating the session.

## Decision

1. **Async compilation offloading via `wasm_bindgen_futures::spawn_local`:**
   In `crates/karakuri-engine/src/swap/mod.rs`, compile operations on `wasm32` are wrapped in an asynchronous task using `wasm_bindgen_futures::spawn_local`. The render loop yields immediately to the browser runtime, allowing frames to continue rendering while WGSL parsing and pipeline creation execute asynchronously.

2. **Debounce stale build requests:**
   Track pending build generation counters per slot. When a new compilation request arrives while a previous build is still in flight, the earlier build's completion is flagged as stale and discarded upon arrival. Only the latest requested Set configuration installs into the active deck slot.

## Consequences

- Prevents main-thread stalls, frame drops, and Metal / WebGPU watchdog crashes during intensive procedure editing in the browser.
- Rapid successive edits from LLMs or performers debounce automatically without wasting GPU pipeline resources or corrupting deck state.
- Adheres to [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) by ensuring the visual show never halts during online compilation.
