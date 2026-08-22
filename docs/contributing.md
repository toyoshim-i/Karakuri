# Karakuri Contributing & Development Guide

This document outlines the engineering principles, workflow guidelines, build/test commands, and verification practices for **Karakuri**. Both human contributors and AI coding agents MUST adhere to these rules when submitting changes.

---

## 1. Project Invariants & Core Rules

Every change MUST respect the standing rules in [docs/principles/](principles/) — **one file each**,
so a rule is stated once and cannot drift between documents. `ls docs/principles/` is the index,
because each filename is the rule it states.

They were previously copied here and into [architecture.md](architecture.md), and the two copies had
already stopped agreeing on which four were foundational, which is what moved them.

Start with these, and read the rest before changing anything they touch:

- [Nothing allocates or compiles a shader on the render thread](principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md)
- [Simulation time comes from a record, never from a clock](principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md)
- [Compaction preserves order](principles/0003-compaction-preserves-order.md)
- [A live Set is never mutated in place](principles/0004-a-live-set-is-never-mutated-in-place.md)
- [Nothing checks clean and comes up short at runtime](principles/0024-nothing-checks-clean-and-comes-up-short-at-runtime.md)
- [A test meant to catch something is run against the defect](principles/0025-a-test-meant-to-catch-something-is-run-against-the-defect.md)
- [Only reviewed work enters history](principles/0017-only-reviewed-work-enters-history.md)
- [Run what the question needs, when it is asked](principles/0057-run-what-the-question-needs-when-it-is-asked.md)

**Why** each is the way it is, and what was rejected on the way, is in [docs/adr/](adr/). A rule that
stops being true is deleted and re-recorded under a new number rather than edited — see
[ADR-0000](adr/0000-record-decisions-here-and-standing-rules-in-principles.md).

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

