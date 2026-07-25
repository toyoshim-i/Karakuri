# Karakuri

A GPU-native, AI-native real-time visual generation system for VJ work.

Procedural generation centred on primitive drawing, executed on the GPU, where the
procedures themselves are generated from prompts, saved, and recalled. The design goal is
that manual human control and autonomous AI control coexist on the same mechanism.

---

## Stack

- Rust + wgpu (WGSL)
- No UI for now. V1 is a CLI plus a window
- Audio and external sync are out of V1 scope

---

## Vocabulary

| Term | Meaning |
|---|---|
| Procedure | Code that runs every frame on the GPU, written in the IR |
| Artifact | A saved procedure. Content-addressed and immutable |
| Slot | A layer position within a Set (L1/L2/L3/L4) |
| Set | Filled slots forming one video source. The unit of compilation and of lifecycle |
| VideoSource | The interface L5 consumes. Set is one implementation of it |
| Signal bus | Input distributed to every layer. Always complete; values carry a confidence |
| Local oscillator | The single source of truth for phase and tempo. External input is only correction |
| Record stream | The only path that mutates engine state. ndjson |
| Set file | A Set's state projection. No time in it |
| Session stream | The timeline. A Set file followed by `tick` records and the edits between them |

---

## Invariants

These are the skeleton of the design. Breaking them means rebuilding later. When in doubt
during implementation, these win.

### Render thread

- **Never allocate on the render thread**
- **Never compile shaders on the render thread**
- Pipelines are double-buffered; swaps happen only on frame boundaries
- If a new pipeline exceeds the frame budget, roll back automatically

### State mutation

- **Every structural change goes through forking a Set.** Never mutate a live Set in place
- Parameter value changes may apply in place (uniform writes)
- **The record stream is the only path that mutates engine state.** CLI, GUI, and agents
  all write the same records. The engine cannot distinguish a human from an agent

### Signals

- **Consumers never branch on whether a provider exists.** They branch only on confidence
- The signal bus is always complete. A signal with no provider returns a synthesized value
- Rendering reads only the local oscillator, never an external clock directly

### Determinism

- **All randomness comes from an explicit seed stream**
- No implicit randomness, no time-derived seeds, no thread-ID-derived randomness
- `dt` is a fixed simulation step, not the real frame delta
- **Frame advance comes from a `tick` record, never from a measurement.** Live, the engine
  derives it from real time and emits it; on replay it reads it back and measures nothing
- **Element order is preserved.** Compaction is order-preserving, so the live set stays a
  stable subsequence and the additive blend order never drifts
- The same record stream plus the same seeds reproduces the same output, bit for bit

### IR

- IR blocks cannot read the signal bus. External values arrive only through `param`
- Loop bounds are compile-time constants. There is no `while` and no recursion
- **IR cannot observe a buffer slot index.** Element identity is `seed`, a monotone spawn
  ordinal that travels with the element through compaction
- `capacity` is a Set-level dial, not part of a procedure's identity. Artifacts record cost
  per element, not total cost
- See `docs/ir-spec.md` for the full specification

### Color

- The pipeline is linear and HDR end to end. Internal targets are `Rgba16Float`
- sRGB encoding happens once, at final output
- Values above 1.0 are expected; they are what feeds bloom

---

## V1 scope

There is exactly one assumption to prove:

> Can an LLM generate constrained IR, can we validate it and compile it to WGSL, and can
> we hot-swap it without dropping a frame?

### In

- One Set. Slots hardcoded, no UI
- Local oscillator plus synthesized signals only (no external input)
- IR covering L1 (point/particle generation) and L4 (point rendering)
- Stateful elements: dynamic live count, indirect dispatch, spawn and kill,
  double-buffered attributes, order-preserving prefix-sum compaction
- Attribute derivation for `velocity` and `age`
- Accumulator-quantized spawning with engine-applied birth fractions
- Session stream with `tick` records, always `steps: 1`
- Set-level `capacity` override within the artifact's declared range
- IR → WGSL code generation
- Background compilation and frame-boundary swap
- Linear HDR pipeline with bloom
- Prompt-driven L1 procedure generation
- Artifact save and recall
- The `VideoSource` interface defined (Set is the only implementation)

### Out

Node editor / multiple Sets / agents / audio input / Ableton Link / L2 and L3 as
independent slots / phrase scheduling / L5 mixing / porting existing shaders /
AOV implementation (types only)

---

## Working style

- **Vertical slices, not layers.** Not "build the whole signal bus" but "get a triangle on
  screen, then never break it"
- Keep it running. Do not commit a state that does not build
- Before adding an abstraction, confirm it has at least two call sites
- Any change touching performance comes with a GPU-timestamp measurement

---

## Layout

```
crates/
  karakuri-ir/        IR parser, type checker, cost estimation
  karakuri-codegen/   IR → WGSL
  karakuri-engine/    render graph, Set lifecycle, pipeline management
  karakuri-signal/    local oscillator, synthesized signals, signal bus
  karakuri-store/     content-addressed artifact store, ndjson I/O
  karakuri-cli/       V1 entry point
docs/
  ir-spec.md          IR specification — settle this before implementing
library/              artifact store (gitignored)
```
