# Karakuri Contributing & Development Guide

This document outlines the engineering principles, workflow guidelines, build/test commands, and verification practices for **Karakuri**. Both human contributors and AI coding agents MUST adhere to these rules when submitting changes.

---

## 1. Project Invariants & Core Rules

Any change to the codebase MUST respect the following foundational invariants:

1. **Zero Allocations & Zero Shader Compilations on the Render Thread**
   - The frame rendering path (inside `karakuri-engine::deck` / `present`) MUST NOT allocate heap memory or trigger synchronous WGSL compilation. All buffer management and pipeline creation happen asynchronously or during initialization.
2. **Deterministic Step-Based Time (`tick` and `dt`)**
   - Time in simulation moves forward strictly via integer `tick` increments and fixed `dt` (`1.0 / 60.0` seconds). Never query system wall clocks inside the engine; external time is only supplied as oscillator drift correction.
3. **Immutability & Structural Forking**
   - Live `Set` structures are immutable when node topology or procedures change. A structural modification MUST fork a new `Set` instance. Uniform parameter updates are the only allowed in-place modification.
4. **Order-Preserving Compaction**
   - Element indices and execution order MUST be strictly preserved during dead-element compaction to ensure bit-exact reproducibility across runs.

---

## 2. Environment Setup & Development Tools

### Required Prerequisites
- **Rust**: Stable Rust (`1.84` or higher) with `cargo`.
- **GPU Driver & WebGPU Environment**: `wgpu` compatible Vulkan, Metal, or DX12 backend.

### Git Hooks
Karakuri provides pre-commit and pre-push hooks under `.githooks/`. Enable them in your local repository:

```sh
git config core.hooksPath .githooks
```

- **`pre-commit`**: Runs `cargo fmt --check` on the **staged content**, and nothing else. It
  reads what is being committed rather than the working tree, so a half-finished edit on
  disk neither blocks a good commit nor hides a bad one.
- **`pre-push`**: Runs `cargo fmt --check`, `cargo clippy` and the whole suite **on a tag
  push, and on nothing else**. A tag is the deploy; a branch push is part of working —
  backing up, moving between machines, opening something for review — and a gate there asks
  whether the work is good at a moment nobody was claiming it was.

Neither hook runs tests on an ordinary commit or branch push, and that is a decision rather
than an omission: this workspace has nearly nine hundred tests and most want a GPU, so any
fixed subset spends minutes answering a question nobody asked. What replaces it is
deliberate: whoever makes a change names the smallest suite that answers it and runs that
(§3 lists them per crate), and the whole workspace runs at a boundary — before a tag, after
a refactor, and before a change is called done (§4). The reasoning is written into the hooks
themselves.

---

## 3. Build, Lint, and Test Commands

### Running All Workspace Tests
```sh
cargo test --workspace
```

### Running Workspace Linter (Clippy)
```sh
cargo clippy --workspace --all-targets -- -D warnings
```

### Checking Code Formatting
```sh
cargo fmt --check
```

### Testing Specific Component Packages
- **IR Parser & Type Checker**: `cargo test -p karakuri-ir`
- **WGSL Codegen & Naga Validation**: `cargo test -p karakuri-codegen`
- **Render Engine & HotSwap**: `cargo test -p karakuri-engine`
- **Audio Processing & Beat Lock**: `cargo test -p karakuri-audio`
- **CLI & Replay Verification**: `cargo test -p karakuri-cli`

---

## 4. Verification Checklist for Code Changes

Before marking a task or pull request as complete, ensure the following checklist is satisfied:

- [ ] **All workspace tests pass**: `cargo test --workspace` returns 0 exit code.
- [ ] **Naga validation passes**: Any modifications to `karakuri-codegen` MUST be verified against `naga_test.rs` to guarantee generated WGSL text parses and validates cleanly.
- [ ] **Documentation integrity**: Existing comments, docstrings (`//!` and `///`), and Markdown documentation are updated accordingly.
- [ ] **No unhandled errors or silent fallbacks**: Core logic should produce explicit error types (`IrError`, `SetError`, `GpuError`, etc.) rather than swallowing exceptions.
- [ ] **Relative links in documentation**: Markdown links to repository files MUST use relative paths (e.g. `../crates/karakuri-ir` or `architecture.md`).

---

## 5. Related Architecture & Specification Reference

- [architecture.md](architecture.md): Source code structure, multi-crate map, pipeline, and threading model
- [ir-spec.md](ir-spec.md): `.kir` DSL specification and language invariants
- [manual.md](manual.md): CLI arguments and VJ keyboard controls reference
- [plugins.md](plugins.md): Out-of-process helper plugin specification
- [roadmap.md](roadmap.md): Architectural roadmap and future milestones

