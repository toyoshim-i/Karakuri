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
- The range is mandatory. It is the UI fader range, the agent's search range, the
  normalization basis for signal binding, and the basis for incremental revision: an
  instruction like "a little slower" can only be translated into a value by something that
  knows what the range is. Without one, every revision falls back to regenerating code.
- **Every name the language already gives meaning to is reserved.** A `param` may not be
  called `position`, `size`, `tint` or any other attribute; nor `seed`, `t`, `dt`,
  `capacity`, `camera` or `point_coord`; nor `clip`, `point_size` or `color`; nor `id`.
  Params, attributes, ambients and stage outputs share one scope inside a block, and the
  lowering packs params and ambients into one uniform struct — `param t : float` would
  collide with the ambient `t` in the generated WGSL, silently, at a point far from the
  declaration. The same reservation applies to `let`, `var`, and loop variables.
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

Compatibility is a subset check **between two procedures**, not within one. `emit` is
declared by the L1 procedure in a slot and `consumes` by the L4 procedure paired with it, so
the check belongs to Set composition and cannot be made against a single `.kir` — the
`soft_points` example below consumes three attributes and emits nothing, which is not an
error but the normal shape of an L4 file. When the Set is built and `consumes` is not
contained in the paired `emit`, the Set is rejected — **there is no derivation step in
v0.2.** Synthesizing a missing attribute from what an L1 procedure does emit was designed —
see "Attribute derivation" under [Beyond v0.2](#beyond-v02--specified-not-implemented) — but
implementing it means new per-element state every procedure would carry, whether or not
anything ever reads the derived value, so it waits for the milestone that adds general
attribute adapters rather than being special-cased into v0.2's binary check.

Within one procedure the useful check is narrower: an attribute is readable or writable
only if that procedure declares it, since only a declared attribute gets a buffer pair.

Available attributes:

| Name | Type | Notes |
|---|---|---|
| `position` | `vec3` | |
| `velocity` | `vec3` | |
| `normal` | `vec3` | |
| `uv` | `vec2` | |
| `seed` | `uint` | spawn ordinal. Implicit — never declared, always readable. See [Element identity](#element-identity) |
| `age` | `float` | seconds since spawn |
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

One gap to close before a second L4 style exists. Quad expansion is justified by
`topology points`, but `topology` is declared on the **L1** header — an L4 procedure never
declares one and the checked tree has no field for it. With one topology and one blend mode
the question does not arise, and the moment there are two of either, an L4 procedure will
need a way to say which it is written for.

---

## Ambient values and the signal rule

The only implicit values readable inside a block:

| Name | Type | Meaning | Available in |
|---|---|---|---|
| `capacity` | `uint` | allocated element count, set per Set | L1 |
| `t` | `float` | simulation seconds since Set start | all |
| `beats` | `float` | musical position on the session's tempo grid, at the instant `t` names | all |
| `dt` | `float` | fixed simulation step. Scaled on an element's first update — see [Spawn timing](#spawn-timing) | L1 |
| `camera` | `mat4` | view-projection matrix | L4 |
| `point_coord` | `vec2` | 0..1 within point sprite | L4 fragment |

`seed` is readable in every block as well, but it is a carried attribute rather than an
ambient value — see [Element identity](#element-identity).

### `beats`, and why it is not a signal

**Without it a procedure runs at wall time whatever the music does.** `t` is the only clock
IR could read, so a tempo change moved the grid every binding is sampled on and moved
nothing that was drawn. A `bind` cannot close that: a binding writes one `param`, and what
follows a tempo is not a parameter value but the passage of time itself.

It is an ambient rather than a bus name for the same reason `t` is. The signal rule below
says IR cannot read the bus — every external value arrives through a declared `param` — and
that rule exists so a procedure's inputs are enumerable. A clock is not an input in that
sense; it is the thing the procedure is evaluated *at*. `beat` and `bar` remain bus names,
and they are a different thing: phases in `[0, 1]`, for driving a parameter through a
`curve` and a `range`. `beats` is unbounded and monotone, for driving a position.

Three properties, all of them load-bearing:

- **The same instant as `t`.** At 60 bpm the two are numerically equal, and a procedure
  reading both is reading one clock rather than two nearly-equal ones.
- **Per substep**, exactly like `t`. A frame that advances three steps sees three musical
  instants, so a frame rate drop does not move the beat.
- **Continuous across a tempo correction.** A correction bends the rate and never moves a
  beat that has already happened, so following the room does not mean jumping every time
  the tracker trims — and it trims several times a second.

What it is **not** is a seek. Reading `beats` says where the room is; it does not let a
procedure be evaluated at another time. That is the transport, it needs `closed_form`, and
it is not built. Nor does reading `beats` disqualify a procedure from being closed form: the
grid is a pure function of `t` given its current tempo and anchor, so a procedure of
`(seed, t, params, beats)` is still evaluable at any `t` — against the grid **as it stands**,
which is what a scrub wants and is not what the grid historically was.

The **live** element count is deliberately not exposed. `capacity` is a compile-time
constant; the live count is engine state, and a procedure that branched on it would
produce different geometry at different fill levels for no expressible reason.

### Viewing conditions

A procedure has to choose a world scale and a brightness, and neither is guessable. Stating
them is not a courtesy: a generated procedure that picks the wrong scale renders a flat
sheet, and one that picks the wrong exposure renders a white blob, and **the compiler has
nothing to say about either**. These are the only numbers in this document that exist
solely so that something writing a procedure can aim.

Until L3 exists the camera is a built-in orbit:

| | |
|---|---|
| Radius | 8.0 world units from the origin |
| Height | 2.0 |
| Field of view | 60° vertical |
| Near / far | 0.1 / 100.0 |
| Target | the origin |

So **material should occupy roughly a radius of 1 to 4 around the origin.** Much smaller
and it is a dot; much larger and it fills the frame with no silhouette.

Brightness is harder, because the pipeline is additive and has no tone mapper yet. `color`
is linear and unbounded, values above 1.0 clip at output, and every element in a sprite's
footprint adds. The practical consequence is that **usable exposure scales inversely with
element count**: what reads correctly at 4096 elements is a solid white disc at 262144. A
procedure cannot know the capacity it will run at — that is a Set-level dial — so this is
a problem the engine has to solve rather than the author. Until it does, an artifact
authored for a high element count carries an exposure far below 1.0, and that value is a
workaround rather than a property of the material.

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
the nondeterministic quantity has been moved onto the record stream, which is the path
engine state is meant to be mutated through. Adding substepping later is then an engine
change alone — no format change, no determinism argument to reopen.

**Substepping landed and the record did not.** `steps` is no longer always 1: it is derived
per frame and capped at 4, and the engine advances by that count. What has not been built
is the emission — nothing constructs a `Record::Tick`, so the value goes to the engine as a
number. The determinism argument above survives that intact, because what it rests on is
the engine advancing by a **step count** rather than by a duration, and that is what it
does. The record is a serialisation of a quantity that already exists in the right shape;
`README.md`'s Invariants list what else is in the same position.

- `t` is `steps_taken * dt`, where `steps_taken` is the whole number of steps the session
  has advanced. **Computed from the count, not accumulated into a running sum** — a float
  sum drifts by an ULP or two depending on how the steps were grouped, and two tick
  histories reaching the same elapsed time have to be the same point in the session
- `steps` is capped at 4. Past that the simulation is allowed to fall behind rather than
  catch up, because unbounded catch-up turns a load spike into a death spiral
- `t` is therefore **simulation time, never wall clock**. Once the cap is hit, `t` lags
  real time permanently and never resynchronizes. That is intended — `t` is reproducible
  and a wall clock is not
- **`t` advances once per substep, not once per frame.** A frame of two steps runs its two
  `element` passes at the two instants that two frames of one step would, and a procedure
  reading `t` sees the same sequence either way. Holding `t` constant across a frame's
  substeps would run both passes at the same instant, which any accumulating procedure can
  see — and substepping exists precisely so the simulation does not depend on frame rate
- `param` values and signal bindings are the other half of that split: they are **input**,
  genuinely sampled once per frame and held constant across every substep of it. `t` is not
  input, it is the simulation's own clock

---

## Types

`float` `int` `uint` `bool` `vec2` `vec3` `vec4` `mat3` `mat4`

- No implicit conversions. `1` and `1.0` are different.
- Convert with the scalar constructors `float(x)`, `int(x)`, `uint(x)`. The loop variable
  is `int`, so `float(i)` is how it enters float arithmetic.
- Vector constructors follow GLSL: any mix of scalars and shorter vectors whose component
  counts sum to the target width, or a single scalar to broadcast. `vec3(1.0, 0.0, 0.0)`,
  `vec3(0.0)`, and `vec4(position, 1.0)` are all well formed — the last one is what the L4
  example uses to get a clip-space position, so a narrower rule would reject this
  document's own code.
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
for i in <start>..<end> { ... }   // both are integer literals
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
- Loop bounds are integer literals, not expressions. Constant bounds are what makes cost
  estimation possible, and a literal is the only form the check pass need not reason about.
- Nested loops multiply into the cost estimate.
- Statements are terminated by `;`. **Header declarations are not terminated at all** — one
  ends where the next declaration or block begins. That is a real ambiguity and worth
  knowing about: a `param` default ending in a call whose closing parenthesis is missing
  will swallow the following line as arguments, and the diagnostic will land nowhere near
  the mistake.

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

LLMs will write `id`. **`id` is therefore a reserved word, not merely an absent one** — it
cannot be a `param`, a `let`, a `var`, or a loop variable either. Reserving it means the
parser can reject it outright and name it, instead of a later stage reporting an undefined
identifier or, worse, a procedure that declares `param id` and quietly works.

Signal names are the opposite case and belong to name resolution, not to the parser. A
procedure may legally declare `param energy`, so whether a bare `energy` resolves depends
on what was declared, and only the check pass knows that.

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

### Closed form versus accumulating

A procedure is **closed form** if its state at time `t` is a pure function of `seed`, `t`,
and its parameters. That takes three things, not one: it never reads an attribute it emits,
it has no `spawn` block, and it never calls `kill()`. All three are spelled out below and
all three are independent — a procedure that spawns is accumulating however pure its
`element` block is, because *which elements exist* is history.

A procedure is **accumulating** if it fails any of them. Reading its own previous output is
the usual way, and is what integration looks like.

```
element {                                  // closed form
  position = sphere_point(hash1(seed), hash1(seed + 1u)) * (1.0 + sin(t));
}

element {                                  // accumulating
  velocity = velocity + gravity * dt;      // reads `velocity`, which it emits
  position = position + velocity * dt;
}
```

**The check pass decides this and records it on the compiled procedure.** It is not a
diagnostic — neither answer is an error — but it is the property the engine's whole
lifecycle turns on, and it is worth writing a procedure one way rather than the other on
purpose.

**What being closed form buys.** Both halves are worth knowing, and the second is the
larger one:

- **No priming.** A closed-form Set goes Cold to Live with no warm-up, because there is no
  accumulated state to warm; evaluating it at `t` *is* arriving at `t`. An accumulating one
  has to be run forward from its start, one step at a time, which is what priming is for
  and what makes a deck slot expensive to hold ready.
- **It can be scrubbed.** Any `t` can be evaluated directly, so the material can be run
  forward at any rate, held still, or **run backwards** — tape-style transport, locked to
  the beat grid. An accumulating procedure can only ever go forward one step at a time.
  Reversing it is not slow, it is impossible: there is no un-integrating a sum.

So closed form is not a compiler-internal classification, it is an authoring choice with a
consequence a performer can feel. **Prefer closed form when the look allows it.** Writing
`position = f(seed, t)` where an accumulator would also have worked gives up nothing and
buys a scrubbable instrument; reaching for `position = position + …` is a decision to
trade that away, and is worth making deliberately rather than by habit.

Three things make a procedure accumulating, and the classifier is conservative about all
of them — it under-claims rather than over-claims, because a wrongly-claimed closed form
does not merely show an unwarmed first second, it produces garbage the moment anything
seeks:

1. **It reads an attribute it emits**, anywhere in any block — inside an `if`, inside a
   `for`, or bound to a `let` and used later. Reaching the value through a local is still
   reading the attribute.
2. **It has a `spawn` block.** This is not about attributes: an element that does not exist
   yet cannot be evaluated, and *whether it exists* is engine state accumulated from every
   frame since the Set started. Jumping to `t = 30` on a procedure that spawns does not
   produce thirty seconds' worth of elements. **Spawning and closed form cannot coexist.**
3. **It can `kill()`.** A killed element stays killed, so which elements are alive at `t`
   depends on every step taken to get there rather than on `t`.

**One caveat that applies to seeking and not to priming.** Priming only ever runs a
procedure forward from a state it already has, so the procedure is all it needs. Seeking to
an arbitrary `t` also needs everything *else* that is a function of time at that instant to
be evaluable there. **Tempo correction has landed, and `beats` has made this concrete
rather than hypothetical**: a procedure reading `beats` is a function of the grid as well
as of `t`, and the grid at a past `t` depends on the correction history, which is not a
function of `t`.

That does not disqualify it. `Oscillator::at_time` answers with the grid **as it stands**,
which is a pure function of `t` given the current tempo and anchor, so such a procedure is
evaluable anywhere — it simply answers about today's grid rather than about the grid that
was running then. For a scrub that is the wanted answer: seeking to bar 32 means bar 32 of
the grid the room is on now. For an exact re-run of a past moment it is not, and nothing
keeps the history that would be. **`closed_form` is a property of the procedure and is
necessary for a seek, not sufficient for one.** Whatever builds the transport owes the
other half and owes this distinction with it.

The same distinction decides whether beat-resolution variant selection is affordable:
switching between closed-form alternatives costs nothing, while keeping three accumulating
alternatives selectable means three simulations resident.

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
fractional remainder carries into the next substep, so the long-run rate is exact and the
sequence stays a pure function of the record stream.

```
per substep:
  carry += spawn_rate * dt
  count  = floor(carry)
  carry -= float(count)
```

Once per substep, not once per frame with `steps` folded in. The frame's total is the same
either way — the carry is a running real number — but a single batch would put a frame of
two steps' worth of elements in before the second `element` pass, where two frames of one
step interleave them. That is the same reason `t` advances per substep.

There is deliberately no Poisson option. Irregular spawning is available by binding a noise
signal to `spawn_rate`, where its depth, period, and character are declarative and
adjustable; baking a distribution into the engine would turn that irregularity into a fixed
property nobody can reach. All three have to be reachable for that argument to hold — depth
is the binding's `range`, and period and character are the `noise` fields described under
[Set file format](#set-file-format).

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

Cost is one value per element and no branch at all. The fraction expires itself: `element`
computes its step as `dt * birth_frac` and unconditionally writes `birth_frac = 1.0` for
the next frame, so the correction applies exactly once without anything having to ask
whether this is an element's first update. The scaled step lands on that first update,
which is the frame after the element was spawned — `spawn` runs after the `element` pass,
so a new element is rendered once at its spawn state before it first moves.

### Dispatch

`element` dispatches indirectly over the live count, which requires the live set to be
contiguous in the buffer. That contiguity, not determinism, is the primary reason for
compaction: a free list leaves live elements scattered, and dispatching over the live count
then needs a separately maintained compact list of live slots — which costs a scan anyway.

- `kill()` clears the element's alive flag. The slot is not reclaimed in that step — the
  element still occupies it, with the flag clear, until the *next* step's scan counts it
  out. It is inside the draw range for exactly one frame, scattered among the survivors
  rather than gathered at one end, which is why the vertex stage reads the flag per
  instance and collapses a dead element's quad to nothing
- Once per step a prefix sum over the previous step's live range gives each survivor its
  destination index. The compaction is **order preserving**: survivors keep their relative
  order, so the live set is always a stable subsequence
- The scan result is fused into the `element` pass, which writes each survivor straight to
  its compacted index in the next buffer. Only the scan itself is an extra pass
- The scan writes the survivor total and **nothing else**: `element` runs after it and
  still has to cover the pre-scan range, so rolling the range forward there would make
  every survivor's own step dispatch over the count it is about to become. A separate
  single-invocation pass, after both `element` and `spawn`, sets
  `range = survivors + min(spawn_count, capacity - survivors)` and derives the dispatch and
  draw arguments from it
- Free slots are therefore the contiguous range `[survivors, capacity)`, and `spawn`
  dispatches over the head of that range. There is no free list

Order preservation costs a scan per frame and buys a stable blend order. Floating-point
addition is not associative, so this matters under `blend additive` too and not only for a
future non-commutative mode: bit-exact reproduction depends on elements being combined in
the same order on every run. Atomic allocation would be cheaper, but its ordering is
scheduling-dependent, which forfeits exactly that. Measure the scan at full `capacity`
rather than assuming it is free — it is 0.083 ms at 262144 on the development machine, and
that is the number a procedure with no `spawn` block and no `kill()` does not have to pay:
its live set cannot change, so the engine skips the scan for it and `element` writes in
place.

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
- All per-element state packs into **one** `Element` struct per direction (prev / next),
  not one buffer pair per attribute: `seed`, then the birth fraction, then `emit` in
  declaration order, one 16-byte slot each. `seed` and the birth fraction are always
  allocated whether or not the procedure names them — neither is nameable from IR. A
  procedure emitting three attributes therefore binds four storage buffers in its compute
  stage rather than twelve, which matters because the WebGPU default limit is 8 and the
  downlevel default is 4
- The alive flag is the one exception: it leaves the struct and becomes its own dense
  `array<u32>`, four bytes per element, because the compaction scan reads alive flags with
  no stride arithmetic and packing them into `Element` would make the scan depend on a
  per-procedure struct size
- A scan pass runs first, once per **step**: an exclusive prefix sum over the previous
  step's alive flags, producing each survivor's destination index and the survivor total.
  Multi-pass over the previous live range, not over `capacity`
- `element` uses `dispatch_workgroups_indirect` over the previous live range and writes
  each survivor to its scanned destination index in the next buffer. On an element's first
  update it substitutes `birth_frac * dt` for `dt` — the whole of [Spawn timing](#spawn-timing)
  is this one substitution
- `spawn` dispatches over `[survivors, capacity)`, bounded by the spawn count the engine
  computed for **this substep**, and writes `seed` from the monotone counter and the birth
  fraction from its index within that substep's batch. It clamps against `capacity` on the
  GPU and drops what does not fit; the range grows by what landed, not by what was asked
  for, and the host's seed counter advances by the request so a dropped element leaves a
  gap in the seed sequence rather than a repeat
- `param` values pack into a single uniform buffer along with `dt`, `capacity`, and the
  layer's seed salt. `capacity` is a uniform so that changing it needs no recompile. What
  is *not* in it is anything that is not constant across the frame: where the live range
  ends is GPU state once compaction owns it, and lives in the same buffer as the indirect
  arguments derived from it; `t`, `spawn_count` and `seed_base` differ between the substeps
  of one frame, and live in a small per-substep buffer the engine writes ahead of the frame
  and binds one entry of at a time
- Every slot of `Element` is a full 16-byte `vec4` regardless of the attribute's width, so
  `(2 + emit.len()) * 16` is the stride of every element buffer and no attribute needs its
  own arithmetic. It costs memory — a `float` attribute occupies four times what it needs —
  and that is worth revisiting if VRAM becomes the binding constraint before something else
  does
- Buffers swap at the end of every **step**, not every frame: what one substep wrote as
  "next" is the next substep's "prev", and after the last one it is what L4 reads

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
3. **Contract check** — no signal-bus reads, no shadowing, no assignment to a `let` or to
   an undeclared name, no reference to an attribute the procedure does not declare, and
   each emitted attribute and required stage output assigned on every path. Note that
   `consumes` ⊆ `emit` is **not** checked here: it relates two procedures and belongs to
   Set composition, at stage 6. This stage also decides
   [closed form versus accumulating](#closed-form-versus-accumulating) and records it —
   the one thing it produces that is not a diagnostic, since neither answer is an error
4. **Cost estimation** — static instruction count × loop bounds, yielding a **per element**
   figure. `capacity` belongs to the Set, so an artifact has no total cost to be judged on;
   rejection here is against a per-element ceiling only
5. **WGSL generation**
6. **Background compilation**
7. **Probe measurement** — GPU timestamps on a small offscreen render
8. **Promotion** — into the live pool if measured cost fits the frame budget

Cost estimation may be conservative. If it wrongly lets something through, stage 7 catches it.

**There is one severity.** A diagnostic is an error and failure means no artifact; there
are no warnings. The response to a rejection is to regenerate, not to proceed with a
caveat, and a caveat nobody can act on is worse than a refusal. That puts a requirement on
the diagnostics themselves: a rejection has to carry what a regeneration needs to aim at.
A cost rejection states the estimate and the ceiling as numbers, not merely that one
exceeded the other — "over budget" tells a repair prompt nothing about how much to cut.

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
- `param` and `bind` are keyed by `layer` in the record, but the engine holds one value per
  **name** across the whole Set. Two procedures that happen to declare the same param name
  would therefore be one value, with the second declaration's default quietly taking the
  first's place — so a pair that collides is **rejected at Set build time**, naming every
  colliding param at once. Keying values by layer, the way these records already do, is
  what would let the collision through harmlessly; until then a rejection the generator can
  act on beats a value nobody chose.
- Unknown `t` values are ignored, for forward compatibility.
- **`gain`, `residency`, `look` and `transport` are not in this list and must never be.**
  They are the session's state rather than any Set's — see the session stream format.

**Implemented, and the engine obeys it.** `karakuri-cli`'s `--save-set` writes one of
these and puts both `.kir` sources in the store as content-addressed artifacts;
`--load-set` reads it back and builds from it, resolving each `slot` by hash or from
inlined `src` records when the file is bundled. A run driven by a Set file renders the
same frame as the run whose flags wrote it.

`--bind` survives as a way of *writing* a `bind` record rather than as a path beside one:
the flag parses its fields into a `Record::Bind` and hands it to the same decoder a Set
file uses, so a command line and a file cannot mean different things by the same fields.

**Three places this format is finer than the engine**, all of them reported on load rather
than dropped: `seed`, `capacity` and `param` are keyed by layer while the engine holds one
seed, one capacity and one flat parameter map per Set; a `param` may be a vector while the
engine's map holds `f32`; and `camera` carries two of the six fields the engine's orbit
has. Each is a disagreement between the format and the engine rather than a gap in the
loader, and settling them is the format's business and the engine's, not the reader's.

The *session* records are further along: `audio` and `tempo` per frame, and `gain`,
`residency`, `look` and `transport` per key press, are each built by the CLI, decoded back,
and only then applied. Nothing writes them to disk yet, so what exists is the live half of the
round trip and not a file — but the path the engine is driven through is the record's.

### What a binding does

Every frame, for each binding, in this order:

1. **Sample** the `signal` by name. The bus is always complete: an unknown name is not an
   error and not an absence, it is a value with a confidence of 0.0. Nothing anywhere tests
   whether a provider exists.
2. **Curve** the sample's value, which is expected in `[0, 1]` and is clamped into it.
3. **Map** onto `range`, so `range` is what the param is written with at the two ends.
4. **Blend by confidence**: the value written is `lerp(the param's own value, the mapped
   value, confidence)`.
5. **Write** it as a uniform — the same path a `param` record takes. A parameter change
   needs no fork and no recompilation.

Step 4 is what "consumers branch only on confidence" means in practice, and its
consequences are meant to be followed rather than softened:

- `bpm`, `beat` and `bar` come off the local oscillator, which is the single source of
  truth for phase and tempo, so they carry confidence 1.0 and a binding to them takes full
  effect.
- `energy`, `onset` and `band<N>` carry whatever the provider behind them says. **With an
  audio input open they are measurements at confidence 1.0 and a binding to them decides
  its param outright; with none they are invented at 0.1** — except `onset`, which nothing
  invents — and the same binding moves the same param a tenth as far. That is the system
  being honest about what it knows, and it is the whole of what changed when audio landed:
  the same name, sampled by the same call, answering with a different confidence. See
  [Measurement in the stream](#measurement-in-the-stream--audio-and-tempo).
- A signal nobody provides leaves the param at its own value, exactly, because confidence
  is 0.0. That is the same arithmetic rather than a special case.

A param's own value is the **base** of the blend and is never overwritten. So a param that
is both bound and given a `param` record has one answer, and it does not depend on which of
the two was written last: the record moves the base, the binding blends from it every
frame. There is at most one binding per (`layer`, `key`); a second replaces the first,
because two would be resolved in some order and the order would decide the value.

**Curves.** Four, which is the number of distinct shapes a monotone `[0,1] -> [0,1]` map
has. A fifth would be a re-parameterisation of one of these, and a vocabulary an LLM
generates against pays for every name twice.

| `curve` | | |
|---|---|---|
| `lin` | `x` | The signal as it is |
| `pow2` | `x²` | **Emphasises the peak.** The floor is flattened, so the param only moves near the top — a `beat` through `pow2` reads as a hit rather than a wobble |
| `sqrt` | `√x` | **Emphasises the floor**, the exact complement of `pow2`: rises fast off zero and compresses the top, so a signal that lives near the bottom still produces visible movement |
| `smooth` | `x²(3 - 2x)` | **Eases both ends.** Zero derivative at 0 and at 1, so the param neither jumps off the floor nor slams into the ceiling. For a param that is a position rather than an intensity |

An unrecognised `curve` is a diagnostic, not a silent fall back to `lin`.

**Unit range.** Steps 2 and 3 are what make `range` mean what it says, and they assume the
signal is in `[0, 1]`. Two signals are not, and both are stated rather than papered over:

- `bpm` is a tempo in beats per minute, so it clamps to 1.0 and a binding to it is pinned
  at the top of its range for the whole run. Normalising it would need an invented tempo
  range that nothing could justify, so it is **refused** instead: `karakuri-cli`'s `--bind`
  rejects `signal=bpm` and names `beat` and `bar`, which carry the same tempo in the range
  a binding is defined over. A decoder that loads `bind` records off disk owes the same
  diagnostic — a pinned binding and a working one are the same number on a status line, and
  the whole cost of getting this wrong is paid by whoever cannot tell them apart.
- noise is signed, in `[-1, 1)`. A binding maps it to `[0, 1]` before the curve, because
  clamping would discard the half of the signal below zero — a `spawn_rate` bound to noise
  would then sit at the bottom of its range half the time.

### Binding noise

A `bind` whose `signal` is `noise` takes a `noise` object, because a noise generator has
parameters no other signal has:

```ndjson
{"t":"bind","layer":"L1","key":"spawn_rate","signal":"noise",
 "noise":{"kind":"perlin","rate":0.5,"stream":3},"curve":"lin","range":[4000,16000]}
```

- `kind` is `white`, `value`, `perlin`, or `fbm`, defaulting to `perlin`. The names are the
  ones the IR builtins already use; there is no reason for the bus to have a second
  vocabulary for the same thing.
- `rate` is in **cycles per beat**, so period is tempo-relative and follows the local
  oscillator rather than a second notion of time. `white` holds each value for `1 / rate`
  beats, which gives even the jittery kind a period. Defaults to `1.0`.
- `stream` decorrelates one binding from another. Two bindings sharing a stream move
  together, which is occasionally what you want and never what you get by accident.
  Defaults to `0`.
- `octaves` is `fbm`'s layer count and is ignored by the other three kinds — a decoded
  record cannot distinguish an omitted `octaves` from one written as `4`, so ignoring is
  the only semantics available to it. `--bind` can tell, and therefore refuses
  `noise.octaves` alongside any `noise.kind` but `fbm` rather than silently promoting the
  kind. Defaults to `4`. **This field was missing from v0.2**, which listed `fbm` as a
  `kind` and gave it nowhere to say how many octaves — leaving the count baked into the
  engine, which is
  precisely the "fixed property nobody can reach" that [Spawn timing](#spawn-timing)
  rejects Poisson for. The omission is the specification's, not the implementation's.

Every field defaults, and the whole `noise` object does too: a `bind` naming `noise` and
saying nothing else is one cycle per beat of perlin on stream 0. Absent means the default
generator, not the absence of one — there is nothing else for the name to mean. This is the
one record whose numbers a generator has to invent, and each of them has a defensible
default.

`noise` is also the one `signal` name that is **not** a lookup on the bus. The bus takes a
name and nothing else, and there is no collision-free grammar for four fields inside one
string — which is why this object exists at all. So a binding whose `signal` is `noise`
reads the generator it declares here, and **the bus does not answer that name at all**: a
parameterless stand-in on the bus would put a second, weaker meaning behind a name the
`bind` record already owns, which is the shape the [`t` vocabulary rule](#set-file-format)
refuses when it asks for vocabularies disjoint by name rather than in practice. The bus
stays complete regardless — an unknown name is a value at confidence 0.0, as always.

**A noise binding carries full confidence.** It is not a guess at something unobserved:
the binding declares the generator, the generator is deterministic in the session seed,
and its value is exactly
what it claims to be — the same reason the local oscillator's own signals are certain. The
alternative would make [Spawn timing](#spawn-timing) unwriteable: irregular spawning is
available *only* by binding noise to `spawn_rate`, and a binding at a tenth effect is not
an alternative to the Poisson option that section rejects.

Depth is `range`, as for any binding. Together those are the three axes the spawn-timing
decision depends on being reachable — see [Spawn timing](#spawn-timing).

Tempo-relative rate had a consequence to settle before external sync arrived, and external
sync has now arrived: the oscillator is corrected against tracked audio, so **every** noise
binding could start tracking those corrections, including ones with no musical intent.

**The decision, made rather than deferred: a noise rate stays cycles per beat and follows a
*tempo* correction; it does not follow a *phase* correction.** The oscillator carries two
beat counts for this — musical position, which takes phase shifts, and elapsed beats, which
does not — and noise reads the second. A tempo correction is a rate change and reaches
noise continuously, with no jump at the instant it lands, which is what the accumulators
are for. A phase correction is the beat grid being realigned with a room, and re-hashing
every noise stream because of it would make a flicker with no musical intent jump whenever
the tracker nudged the grid.

The alternatives were a seconds-relative mode and a rate frozen at bind time. Both were
rejected for the same reason: they make two kinds of noise, and every existing `bind`
record becomes ambiguous about which kind it asked for. What is given up is the claim that
a noise lattice point coincides with a beat instant after a correction — nothing depends on
that, and nothing can observe it.

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

### Measurement in the stream — `audio` and `tempo`

Live audio is not reproducible, and the same record stream is required to reproduce the
same output bit for bit. Those can only both be true if **the measurement joins the
stream**, which is exactly what `tick` already does with elapsed time: derived from the
world when live, read back verbatim on replay, and the engine cannot tell which happened.
Audio adds two records on those terms, and no third path.

```ndjson
{"t":"audio","energy":0.42,"onset":0.75,"bands":[0.9,0.4,0.2,0.11,0.05,0.02,0.01,0.0],"confidence":1.0}
{"t":"tempo","bpm":128.03,"shift":-0.0041,"confidence":0.86}
{"t":"tick","steps":1}
```

**`audio` is one frame's worth of measured signals**, at most one per frame, before the
tick it belongs to. Named fields for the named signals and a positional array for the
bands, because `band0`…`bandN` *are* positions — the array is the naming scheme rather
than a second one, and a map of names would both repeat those names 200,000 times an hour
and let a stream invent names the bus has rules about. The array's length is the band
count, so a stream with more bands than a reader knows about still decodes.

One `confidence` for the whole line, not one per signal: these values came out of one block
of samples at one instant, so their staleness is one number. It is **full while a device is
open and delivering, falling as the last block goes stale, and 0.0 when nothing has
arrived**. A silent room is `energy` 0.0 at confidence 1.0 — a measurement, which drives a
bound parameter to the bottom of its range — and an interface pulled out mid-set is the
same zeroes at a confidence sliding to 0.0, which hands every bound parameter back to
whoever set it. The two are different lines, and the difference is the whole reason
confidence is a number rather than a flag.

**A frame with no `audio` record is not a frame of silence.** It is a frame with no
provider, and every name answers exactly what it answered before audio existed — `energy`
and the bands invented at confidence 0.1, `onset` at 0.0 and confidence 0.0. Nothing
branches on which of the two it is; the bus is complete either way.

**`tempo` is what the local oscillator is corrected to**, and it carries the *correction*
rather than the estimate behind it: a new `bpm`, and `shift`, a phase shift in beats,
positive meaning the next beat arrives sooner. Recording the decision rather than the
observation is what keeps a session replayable after the analyser has been improved — and
it is why replay needs no audio at all. The first `tempo` in a stream is also what sets the
session tempo, which **v0.2 had no record for**.

Neither is state, so neither appears in a Set file: both are what a frame *saw* or
*decided*, and the tempo belongs to the session rather than to any one Set.

### The mix in the stream — `gain`, `residency`, `look` and `transport`

A session that carried the material and not the performance would replay the same Sets, on
the same beat, all at whatever gain they happened to start at, with nothing ever going on
or off air. Three records carry what an operator moves:

```ndjson
{"t":"gain","slot":0,"value":0.75}
{"t":"residency","slot":1,"level":"priming"}
{"t":"look","op":"aces","exposure":1.2,"white_point":4.0}
```

**`gain` is a deck slot's linear gain into the mix.** The slot is a position on the deck,
not anything about the Set in it: moving a Set to another slot moves it under another
fader, which is what a fader is.

**`residency` is what a slot is asked to do** — `live`, `priming` or `allocated` — and it
is always the *request*, never the effective level. The governor recomputes the second
every pass from the budget of the machine that is running, so a session recorded on a fast
machine and replayed on a slow one must re-derive it; recording what was decided would
replay one machine's budget onto another's. That is the opposite of the choice `tempo`
makes, and for the opposite reason: there, what was decided is the reproducible thing.

**`look` is the output look, all of it in one line.** Tone map operator, exposure and white
point together, because it is one value written to one uniform, and a stream that could set
the exposure without saying which operator it applies to would describe a look nobody can
reconstruct. Session-wide rather than per slot, since tone mapping happens once, after the
mix.

`level` and `op` are strings for the same reason `curve` and `noise.kind` are: an
unrecognised value is the engine's to diagnose against what it actually supports, not the
decoder's to reject before anything can say what the alternatives were.

**`transport` is what a slot's clock does with the session's**, and it carries two numbers
because the mode alone does not mean anything without them:

```ndjson
{"t":"transport","slot":0,"sync":"beat","anchor_bpm":126.0,"offset_beats":-0.25}
```

`anchor_bpm` is the tempo at which this material runs at 1×, and it has to be recorded
because **material has no intrinsic tempo**: a `.kir` declares parameters and a capacity,
not a bar length, so "one beat of music is how many seconds of material" is an operator's
answer rather than the artifact's. `offset_beats` is the scrub — signed, unbounded, and the
one value in this format that is meant to go backwards. Both are carried under every mode,
including `free` where neither does anything, so that a slot moved back onto the grid
returns to where the operator left it rather than to a default.

**These four are state, and a Set file still must not contain them.** That is a second
reason for a record to be absent from a Set file and it is not the `audio` one: there is
something to fold here, and this is not the projection it folds into. A Set file that
restored a gain would apply it to whatever slot it was next loaded into, and one whose
`residency` said `live` would put a Set on air by being opened. The session projection they
*do* belong to is not written yet, because nothing writes a session stream.

**Signal names.** `energy` and `band<N>` are the ones the synthesized bus already answers,
deliberately: a measured `energy` and an invented one are the same signal from different
sources, and a binding that had to be rewritten when a microphone appeared would defeat the
arrangement. `onset` is new — a **decaying envelope in `[0, 1]`**, 1.0 at a detected
transient and falling from there, so that a `bind` with a `curve` reads it as a hit. It is
not an impulse: analysis blocks and rendered frames are not locked to each other, so an
impulse one block wide would be missed by some frames and counted twice by others. The
synthesized bus does **not** invent an `onset`, because an invented one would be `beat`'s
pulse under a second name.

Levels are `[0, 1]` because `curve` and `range` are defined over that: they are RMS in
dBFS, mapped from −60 dBFS to −6 dBFS, and a band reads the level of the part of the signal
inside it on the same scale. So a full-scale tone pins, a well-mastered track lives in the
top third, and silence is exactly 0.0.

---

## Beyond v0.2 — specified, not implemented

Everything above this line is implemented and tested. Everything below is design that has
been settled but not built: **no parser accepts it, no checker enforces it, and no
generator emits it.** It is written down because later milestones depend on these shapes
and because deciding them now keeps V1 from foreclosing them. Each carries the milestone
it belongs to; see `docs/roadmap.md`.

### Metadata file format — M4

Nothing writes or reads one. `karakuri-store`'s record vocabulary covers the Set file
and the session stream — `set`, `slot`, `capacity`, `param`, `bind`, `camera`, `seed`,
`src`, `tick` — and **none of the records below**. The store is content-addressed from
M1, which is half of what M4 asks of it; the separate metadata file is the other half
and does not exist yet. `parent` in particular has to start being recorded with the
first generated artifact or the genealogy has a hole at its root.

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
{"t":"perf","kind":"L1","ns_per_element":0.9,"bytes_per_element":48}
{"t":"tag","values":["organic","slow","volumetric"]}
{"t":"preview","path":"previews/a3f2c1….mp4"}
```

The store regenerates metadata from the `.kir` plus a compile pass, so metadata files are
reproducible artifacts rather than hand-authored ones.

`origin` records what produced *this* revision, not the original intent. When an artifact
came from revising another one, `parent` points at what it was revised from and `prompt`
holds the instruction that did it — "more aggressive, red", not the paragraph that started
the lineage. Walking `parent` recovers the whole path; storing only the final wording loses
it. Anything else fed to the generator belongs here too, for the same reason: two artifacts
from the same prompt that differ because different reference material was supplied are
otherwise unexplainable.

**One `t` means one shape, across every file.** A metadata file describes what an artifact
*declares*; a Set file records what a value *is*. Those are different records, so they get
different names — `param_decl` and `capacity_decl` here, `param` and `capacity` there. The
alternative, reusing a `t` for two different shapes in two different files, cannot be read
by a decoder that dispatches on `t` alone, which every ndjson reader does. It is not enough
for the two vocabularies to be disjoint in practice; they have to be disjoint by name.

#### On `perf`

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

### Attribute derivation — M3

When a consumer requires an attribute the producer does not emit, a rule could let the
compiler synthesize it instead of rejecting the Set — see [emit / consumes](#emit--consumes).
v0.2 does not do this; an unmet `consumes` is an unconditional error, and the check pass
says so by name for the two attributes below rather than leaving a model to wonder whether
the rejection contradicts something the spec promised elsewhere.

| Attribute | Derived from | Rule | What it needs |
|---|---|---|---|
| `velocity` | `position` | `(position - prev_position) / dt` | The position from **two** frames ago. Double buffering only ever retains one previous frame, so this is a third buffer, not a reinterpretation of the two that already exist. |
| `age` | spawn time | accumulated `dt` since spawn | A stored spawn time. Nothing records when an element was spawned today — `t` at spawn would have to become new per-element state, paid for whether or not any consumer ever asks for `age`. |

Both rules cost per-element storage that every procedure emitting the source attribute would
carry regardless of whether anything downstream ever reads the derived one — the same
always-on-cost shape the buffer-padding rule elsewhere in this document already normalizes
rather than special-cases. That cost is worth taking on once, when Geometry's attribute-subset
compatibility model gains general adapters (the slot interface contracts this milestone also
adds), not as a one-off bolted onto v0.2's binary `emit`/`consumes` check — which is why this
waits for M3 rather than landing sooner as an isolated feature.

Once implemented, a derived attribute belongs in the compiled metadata as its own record —
`{"t":"derived","attr":"age","from":"spawn_time"}` — so the UI can show it was inferred
rather than authored, the same distinction [Metadata file format](#metadata-file-format)
already draws between what an artifact declares and what a Set records.

### Multiple L1 sources, and `source` — M3

Merging several geometry sources into one Set raises the question the removal of `id` left
open: how two sources avoid colliding identities. `seed` is a monotone ordinal from a
counter that resets at Set start, so two sources either share it — and the second one's
`seed` no longer starts at zero, which breaks every structured layout — or hold one each
and collide immediately.

The resolution keeps `seed` zero-based per source and adds a fourth implicit attribute:

| Name | Type | Notes |
|---|---|---|
| `source` | `uint` | which L1 source produced this element. Implicit, carried, never declared |

- Each source counts its own `seed` from zero, so `seed % side` and every other structured
  layout works identically in every source.
- The hash builtins' salt becomes **per source** rather than per layer, so `hash1(seed)`
  differs between sources automatically while `seed % 512u` stays the same in both. Two
  grids of identical shape in different colours is then the default, not something to
  arrange.
- `source` is what downstream layers mask on. It is an ordinary attribute for that purpose.

**Downstream treats sources differently by writing attributes, not by branching on
`source` in L4.** An L2 modulator masked to `source == 0` writes `tint`, and L4 renders
`tint` without learning that there was more than one source — so an L4 written against one
source works unchanged against five. An L4 that branches on `source` is coupled to a
particular Set's composition and stops being reusable.

### Combining two sources — M3

Three things get called "mixing two sources" and only one of them needs anything new.

| | What it is | What it needs |
|---|---|---|
| Crossfade | draw both, blend opacity | Nothing here. Two Sets and the L5 mixer |
| Dissolve | hide one source's elements progressively | Nothing here. An L2 mask on `source` writing `size` or `tint` |
| Interpolation | pair elements and blend their attributes | **A cross-source read**, which the IR does not have |

Interpolation is the only real addition, and it is not a count change — it is an
element-wise operation that needs to read *another source's* element at the corresponding
index. `seed` provides the correspondence: element 5 of source A pairs with element 5 of
source B, which is exactly what per-source zero-based seeds buy.

It fights compaction, though: two sources kill independently, so after compaction the
paired elements sit at different slot indices and the correspondence needs a seed-to-slot
lookup. **Restricted to sources that are static — no `spawn`, no `kill()`, therefore no
compaction, therefore `seed` is the slot index — it is a direct indexed read and costs
nothing.** That restriction is statically checkable and covers lattices and shells, which
is most of what anyone wants to interpolate. Dynamic sources can wait for a correspondence
structure.

### L2 amplification — M3

`L2 : Geometry -> Geometry` is an endomorphism, which is what makes L2 freely stackable and
also what makes kaleidoscopes, instancing, trails, and subdivision inexpressible: **nothing
in the layer model can change the element count.**

The gap is filled by a second kind of L2 rather than a new layer number. Both take geometry
and return geometry, so they occupy the same slot position; what differs is the count mode,
which `Geometry` already declares.

```
kind    L2
amplify 8
```

- The factor is a compile-time constant, the same rule as loop bounds and `capacity`, so
  the output buffer can be sized and the cost multiplied out statically.
- Amplifying stages are **not** freely stackable: counts multiply rather than compose, so
  each one's factor enters the estimate as a product.
- Copies need distinguishing — eight mirror images are at eight positions — so a copy index
  is readable inside an amplifying block, and identity downstream is the pair of the parent
  `seed` and that index.

Amplification is cheaper to build than its position suggests. **Its output is derived and
recomputed every frame**, so unlike an L1 buffer it needs neither double buffering nor
compaction: nothing reads its previous value and its liveness is decided upstream.

This is where the primitive-centric bet pays out on the geometry side, for the same reason
the spec already gives for drawing one point cloud several ways: one simulated element
producing eight mirrored copies costs one simulation and eight draws, not eight simulations.

---

## Resolved

- **Measured signals in the record stream.** An `audio` record per frame and a `tempo`
  record per correction, on `tick`'s terms: derived live, read back verbatim on replay,
  and the analyser never runs twice on one session. Recording the *correction* rather than
  the estimate is what lets the analyser improve without changing how an old session
  replays. See
  [Measurement in the stream](#measurement-in-the-stream--audio-and-tempo).
- **A measured signal versus an invented one.** Same names, same `sample` call, different
  confidence — nothing anywhere asks whether a device exists. A frame with no `audio`
  record is not a frame of silence: it is a frame with no provider, and every name answers
  what it answered before audio existed. See
  [What a binding does](#what-a-binding-does).
- **Noise rate under tempo correction.** Stays cycles per beat, follows a tempo
  correction, ignores a phase one. The alternatives made two kinds of noise and left every
  existing `bind` record ambiguous. See [Binding noise](#binding-noise).
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
