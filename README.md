# Karakuri

A GPU-native, AI-native real-time visual generation system for VJ work.

Procedural generation centred on primitive drawing, executed on the GPU, where the
procedures themselves are generated from prompts, saved, and recalled. The design goal is
that manual human control and autonomous AI control coexist on the same mechanism.

This file is the current state: how to run it, what the rules are, and what does and does
not work today. Where it is going is [docs/roadmap.md](docs/roadmap.md); what the IR is is
[docs/ir-spec.md](docs/ir-spec.md).

---

## Running it

Rust stable and a GPU. Two `.kir` files go in — one L1, one L4 — and the CLI puts them
through parsing, type and contract checking, cost estimation, WGSL generation, and pipeline
creation. There is no hand-written shader anywhere in that path.

```sh
cargo run -p karakuri-cli                                    # a window
cargo run -p karakuri-cli -- --render out.png --frames 240   # one frame
cargo run -p karakuri-cli -- --seq frames/ --frames 420      # every frame
cargo run -p karakuri-cli -- --param turbulence=2.6          # a uniform write
cargo run -p karakuri-cli -- --capacity 65536                # a Set-level dial
cargo run -p karakuri-cli -- a.kir b.kir                     # a different pair
```

Defaults are the pair in [examples/](examples/) at 262144 elements. A procedure that fails
any stage prints every diagnostic it has, against the source, and stops — there is one
severity, because the response to a rejection is to regenerate rather than to proceed with
a caveat.

```
$ cargo run -p karakuri-cli -- broken.kir examples/soft_points.kir
12:61: contract: `energy` does not resolve to a local, a param, an attribute, or an ambient value
12 |     position = sphere_point(u, hash1(seed + 1u)) * radius * energy;
                                                                 ^^^^^^
hint: the signal bus is not readable from IR — declare `param energy` and attach a `bind` record to it in the Set file
```

---

## Stack

- Rust + wgpu 26 (WGSL), winit
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

- **Every structural change produces a new Set value.** Set values are immutable and
  content-addressed; the compiled instance behind one is not, and may be updated in place
  whenever the change needs no reallocation and no recompilation. Editing in the background
  is fast for that reason, without the record stream losing track of anything
- Parameter value changes may apply in place (uniform writes) and are not structural
- The cheapest correct response to "make it slower" is usually a parameter move, then a
  range change, and only then new code
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

One Set, slots hardcoded, no UI. Local oscillator and synthesized signals only. L1 for
geometry and L4 for rendering, nothing else. What V1 deliberately leaves out — multiple
Sets, agents, audio, L2 and L3, the node editor — is scheduled in
[docs/roadmap.md](docs/roadmap.md) rather than listed here.

### Status

**Two of the three clauses hold.** `.kir` text goes through every stage and 262144 elements
come out on a GPU with no hand-written shader in the path.

An LLM can write this language from the specification alone. Three models were each given
`docs/ir-spec.md`, a one-line aesthetic prompt, and nothing else — no example files, no
source, no tests. **Two of the three compiled on the first attempt with no diagnostics at
all**; the third failed on one type error and passed on the second attempt after reading
it. That is a result rather than an assumption, at n=3.

What none of them got was the *look* they were asked for. All three rendered a saturated
blob, and the same procedures rendered correctly once exposure and scale were turned down —
the code was right and the viewing conditions were guesses. The specification did not say
where the camera is, what world scale to work in, or that usable exposure falls as element
count rises. It does now, but the underlying problem is that a generator has no way to know
a value that depends on a Set-level dial. That is the tone mapper's job, not the prompt's.

The third clause is untested: pipelines are built once at startup, so "without dropping a
frame" has not been asked of anything.

| | |
|---|---|
| IR: parse, type and contract check, cost estimation | Works. Diagnostics carry a span, a hint, and every error at once |
| WGSL generation | Works, validated through naga. Generated names cannot be captured by anything a `.kir` can spell |
| Set, buffers, pipelines, compute and render | Works. Double buffering, Set-level `capacity`, parameters as uniform writes |
| Linear HDR end to end, sRGB once at output | Works |
| Store, oscillator, synthesized bus, noise | Work standalone |
| **Compaction** | Correct and tested against a CPU reference, **not wired into the L1 dispatch**. Until it is, `spawn` and `kill()` cannot run: there is no contiguous free range for a new element to land in |
| **Indirect dispatch** | `element` dispatches over a host-side live count. The indirect args buffer exists and is what compaction writes |
| **Attribute derivation** | The check pass resolves and records it; the generator does not emit it. A `consumes` satisfied only by derivation checks clean and is then missing at runtime |
| **The store, in use** | The CLI does not use it. Artifacts are loose files |
| **Signal binding** | Nothing binds a signal to a parameter, so `bind` records do nothing |
| **Tone mapping and bloom** | Neither exists. The pipeline clips at output, which is why `examples/soft_points.kir` carries a low exposure — a workaround standing in for a tone mapper |
| **Prompt-driven generation, hot swap** | Not started |

Two things known to be fragile rather than merely absent:

- **Storage buffer count.** One buffer pair per attribute means the L1 compute stage binds
  12 storage buffers for a procedure emitting three attributes. This machine allows 31, the
  WebGPU default is 8, and the downlevel default is 4. It runs here because the engine
  requests the adapter's limits; it would not run on a conservative one. Packing attributes
  into one struct per direction is the fix, and it also means compaction moves one struct
  instead of touching N buffers.
- **GPU timestamps.** See Working style below. Every performance number in this repository
  came from a host clock.

The hand-written `Points` pipeline is still present. It was the vertical slice that had to
keep working while everything else was built, and it can go once the generated path covers
what it covers.

---

## Working style

- **Vertical slices, not layers.** Not "build the whole signal bus" but "get a triangle on
  screen, then never break it"
- Keep it running. Do not commit a state that does not build
- Before adding an abstraction, confirm it has at least two call sites
- Any change touching performance comes with a GPU-timestamp measurement — **which does not
  currently work on the development machine.** On Apple M4 Pro via Metal, wgpu advertises
  and enables both `TIMESTAMP_QUERY` and `TIMESTAMP_QUERY_INSIDE_ENCODERS`, and a
  deliberately enormous workload still resolves to zero, to a negative delta, or
  occasionally to something plausible. Flaky is worse than broken: a probe returning 0.0 ms
  reads as a very fast shader. `Probe` therefore calibrates against a known-heavy workload
  rather than trusting the feature flag, and falls back to a host measurement that says so
  in the result. Treat every performance number produced here as host-side and biased high
  until this rule can be honoured on real hardware

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
  ir-spec.md          the IR. Settled; open questions are empty
  roadmap.md          where this goes after V1
examples/             a runnable .kir pair
library/              artifact store (gitignored)
```

---

## Documents

| | |
|---|---|
| This file | What exists, how to run it, and the rules that hold now |
| [docs/ir-spec.md](docs/ir-spec.md) | The IR: grammar, semantics, lowering, record formats. Its Resolved section records the decisions and why, including the ones implementation forced |
| [docs/roadmap.md](docs/roadmap.md) | Milestones after V1, and the **Demands on earlier work** each one places on code written now. That is the part worth reading before making a decision here |
