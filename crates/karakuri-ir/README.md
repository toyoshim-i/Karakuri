# `karakuri-ir`

Intermediate representation (`.kir`), parser, type-checker, and static cost estimator for the Karakuri real-time visual system.

---

## 1. Role in Architecture

`karakuri-ir` sits at **Layer 1 (Pure Foundations)** in the Karakuri workspace:

- **Source of Truth for Shaders**: Provides the AST, type system, and semantic validator for Karakuri's custom domain-specific shading language (`.kir`).
- **Zero Heavy Dependencies**: Pure leaf crate depending only on standard serialization (`serde`) and error ergonomics (`thiserror`). No GPU or runtime bindings.
- **Upstream of Code Generation**: Consumed by [`karakuri-codegen`](../karakuri-codegen/README.md) to generate optimized WGSL compute and render pipelines.
- **Static Safety Gate**: Enforces execution bounds, static loop limits, and budget ceilings prior to compilation, preventing GPU hangs or frame drops.

---

## 2. Pipeline Layers Supported

The IR type-checks procedures across five distinct visual stages:

1. **`L1` (Generator / Simulation)**: Particle emission, initialization, and physics updates.
2. **`L2` (Deformation)**: Spatial vector transformations, noise warping, and element modifiers.
3. **`L3` (Camera / Raymarching)**: Ray origins, directions, and volume sampling.
4. **`L4` (Renderer)**: Vertex and fragment rasterization (points, lines, meshes).
5. **`Field`**: Continuous mathematical scalar/vector evaluations.
6. **`L5` (Master Post-Processing)**: Ordered post-processing passes (feedback, bloom, chromatic aberration) over the composited frame buffer.

---

## 3. Key Invariants

1. **Guaranteed Termination**:
   - Loops must be bounded by compile-time constants. Arbitrary `while` loops are rejected.
2. **Deterministic Typing**:
   - Explicit swizzles, vector sizing, and parameter ranges (`[min, max]`) are strictly validated.
3. **Static Cost Floor & Ceiling**:
   - Static instruction weights and loop bounds determine cost estimates (`cost.rs`). Shaders exceeding complexity limits are rejected at check time.

---

## 4. Testing & Verification

Run the test suite:

```sh
cargo test -p karakuri-ir
```
