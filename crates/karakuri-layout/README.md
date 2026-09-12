# karakuri-layout

Pure geometric layout tree, constraint solver, and panel arrangement specification for the Karakuri console.

---

## 1. Overview

`karakuri-layout` is a foundational leaf crate (Layer 1) responsible for pane geometry, split hierarchies, divider manipulation, and folding state. It contains no dependencies on UI toolkits (`egui`), windowing libraries (`winit`), or GPU devices.

It provides the mathematical model behind Karakuri's multi-bay arrangement system (Program, Staging, Master, Sequencer, Library, Inspector).

---

## 2. Key Design Invariants

1. **Solver Never Mutates the Model**:
   - Solving constraints for a given window size calculates placement into ephemeral derived buffers without mutating the stored arrangement tree.
   - Resizing a window smaller and larger again preserves the original proportional arrangement bit-for-bit.

2. **Decoupled Operator Folds vs. Ephemeral Dismissal**:
   - Operator folds ([`Layout::collapse`]) are persistent arrangement state, preserved upon save and load.
   - Dismissals ([`Layout::set_aside`]) are ephemeral, runtime-only states (e.g. hiding a panel during window constriction) and are never saved to disk.

3. **Strict Validation Invariants**:
   - Layout files must form a valid tree: exactly one root, no cycles, valid child indices, and unique node names.
   - Loading malformed arrangements produces explicit, typed [`LoadError`]s before execution.

---

## 3. Module Structure & Core Types

| Module | Description | Key Types / Functions |
|:---|:---|:---|
| [`lib.rs`](src/lib.rs) | Geometry primitives, axes, and common layout types | [`Rect`], [`Size`], [`Axis`], [`Edge`], [`Gap`], [`Claim`] |
| [`spec.rs`](src/spec.rs) | Authoring specification and builder AST for declarative layouts | [`Spec`], [`SplitSpec`], [`ViewSpec`] |
| [`layout.rs`](src/layout.rs) | Core arena, constraint solver, hit testing, and divider tracking | [`Layout`], [`Node`], [`LoadError`], `solve()`, `hit()` |

---

## 4. Verification

Run unit and integration tests:

```sh
cargo test -p karakuri-layout
```

