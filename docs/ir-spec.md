# Karakuri IR Specification (draft v0.2)

Extension `.kir`. Plain text. Written by LLMs, validated by the compiler, lowered to WGSL.

One `.kir` file is one procedure and fills exactly one slot. A Set is a combination of
procedures, one per slot. Cross-slot combination works because of the `emit` / `consumes`
contract described below.

## Design principles

- **No surprises.** Expression syntax follows GLSL/WGSL. Use the shape LLMs already know.
- **Statically checkable.** Only what type checking and cost estimation can verify.
- **An instrument, not a picture.** What gets generated is a procedure with operable parameters.

### Non-goals

- Being a general-purpose language
- User-defined functions, structs, arrays (not in v0.2)
- Porting from shader-sharing sites (a separate Raw node type, later)

---

## File structure

```
proc <name> {
  <header declarations>

  <block> { <statements> }
}
```

One `proc` per file.

---

## Header declarations

### kind

```
kind L1        // geometry generation
kind L4        // rendering
```

v0.2 defines `L1` and `L4` only. `L2` (deformation) and `L3` (camera) come later; until
then the camera is a built-in.

### capacity / topology (L1 only)

```
topology points
capacity [65536, 1048576] = 262144
```

`capacity` is the allocated element count. It takes a mandatory range and a default — the
same shape as a `param` — because a Set overrides it:

```ndjson
{"t":"capacity","layer":"L1","value":524288}
```

**`capacity` is not part of a procedure's identity.** The same procedure at 65536 and at
524288 is one artifact with one hash and one preview, not two. It is a performance dial
turned per Set, and the range is what the artifact declares it can be turned to. A Set
asking for a value outside that range is rejected at build time; the engine keeps its own
hard ceiling on top, from the VRAM budget.

In a procedure with no `spawn` block it is also a **visual** parameter, because `seed`
equals the initial slot index there — `capacity` sets the density of a lattice as well as
its cost. Derive structure from it rather than hardcoding, and one procedure covers its
whole declared range:

```
let side = uint(sqrt(float(capacity)));
```

The ambient `capacity` reaches the shader as a uniform, not as a compile-time constant, so
changing it needs no recompilation. Reallocating the buffers still forces a fork and swap,
but a fork with no compilation in it, which primes almost immediately.

The **live** count is dynamic, lives in an indirect-dispatch args buffer, and is not
readable from IR. See [Element lifecycle](#element-lifecycle).

### param

```
param <name> : <type> [<min>, <max>] = <default>
```

- Types: `float`, `vec2`, `vec3`
- The range is mandatory. It doubles as the UI fader range, the agent's search range,
  and the normalization basis for signal binding.
- Attribute names are reserved. A `param` may not be called `position`, `size`, `tint`,
  `seed`, or any other name from the attribute table — parameters and attributes share one
  scope inside a block, and the shadowing would be silent.
- 3 to 8 parameters recommended. Zero is legal but such a procedure has no reuse value.
- Intensity-like parameters may exceed 1.0. The pipeline is HDR; see [Color](#color).

```
param radius : float [0.1, 8.0] = 2.0
param glow   : vec3  [0.0, 4.0] = vec3(0.4, 0.7, 1.0)
```

### emit / consumes

```
emit     position, velocity, age
consumes position, age
```

Compatibility is a subset check. If `consumes` is not contained in `emit`, the compiler
first attempts [attribute derivation](#attribute-derivation); if no rule applies, it is a
compile error.

Available attributes:

| Name | Type | Notes |
|---|---|---|
| `position` | `vec3` | |
| `velocity` | `vec3` | derivable |
| `normal` | `vec3` | |
| `uv` | `vec2` | |
| `seed` | `uint` | spawn ordinal. Implicit — never declared, always readable. See [Element identity](#element-identity) |
| `age` | `float` | seconds since spawn, derivable |
| `size` | `float` | |
| `tint` | `vec3` | linear RGB |

### blend (L4 only)

```
blend additive
```

`additive` is the only legal value in v0.2. The declaration exists now so that adding the
next mode is a format addition rather than a format change — the same move as defining
`VideoSource` before there is a second implementation of it.

Additive needs no sorting, which is why it is where v0.2 starts: `capacity` elements cannot
be depth-sorted per frame at this scale, even sorting indices alone. The successor is not
depth sorting but **weighted blended OIT** — order independent, two targets (accumulation
and revealage) that the existing `Rgba16Float` pipeline accommodates naturally, and an
approximation whose coarseness does not show on soft sprites. Being order independent, it
also does not interact with compaction at all.

When `blend weighted` lands it will need a depth range from the camera for its weight
function, plus the revealage target and a resolve pass, and wide depth ranges will degrade
the approximation for bright distant elements. None of that reaches the IR surface.

Blend mode is part of an artifact's identity: a procedure writes its `color` and alpha
knowing how they will be combined.

---

## Ambient values and the signal rule

The only implicit values readable inside a block:

| Name | Type | Meaning | Available in |
|---|---|---|---|
| `capacity` | `uint` | allocated element count, set per Set | L1 |
| `t` | `float` | simulation seconds since Set start | all |
| `dt` | `float` | fixed simulation step. Scaled on an element's first update — see [Spawn timing](#spawn-timing) | L1 |
| `camera` | `mat4` | view-projection matrix | L4 |
| `point_coord` | `vec2` | 0..1 within point sprite | L4 fragment |

`seed` is readable in every block as well, but it is a carried attribute rather than an
ambient value — see [Element identity](#element-identity).

The **live** element count is deliberately not exposed. `capacity` is a compile-time
constant; the live count is engine state, and a procedure that branched on it would
produce different geometry at different fill levels for no expressible reason.

**The signal bus (energy, beat, band, bpm, …) is not readable from IR.**

To react to audio or tempo, declare a `param` and attach a signal to it with a `bind`
record in the Set file. This keeps every external coupling declarative, so the UI, the
agents, and the persistence layer all see the same thing.

LLMs violate this rule readily. State it explicitly in the generation prompt.

### On `dt` and simulation time

`dt` is a fixed simulation step, not the real frame delta. This preserves the
reproducibility invariant (same records + same seed → same output).

How far a frame advances comes from a `tick` record, never from a measurement:

```ndjson
{"t":"tick","steps":1}
```

Live, the engine derives `steps` from elapsed real time and emits the record. On replay it
reads the record and measures nothing. Substepping therefore cannot threaten determinism:
the nondeterministic quantity has been moved onto the record stream, which is already the
only path that mutates engine state. **v0.2 always writes `steps: 1`.** Adding substepping
later is then an engine change alone — no format change, no determinism argument to reopen.

- `t` advances by `steps * dt`
- `steps` is capped at 4. Past that the simulation is allowed to fall behind rather than
  catch up, because unbounded catch-up turns a load spike into a death spiral
- `t` is therefore **simulation time, never wall clock**. Once the cap is hit, `t` lags
  real time permanently and never resynchronizes. That is intended — `t` is reproducible
  and a wall clock is not
- `param` values and signal bindings are sampled once per frame and held constant across
  every substep of that frame

---

## Types

`float` `int` `uint` `bool` `vec2` `vec3` `vec4` `mat3` `mat4`

- No implicit conversions. `1` and `1.0` are different.
- Convert with the scalar constructors `float(x)`, `int(x)`, `uint(x)`. The loop variable
  is `int`, so `float(i)` is how it enters float arithmetic.
- Vector constructors take either one scalar per component or a single scalar to
  broadcast: `vec3(1.0, 0.0, 0.0)`, `vec3(0.0)`.
- No structs, arrays, or pointers.
- Swizzles allowed (`v.xy`, `v.zyx`).

---

## Statements and expressions

```
let <name> = <expr>;          // immutable binding
var <name> = <expr>;          // mutable local
<name> = <expr>;              // reassign a var
<name> += <expr>;             // also -= *= /=, var only
<attr> = <expr>;              // write to an attribute
if <cond> { ... } else { ... }
for i in 0..<N> { ... }       // N is a compile-time constant integer
kill();                       // L1 element block only
```

- `let` bindings cannot be reassigned; `var` locals can. No `while`. No recursion.
- `var` requires an initializer and its type is fixed there. There are no uninitialized
  locals and no separate declaration form.
- A `var` is scoped to the block it appears in and does not survive the frame. Element
  state lives in emitted attributes and nowhere else.
- Compound assignment applies to `var` only, never to an attribute. On an attribute it
  would read like accumulation while in fact re-reading the previous frame's value every
  time — see [State semantics](#state-semantics).
- Assigning to a name that was never declared is an error, not a declaration.
- `let`, `var`, and the loop variable may not shadow a param, an attribute, or an ambient
  value.
- Constant loop bounds are what makes cost estimation possible.
- Nested loops multiply into the cost estimate.

`var` is what makes `for` useful. Without it a loop body has nothing to carry between
iterations: `let` cannot be reassigned, and an attribute read always returns the previous
frame's value no matter how many times the loop writes it.

```
element {
  var flow = vec3(0.0);

  for i in 0..4 {
    let f = 0.3 + float(i) * 1.7;
    flow += curl(position * f + vec3(0.0, t * 0.1, 0.0)) / f;
  }

  let v = velocity * 0.96 + flow * dt;
  velocity = v;
  position = position + v * dt;
}
```

Prefer `let` wherever it works. `var` is for accumulation and for values that genuinely
differ between the arms of an `if`. Everything else reads better immutable, and an
immutable binding cannot be accidentally clobbered by a later line the LLM added.

Operators follow GLSL. `%` on `float` follows `mod` semantics (always non-negative), which
differs from WGSL — the lowering inserts a wrapper. On `int` and `uint` it is the ordinary
remainder.

---

## Element lifecycle

L1 procedures are stateful. Attribute buffers are double-buffered.

### Element identity

**There is no `id`.** A buffer slot index is not an identity. The live set is compacted
every frame, so an element's slot number changes while it is alive — a procedure that
wrote `hash1(id)` to pick a color would watch a particle change color mid-flight.

Identity is `seed`: a `uint` assigned at spawn from a monotone counter, stored per element,
and moved with the element by compaction. It is implicit — never declared in `emit` or
`consumes`, always readable in every L1 and L4 block, always allocated. The counter is
engine state and resets at Set start, so it is a pure function of the record stream.

`seed` is an **ordinal, not a random number**. It covers both jobs the old `id` did:

```
let u   = hash1(seed);        // randomness
let col = seed % 512u;        // structured layout
```

In a procedure with no `spawn` block, all `capacity` elements are live from frame zero and
`seed` equals the initial slot index, so grid and lattice generators work as written.

Randomness enters through the hash builtins, which are salted with the `{"t":"seed"}`
value of the slot's layer. `hash1(seed)` therefore changes when the Set is re-seeded while
`seed % 512u` does not, which keeps structure and randomness independently controllable
and satisfies the "all randomness comes from an explicit seed stream" invariant.

LLMs will write `id`. The type checker special-cases the name and suggests `seed` rather
than reporting an undefined identifier.

### State semantics

**Reading an attribute always yields the previous frame's value, regardless of where the
read appears in the block. Assignment only writes the next frame's buffer.**

```
element {
  velocity = velocity + vec3(0.0, -9.8, 0.0) * dt;   // reads prev velocity
  position = position + velocity * dt;               // still reads PREV velocity
}
```

This is deliberate and unambiguous, but it is surprising. Use `let` for intermediates when
you want the updated value:

```
element {
  let v = velocity + vec3(0.0, -9.8, 0.0) * dt;
  velocity = v;
  position = position + v * dt;
}
```

Every attribute in `emit` must be assigned on **every path** through the block, so that the
next frame's buffer is never left undefined. Both arms of an `if` must assign it or neither
may. Repeated assignment is legal and the last write wins; the rule is about coverage, not
about counting. The compiler checks this.

### Blocks

```
spawn   { ... }      // initialize a newly allocated element
element { ... }      // update a live element; may call kill()
```

`spawn` requires a spawn rate. Declare it as a parameter named `spawn_rate`
(elements per second); the engine reads it specially:

```
param spawn_rate : float [0.0, 40000.0] = 8000.0
```

If a procedure declares no `spawn` block, all `capacity` elements are live from frame zero
and `element` alone drives them. This is the simplest form and a good default. Emitted
attributes are zero-initialized before the first frame — the frame-zero `element` pass
reads zeros, not garbage — and `seed` equals the element's initial slot index.

### Spawn timing

`spawn_rate * dt` is rarely an integer. The count is quantized with an **accumulator**: the
fractional remainder carries into the next frame, so the long-run rate is exact and the
sequence stays a pure function of the record stream.

```
carry += spawn_rate * dt * float(steps)
count  = floor(carry)
carry -= float(count)
```

There is deliberately no Poisson option. Irregular spawning is available by binding a noise
signal to `spawn_rate`, where its depth and period are declarative and adjustable; baking a
distribution into the engine would turn that irregularity into a fixed property nobody can
reach.

Count regularity is not what shows, in any case. What shows in a particle stream is
**spatial banding**: every element born in the same frame starts at the same phase, so
discrete shells or rings travel outward. Poisson does not fix that either — the elements of
one frame still share a phase.

The engine fixes it instead. Each new element stores a birth fraction, and its **first
`element` pass runs with `dt` scaled by that fraction**, spreading one frame's worth of
elements evenly along their first step:

```
frac_j = (float(j) + 0.5) / float(count)      // j-th element spawned this frame
```

Even spacing, not a carry-exact birth time; the difference is below a frame and the formula
has to be pinned for replay to match.

This is engine-side on purpose. Exposing the fraction as an ambient and asking the
procedure to apply it would mean banding returns whenever a generator forgets to use it,
and doubles the correction whenever a procedure applies it on top of the engine's — the
same silent-footgun shape as [`id`](#element-identity). Nothing in the IR mentions spawn
timing and no generation prompt has to.

Cost is one value per element and one branch in `element`. The scaled step lands on the
element's first update, which is the frame after it was spawned: `spawn` runs after the
`element` pass, so a new element is rendered once at its spawn state before it first moves.

### Dispatch

`element` dispatches indirectly over the live count, which requires the live set to be
contiguous in the buffer. That contiguity, not determinism, is the primary reason for
compaction: a free list leaves live elements scattered, and dispatching over the live count
then needs a separately maintained compact list of live slots — which costs a scan anyway.

- `kill()` clears the element's alive flag
- Once per frame a prefix sum over the previous frame's live range gives each survivor its
  destination index. The compaction is **order preserving**: survivors keep their relative
  order, so the live set is always a stable subsequence
- The scan result is fused into the `element` pass, which writes each survivor straight to
  its compacted index in the next buffer. Only the scan itself is an extra pass
- The new live count goes into the indirect args buffer
- Free slots are therefore the contiguous range `[live_count, capacity)`, and `spawn`
  dispatches over the head of that range. There is no free list

Order preservation costs a scan per frame and buys a stable blend order. Floating-point
addition is not associative, so this matters under `blend additive` too and not only for a
future non-commutative mode: bit-exact reproduction depends on elements being combined in
the same order on every run. Atomic allocation would be cheaper, but its ordering is
scheduling-dependent, which forfeits exactly that. Measure the scan at full `capacity`
rather than assuming it is free.

---

## Attribute derivation

When a consumer requires an attribute the producer does not emit, the compiler inserts a
derivation if a rule exists. v0.2 rules:

| Attribute | Derived from | Rule |
|---|---|---|
| `velocity` | `position` | `(position - prev_position) / dt` |
| `age` | spawn time | accumulated `dt` since spawn |

Derivation is nearly free because the previous-frame buffers already exist for the state
model above. Derived attributes are marked in the compiled metadata so the UI can show
that they are inferred rather than authored.

---

## L1 example

```
proc drift_shell {
  kind     L1
  topology points
  capacity [65536, 1048576] = 262144

  param spawn_rate : float [0.0, 40000.0] = 12000.0
  param radius     : float [0.1, 8.0]     = 2.0
  param turbulence : float [0.0, 3.0]     = 0.8
  param lifetime   : float [0.5, 20.0]    = 6.0

  emit position, velocity, age

  spawn {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius;
    velocity = vec3(0.0, 0.0, 0.0);
    age      = 0.0;
  }

  element {
    let flow = curl(position * 0.3 + vec3(0.0, t * 0.1, 0.0)) * turbulence;
    let v    = velocity * 0.96 + flow * dt;

    velocity = v;
    position = position + v * dt;
    age      = age + dt;

    if age > lifetime {
      kill();
    }
  }
}
```

---

## L4 blocks and outputs

```
vertex   { ... }     // once per element
fragment { ... }     // once per rasterized fragment
```

Outputs are written with the same syntax as attributes but are stage outputs, not stored
state. Reading one is a compile error.

| Name | Type | Stage | Meaning |
|---|---|---|---|
| `clip` | `vec4` | vertex | clip-space position. Required |
| `point_size` | `float` | vertex | sprite size in pixels. Required when the source topology is `points` |
| `color` | `vec4` | fragment | linear RGB, straight alpha. Required |

Each required output must be assigned on every path, under the same rule as emitted
attributes.

Consumed attributes and `seed` are readable in both blocks. Per-element values reach
`fragment` with **flat** interpolation, which is exact for point sprites; the rule will
need revisiting when a topology with real vertices arrives.

---

## L4 example

```
proc soft_points {
  kind  L4
  blend additive

  consumes position, velocity, age

  param point_scale : float [0.5, 40.0] = 6.0
  param hue         : float [0.0, 1.0]  = 0.6
  param exposure    : float [0.0, 8.0]  = 1.4
  param falloff     : float [0.5, 8.0]  = 2.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = point_scale * (0.3 + 0.7 * clamp(length(velocity) * 0.2, 0.0, 1.0));
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    let a = pow(max(0.0, 1.0 - d), falloff);
    let c = hsv_to_rgb(vec3(hue + hash1(seed) * 0.05, 0.7, 1.0));
    color = vec4(c * exposure, a);
  }
}
```

---

## Color

The pipeline is linear and HDR end to end.

- All internal render targets are `Rgba16Float`
- `color` written from a fragment block is **linear** RGB, straight alpha
- sRGB encoding happens once, at final output
- Values above 1.0 are expected and are what feeds bloom

`hsv_to_rgb` returns **linear** RGB. It performs the sRGB→linear conversion internally so
that authors get the hue they expect without thinking about color space. Explicit
converters `srgb_to_linear` and `linear_to_srgb` are available when needed.

This is a common source of confusion. State it in the generation prompt.

---

## Built-in functions

### Math

`abs floor ceil round fract mod min max clamp mix step smoothstep sign`
`sqrt pow exp log exp2 log2`

### Trigonometry

`sin cos tan asin acos atan atan2`

### Vector

`length distance normalize dot cross reflect refract`

### Hash and noise

```
hash1(uint) -> float                  // 0..1
hash2(uint) -> vec2
hash3(uint) -> vec3
value_noise(vec3) -> float            // -1..1
perlin(vec3) -> float
simplex(vec3) -> float
fbm(vec3, octaves: int) -> float      // octaves is a compile-time constant
curl(vec3) -> vec3
```

Every function in this group is salted with the `{"t":"seed"}` value of the slot's layer,
so re-seeding a Set changes its randomness without touching anything structural. They are
not pure functions of their arguments across Sets; they are deterministic given the record
stream, which is what the invariant requires.

### SDF

```
sd_sphere(vec3, float) -> float
sd_box(vec3, vec3) -> float
sd_torus(vec3, vec2) -> float
sd_plane(vec3, vec3, float) -> float
op_union(float, float) -> float
op_smooth_union(float, float, float) -> float
op_subtract(float, float) -> float
op_intersect(float, float) -> float
```

### Transform

```
rot_x(vec3, float) -> vec3
rot_y(vec3, float) -> vec3
rot_z(vec3, float) -> vec3
rot_axis(vec3, vec3, float) -> vec3
```

### Distribution

```
sphere_point(float, float) -> vec3    // uniform on the unit sphere
disc_point(float, float) -> vec2
```

### Color

`hsv_to_rgb(vec3) -> vec3` `rgb_to_hsv(vec3) -> vec3`
`srgb_to_linear(vec3) -> vec3` `linear_to_srgb(vec3) -> vec3`

---

## WGSL lowering

### L1

`spawn` and `element` become separate compute entry points.

- Workgroup size 64
- Each emitted attribute gets a pair of storage buffers (prev / next), 16-byte aligned.
  `seed`, the alive flag, and the birth fraction are always allocated, whether or not the
  procedure names them — none of the three is nameable from IR
- A scan pass runs first: an exclusive prefix sum over the previous frame's alive flags,
  producing each survivor's destination index and the new live count. Multi-pass over the
  previous live range, not over `capacity`
- `element` uses `dispatch_workgroups_indirect` over the previous live count and writes
  each survivor to its scanned destination index in the next buffers. On an element's first
  update it substitutes `birth_frac * dt` for `dt` — the whole of [Spawn timing](#spawn-timing)
  is this one substitution
- `spawn` dispatches over `[live_count, capacity)`, bounded by the spawn count the engine
  computed for this frame, and writes `seed` from the monotone counter and the birth
  fraction from its index within the frame's batch
- `param` values pack into a single uniform buffer along with `t`, `dt`, `capacity`, and
  the layer's seed salt. `capacity` is a uniform so that changing it needs no recompile
- Buffers swap at the end of the frame

### L4

`vertex` / `fragment` become a render pipeline.

- **`topology points` does not lower to `PrimitiveTopology::PointList`.** WebGPU has no
  point size — a point primitive is always one pixel — so each element expands into a quad:
  six vertices per instance, the corner in `@builtin(vertex_index)` and the element in
  `@builtin(instance_index)`. `point_size` scales the quad in clip space so a sprite keeps
  its pixel size at any depth, and `point_coord` falls out of the corner
- L1 buffers are read as storage, indexed by `@builtin(instance_index)`, not as vertex
  buffers
- Per-element values used in `fragment` become `@interpolate(flat)` varyings
- `blend additive` lowers to additive blending with no depth write, which is what avoids
  any sort requirement. It is the only mode v0.2 accepts
- Target format `Rgba16Float`

### Lowering notes

- `let` lowers to WGSL `let`, `var` to a function-scope WGSL `var`. The semantics match, so
  this is a rename rather than a transformation
- WGSL has no implicit conversions. Insert explicit casts once the IR type check passes.
- `%` differs for negative operands. Emit a wrapper preserving IR `mod` semantics.
- WGSL has no preprocessor. Unroll `fbm` octaves at lowering time.

---

## Validation pipeline

Generated IR passes through these in order. Failure at any stage means no artifact.

1. **Parse**
2. **Type check** — undefined identifiers, type mismatches, missing attribute assignments
3. **Contract check** — `consumes` ⊆ `emit` (after derivation), no signal-bus reads, no
   `param` or local shadowing an attribute name, no assignment to a `let` or to an
   undeclared name, each emitted attribute and required stage output assigned on every path
4. **Cost estimation** — static instruction count × loop bounds, yielding a **per element**
   figure. `capacity` belongs to the Set, so an artifact has no total cost to be judged on;
   rejection here is against a per-element ceiling only
5. **WGSL generation**
6. **Background compilation**
7. **Probe measurement** — GPU timestamps on a small offscreen render
8. **Promotion** — into the live pool if measured cost fits the frame budget

Cost estimation may be conservative. If it wrongly lets something through, stage 7 catches it.

Stages 1–5 are per artifact and, because `capacity` is a uniform rather than a constant,
capacity-independent — the generated WGSL is the same at any capacity. Stages 6–8 run per
Set, which is where `capacity`, the real parameter values, and the frame budget all become
known. The total-cost decision lives there, not in the artifact.

---

## Set file format

`.set.ndjson`. One record per line. Concatenation is composition.

```ndjson
{"t":"set","id":"drift_01","v":1}
{"t":"slot","layer":"L1","proc":"sha256:a3f2c1…"}
{"t":"slot","layer":"L4","proc":"sha256:9c1b04…"}
{"t":"capacity","layer":"L1","value":524288}
{"t":"param","layer":"L1","key":"radius","value":2.4}
{"t":"param","layer":"L4","key":"hue","value":0.58}
{"t":"bind","layer":"L1","key":"turbulence","signal":"energy","curve":"pow2","range":[0.1,2.4]}
{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15}
{"t":"seed","stream":"L1","value":19274}
```

- `proc` is a hash reference. When bundled, the source is inlined as a run of
  `{"t":"src","hash":"…","line":0,"s":"…"}` records.
- `capacity` is optional; without it the `.kir` default applies. A value outside the range
  the `.kir` declares is rejected at Set build time.
- Unknown `t` values are ignored, for forward compatibility.

A Set file is a **state projection**: what is loaded, and what every value currently is. It
carries no time.

---

## Session stream format

What the engine actually consumes is a **timeline**, and it lives in its own file. Same
ndjson, same records, plus `tick` — a session stream is a Set file followed by the record
of what happened:

```ndjson
{"t":"set","id":"drift_01","v":1}
{"t":"slot","layer":"L1","proc":"sha256:a3f2c1…"}
{"t":"slot","layer":"L4","proc":"sha256:9c1b04…"}
{"t":"tick","steps":1}
{"t":"tick","steps":1}
{"t":"param","layer":"L1","key":"radius","value":2.6}
{"t":"tick","steps":1}
{"t":"bind","layer":"L1","key":"turbulence","signal":"energy","curve":"pow2","range":[0.1,2.4]}
{"t":"tick","steps":1}
```

Keeping the two files apart pays in both directions. A Set file stays a few dozen lines a
human can read, instead of accumulating 216,000 `tick` records an hour. And a session gains
something a Set file cannot express: every edit lands at an exact frame position, because
it sits between two known ticks. That is what makes replaying a live performance exact
rather than approximate.

A Set file is the session stream with the ticks dropped and the state folded down. Saving a
Set is that projection; loading one is a session whose head is a Set file and whose tail has
not been written yet.

---

## Metadata file format

Artifact metadata lives in a separate file, not in the `.kir` header. The `.kir` stays
purely a source file that a human or an LLM can read and edit.

`<hash>.meta.ndjson`:

```ndjson
{"t":"meta","hash":"sha256:a3f2c1…","name":"drift_shell","kind":"L1","v":1}
{"t":"origin","prompt":"organic drifting shell, slow","model":"…","seed":19274}
{"t":"parent","hash":"sha256:7e01aa…"}
{"t":"param_decl","key":"radius","type":"float","min":0.1,"max":8.0,"default":2.0}
{"t":"capacity_decl","min":65536,"max":1048576,"default":262144}
{"t":"emit","attrs":["position","velocity","age"]}
{"t":"derived","attr":"age","from":"spawn_time"}
{"t":"perf","kind":"L1","ns_per_element":0.9,"bytes_per_element":48}
{"t":"tag","values":["organic","slow","volumetric"]}
{"t":"preview","path":"previews/a3f2c1….mp4"}
```

The store regenerates metadata from the `.kir` plus a compile pass, so metadata files are
reproducible artifacts rather than hand-authored ones.

**One `t` means one shape, across every file.** A metadata file describes what an artifact
*declares*; a Set file records what a value *is*. Those are different records, so they get
different names — `param_decl` and `capacity_decl` here, `param` and `capacity` there. The
alternative, reusing a `t` for two different shapes in two different files, cannot be read
by a decoder that dispatches on `t` alone, which every ndjson reader does. It is not enough
for the two vocabularies to be disjoint in practice; they have to be disjoint by name.

### On `perf`

The record takes a different shape per kind, because the two do not scale the same way.

**L1 is per element.** It is a compute pass over the live count, so the cost really is
linear and the cost at any `capacity` is a multiplication. That per-element figure is an
intrinsic property of the procedure, which is exactly what an artifact should record —
a total would be a property of a Set that happened to instantiate it.

**L4 is a measurement at reference conditions.** Point sprite cost is dominated by fill
rate: `point_size` and resolution decide the overdraw, so it is not linear in element count
and a per-element figure would be a fiction.

```ndjson
{"t":"perf","kind":"L4","res":"1920x1080","capacity":262144,"params":"default","ms":2.4}
```

The L4 number is for the library to display and for humans to compare. The budget decision
belongs to the probe in stage 7, which measures the real Set with its real parameters. That
is the existing division of labour — estimate conservatively, let the probe be the
authority — and L4 is simply a case where the estimate cannot be made sharp.

---

## Resolved

- **Element identity.** `id` is gone. A slot index is not an identity once compaction
  moves elements, so identity is `seed`, a monotone spawn ordinal carried per element. See
  [Element identity](#element-identity).
- **Compaction strategy.** Order-preserving prefix-sum stream compaction. Indirect
  dispatch over the live count needs contiguity, so a free list would have required a scan
  to build a live-slot list anyway; going straight to compaction also makes the blend order
  stable and reproduction bit-exact. See [Dispatch](#dispatch).
- **Loop accumulators.** `var` is in. `for` could not carry a value between iterations
  with immutable `let` and previous-frame attribute reads as the only options, which left
  it unusable for anything but repeated attribute writes. A `var` is a plain function-local
  in the lowering, costs nothing statically, and does not weaken cost estimation. Scalar
  constructors (`float(i)`) came with it, since without them the loop variable could not
  enter float arithmetic. See [Statements and expressions](#statements-and-expressions).

- **Spawn quantization.** Accumulator, no Poisson: irregularity belongs on a noise signal
  bound to `spawn_rate`, where it stays adjustable, rather than baked into the engine as a
  property nobody can turn. The problem that actually shows is spatial banding, which no
  choice of distribution fixes, so the engine scales an element's first `dt` by its birth
  fraction instead. See [Spawn timing](#spawn-timing).
- **Substepping.** The step count is a `tick` record, not a measurement — emitted from real
  time when live, read back verbatim on replay. v0.2 always writes `steps: 1`, so adding
  substepping later touches the engine and nothing else. Capped at 4. See
  [On `dt` and simulation time](#on-dt-and-simulation-time).
- **Sorting.** Not depth sorting — weighted blended OIT, which is order independent and so
  cannot conflict with compaction. `blend additive` is declared in the L4 header now, as
  the only legal value, so the second mode is an addition rather than a change. See
  [blend](#blend-l4-only).
- **Runtime capacity.** This one was a design error, not a constraint to keep: `capacity`
  is a performance dial, not part of a procedure's identity, and leaving it in the artifact
  would have multiplied the library by every size anyone wanted. It moves to the Set, with
  a range declared in the `.kir`, and the ambient becomes a uniform so that changing it
  costs a fork but no compile. Cost estimation gets cleaner as a result — an artifact
  records cost per element, which is the intrinsic figure. See
  [capacity / topology](#capacity--topology-l1-only).

## Open questions

None. The four that were open are recorded above with their decisions and their reasons.

What remains before implementation is not specification but measurement, and the first
slice should produce it: the scan and compaction at full `capacity`, and the probe pass
that stage 8 promotes on.
