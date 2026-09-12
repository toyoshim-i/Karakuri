# `karakuri-codegen`

WGSL shader code generator for the Karakuri runtime: compiles typed `.kir` intermediate representation into optimized WebGPU Shading Language (WGSL) modules and binding metadata.

---

## 1. Role in Architecture

`karakuri-codegen` sits at **Layer 2 (Domain Extensions & Storage)** in the Karakuri workspace:

- **Input**: [`karakuri_ir::typed::Checked`](../karakuri-ir/README.md) tree, representing parsed, type-checked, contract-validated, and cost-estimated procedures.
- **Output**: Pure WGSL shader source code string alongside layout descriptors (bind groups, uniform layouts, storage buffer layouts, workgroup dimensions).
- **Consumer**: [`karakuri-engine`](../karakuri-engine/README.md), which submits the generated WGSL to `wgpu` (`create_shader_module`) and creates pipelines and bind groups.

---

## 2. Pipeline Lowering Targets

Lowering is separated by procedure `Kind` (Layers L1–L5):

| Module | Kind | Pipeline Type | Role & Memory Model |
|:---|:---:|:---:|:---|
| [`l1`](src/l1.rs) | `Generator` | Compute | `spawn` and `element` execution; ping-pong double buffering for particle state without CPU readbacks. |
| [`l2`](src/l2.rs) | `Modulator` | Compute | Geometry modulation; operates over element buffers in storage memory. |
| [`l3`](src/l3.rs) | `Camera` | Compute | 1-invocation compute pass updating camera matrices and view projections on the GPU. |
| [`l4`](src/l4.rs) | `Renderer` | Render | Vertex and fragment raster pipelines for point sprites, screen-space thick lines, and meshes. |
| [`l5`](src/l5.rs) | `Compositor` | Render | Fullscreen raster quad passes; multi-texture sampling and feedback ping-pong buffers. |
| [`field`](src/field.rs) | `Field` | Spliced | Scalar and vector spatial math functions spliced directly into calling shaders. |
| [`fusion`](src/fusion.rs) | *Multi* | Compute/Render | Combines consecutive compatible operations to minimize render passes and memory round-trips. |

---

## 3. Key Invariants

1. **Validation Precondition**:
   - `karakuri-codegen` assumes the input AST is already well-typed and contract-validated by `karakuri-ir`.
   - It never re-typechecks or enforces syntax rules; lowering is a pure translation pass.
2. **Deterministic Output**:
   - Given the same `Checked` AST and bound field resolutions, the generated WGSL text is bit-for-bit identical across runs.
   - Declarations and uniform struct members strictly follow declaration order to avoid memory layout skew.
3. **No Allocation on Render Thread**:
   - Codegen operates asynchronously during procedure compilation and hot-swapping in background workers.
4. **Memory Layout Alignment**:
   - [`layout`](src/layout.rs) guarantees strict conformance to WebGPU uniform and storage buffer alignment rules (16-byte vector alignments, array stride requirements).

---

## 4. Testing & Verification

Run all unit and integration tests:

```sh
cargo test -p karakuri-codegen
```
