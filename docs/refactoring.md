# Code Health & Line Limits (ADR-0345)

This document records the workspace file length limits, modularization rules, and continuous quality gates for **Karakuri**.

---

## 1. Single-File Ceiling (<1,000 Lines)

All Rust source and test files across the workspace must strictly remain under **1,000 lines** ([ADR-0345](adr/0345-a-file-that-crosses-1000-lines-gets-a-nudge-not-a-gate.md)).

- **Hard Pre-commit Gate (`hard_limit=1000`)**: Any commit introducing or modifying a file that exceeds 1,000 lines is automatically rejected by `.githooks/pre-commit`.
- **Warning Threshold (`threshold=800`)**: When a file crosses 800 lines for the first time, git pre-commit emits a warning nudge to encourage proactive decomposition before reaching the ceiling.
- **Cognitive Sweet Spot**: Files are intended to remain in the **500–800 line** range to optimize readability, cognitive isolation, and LLM-assisted pair programming.

---

## 2. Modularization Guidelines

When a module grows toward 800 lines:
- Extract cohesive subdomains into dedicated sibling files or subdirectories (e.g., `status.rs`, `chooser.rs`, `pill.rs`, `surface.rs`, `tests.rs`).
- Avoid accumulating monolithic "God modules".
- Group tests into separate `tests.rs` or `tests/` submodules when test suites exceed a few hundred lines.

---

## 3. Continuous Quality Gates

All commits must pass the following checks without warnings:

```sh
cargo fmt --check
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo clippy --workspace --all-targets -- -D warnings
DEVELOPER_DIR=/Library/Developer/CommandLineTools cargo test
```
