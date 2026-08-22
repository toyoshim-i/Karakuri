# Karakuri IR Specification (draft v0.2)

Extension `.kir`. Plain text. Written by LLMs, validated by the compiler, lowered to WGSL.

One `.kir` file is one procedure. A Set is a combination of procedures: one L1 that
simulates, and one or more L4s drawn over it in order. Combination works because of the
`emit` / `consumes` contract described below.

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
kind L2        // geometry modulation
kind L3        // the camera
kind L4        // rendering
kind Field     // a signed distance at a point
```

All five are built. `L5` is **not** among them and should not be: a `kind` says what a
procedure *lowers to*, and an L5 has no code to lower since the compositing is fixed — so
there is no `kind L5` file, and `crate::node::Merge` is the node. See
[L5](#l5--built) and `docs/roadmap.md`.

**`Field` is the mirror of that argument**, which is why it is a kind and has no node: it
has *only* code to lower. What it lowers to is a WGSL function spliced into whichever
procedures evaluate it, so it needs no buffer, no pass and no position in the chain — see
[The `field` block](#the-field-block).

Each brings its own header rules, and a declaration belonging to another layer is refused
where it is written rather than ignored: `capacity` and `topology` are L1's, `amplify` is
L2's, `blend` is L4's, `emit` belongs to procedures that write elements. An L3 declares none
of them — it produces a viewpoint, and what is drawn with it is the renderer's business.

`uses` is the one declaration whose rules are about its **type** rather than about the
layer. `uses far : Geometry` is L2's alone and there is at most one, because a second bound
element buffer per node is not built; `uses shape : Field` is legal on L1, L2, L3 and L4 —
the four kinds that can evaluate a field — and several are legal, because a marcher wanting
a shape and a cutter is the ordinary case; `uses view : Camera` is L4's alone and there is
at most one, because a renderer draws one picture and a picture is seen from one place.
`amplify` and a *geometry* slot are refused together on one procedure: one node cannot both
take a second geometry and return several copies of each element. A Field slot and a Camera
slot change no count and are refused beside neither.

**Each kind also has a declaration and a block it cannot do without**, and the refusal is by
name at the header rather than by a missing symbol later:

| Kind | Must declare | Must have |
|---|---|---|
| L1 | `capacity`, `topology` | `element` |
| L2 | — | `deform` |
| L3 | — | `camera` |
| L4 | `blend` | `fragment` |
| Field | — | `field` |

A block declared twice is refused, on the terms every duplicate is: two `element` blocks are
two answers to one question and picking either is picking silently.

### capacity / topology (L1 only)

```
topology points          // or: topology lines
capacity [65536, 1048576] = 262144
```

`topology` says **what the geometry is meant to read as**. `points` means one sprite per
element; `lines` means one *segment* per element, drawn from the paired L4's `clip` to its
`clip_b`. See [L4 blocks and outputs](#l4-blocks-and-outputs) for how a renderer says which
it draws.

**It constrains no renderer, and that is deliberate.** A segment gets both of its ends from
attributes the L4 consumes, so a line renderer needs nothing from the geometry that the
`consumes ⊆ emit` check does not already cover — which means one L1 file can be paired with
a sprite renderer and a stroke renderer alike. `examples/drift_shell.kir` is paired with
both `soft_points.kir` and `drift_streaks.kir` for exactly that reason. Requiring the two
declarations to agree would have invented a dependency the lowering does not have, and would
have made "the same cloud, drawn two ways" cost two L1 *files*.

It does still cost two *simulations*: a Set owns its element buffers, so the same L1 in two
slots is stepped twice. Sharing one simulation between several renderers is "multiple L4
renderers over shared geometry" in `docs/roadmap.md`, and it is not built.

What it *is* for: it is the geometry's own statement of intent, which is what a reader
picking a pairing goes on, and what a default renderer per topology will select on when one
exists.

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

### uses

```
kind L2
uses far : Geometry

kind L4
uses shape : Field

kind L4
uses view : Camera

kind L2
uses only : Source
```

**One named input a procedure takes, with a type that decides every rule about it.** The
type is not decoration and never was: which kinds may declare one, how many are legal, what
an `edge` may bind it to and how it is *read* all differ between the four, and each of
those refusals is a sentence about a type rather than about `uses`.

| | `Geometry` | `Field` | `Camera` | `Source` |
|---|---|---|---|---|
| Legal on | L2 | L1, L2, L3, L4 | L4 | L1, L2, L4 |
| How many | one | any number | one | any number |
| Bound to | an L1 | a `kind Field` procedure | an L3, or the built-in camera | an L1 |
| Read as | `far.position` — `expr . ident` | `shape(p)` — a call | `view.clip` — `expr . ident` | `only` — a value |
| What it binds | an element buffer | a spliced function body | a bind group | a `u32` in a uniform |

All four are bound by an [`edge`](#set-file-format) and an unbound one is refused. What
follows is the geometry slot; the Field slot is under [The `field` block](#the-field-block),
the Camera slot under [Several cameras](#several-cameras) and the Source slot under
[`weight` and the `mask` block](#weight-and-the-mask-block-l2).

**A Camera slot is the one whose members are resolved against its type.** `far.position` is
read against the attribute table, because a geometry's parts *are* attributes; a camera's
are not parts of anything else, so the checker asks the slot's declared type what members
it has. That is the only cost the third type had, and it is the reason the type is written
in the header rather than inferred from what an `edge` binds.

**`Geometry` and `Source` take the same kind of node and are not the same slot**, and the
last row is the whole of the difference. A `Geometry` slot binds an element *buffer* — a
bind-group entry on every node that has one, which is why there is at most one: a second
bound buffer is not built. A `Source` slot binds the `u32` that says which geometry a chain
instance is running over, in a uniform block the module already has. A mask wants the
identity and reads no elements, so writing it `: Geometry` would allocate a buffer nothing
reads *and* collide with that cap on a node which already declares a `far`. Which is also
why several `Source` slots are legal and cost nothing to have: `source == a || source == b`
is an ordinary thing to want.

#### A geometry slot (L2 only)

**An L2 that takes a second geometry and produces one.** `L2 : Geometry -> Geometry` is an
endomorphism, and this breaks it in the second of the two possible directions: `amplify`
breaks it on the *count* axis, one element in and several out, and this breaks it on the
*arity* axis. Both stay in the same slot position and both are header declarations, because
what a `deform` writes is decided before it runs.

**The name is the procedure's own**, exactly as `consumes position` names an attribute
without naming which L1 supplies it. That is the load-bearing rule and the reason the
declaration exists at all: **a `.kir` may not name a node**, because naming one couples the
procedure to one Set and it stops being a library part. So the file says what it takes and
the Set says what fills it — an [`edge`](#set-file-format) record, written from the command
line as `--edge <node>.<slot>=<geometry>`.

The far element's attributes are read through that name:

```
deform {
  position = mix(position, far.position, vec3(k, k, k));
}
```

- **`far.position` is not a swizzle**, and it reaches the checker because it is *shaped*
  like one — `expr . ident` is the grammar. Deciding it there costs no new syntactic
  category, which is the whole reason the read is spelled this way: `uses` adds one header
  declaration and one name, and nothing else in the language moves.
- **A declaration of its own, rather than a second meaning for `consumes`.** The two say
  different kinds of thing: `consumes position` names an *attribute*, and a node's inputs are
  a set of them with no structure; `uses far : Geometry` names a *node input*, one slot with a
  type and a binding in the Set. Spelling both with one word would make `consumes` a list of
  two unrelated things — an author reading a header could not tell which a name was without
  knowing every attribute the language has — and it would give one declaration two arities,
  since an attribute is not bound by an `edge` and a slot is. The header gains one keyword and
  every existing one keeps meaning exactly what it meant.
- **One `consumes` covers both sides.** The second geometry is an input edge and a node
  reads the same attribute from each, so an attribute this node does not take is not
  readable on either side.
- **The name is ordinary and shares one scope.** A local, a param or a slot of the same name
  is refused, on the terms every other collision here is; and a name nothing declared is
  simply a name that resolves to nothing. Nothing is reserved language-wide — that was what
  the previous spelling did, and reserving one word is exactly what caps a procedure at one
  input. `other` was such a word for the paired geometry and `field` was one for the field;
  neither is reserved now, and neither layer is capped at one any more — see [Evaluating
  one](#evaluating-one) for the second half of the same argument.
- **A geometry is not a value.** `far` alone is refused: the language has no type for a
  whole source and no way to pass one, so what can be said about it is what one of its
  elements holds. It is not assignable either — the far side is an input edge.

  **A `Source` slot *is* read as a value, and that does not contradict the sentence above.**
  The sentence is about `Geometry`, and it stays true: `far` is a whole source and there is
  no type for one. `only` is not a source — it is the `uint` that *identifies* one, which is
  a type the language does have. The two are as far apart as an element buffer and an `id`.
- **The type is written and it is what the rules are about.** Writing it is what let the
  Field slot arrive without the declaration changing shape — and what kept every refusal
  about a geometry slot a sentence that grew an arm beside itself rather than being rewritten
  around a distinction it had never drawn. Anything but `Geometry` or `Field` is refused by
  name.
- Readable in a `deform` and a `mask`, and nowhere else.
- **One geometry slot per node.** A second is refused with a sentence: a second bound buffer
  per node and a second edge per Set are not built, and one name silently winning would put
  back exactly the failure this notation removes. This is a rule about *geometry* — a Field
  slot is not counted by it, and several are legal.

**Which geometry fills the slot is the Set's answer, not the file's, and it is written
down.** It used to be `--set` position 1 — written nowhere, so reordering the command line
silently changed the picture. It is an `edge` now, at both ends by name, and **an unbound
slot is refused**. Not "if there is exactly one, use it": that implicit rule is the thing
being removed, and reintroducing it under a new spelling would cap the next fan-in at one
the same way.

**Five things are refused where such a Set is built**, because none of them is a property
of the file: exactly two sources, the node with the slot *first* in the chain (so its second
input is a source rather than whatever reached its position), both sources static, both the
same size, and the far source able to supply whatever the chain derives — a chain that
consumes `velocity` needs it on both sides, and the far source's element struct is built
with the same derived slots as the near one or the far read comes from the middle of the
element before it.

The near geometry — the one the chain runs over and the renderers draw — is whichever source
no edge bound. Not position 0: an edge exists so that the list's order decides nothing, and
a near side still read off a position would keep half of the rule this replaced.

**The correspondence is the element's slot index**, which is the same element in both sources
only while neither compacts. So both sources must be *static*: no `spawn` block and no
`kill()`, which `Checked::is_static` answers and which is why that predicate had to start
asking about both.

### amplify (L2 only)

```
amplify 6
```

**The one declaration in the language that changes an element count.** Without it an L2 is
an endomorphism and returns as many elements as it was given; with it, every element
reaching the node becomes `factor` of them. See
[L2 amplification](#l2-amplification--built) for what that buys and what it costs.

- **A compile-time constant**, the same rule loop bounds and `capacity` follow, because the
  output buffer is sized from it and the cost is multiplied out by it. There is no range and
  no Set override: how much material to make is the operator's question and is `capacity`;
  how many copies a kaleidoscope has is the procedure's own, and putting it on a fader would
  resize a buffer mid-performance.
- **At least 2, and at most 1024.** Zero would make the layer decide liveness, which is
  settled entirely by the compaction that runs once after L1. One is the endomorphism an L2
  already is, and would buy a second buffer holding a copy of the first. The ceiling is on
  the single declaration and keeps the *product* inside a `u32`; it is not what keeps a
  buffer inside the device.

**What is too large is decided where the Set is built, not here.** The buffer a node needs is
the Set's `capacity`, times every factor above the node, times the element stride — and a
`.kir` file knows none of the three. So `amplify 256` is a legal declaration that a Set at a
large capacity is refused for, by name, with the number and the device's limit in the
message. It is device-dependent by nature: what is too large is a property of where it is
being asked to run. The alternative is not a portable answer, it is the same refusal with no
sentence attached — wgpu answers an over-limit binding by panicking the thread that made it,
which at startup takes the process down.

### param

```
param <name> : <type> [<min>, <max>] = <default>
```

- Types: `float`, `vec2`, `vec3`
- The range is mandatory. It is the UI fader range, the agent's search range, the
  normalization basis for signal binding, and the basis for incremental revision: an
  instruction like "a little slower" can only be translated into a value by something that
  knows what the range is. Without one, every revision falls back to regenerating code.
- **Every name the language already gives meaning to is reserved, and how widely depends on
  the name.** Reserved in every layer: `id`, every attribute (`position`, `size`,
  `tint`, …) and every ambient (`seed`, `copy`, `point`, `t`, `beats`, `dt`, `capacity`,
  `camera`, `point_coord`, `eye`, `ray`). Reserved in the layer that *writes* it: a stage
  output — `clip`, `clip_b`, `point_size` and `color` in an L4, `target`, `up`, `fov_y`,
  `near` and `far` in an L3, `strength` in an L2's `mask`, `distance` in a `field`. So
  `param color : vec3` is accepted on an L1, which declares no `color` output, and refused
  on the L4 that would have to write one — the collision is with the *lowering's* name for
  the output, and an L1 has no such name.
  A **declared slot** of either type joins that scope where a procedure declares one — `uses
  far : Geometry` makes `far` this procedure's and `uses shape : Field` makes `shape` its
  own, so a local or a param of either name is refused. Neither is reserved anywhere else,
  and that is the point: the name is the procedure's own, so reserving it language-wide would
  be reserving one word for one input and capping every procedure at that one. Both spellings
  this replaced — `other` for the paired geometry, `field` for the field — were exactly such
  words.

  A **Field** slot has one collision a geometry slot does not: it is *called*, so a slot
  named after a builtin or a type constructor is refused as well. `sin` is a fine name for a
  param, a local and a geometry slot, and not for a field — `sin(x)` would otherwise mean two
  things at one call site, and either resolution order is a spelling that quietly means
  something other than it says.

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
error but the normal shape of an L4 file.

**Two attributes are satisfied by a rule where nothing emits them**, and every other one is
a rejection: `age` and `velocity` are synthesised by the engine — see
[Attribute derivation](#attribute-derivation--built). The check is therefore
`consumes ⊆ available`, where *available* is what is emitted at this position plus what can
be derived, and the Set is the only place that can evaluate it because it is the only place
holding every procedure at once.

**The cost is conditional, which is what let the rule land at all.** Each rule reads one
engine-written slot, and the slot exists only where something actually consumes the
attribute it feeds. A Set nobody asks `age` in pays nothing for `age`.

Within one procedure the useful check is narrower: an attribute is readable or writable
only if that procedure declares it, since only a declared attribute gets a buffer pair.

**Which layers may declare which.** `emit` belongs to the layers that write elements — L1
and L2 — and is refused on an L3, an L4 and a field, each with its own reason: a camera and
a field have no element, and a renderer draws what reaches it and stores nothing.
`consumes` is refused on an L3 and a field for the same reason and is accepted everywhere
an element is read.

Available attributes:

| Name | Type | Notes |
|---|---|---|
| `position` | `vec3` | |
| `normal` | `vec3` | |
| `uv` | `vec2` | |
| `seed` | `uint` | spawn ordinal. Implicit — never declared, always readable. See [Element identity](#element-identity) |
| `age` | `float` | seconds since spawn. Derived where nothing emits it |
| `velocity` | `vec3` | derived where nothing emits it, from the change in `position` over a step |
| `size` | `float` | |
| `tint` | `vec3` | linear RGB |

### blend (L4 only)

```
blend additive
```

```
blend weighted
```

Two legal values. `additive` was the only one in v0.2, and the declaration existed before
there was a second so that adding one would be a format addition rather than a format
change — the same move as defining `VideoSource` before there is a second implementation of
it. That is what it turned out to be.

Additive needs no sorting, which is why it is where v0.2 started: `capacity` elements cannot
be depth-sorted per frame at this scale, even sorting indices alone. The successor is not
depth sorting but **weighted blended OIT** — order independent, two targets (accumulation
and revealage) that the existing `Rgba16Float` pipeline accommodates naturally, and an
approximation whose coarseness does not show on soft sprites. Being order independent, it
does not interact with compaction at all.

**The two modes read `color`'s alpha differently, and that is the part an author has to
know.** Under `additive`, alpha is emission strength: it scales what a fragment adds, and
the spec's "values above 1.0 are expected" applies to it as much as to the colour. Under
`weighted`, alpha is **opacity**, and opacity above 1.0 is not a thing — the revealage a
weighted pass accumulates is `prod(1 - a)`, which stops meaning "what is still visible
behind this" the moment a term goes negative. The generated shader clamps it to `[0, 1]`,
so a fragment block that writes 1.5 gets 1.0 rather than a picture with negative light in
it.

Blend mode is part of an artifact's identity: a procedure writes its `color` and alpha
knowing how they will be combined.

**The equivalent gap on the topology side closed first**, and how it closed is what settled
the shape of this declaration. Quad expansion used to be justified by
`topology points` while `topology` was declared on the **L1** header, so an L4 had no way to
say what it was written for. The fix was not a `topology` declaration on the L4 header: an
L4 that assigns `clip_b` is drawing a segment and there is nothing else it could be doing,
so the check pass *infers* it and there is exactly one place the fact can be stated.

`blend` is not the same shape. Nothing an L4 writes could imply `weighted` rather than
`additive` — the two differ in how the results of identical assignments are combined, not in
what is assigned — so `blend` stays a declaration.

---

## Ambient values and the signal rule

The only implicit values readable inside a block:

| Name | Type | Meaning | Available in |
|---|---|---|---|
| `capacity` | `uint` | allocated element count, set per Set | L1 |
| `t` | `float` | simulation seconds since Set start | all |
| `beats` | `float` | musical position on the session's tempo grid, at the instant `t` names | all |
| `dt` | `float` | fixed simulation step. Scaled on an element's first update — see [Spawn timing](#spawn-timing) | L1, L3 |
| `copy` | `uint` | which copy of an amplified element this is, `0` where nothing upstream amplified — see [`amplify`](#amplify-l2-only) | L2, L4 |
| `source` | `uint` | **which geometry this chain instance is running over**, as the assigned value that identifies it. A per-source uniform, never declared — see [Multiple L1 sources](#multiple-l1-sources--built-and-source--built) | L1, L2, L4 |
| `point` | `vec3` | the position a field is being asked about. The one input a field has | `field` block |
| `camera` | `mat4` | view-projection matrix, of **the Set's camera** — the node at `L3:0`. A renderer that draws from another one declares a slot and reads `view.clip`; see [Several cameras](#several-cameras) | L4 |
| `point_coord` | `vec2` | 0..1 across the primitive: within the sprite under `points`, along-by-across the stroke under `lines`, across the frame under `fullscreen` | L4 fragment |
| `eye` | `vec3` | the Set's camera's world-space position, or `view.eye` through a slot. `fullscreen` only | L4 fragment |
| `ray` | `vec3` | unit direction from `eye` through this fragment, or `view.ray` through a slot. `fullscreen` only | L4 fragment |

`seed` is readable as well, but it is a carried attribute rather than an ambient value —
see [Element identity](#element-identity). It is available **wherever there is an element**,
which is every block except three: a `camera` block, which runs once a frame over nothing;
a fullscreen L4, which covers the frame and draws no element; and a `field` block, which is
handed a `point` and is spliced into callers that may have no element at all. Each of those
three is a refusal with its own sentence, and each was added after a `.kir` that read `seed`
there compiled to WGSL naming a value nothing declared.

**`source` is refused in two of those three and readable in the third**, which is the whole
of what separates the two values. It is refused in a `camera` block and in a `field` block
for `seed`'s exact reasons — an L3 runs once a frame over nothing and the identity is a
property of a Set's *geometry*, which a camera is not; a field is spliced into callers that
may be running over different geometries, so a distance that varied with the source would be
two shapes in one frame out of one body. But it is readable in a **fullscreen L4**, where
`seed` is not: `seed` is per element and a fullscreen procedure has none, while `source` is
per chain *instance*, and a procedure with no element still knows whose chain it is running
in.

**And it is refused in a procedure that declares a `Geometry` slot**, which is a rule about
the pair rather than about either half. A pairing Set is one source made of two simulations —
the far one feeds the slot and shares the near one's uniform — so there is one salt for two
geometries and `source` would answer for the near one without saying so. Refused with the
slot's name in the sentence, because a hint that says which reading was ambiguous is better
than a rule an author has to infer from the pairing rules three sections away.

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
it is built — see the `transport` record. Nor does reading `beats` disqualify a procedure from being closed form: the
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

A Set with no L3 in it is watched from a built-in orbit, and **that is what a procedure should
be written against** — an L1 cannot know which camera it will be paired with, and the orbit is
the one it gets by default:

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

**Substepping landed, and so did the record.** `steps` is no longer always 1: it is derived
per frame from real time, capped at 4, and written as the `tick` that closes each frame in a
recorded session — `--replay` then reads the number back rather than deriving it again,
which is the whole of why a replay is frame-exact on a machine that runs at a different
speed. The determinism argument above is what it rests on: the engine advances by a **step
count** and never by a duration.

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
for i in <start>..<end> { ... }   // both are signed integer literals
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
- Loop bounds are signed integer literals, not expressions. Constant bounds are what makes
  cost estimation possible, and a literal is the only form the check pass need not reason
  about. `for i in -3..3` is well formed; a range that does not ascend — `for i in 4..0` —
  is accepted and runs zero times, which is what the lowering does with it and what the
  cost estimate charges for it.
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
spawn   { ... }      // L1: initialize a newly allocated element
element { ... }      // L1: update a live element; may call kill()
deform  { ... }      // L2: rewrite a live element's attributes; may not kill()
mask    { ... }      // L2: where the deformation applies. Optional
camera  { ... }      // L3: produce this frame's camera
vertex  { ... }      // L4: once per element
fragment{ ... }      // L4: once per rasterised fragment
field   { ... }      // Field: the signed distance at `point`
```

**A block name belongs to exactly one layer**, and a block in the wrong kind of procedure is
refused. That is what each new name buys — `deform` rather than a second meaning for
`element`, `camera` rather than a mode of `vertex` — and it is why the list above is a
partition rather than a menu.

`spawn` requires a spawn rate. Declare it as a parameter named `spawn_rate`
(elements per second); the engine reads it specially:

```
param spawn_rate : float [0.0, 40000.0] = 8000.0
```

It must be a `float`, and a `spawn` block with no `spawn_rate` is refused — a rate the
engine reads by name has to be there and has to be one number.

If a procedure declares no `spawn` block, all `capacity` elements are live from frame zero
and `element` alone drives them. This is the simplest form and a good default. Emitted
attributes are zero-initialized before the first frame — the frame-zero `element` pass
reads zeros, not garbage — and `seed` equals the element's initial slot index.

### `weight` and the `mask` block (L2)

```
proc twist {
  kind L2

  // Read by the lowering, on the same terms `spawn_rate` is read by the
  // engine. Optional; absent is 1.0, and it must be a `float`.
  param weight : float [0.0, 1.0] = 0.6

  consumes position, age

  // Optional. `strength` is required inside it, and is in [0, 1].
  mask {
    strength = smoothstep(0.2, 0.6, age);
  }

  deform {
    position = position * 1.5;
  }
}
```

A modulator can apply **partially**, and the two ways it can are deliberately
separate mechanisms that end at one number:

```
dst = mix(what reached this node, what `deform` wrote, clamp(weight * strength, 0, 1))
```

**`weight` is a declared `param` and `mask` is a block, because their audiences
differ.** A `param` goes on a fader, takes a signal binding, moves under a
transition, is published by a Set's interface and is saved in a Set file;
`strength` is an expression over the attributes reaching this node, and decides
*where*. Folding the operator's control into the expression would take every one
of those surfaces away from it, and would make the common case — a weight and no
mask — require writing a block.

**`strength` is a scalar rather than a predicate**, which is what makes it the
same word the mixer already uses: a mask there is a fader varying across the
*frame*, and this is one varying across the *material*. A predicate is
expressible — `step(0.7, age)` — and a soft boundary, which any spatial mask
wants, is not expressible the other way round.

**It is clamped**, because `mix` extrapolates: a `strength` of 2 would apply the
deformation twice over and one of -1 would apply its inverse. Both are a wrong
picture rather than a missing one.

**A mask reads `consumes` and never `emit`**, unlike the `deform` beside it. It
runs *before* the body — a mask evaluated on the deformed element would be
deciding where to apply a deformation from a position that deformation had
already moved — and at that point an emitted attribute holds the zero the
pass-through left rather than a value. It writes nothing but `strength`.

An L2 with neither is what every L2 was before they existed, and lowers to the
identical shader.

#### A mask that names the source it applies to

**A Set instantiates its chain per geometry, so one `.kir` is already running
in several places — and until now it could not tell them apart.** `source` is
the identity of the instance this invocation is in; a `Source` slot is the
identity of a geometry the *Set* named. Comparing them is a mask that applies
to one source and leaves the others standing:

```
proc dissolve {
  kind L2

  uses only : Source

  consumes position, age

  mask {
    strength = 0.0;
    if source == only { strength = 1.0; }
  }

  deform { size = size * 0.2; }
}
```

and the Set says which geometry fills it — `--edge dissolve.only=lattice`, or
an [`edge`](#set-file-format) record.

**The comparand arrives through a slot because a `.kir` may not name a node.**
That is the same rule the geometry, the field and the camera all follow, and
for the same reason: a file that named `lattice` would be coupled to one Set
and stop being a library part. What is different is what the slot *binds* — a
`u32` rather than a buffer, a spliced body or a bind group — which is why
[several are legal](#uses) and why a mask that wants two sources writes `source
== a || source == b` rather than being told to split into two nodes.

**Not only a mask, and not only an L2.** The slot and the ambient are legal
wherever a chain instance runs — L1, L2 and L4 — so a generator that lays out
one geometry differently from another, or a renderer that tints one of them, is
the same sentence in a different block. The mask is where it was asked for
first, not where it is confined.

**What it is *not* is a build-time selection**, and the difference is worth
stating because the shader says otherwise. `source == only` compares two
uniforms: the same value in every lane, the cheapest branch a GPU has — and
also a constant for the whole instance, which makes this an on/off for the
instance rather than a mask across the material. The honest optimisation is not
to instantiate the node in the chains it excludes at all. That is recorded in
`docs/roadmap.md` with the trigger for doing it; it is an optimisation and not a
change of meaning.

### The `camera` block (L3)

```
proc sweep {
  kind L3

  param radius : float [1.0, 40.0] = 8.0
  param speed  : float [0.0, 2.0]  = 0.15

  camera {
    let a  = t * speed * 6.2831853;
    eye    = vec3(cos(a) * radius, 2.0, sin(a) * radius);
    target = vec3(0.0, 0.0, 0.0);
  }
}
```

Six outputs, of which **two are required**:

| Output | Type | Default |
|---|---|---|
| `eye` | `vec3` | — |
| `target` | `vec3` | — |
| `up` | `vec3` | `vec3(0.0, 1.0, 0.0)` |
| `fov_y` | `float` | `1.0471976` (a third of pi) |
| `near` | `float` | `0.1` |
| `far` | `float` | `100.0` |

Where the camera is and what it looks at are the whole of what makes one camera different
from another. The other four are written before the block runs — the same shape as an L2's
pass-through — so an author overrides what they mean to change and restates nothing. Their
values are the built-in orbit's, so replacing it with the simplest L3 anyone would write does
not move the field of view underneath the picture.

`target` is a **point**, not a direction, because pointing at a thing is what a camera here is
asked to do; a direction would make every "follow this" a subtraction and a normalize the
author had to remember. `near` and `far` are not decorative — see the note under
[L3 — the camera](#l3--the-camera).

**It reads the clock, `dt`, and its params, and nothing else.** No `seed`, because that is a
per-element value and an L3 runs once a frame over no element. No attributes, and `consumes`
is refused rather than ignored: an L3 *will* read geometry, and what it will read is a
reduction or element zero — see [What an L3 can point at](#what-an-l3-can-point-at-a-reduction-or-element-zero) —
which is addressing this language does not have yet. `dt` is there although nothing can use it
yet, because an L3 is allowed to hold state and smoothing is written against a step.

`eye` is also readable, as an ambient, in a marching L4's `fragment` block. One concept,
written here and read there — the same relationship `position` has between an L1 and an L4.

### Several cameras

**A Set holds as many cameras as its files declare, and each renderer says which one it
reads.** This was the last of the three inputs capped at one, and the cap was the same one
the geometry and the field had: there was one, so it needed no name, so there could only be
one. A renderer names it now:

```
proc lens {
  kind  L4
  blend additive

  uses view : Camera

  consumes position

  vertex {
    clip       = view.clip * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment { color = vec4(1.0, 1.0, 1.0, 1.0); }
}
```

and the Set says which camera fills it — `--edge lens.view=east`, or an
[`edge`](#set-file-format) record.

**Three members, and they are the three ambients under another name:**

| Member | Type | The same value as | Readable in |
|---|---|---|---|
| `view.clip` | `mat4` | `camera` | any L4 stage |
| `view.eye` | `vec3` | `eye` | a fullscreen L4's `fragment` |
| `view.ray` | `vec3` | `ray` | a fullscreen L4's `fragment` |

The stage rules are the ambients' own rather than a second set beside them: `eye` and `ray`
are built by the ray prologue a fullscreen fragment stage opens with, so reaching them
through a slot is not a way round the rule that a per-element renderer has no ray.

**The three ambients stay, and they mean the Set's camera** — the node at `L3:0`. That is
what every renderer written before this notation reads, and it keeps meaning exactly what it
meant: a Set held one camera, so "the Set's" and "the one there is" were the same sentence.
A renderer that declares a slot is saying *which*, which is the question a Set of several has
no other way to be asked.

**The built-in orbit is a node.** A Set whose files declare no `kind L3` holds one camera all
the same, produced from the `camera` record's six numbers, and it is called `orbit` so that an
edge can name it: `--edge lens.view=orbit`. It was a field on the Set before, which is why a
renderer could draw from it only by saying nothing — one fact with two shapes, and the shape
without a name was the one that could not be pointed at. So `L3:0` addresses something in
every Set now, `node_names` has an entry for it, and the parameter map beside it is empty,
because the built-in is a node rather than a procedure.

**Fan-out needs no rule.** Two renderers naming one camera is the ordinary case — one
viewpoint drawn two ways — and what is refused is two edges into one *slot*, which is one
renderer with two answers about where it is looking from. A renderer may declare at most one
Camera slot: a frame drawn from two viewpoints is two renderers.

**What an L4 still cannot read is the other four values a camera writes** — `target`, `up`,
`fov_y`, `near` and `far`. Those are a new capability rather than a new spelling for an old
one, and they are deferred for the reason [L3 — the camera](#l3--the-camera) gives: an L3
produces state, not a matrix, and handing a renderer the state is a decision about what a
camera *is* to a renderer. See `docs/roadmap.md`.

### The `field` block

A `kind Field` procedure is a signed distance function: it is handed a position and returns
a distance, and that is the whole of it.

```
proc blob {
  kind Field

  param ball    : float [0.1, 2.0] = 0.8
  param blend_k : float [0.0, 1.5] = 0.55

  field {
    let sph = sd_sphere(point - vec3(0.9, 0.0, 0.0), ball);
    let bx  = sd_box(point + vec3(0.9, 0.0, 0.0), vec3(0.7, 0.7, 0.7));
    distance = op_smooth_union(sph, bx, blend_k);
  }
}
```

- **`point` is the one input**, in world space, and is readable in no other block. Every
  other block is handed an element or a fragment; this one is handed a position, which is
  what makes it a function of space rather than of the material in it.
- **`distance` is required and is the only output.** Signed rather than unsigned and a
  distance rather than a density, because that is what the whole [SDF](#sdf) table returns
  and what the CSG operators compose. A field returning anything else would have no
  operators.
- **No attribute is readable.** A field has no element — refused with that sentence rather
  than with advice to declare one, since declaring one is not available and would not help.
- **No geometry declaration is legal**: no `capacity`, `topology`, `blend`, `amplify`,
  `emit`, `consumes` or `uses … : Geometry`. It counts nothing, draws nothing and carries
  nothing — a field is handed `point` and returns a distance, so there are no elements here
  for a second geometry to be read beside.
- **And no `uses … : Field` either**, which is the one kind that may not take one. A field
  bound to itself is a function calling itself, which WGSL forbids outright; two fields
  naming each other is the same failure at one remove, and telling that apart from a legal
  chain of shapes is a walk over every edge in the Set. That is a graph question, answerable
  where the Set is built and nowhere in one file, so the whole thing is refused here rather
  than half-checked.

**Its cost is on an axis of its own.** A field is reported in **ops per evaluation**, and
the three other figures are zero for one. What it scales with is *how often its caller calls
it* — once per element in a `vertex`, forty-eight times in a march loop — which is a property
of the caller. Charging it per element would be a rate against a quantity a field does not
have, and would give it a ceiling that says nothing about what evaluating it costs anybody.
The number that has to fit under a ceiling is the caller's with this multiplied into it.

**Why `Field` is a `kind` and has no node.** `docs/roadmap.md` settled that a `kind` says
what a procedure *lowers to*, and that an L5 has no `kind` because it has no code to lower.
A field is the mirror: it has only code to lower. So it has a file and no node, needs no
buffer and no pass, and takes no position in the chain — which is also why it introduces no
new syntactic category. There is no user-defined function here; there is one more kind, one
more block, one more ambient and one more output, which is the shape L2 and L3 already
established.

**As many per Set as its files declare.** It used to be one, for the camera's reason —
several would need naming, and naming is fan-in — and the naming arrived first: a caller
declares [`uses shape : Field`](#uses) and a Set's `edge` says which field fills it. The
plumbing followed, and it was a `Vec` where the `Option`s were rather than a design: a Set
holds a list of fields, each is a node with a name for an edge to point at and an address of
its own (`--param Field:1:radius`), and a Set file writes one `slot` record per field.

**The camera followed, and the rule it was said to have turned out to be the same cap.**
"A Set is a grouping around one viewpoint" is what this document said the day the field's
cap came off, and it did not survive the notation: a renderer declares [`uses view :
Camera`](#uses) and an `edge` says which camera fills it, so a Set holds as many as its
files declare — see [Several cameras](#several-cameras).

#### Evaluating one

**A procedure declares the field it takes and calls it by that name.** The slot is the
procedure's own, exactly as a geometry slot's is, and which field fills it is the Set's
answer:

```
proc field_lens {
  kind L4
  uses shape : Field

  fragment {
    for i in 0..40 {
      let d = shape(p);
      if d < 0.005 { hit = 1.0; }
      p = p + ray * max(d, 0.005);
    }
  }
}
```

```
karakuri-cli --set drift_shell.kir,melt_blob.kir,field_lens.kir \
             --edge field_lens.shape=melt_blob
```

**The read site is a call**, because a field has no members and does have an argument — one
`vec3` in, a `float` out. That reuses `Expr::Call` the way `far.position` reuses `expr .
ident`: one existing shape given a second meaning, resolved against what the header declared
rather than against a word the language reserved. The header wins over the builtin table,
and a slot named after a builtin or a type constructor is refused where it is declared so
that nothing is hidden by that order.

**It used to be `field(p)`, and that word is gone rather than kept as an alias.** Keeping it
would mean "if there is exactly one field, use it", which is precisely the rule the slot
exists to remove — and [`uses`](#uses) says the same thing about the geometry slot's own
discarded reserved word, `other`. A file written against the old spelling is refused with a
sentence saying what to write instead. This is a breaking change to the language.

**A slot nothing binds is refused**, and so is one bound to a node that is not a field —
both where the Set is built, which is the first point holding the caller and the field at
once. Not "if there is exactly one, use it": that is the implicit rule being removed, and
reinstating it here would cap the next Set at one field with nothing in the language to say
so. The refusal names the slot and the flag that would bind it, which is what the one it
replaced could not do: a call carried no name, so all it could report was that some
procedure somewhere evaluated a field.

**The clock is passed in rather than read.** A field cannot know how its caller spells `t`:
an L1 reads it from its per-substep arguments and everything else from its uniform, and one
spliced body cannot say both. So the call site supplies the caller's own answer, which is the
spelling that caller would have written inline.

**A field's `param`s live in the uniform of every procedure that evaluates it**, under a
prefix of their own — so a renderer and the field it draws may both declare `exposure`, and
`--param Field:0:exposure` and `--param L4:0:exposure` reach different things. **The slot is
in the prefix as well**, so a procedure reaching two fields addresses each one's params
separately: one shape's `radius` is not the other's.

**One value per field, and the address is what picks the field.** `--param Field:0:radius`
and `--param Field:1:radius` are two numbers because they are two procedures; every caller
that reaches a field through a bound slot writes *that* field's value into its own uniform,
and reaches no other. The slot is what carries the answer from the edge to the uniform, which
is why it is in the key and not only in the WGSL spelling.

The prefix has to make the name unforgeable rather than unlikely. It is a separator no
`.kir` identifier can contain, because a caller declaring `param field_radius` beside a field
declaring `param radius` is otherwise one name with two meanings — and the engine looks a
uniform field up by that name.

**A field is spliced only into the procedures that evaluate it.** Otherwise a field's body
takes down shaders with nothing to do with it, and every node in the Set carries its params.

**A field cannot evaluate a field** — refused at the declaration, above.

**The two are costed together, at the Set.** A field call weighs nothing where a single file
is estimated, since what one evaluation costs lives in another file — so the ceiling each of
them passed was applied to a figure missing the other. A marcher evaluating a field forty
times pays for it forty times, and a pair over the ceiling is refused with both figures in
the message. Neither file need be over on its own.

**The counts are per slot**, and the slots on one axis are *added*. A procedure taking a
shape and a cutter pays for both against one fragment ceiling, so checking each on its own
would let a pair through that neither half is over with — the same failure this check exists
to close, one level up. The refusal names whichever slot contributes most, since that is the
one worth cutting first.

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
  order, so the live set is always a stable subsequence.

  Three things rest on that and only two were foreseen. The blend order is stable and
  reproduction is bit exact, which is why it was chosen; and **element zero is the oldest
  living element**, which an L3 camera can point at for the cost of one buffer read. Spawn
  order is age order, and a live element at index 0 has nothing alive before it, so it stays
  there until it is the one that dies. An L1 that expects to be looked at can make that
  element mean something — see [L2 and L3](#l2-and-l3--built)
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
| `clip` | `vec4` | vertex | clip-space position. A sprite's centre, or a segment's near end. Required *in a `vertex` block* |
| `clip_b` | `vec4` | vertex | a segment's far end, in clip space. **Optional — assigning it is what makes the procedure draw lines** |
| `point_size` | `float` | vertex | sprite size in pixels, or stroke width in pixels. Required *in a `vertex` block* |
| `color` | `vec4` | fragment | linear RGB, straight alpha. Required |

Each required output must be assigned on every path, under the same rule as emitted
attributes. **A `vertex` block is itself optional**: an L4 without one draws the frame rather
than an element, so `clip` and `point_size` are required of a block that exists rather than
of every L4. `color` is required unconditionally, because every renderer has a fragment
stage.

### How an L4 says what it draws

By assigning `clip_b`, or by not assigning it — and, for the whole frame, by
having no `vertex` block at all. There is no `topology` declaration on an L4
header and adding one is an error — a second endpoint is the only thing that could make a
procedure a line renderer, so a header field would be a second place for the same fact to
be stated and a first place for the file to contradict itself.

`clip_b` is the only optional output, and it is optional in a strict sense: assigning it on
*some* paths through `vertex` is rejected. A procedure either draws segments or it does not,
and one that wrote a far end under a condition would lay a stroke along an uninitialised
value on the other path.

Under `lines`:

- **the segment runs from `clip` to `clip_b`**, and the two ends belong to the same element.
  The IR has no notion of one element linking to another, deliberately: an index into the
  element buffer is invalidated by compaction, so the one shape that would express a shared
  vertex is also the one that spawning material breaks. A polyline is *n* segments, a trail
  is one segment per particle, and both survive spawning and killing because neither refers
  to anything outside itself. The cost is that a strip stores its interior points twice, and
  that the ends are **butt caps** — the quad stops exactly at each endpoint, so two wide
  segments meeting at an angle leave a wedge at the joint. Invisible on thin or densely
  sampled strokes, and the reason a heavy polyline wants its samples closer together rather
  than its width raised
- **`point_size` is the width of the stroke in pixels**, uniform along it — the same number
  and the same units a sprite's extent uses. A stroke that tapers takes its taper from the
  `fragment` block, or from consecutive elements carrying different widths
- **`point_coord.x` runs along the segment** — 0 at `clip`, 1 at `clip_b` — and
  **`point_coord.y` runs across it**, 0 at one edge and 1 at the other. So a soft edge is
  `abs(point_coord.y * 2 - 1)`, distance from the centre *line*, where the sprite equivalent
  is `length(point_coord * 2 - 1)`
- **a segment with either endpoint behind the eye is dropped, not clipped at the near
  plane**, and **a segment whose two ends coincide draws nothing** — zero length is zero
  area, where a sprite at zero velocity is still a sprite. See the [L4 lowering](#l4)
  section for both

### fullscreen

An L4 with **no `vertex` block** covers the frame once and draws no geometry. A procedure
with no per-element position has nothing for a vertex block to do, so its absence is the
declaration, on the same principle `clip_b` settles.

Two rules come with it:

- **`consumes` must be empty.** There is nowhere to read an element from. It is stated as a
  rule rather than left as a consequence because it is what makes the next line provable:
  **the paired L1's simulation is skipped entirely.** Nothing reads those buffers, so
  nothing steps them — `t` still advances, because a marcher reads it.
- **`eye` and `ray` are readable, and only here.** The camera's position in world space, and
  the unit direction from it through this fragment. Given rather than derived: the camera is
  a built-in, so the projection convention is the engine's, and a procedure that rebuilt it
  would be restating that convention in every shader that marches — where getting it
  slightly wrong is a picture that looks nearly right. It also keeps a `mat4` inverse out of
  a language with no operator for one.

`point_coord` runs 0..1 across the frame, x to the right and y down, which is the same
sentence it already means: 0..1 across the primitive.

A Set always has an L1, so a fullscreen renderer is still paired with geometry — and if it
is the *only* renderer, that geometry is dead weight: pick something small, since only its
`capacity` declaration is read. In a stack beside a per-element renderer it is not dead
weight at all, and the simulation runs for the node that reads it.

**The fragment cost ceiling is different here, and deliberately.** A per-element renderer is
held to 512 ops per fragment because nobody knows how many fragments there will be —
`capacity` sprites, times their area, times whatever they overlap. A fullscreen procedure
covers the canvas exactly once and nothing overdraws, so the number is known and the ceiling
is an element's instead. A raymarch is thirty-odd iterations of a distance function by
construction; under 512 there is no marcher at all, which is a ceiling refusing the mode
rather than an excess.

**This is what the SDF builtins were for.** `sd_sphere`, `sd_box`, `sd_torus`, `sd_plane`
and the four CSG operators have passed the checker since the beginning with nothing able to
draw them. See `examples/field_march.kir`.

**`seed` and `copy` are refused here**, and it is the same rule as `consumes` rather than a
second one: they are per-element identity, and a fullscreen pass has no element. Left
available they lowered to a varying against the engine's own vertex stage, which has no such
field — valid `.kir`, invalid WGSL, and the process down before a frame was drawn.

Consumed attributes, `seed` and `copy` are readable in both blocks of a *per-element* L4.
Per-element values reach
`fragment` with **flat** interpolation. That is exact under both topologies for the same
reason: a sprite and a segment are each one element's worth of values, so there is nothing
to interpolate between. It is a topology with real *shared* vertices that would need the
rule revisited.

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

## L4 example — the same geometry as lines

One assignment apart from the one above, and it pairs with the same L1. The far end is this
procedure's own arithmetic on attributes it already consumes; nothing about the geometry
changed.

```
proc soft_streaks {
  kind  L4
  blend additive

  consumes position, velocity, age

  param width    : float [0.5, 24.0] = 2.0
  param streak   : float [0.05, 2.0] = 0.8
  param exposure : float [0.0, 8.0]  = 0.5
  param falloff  : float [0.5, 8.0]  = 2.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    clip_b     = camera * vec4(position - velocity * streak, 1.0);
    point_size = width;
  }

  fragment {
    // Across the stroke, not around a point: `point_coord.y` is the axis that
    // crosses it, and `point_coord.x` runs from `clip` to `clip_b`.
    let across = abs(point_coord.y * 2.0 - 1.0);
    let a      = pow(max(0.0, 1.0 - across), falloff) * (1.0 - point_coord.x);
    color      = vec4(hsv_to_rgb(vec3(0.55, 0.75, 1.0)) * exposure, a);
  }
}
```

Two things about that file are consequences of the topology rather than of taste.

Exposure is well below the sprite version's because a streak covers many more texels than
the sprite it replaces, and the blend is additive — `examples/drift_streaks.kir` sits a
factor of ten under `soft_points.kir` at the same element count. **Changing the topology
changes what a given exposure means.**

And `streak`'s range starts above zero. A segment whose two ends coincide is zero-area and
draws nothing, where a sprite at zero velocity is still a sprite, so a parameter that scales
the distance between the ends has a value that blanks the material — and a declared range
that includes it is a fader that turns the slot off at one end.

---

## Color

The pipeline is linear and HDR end to end.

- All internal render targets are `Rgba16Float`
- `color` written from a fragment block is **linear** RGB, straight alpha
- sRGB encoding happens once, at final output
- Values above 1.0 are expected in **RGB** and are what feeds bloom
- **Alpha is coverage, and belongs in `[0, 1]`.** It is what the sprite covers of the texel,
  and L5's `over` blend mode composites against it — see the session stream's `blend`
  record. Nothing rejects an alpha outside that range and the L5 mix saturates what it
  reads, so writing `1.5` costs a layer some of the hiding it asked for rather than a
  diagnostic. It is not a second brightness: RGB is where a value above 1.0 means something

`hsv_to_rgb` returns **linear** RGB. It performs the sRGB→linear conversion internally so
that authors get the hue they expect without thinking about color space. Explicit
converters `srgb_to_linear` and `linear_to_srgb` are available when needed.

This is a common source of confusion. State it in the generation prompt.

---

## Built-in functions

### Math

`abs floor ceil round fract mod min max clamp mix step smoothstep sign`
`sqrt pow exp log exp2 log2`

**Componentwise over one type, which is narrower than GLSL.** Where GLSL lets a scalar stand
in for a vector — `mix(a, b, 0.5)` with `a` and `b` `vec3`, or `clamp(v, 0.0, 1.0)` — this
language requires every argument to resolve to the same type and refuses the mixed form by
name: ``mix` expects every argument to be the same type; found `vec3` and `float``. Widen it
at the call: `mix(a, b, vec3(k))`, `clamp(v, vec3(0.0), vec3(1.0))`. This applies to `mod`,
`min`, `max`, `clamp`, `mix`, `step`, `smoothstep`, `pow` and `atan2`.

One rule and no exceptions is what makes it worth the extra characters: a language where
*some* positions broadcast is one where an author has to remember which, and where a wrong
guess is a shader that compiles.

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

**There is no `field` builtin.** There was one, `field(vec3) -> float`, and deleting it is
what this notation *is*: a field is reached through a slot the header declares and the Set
binds — `uses shape : Field`, then `shape(p)`. See [Evaluating one](#evaluating-one).

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
  not one buffer pair per attribute: `seed`, then the birth fraction, then `copy` where
  something above amplified, then whatever slot a derivation rule needs, then `emit` in
  declaration order. `seed` and the birth fraction are always allocated whether or not the
  procedure names them — neither is nameable from IR. A procedure emitting three attributes
  therefore binds four storage buffers in its compute stage rather than twelve, which matters
  because the WebGPU default limit is 8 and the downlevel default is 4
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
- **Every slot is its own width, at the offset WGSL's own layout rules give it.** Slots used
  to be padded to a full 16-byte `vec4` each, which made the stride `(2 + emit.len()) * 16`
  and meant nothing had to do any arithmetic; it also spent 39% of every element buffer
  across the geometries this repository ships, and the same 39% of the largest allocation in
  the system, since an amplifying stage multiplies exactly this number. Declaration order is
  kept and nothing is reordered to pack tighter — an author's `emit` order is the order the
  struct reads in — and it packs well anyway, because a `vec3` is 16-byte aligned and 12
  bytes long, so a scalar declared after one lands in the four bytes it leaves: `position,
  size` is one 16-byte block rather than two. The stride is the struct's size rounded up to
  its own alignment, which is the rule WGSL applies to `array<Element>`.

  **What it costs is that something now knows WGSL's layout rules where nothing did.** The
  host writes bytes at a published offset and the shader reads them through the struct, so a
  disagreement is not a compile error anywhere — it is an element reading the middle of the
  element before it. The align/size table lives in one place,
  `crates/karakuri-ir/src/layout.rs` — the crate at the bottom of the dependency graph, so
  that the lowering and the engine read one rule rather than each carrying a copy — and the
  naga tests assert it against a real WGSL module: they compile the emitted struct and
  compare the member offsets and array stride naga computed from its own rules against the
  ones the table gave. Validating the module is the weaker check and would not catch a wrong
  table, because nothing emits an `@offset` — a front end handed one recomputes the same
  offsets from the same declarations and agrees with itself
- Buffers swap at the end of every **step**, not every frame: what one substep wrote as
  "next" is the next substep's "prev", and after the last one it is what L4 reads

### L4

`vertex` / `fragment` become a render pipeline.

- **`topology points` does not lower to `PrimitiveTopology::PointList`.** WebGPU has no
  point size — a point primitive is always one pixel — so each element expands into a quad:
  six vertices per instance, the corner in `@builtin(vertex_index)` and the element in
  `@builtin(instance_index)`. `point_size` scales the quad in clip space so a sprite keeps
  its pixel size at any depth, and `point_coord` falls out of the corner
- **`topology lines` does not lower to `PrimitiveTopology::LineList` either**, and for the
  same reason: a line primitive is one pixel wide. It is the *same quad*, laid along the
  segment instead of around a point — six vertices per instance, one instance per element,
  a `TriangleList` pipeline, and the same indirect draw arguments. The topology costs the
  engine nothing; only where the corners land changes.

  The placement happens **in pixels**, because that is the space `point_size` is given in:
  both endpoints are divided through by `w`, the segment's direction and the perpendicular
  the width is laid along are computed there, and the result is multiplied by `w` again so
  the rasterizer's own divide returns the position computed here. That last multiply is what
  keeps the stroke straight on screen at any depth. Each of the six vertices belongs to one
  end of the segment and carries that end's `w` and that end's divided `z`; nothing is
  interpolated in the vertex stage, because the rasterizer is what interpolates.

  **A segment with an endpoint behind the eye is dropped rather than clipped.** Doing it
  properly means intersecting the segment with the near plane; the rasterizer would have
  done that for free had the divide not already happened, and the divide is what makes a
  width in pixels expressible at all. The honest failure is a missing stroke rather than one
  drawn through the camera. A zero-length segment needs no special case — both ends land on
  the same pixel and the quad is zero-area, which is what a guard would have arranged
- L1 buffers are read as storage, indexed by `@builtin(instance_index)`, not as vertex
  buffers
- Per-element values used in `fragment` become `@interpolate(flat)` varyings
- `blend additive` lowers to additive blending with no depth write, which is what avoids
  any sort requirement. **Colour is what it applies to.** The alpha channel of the target
  composes as `over` instead, accumulating `1 - prod(1 - a_i)` — the coverage L5's `over`
  blend mode needs, which nothing else writes. So the colour that leaves L4 is
  premultiplied by coverage, and a fragment block that assigns an alpha above 1.0 is
  writing a coverage the mix will saturate
- `blend weighted` lowers to **two colour attachments and a resolve pass**, still with no
  depth write and still needing no sort. The fragment stage returns a struct rather than a
  `vec4`: `sum(c * a * w)` and `sum(a * w)` into an `Rgba16Float` accumulation, and `a`
  into an `R16Float` revealage whose blend state is `dst * (1 - src)`, so what it holds is
  `prod(1 - a_i)` however the fragments were ordered. The resolve divides the colour sum by
  the weight sum and composites against `1 - revealage`, and **what it writes is exactly
  what the additive path writes** — premultiplied colour, coverage in alpha — so L5 reads a
  weighted slot without knowing the mode exists
- The weight is `a * max(1e-2, (1 - d)^3)` where `d` is the fragment's view depth mapped
  linearly onto the camera's near and far planes. Three things about it are worth carrying.
  **Its absolute scale is nearly arbitrary**, because the resolve divides it out — so it is
  chosen to stay inside `(0, 1]` rather than carrying the published `3e3` factor, which an
  `Rgba16Float` target holding unbounded HDR colour would overflow. *Nearly*: the guard on
  the resolve's divide is compared against the weight sum directly and so does not cancel,
  which is why it is tied to `f16`'s smallest representable value rather than to any weight —
  a floor above that eats the colour of thin material, and since `a * w` goes as `a²` it eats
  it from an opacity of about the square root of the floor upward. And **linear in view
  depth, not in the depth buffer's**: NDC depth needs no uniform at all and is useless
  here, since a 0.1 near against a 100 far crushes everything past ten units into the last
  percent of its range
- **The revealage target's precision is the mode's floor on thin material.** `f16` spacing
  just below 1.0 is one part in 2048, so an opacity under about 2.4e-4 leaves `1 - a`
  rounding back to 1.0 and contributes no coverage at all — a million such fragments still
  contribute none, where `additive` would have summed their light. `R32Float` would fix it
  and is not blendable in WebGPU core, so this is a property of the mode rather than a
  choice left open
- A **fullscreen L4 may not declare `weighted`**, and this is refused by `Set::build`
  rather than by the checker. One fragment per texel makes the resolve the identity *for an
  alpha in `[0, 1]`* — `(c·a·w) / (a·w) · (1 - (1 - a))` is `c·a`, which is what additive
  blending into a cleared target leaves — so the second target and the resolve pass would
  buy a picture `additive` gives for nothing. The qualifier is the clamp: a fullscreen
  fragment writing an alpha of 1.5 is legal under `additive` and *brighter* than the
  weighted version would have been, so the refusal's advice to declare `additive` is not
  always a picture-preserving swap. It is a rule about the *Set* holding one L4, not about the
  procedure, which is why it lives where that assumption does
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
   each emitted attribute and required stage output assigned on every path. What an L1
   consumes is checked here only where one file can answer it — `velocity` is synthesised
   from `position`, so a procedure consuming the first and emitting neither is refused —
   while *whether anybody emits it* belongs to Set composition at stage 6, because it
   relates every procedure in the Set. This stage also decides
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
{"t":"set","id":"morph_01","v":1}
{"t":"slot","layer":"L1","name":"near","proc":"sha256:a3f2c1…"}
{"t":"slot","layer":"L1","index":1,"proc":"sha256:77b9e0…"}
{"t":"slot","layer":"L2","proc":"sha256:1d4a8f…"}
{"t":"slot","layer":"L4","proc":"sha256:9c1b04…"}
{"t":"slot","layer":"L4","index":1,"proc":"sha256:5e7d20…"}
{"t":"capacity","layer":"L1","value":32768}
{"t":"capacity","layer":"L1","index":1,"value":32768}
{"t":"param","layer":"L1","key":"radius","value":2.4}
{"t":"param","layer":"L4","index":1,"key":"hue","value":0.58}
{"t":"bind","layer":"L1","key":"turbulence","signal":"energy","curve":"pow2","range":[0.1,2.4]}
{"t":"edge","node":"morph","slot":"far","to":"sphere_shell"}
{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15}
{"t":"seed","stream":"L1","value":19274}
{"t":"seed","stream":"L1","index":1,"value":48113}
```

**One `slot` record per node, in node order** — the geometries, the deformers, the cameras,
the renderers, then the fields. **The built-in camera has no `slot` record**, because it has
no procedure to reference: it is the last node of the L3 layer in every Set, and the `camera`
record below is what describes it. The chain above is two geometries, a morph between them and
two renderers over the result; a Set of one geometry and one renderer is the two `slot` lines
it always was. Only `near` was written down: the other four names are derived from their
procedures where the Set is built, which is why the `edge` can name `morph` and `sphere_shell`
without either appearing in the file.

- `proc` is a hash reference. When bundled, the source is inlined as a run of
  `{"t":"src","hash":"…","line":0,"s":"…"}` records.
- `capacity` is optional; without it the `.kir` default applies. A value outside the range
  the `.kir` declares is rejected at Set build time.
- **A `slot` may carry a `name`**, which is what an `edge`, a `--param` address, a rebuild
  and an MCP call point at — `{"t":"slot","layer":"L1","index":1,"name":"veil","proc":"…"}`.
  Optional and absent by default, on the terms HTML gives an `id`: it is written only where
  somebody chose one, and **a name is a cost you pay when you want to point at something**.
  What it is *not* is the difference between a node that can be pointed at and one that
  cannot — every node has a name whether or not one was written, and an unwritten one is
  derived from the procedure and disambiguated (`lattice_shell`, `lattice_shell-2`) where the
  Set is built. Written names are unique within the file, and two that collide are refused
  rather than resolved. See "Naming a source, on the terms HTML gives an `id`".
- **`edge` binds one node's declared input slot to another node**, and is **the one record
  addressed by name at both ends**. A procedure declares what it takes and never which node
  supplies it — `uses far : Geometry`, `uses shape : Field`, `uses view : Camera`, see
  [above](#uses) — because a `.kir` that named a node would be coupled to one Set. This is the other half, and it lives
  here for the same reason a `slot`'s name does: an edge belongs to the *use*, and a Set file
  is what a use is recorded as. `--edge morph.far=sphere_shell` writes one, and
  `--edge field_lens.shape=melt_blob` and `--edge lens.view=orbit` write the other two —
  one record for all three, because what an edge says is the same fact whatever the slot
  takes.

  It cannot use `(layer, index)`: a position moves when the list is reordered, and reordering
  silently changing which geometry a morph blends towards is the failure this record exists
  to end. Every node has a name whether or not one was written — a name nobody wrote is
  derived from the procedure where the Set is built, which is a function of the artifact the
  `slot` record already references — so both ends always resolve, and a file that names no
  node at all still round-trips. Written after the `slot` records, so a reader has every name
  in hand by the time it meets one. **A declared slot that no `edge` binds is refused where
  the Set is built**, and so is a slot bound twice.
- **`param` and `bind` may carry an `index`**, which addresses one node of the layer —
  `{"t":"param","layer":"L4","index":1,"key":"exposure","value":0.9}`. **Absent is a
  wildcard, not node 0**: it reaches every node declaring the key, which is what a bare name
  has always meant and is the useful default. That is also what keeps every file written
  before the address existed reading the same way — `layer` on a `param` was a placeholder
  the loader ignored, so it becomes load-bearing exactly when an `index` appears beside it.
  The address is `(layer, index)` present or absent as a unit.
- **`seed` is keyed by node, not only by layer** — `{"t":"seed","stream":"L1","index":1,…}`
  salts the second source. Absent means 0, so a file naming one source per layer reads as it
  always did — and a Set of one geometry writes the line it always wrote. **Written and read
  per geometry**: `--save-set` records the salt each source was running at and `--load-set`
  gives it back to the source that index names, which is what makes a saved Set reproduce its
  colours rather than re-derive them from the order its paths were spelled in. A geometry the
  file names no seed for is salted from the Set's seed and its ordinal, so an older file
  loads and keeps whatever it did say. The value is also what that source's `source`
  attribute will carry, since the discriminator and the salt are one value — the attribute is
  not built, and the record already carries what it would carry.
- **`capacity` is keyed by node too** — `{"t":"capacity","layer":"L1","index":1,"value":…}`
  sizes the second geometry. Absent means 0 rather than a wildcard, on the `slot` rule and
  not the `param` one: it names exactly one node, and a Set that held one geometry had only
  node 0 to size, so no file written before the address existed is retargeted by it.
- **Several `slot` records on `L4` is a stack**: one geometry with a renderer apiece, drawn
  in the order the records appear. It needed no new record to say so — a second one is a
  second renderer rather than a correction of the first. A file with one reads exactly as it
  always did.
- **Several on `L3` is several cameras**, and which renderer reads which is an `edge`. The
  format could always say it; what could not was the engine, and this loader read the first
  and reported the rest as skipped. See [Several cameras](#several-cameras).
- **`camera` carries an `index`** — `{"t":"camera","kind":"orbit","index":0,…}` — on the
  `slot` rule rather than the `param` one: it describes one producer, and "every camera at
  radius 9" is not something a camera has ever said. Absent means 0 and 0 is not written, so
  every file ever written round-trips byte for byte and keeps meaning what it meant. **The
  built-in orbit is the node after the camera procedures** — `L3:0` in a file that names
  none, which is every file written before they could be named — and it is the only node such
  a record can be about, since a camera that is a procedure writes its own six numbers every
  frame. An index naming one of those is reported and skipped rather than applied to a node
  that would overwrite it.
- `param` and `bind` are keyed by `layer` in the record, and the engine holds one value per
  **node**. A name two procedures both declare is two values, each reaching the node that
  declared it — which it had to become, since every L4 in `examples/` declares `exposure`
  and a Set holding two renderers would otherwise have had one. A `param` record carrying a
  bare name reaches **every node that declares it**, which is the useful default: one knob,
  both renderers. Setting two of them *apart* is `layer` plus an optional `index`, which
  both records now carry the way `procedure` does — `--param L4:1:exposure=2.0`,
  `--bind index=1`.
- Unknown `t` values are ignored, for forward compatibility.
- **`gain`, `opacity`, `blend`, `mask`, `transition`, `preview`, `residency`, `look`,
  `canvas`, `procedure` and `transport` are not in this list and must never be.** They are the session's
  rather than any Set's — see the session stream format. `canvas` is the sharpest case: a
  Set renders at whatever size it is handed, so a Set file that carried one would resize
  every *other* Set in the deck by being loaded.

**Implemented, and the engine obeys it.** `karakuri-cli`'s `--save-set` writes one of these
and puts the slot's `.kir` sources in the store as content-addressed artifacts; `--load-set`
reads it back and builds from it, resolving each `slot` by hash or from inlined `src` records
when the file is bundled. A run driven by a Set file renders the same frame as the run whose
flags wrote it.

**And it saves a chain**, which it did not at first: a slot holding an L2, an L3, a `kind
Field` or a second L1 was refused by name rather than saved short, and `--record-session`
refused it for the same reason, since a session head is a Set file. Nothing in the format had
to change to close that — a `slot` has carried a layer, an index and a name since the address
existed. What was missing was a writer that put a node's own `kind` into it and a reader that
honoured the index on every layer rather than on one.

`--bind` survives as a way of *writing* a `bind` record rather than as a path beside one:
the flag parses its fields into a `Record::Bind` and hands it to the same decoder a Set
file uses, so a command line and a file cannot mean different things by the same fields.

**A `camera` record survives a rebuild**, which it did not at first and which was a defect
rather than a property of the format. `swap::Request` carried no camera, so a Set built by a
hot swap started from the built-in orbit's defaults however the Set it replaced was aimed:
under `--load-set X --watch` the first edit to any `.kir` in the slot reset a camera the file
had set, with nothing said — and a live save afterwards recorded `set.camera` faithfully, so
the defaults went into a new preset and the loss outlived the run. The request carries the
orbit now, restated on every rebuild the way the salts and the bindings are, and the build
worker applies it at the point the loader does. The record means what it has always meant;
what changed is that the rebuild no longer discards it.

**Two places this format is finer than the engine**, both reported on load rather than
dropped: a `param` may be a vector while the engine's map holds `f32`, and `camera` carries
two of the six fields the engine's orbit has. Each is a disagreement between the format and
the engine rather than a gap in the loader, and settling them is the format's business and
the engine's, not the reader's.

There used to be a third — `seed`, `capacity` and `param` keyed by layer against an engine
that held one of each per Set. The engine caught up: params are per *node*, the salt is per
*source*, and each source runs at its own capacity.

The *session* records are further along: `audio` and `tempo` per frame, and `gain`,
`opacity`, `blend`, `preview`, `residency`, `look` and `transport` per key press, are each
built by the CLI, decoded back,
and only then applied — and `karakuri-cli`'s `--record-session` writes them to a session
stream as they happen, `--replay` reading it back. The path the engine is driven through is
the record's, in both directions.

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

### The mix in the stream — `gain`, `opacity`, `blend`, `mask`, `transition`, `preview`, `residency`, `look`, `canvas`, `procedure` and `transport`

A session that carried the material and not the performance would replay the same Sets, on
the same beat, all at whatever gain they happened to start at, with nothing ever going on
or off air. Eleven records carry what an operator moves — ten states and one event:

```ndjson
{"t":"gain","slot":0,"value":0.75}
{"t":"opacity","slot":0,"value":0.5}
{"t":"blend","slot":1,"mode":"over"}
{"t":"preview","slot":2}
{"t":"mask","slot":1,"kind":"linear","angle":0.0,"position":0.0,"softness":0.02}
{"t":"transition","slot":1,"control":"mask","to":1.0,"start":64.0,"beats":8.0,"curve":"smooth"}
{"t":"residency","slot":1,"level":"priming"}
{"t":"look","op":"aces","exposure":1.2,"white_point":4.0}
{"t":"canvas","width":1920,"height":1080}
{"t":"procedure","slot":0,"layer":"L4","proc":"sha256:486779…"}
```

**`gain` is a deck slot's level into the mix**, and **`opacity` is its fader.** The slot is
a position on the deck, not anything about the Set in it: moving a Set to another slot moves
it under another fader, which is what a fader is.

The two are separate records because they are separate controls, and what makes them
separate is `blend`. Every mode composites its **colour** as
`acc <- mix(acc, f(acc, gain * src), opacity)`: gain is the level the material arrives at
and touches colour alone, opacity is how much of the blend lands and is the only one of the
two that scales what a layer *covers*. Under `add` they collapse into one multiply and a
stream carrying either would replay the same; under `over` one dims a layer and the other
stops it hiding what is beneath.

Coverage is the exception to that formula and composes as `over` under every mode, because
"there is material at this texel" is an `over` question even when the colour is being added.
`opacity` is a proportion of a blend and is clamped to `[0, 1]`; `gain` is a level into an
HDR mix and deliberately is not clamped above 1.0.

**`blend` is how a slot's layer meets the ones under it** — `add`, `over` or `max`. Slot
order is stacking order, so this is the one mix control whose meaning depends on where the
slot sits.

The set of modes is chosen by what survives an **unbounded linear HDR** mix rather than by
what a VJ mixer usually lists. `screen` is `d + s - d*s` and `multiply` is `d * s`; both
assume display-referred inputs in `[0, 1]`, and nothing has tone mapped this far up the
pipeline — `screen` of two 2.0s is 0.0. They belong after the transfer curve or not at all.

`over` needs to know what a layer covers, which is why **alpha in a slot target is
coverage**: the L4 pass accumulates `1 - prod(1 - a_i)` there while colour adds, so the
colour that reaches L5 is premultiplied and emissive material still sums past what its
coverage would allow. Sparse material barely covers, so `over` on a thin point cloud reads
close to `add` — which is correct rather than a defect, since a handful of sprites does not
occlude anything.

**`transition` is a mix control moving over musical time** — a fade, a cut, or half of a
crossfade. One record for the whole move, and **the values it produces are not recorded**: a
value per frame would be 216,000 lines an hour describing something the grid already
determines, which is the argument `tick` makes from the other end.

`start` is an absolute position on the session's beat count rather than "in two bars",
because a relative instant is a different instant depending on when it is read. Quantising
to the next bar happens where the operator asked, once. `beats` of 0 is a cut. **`from` is
deliberately absent**: it is read where the move is *scheduled*, and a reader replaying the
stream reads it the same way. Capturing it at the start instead would mean capturing it on
the first frame at or after a musical instant, and a machine running at a different rate
would capture it at a different beat — which is the one property this record exists to
have. Nothing can move the control in between: a hand cancels the move and another move
replaces it.

`control` is `gain` or `opacity`, and `curve` is a `bind`'s vocabulary — `lin`, `pow2`,
`sqrt`, `smooth` — because a fade's shape and a signal's shape are the same question. There
is no `crossfade` record and there should not be: a crossfade is two of these sharing a
start and a length, a fade-in is one, and a cut is one with a duration of zero. The
first-class thing is the move.

**`mask` is what shape of the frame a layer reaches.** It multiplies that layer's
opacity per texel, which is what makes it a mask rather than a second fader: everything
opacity does — how much of the blend lands, and under `over` how much the layer covers — is
what a mask wants done to part of the frame. `kind` is `none`, `linear` or `radial`;
`angle` is the linear front's, in radians; `position` is how far it has travelled, `[0, 1]`,
**exact at both ends** — 0 reveals nothing anywhere and 1 reveals everything everywhere, for
any `softness`.

**The two lines above are a wipe, and there is no `wipe` record.** A mask at position 0 on a
layer that blends `over`, and a `transition` carrying `mask` to 1: the front hides what is
beneath it exactly where it has passed. Neither half knows about the other — the transition
moves a number and the mask reads one — which is the same shape as a crossfade being two
`transition`s. Both ends being exact is what makes it *finish*: a `position` of 1 that left
a corner half-lit would be a wipe that stopped short.

What is deliberately not here is a mask read from a **texture** — an arbitrary shape, or
another layer's luminance. That needs somewhere for the shape to come from, and the answer
is M3's `Field` rather than a third `kind`.

**`preview` is which slot the output is showing**, or `null` for the mix. It is not a mix
control — it changes nothing about how the Sets are combined — and it is in the stream for
one reason: today the preview *is* the output, so a replay that ignored it would show the
mix where the operator was looking at one slot. That reason expires. When output routing
gives the deck a second output this becomes the monitor's choice and stops being the
programme's, and the record stops belonging in a session stream where `gain` and `blend`
still will. `null` rather than a sentinel index, so a deck of a different size cannot read
one as the other.

**What no record says is what the deck held.** Every record above names a slot, and the
format has no way to say that a session ran on four slots or what was in them — a Set file
describes one Set. So `--replay` builds a deck of one and reports every record naming
another slot rather than obeying it. That is a gap in this format, not in the replay driver,
and it is the same gap for `gain`, `blend`, `residency` and `preview` alike. It is also the
one place "a session replays what happened" is currently short of true, and the cost grew
the moment a control surface arrived: a map is written per slot, so a four-slot surface
produces a session three quarters of whose moves are skipped on the way back.

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

`level`, `mode` and `op` are strings for the same reason `curve` and `noise.kind` are: an
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

**`canvas` is what the session renders at**, in texels — the surface every layer draws
into, what every deck slot is sized to match, and what an offscreen render writes.

It is **not the size of any window.** A window is a preview of what leaves by some other
route, so it is fitted to the canvas rather than the other way round: dragging one changes
what an operator can see and nothing about what is drawn. Until this record existed the two
were the same number, and a session played in a small window and replayed at a large one
rendered different pixels with nothing in the stream saying which was the performance.

**Written once, at the head, and a stream carries no second one** — which makes it the only
record here that is session state without being something a hand can reach mid-set. Changing
a canvas reallocates every slot's target and the frame path allocates nothing, so the canvas
is a property of a *run*. A reader that meets a later one should report it rather than obey
it: a replay sizes everything it allocates from the first, before any frame exists.

**`procedure` is the material a deck slot is playing, from that moment on.**

```ndjson
{"t":"procedure","slot":0,"layer":"L1","proc":"sha256:9da973…"}
{"t":"procedure","slot":0,"layer":"L4","proc":"sha256:486779…"}
{"t":"procedure","slot":0,"layer":"L4","index":1,"proc":"sha256:5e7d20…"}
```

Written when a hot swap lands and when one is rolled back — the two moments the material
actually changes. **Without it a session recorded the material once, before the first frame,
and replayed the whole run with whatever it started with**: a set in which a procedure was
rewritten at minute ten replayed as though it never had, silently. That was survivable while
the only way to rewrite one was a human with an editor; it stopped being survivable when a
model could, because rewriting procedures is the whole of what that surface does.

It is the **session's** and not the Set's, which is what `is_set_state` is for: a Set file's
`slot` record says what a Set *is*, and this says what a deck slot *became*, at a point in
time. That is a fact about a performance.

`proc` is a content address and the source lives in the store, on the same terms `slot`
uses — so a rewrite costs one line here and a few kilobytes once, however many times the
same procedure comes back. **Every one of a slot's procedures is written together** even
when only one changed, because a Set is built from all of them and a reader that rebuilt on
the first would compile an L1 against the L4 it is replacing.

`index` says **which node of that layer**, since a slot draws with one L1 and however many
L4s. It is 0 for the L1 and for the first renderer, and **absent when it is 0** — so a
stream written before stacks existed replays byte for byte, and a new one carries the field
only where it says something.

**One thing it cannot carry.** A rollback restores the outgoing Set at the `t` it was parked
at; a reader meeting these records builds afresh, so `t` restarts there. A swap *in* is
defined to start cold and therefore replays exactly — only a rollback differs, and a
rollback means the candidate was over budget, which is an exceptional frame already.

**These eleven stay out of a Set file**, ten because they are state that is the
session's rather than any Set's and `transition` because it is not state at all. That is a second
reason for a record to be absent from a Set file and it is not the `audio` one: there is
something to fold here, and this is not the projection it folds into. A Set file that
restored a gain would apply it to whatever slot it was next loaded into, and one whose
`residency` said `live` would put a Set on air by being opened. They belong to a session stream, which
`--record-session` writes and `--replay` reads. What does not exist is a *session*
projection — folding one down to the deck state it ends at — because nothing needs to
resume a deck yet; folding a session down to a **Set file** does exist and is specified
above.

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

### Records with an effect outside the stream — `save`

Every record above describes the deck, and a reader that obeys it reproduces the
performance. **One record describes something else: a file that was created.**

```ndjson
{"t":"save","slot":0,"id":"20260816-143052-271"}
```

**`save` says a deck slot's material was written out as a Set file**, under that id, at that
point in the timeline. It carries the slot and the id and deliberately nothing else — the
Set file under that id already names every node it holds, and a copy of them here would be
one fact in two places, free to be right on the day it was written and wrong the moment the
two are read apart. What this record is for is saying *that* a save happened and *what it is
called*; what was saved is a question the Set file answers.

That makes the general rule worth stating, because it is the first record to need it:

> **A replay is a sandbox.** Some records have an effect outside the stream. A replay does
> not perform those effects, and it says which ones it skipped. Bringing outside state into
> a replay environment is the operator's responsibility.

For `save` the reasoning is concrete. A replay that wrote Set files would be writing into
ids that already exist, in a store nobody asked that run to touch, holding somebody else's
material — and `--replay` would stop being a function from a stream to some frames. So it
renders the frames and prints the id it passed over. **Said rather than silently dropped**,
on the terms everything else in this system follows: a Set file load prints every note it
could not honour, and the session writer counts the batches it lost rather than losing them
quietly. The id is named because it is the thing an operator would go and load by hand if
they wanted the material that record is about.

**This is a second question about the vocabulary, not a third reason inside `is_set_state`.**
That function asks *whose state is this* — the session's or a Set's — and `save` is refused
by it for the ordinary reason: a save is a fact about a performance, and a Set file carrying
one would claim, every time it was opened, that a save had just happened. *Does this reach
outside the stream* is orthogonal, `save` is so far the only record for which the answer is
yes, and folding the two together would give one function two jobs.

**Written at the frame the save landed, not at the key press.** The store write happens off
the render thread, so it finishes some frames later and can fail; a record written when the
key was pressed would claim a file the disk then refused. That is the same rule `procedure`
follows — a record that describes a change already made — and it is why **a failed save
writes no record at all** and prints instead.

What this record does *not* do is make a replay reconstruct the library. There is no
`--replay` mode that writes the Sets a session saved, and there should not be: reconstructing
them means writing into a store, which is the effect the rule above exists to keep out.

---

## Beyond v0.2 — specified, not implemented

Everything above this line is implemented and tested. **Everything below carries its own
state in its heading**, because this section has outlived the sentence that used to open it:
a heading ending in *"— built"* is implemented and tested like the rest of the document, and
one carrying a milestone is design that has been settled but not built — no parser accepts
it, no checker enforces it, no generator emits it.

The unbuilt half is written down because later milestones depend on these shapes and because
deciding them now keeps V1 from foreclosing them. The built half stays here rather than
moving up because the reasoning is what makes each of them worth reading, and cutting a
section into its rules and its argument would put the two in different places.

Checked against a symbol rather than from memory, which is how the mismatch this note
replaces came to be: `Kind::L2`, `Kind::L3`, `set::Published` and `node::Merge` all exist,
so the four sections naming them are built.

### L2 and L3 — built

The layer algebra has been in `docs/roadmap.md` since before there was a compiler, and
everything about L1 and L4 was decided by building them. L2 and L3 have been "later" for
long enough that the vagueness became load-bearing: the engine is being split so that `Ln`
is a node, and the shape of an L2 node and an L3 node constrains that split now rather than
when they are written. What follows is what is decided, why, and what is not.

#### L3 — the camera

**What crosses the edge is camera state, not a matrix**, and the reason is multiplicity.
`docs/roadmap.md` says L3-multiple is a weighted blend of trajectories — an orbit and a
handheld rig mixed at 0.3 — and a lerp of two view-projection matrices is not a projection
of anything. So an L3 node produces the six numbers a camera *is*:

```
eye, target, up, fov_y, near, far
```

and everything derived stays in the engine, where `crates/karakuri-engine/src/camera.rs`
already keeps the derived forms from drifting: `view_proj` for the raster path, `basis` for
the marched one, and — since `blend weighted` — `depth_range`. Three derivations of one
orbit that must agree, which is exactly the argument for deriving them in one place from
state rather than passing any of them along an edge.

Aspect ratio is **not** in that list. It belongs to the canvas, not to the camera, and
`Orbit::basis` already takes it as an argument for that reason.

**`near` and `far` stopped being decorative.** They described a frustum nothing measured
until `blend weighted` normalised its depth weight against them. An L3 that moves `far` per
frame therefore moves every weighted fragment's weight per frame. That is not a defect — it
is what tying the weight to the camera's own planes means — but it is a coupling nobody
would predict from the layer algebra.

**An L3 may read geometry, and the algebra's `L3 : () -> Camera` is wrong about that.** This
section said the opposite for a day — "no geometry input and no GPU pass … the first node
whose evaluation is host-side" — and what falsified it was one sentence of what an author
expects a camera to do: *「1つ目の頂点を追いかける」*. A camera that follows an element takes
geometry as an input, and the value it needs is in a GPU buffer.

Which settles where a camera lives, because it cannot be read back. `Set::read_elements` and
`Set::live_count` both document themselves as stalls — *"never call it on the frame path"* —
and a GPU-to-host readback every frame is the one thing this architecture is built to avoid.
So:

**The `Camera` edge is a GPU buffer, and which producer writes it is the L3's business.** An
L3 that reads only the clock — an orbit, a jump on the beat — is written from the host with
a `queue.write_buffer`, exactly as the built-in orbit's uniform is written today. An L3 that
reads geometry is written by a small compute pass. One consumer, two kinds of producer, and
no readback either way.

The decision above survives this and only changes address: **state crosses the edge, not a
matrix**, blending of trajectories happens on that state, and a derivation step produces the
`view_proj`, `basis` and `depth_range` an L4 reads. What moves is that the host stops knowing
the camera — and nothing was found that needs to.

**All of that is built, and so is the first kind of producer.** `crate::node::Camera` owns the
state buffer, the derivation pass and the bind group every L4 reads; `Ambient::Camera` and
`Ambient::Eye` lower to reads of that group rather than of the renderer's uniform. A Set's
camera is either the built-in `Orbit`, host-written, or a `kind L3` procedure whose `camera`
block is lowered to a compute pass writing the same buffer — one consumer, two producers, and
nothing downstream can tell which ran. What is not built is the second kind: a camera that
reads geometry, which needs the addressing below.

**Building the edge before the producer was the right order, and not obviously so.** The
tempting increment is a `kind L3` that the host evaluates, leaving the uniform alone; it would
have worked for every camera expressible today, and it would have had to be undone by the first
one that followed an element. Deciding *where the camera lives* is what the geometry-reading
case forces, and it forces it whether or not anything reads geometry yet — so the edge is the
part that carries the decision, and the procedure is comparatively ordinary work on top of it.

**An L3 may hold state, where an L2 may not**, and the asymmetry is not an oversight. The
four reasons L2 is stateless barely bite here: L3-multiple is a *blend* of trajectories
rather than a chain, so stackability is not the argument; priming six numbers is instant;
compaction is irrelevant. Only seekability bites, and it bites the same way L1's state does
— **a Set is seekable if every node in it is**, which is why `closed_form` is written as a
conjunction rather than read off one named layer.

What makes the asymmetry worth paying for is that **a camera's craft is mostly smoothing**. A
rigid follow is unwatchable; a damped one lags, and lag is state. The second expectation on
record — *「ビートに合わせてランダムに移動しつつ中央を向き続ける」* — happens to be
expressible without any, since a jump chosen by `hash(floor(beats))` is a pure function of
the clock, and that is worth knowing: **the dynamism an author asks for first is often closed
form, and the smoothing they ask for next is not.**

#### L2 — deformation

**An L2 is stateless, and that is a rule rather than an observation.** It reads the
attributes it consumes, `t`, `beats`, `seed`, and its params; it writes attributes; it keeps
nothing between frames. Four things follow, and the fourth is why it is stated as a rule:

- **It is what makes L2 freely stackable**, which is the whole justification the layer
  algebra gives for the endomorphism. Two stateful stages in a row would each need their own
  double buffer and their own place in the compaction ordering.
- **`closed_form` stays decidable.** A stateless L2 is vacuously seekable, so a Set's
  seekability is still its L1's.
- **Priming stays an L1 question.** Warming a Set means warming what accumulates, and
  nothing else does.
- **It makes fusion legal later.** A stateless stage can be composed into its consumer at
  codegen time, which is what `docs/roadmap.md`'s graph compiler means by fusing `Field`
  chains. Left unstated, fusion would rest on a reading of the language rather than on a
  rule — the same distinction `fullscreen`'s empty `consumes` draws.

**An L2 cannot `kill()`.** It follows from statelessness but is worth stating separately,
because what it buys is specific: liveness is decided upstream, so compaction runs once
after L1 and nothing after an L2 has to reconsider it. An L2 that could kill would put a
scan between every pair of stages.

**An L2's output is materialised, not fused.** One derived buffer per amplifying chain,
rebuilt every frame, single-buffered and never compacted — which is what
[L2 amplification](#l2-amplification--built) already says of the amplifying kind, said now of
both. The alternative is to fuse each L2 into whatever reads it, which costs no memory and
**pays the L2's cost once per reader** — so a heavy noise deformation read by three
renderers costs three times. Materialising pays once regardless of how many nodes read it,
which is the shape that survives the thing this milestone is for. Fusion is then an
optimisation the graph compiler may apply, in the place the roadmap already puts it, rather
than the only implementation there is.

**`emit` and `consumes` are both L2 declarations.** An L2's `emit` widens what is available
*downstream of it* rather than what its L1 wrote — the derived buffer is the L2's, not the L1's — so an L2
may emit an attribute no L1 in the library produces. Composition becomes a chain check: each
node's `consumes` must be a subset of what is available at its position.

**The block is `deform`.** A new name rather than reusing `element`, because
`BlockKind::kind()` maps a block to the layer it belongs to and the checker refuses a block
in the wrong kind of procedure — a name that appeared in two layers would take that away.

**Amplification carries `copy` as an implicit attribute.** `seed` downstream of an
amplifying node stays the **parent's**, and the copy index is a second value beside it;
identity is the pair. That way `hash1(seed)` still gives every mirror image of one element
the same colour, which is what makes eight copies read as one object, and breaking that is
the deliberate `hash1(seed + copy * 8191u)`. `copy` joins `seed` and `birth_frac` as a slot
the engine writes — but **only where something upstream amplified**, unlike those two, which
every element has: a `u32` on every element of every Set, plus whatever alignment it drags
behind it, is what an unconditional slot costs, and a chain with no amplifier in it has
nothing to put there. Where the slot is
absent, `copy` reads `0u`, which is the true answer rather than a stand-in — an element that
passed no amplifier is copy zero of itself.

An earlier revision of this paragraph wrote the combination as `hash1(seed ^ copy)`. The
language has no bitwise operators at all, so that was a spelling nothing could compile, in a
sentence whose whole point was to name the thing an author writes. It survived the correction
twice over — once in this file two hundred lines below, and once in the doc comment on
`Ambient::Copy` that the same commit wrote — which is what a phrase repeated in three places
does when only the one being edited is looked at.

#### Neither per Set nor per deck: a camera belongs to an L4

The question this section asked — *a camera per Set, or one for the deck* — was the wrong
one, and the algebra had already answered it: `L4 : (Geometry, Camera) -> Texture` makes a
camera an **input edge of a renderer**. Nothing about a Set or a deck enters into it. Two L4s
reading one L3 are one viewpoint drawn two ways; two L4s reading different L3s are two
viewpoints composited, which is what makes *"a Set that blends two scenes"* a graph rather
than a feature. Sharing is edge fan-out and needs no rule.

The question was asked because `Set` owns an `Orbit` field today, and this document turned
that implementation into a design question. Worth remembering as a shape: **an existing field
is not a fact about the design.**

An L3 is a `.kir` procedure rather than a `camera` record, and that half is built: a `kind
L3` file is a node, and the `camera` record is what a Set says when it has none and is taking
the built-in orbit. The expectations that settled it are dynamic — follow an element, jump on
the beat while facing the centre — and a record with two numbers in it cannot carry either.

**The fan-out this section describes is built**, and it needed nothing this section did not
already predict: a renderer declares [`uses view : Camera`](#uses), an `edge` names which
camera fills it, and two L4s bound to one camera are one viewpoint drawn two ways. Sharing
needed no rule — `SlotBoundTwice` forbids two edges into one *slot*, which is as right for a
camera as for a geometry. See [Several cameras](#several-cameras).

#### What an L3 can point at: a reduction, or element zero — M4

Two things, and the second is cheaper *and* more meaningful than this document claimed a day
ago.

| Addressing | Cost | What it is |
|---|---|---|
| Centroid, bounds | One reduction pass | Where the material is and how big it is — "keep it framed" |
| **Element zero** | One buffer read | **The oldest living element** |
| The element with `seed == N` | A search over `capacity` | Not built: there is no seed-to-index map |

**The correction is about element zero, and it comes out of a decision made for another
reason entirely.** This document said index 0 was the cheap answer and not the stable one —
*"compaction moves it, not the same element twice"* — and that is **false**. Compaction is
order preserving: survivors keep their relative order, so an element at index 0 that is alive
has zero live elements before it and stays at index 0. It moves only when it dies, and what
it moves to is the next survivor in spawn order.

So index 0 is not an arbitrary slot that happens to be reachable. Spawn order is age order,
which makes it **the oldest living element** — a thing worth pointing a camera at, and a
thing that survives every step until it is the one that dies. Order preservation was chosen
for stable blend order and bit-exact reproduction; this is the third thing it paid for, and
none of the three were visible from the other two.

For a procedure with no `spawn` block it is simpler still: everything is alive from frame
zero, `seed` equals the initial slot index, and index 0 *is* `seed == 0` for as long as
nothing kills it.

**Which puts the anchor in the L1 author's hands, and that is the right place for it.** An
L1 that expects to be looked at can make element zero mean something — never kill it, spawn
it first and deliberately, give it the role of a leader the rest follow. An L1 that does not
care will still offer its oldest survivor, which is a defensible answer rather than a random
one. Nothing is declared and nothing is checked: this is a convention with a stated
consequence, which is what the language does everywhere it can instead of adding a header
field.

Two consequences worth stating rather than discovering:

- **When element zero dies the camera's subject changes**, to the next oldest. For a fountain
  that is continuity; for a cloud where the anchor was chosen deliberately it is a jump. The
  fix is upstream — do not kill the anchor — and stating that is better than a mechanism that
  hides it.
- **When the live range is empty there is no element to point at**, and the camera holds its
  last position rather than snapping to the origin. Holding needs state, which an L3 is
  allowed to have; snapping would put a hard cut in a set at the exact moment material ran
  out, which is when an operator is least able to answer for it.

### What a Set publishes — built

A Set publishes every `param` every procedure in it declares, addressed by the node that
declares it. With one L1 and one renderer that is about nine controls and the addressing is
invisible — a bare name still reaches them, because it reaches every node that declares it.
With the graph M3 introduces — several pipelines, L2s, an L3, a nested L5 — it is twenty-five, most of which
are **authoring decisions the Set's author already made** rather than anything an operator
wants under their hands at two in the morning.

So a Set declares an interface: **which of its internal controls appear on the console, under
what names, and over what part of their declared range.** This is the node-group analogy the
whole node model came from — a group promotes selected inputs and hides the rest — applied to
the one place it matters most, which is the mixing desk.

**Built.** `karakuri_engine::set::Published` is one control — a name, an address, and a range;
`Set::publish` refuses a range that is not a subset of the declared one, a control nothing
declares, and a name already taken. `--publish name=L4:0:exposure[0.2..0.8]` is the command
line, whose address is `--param`'s and whose range is `--bind`'s.

**The address is optional and absent means every declaration**, exactly as it does on a
`param` record, and that is what the default interface is made of: one control per *key*, not
one per declaration. Per declaration would put two controls called `exposure` on a console,
which is a shape `publish` itself refuses. A wildcard control's range is the **intersection**
of what its nodes declare — one knob moving both must not offer a position only one of them
said it still looks like itself at.

It is not in a Set file yet, on the same terms a chain, a camera and a layering are not — but
it **is** in a `Request`, so it survives a hot swap. Leaving it out was worse than a lost
surface: the bindings are restated, so a `control:` binding outlived the control it named, and
a binding whose source is gone holds its param where it found it for the rest of the run.

#### Three states, and the third is not a new mechanism

- **Published.** It appears on the console under a name the Set chose.
- **Fixed.** The author found a value and froze it. This is the *default* state: a control
  nobody publishes keeps whatever the `.kir` default and the Set's `param` records left it
  at, and is simply not on the desk.
- **Driven.** Something moves it that is not a knob.

The third one already exists twice over and should not become a third thing. A `bind` record
attaches a **signal** to a control through a curve and a range. A *macro* — one published
control moving several internal ones, each through its own curve and range — is the same
shape with a different source. So `bind`'s source becomes "a signal, or a published control",
and macros fall out with no new record and no new semantics. A bus signal arrives with a
confidence and is blended by it; a published control is the operator's hand and its
confidence is 1.

**Built, and it did fall out.** A `signal` of `control:<name>` names a published control;
nothing else changed, and no state was added — a published control's value *is* the param it
points at. It is resolved by the Set rather than by the signal bus, which is not an
implementation convenience: a published control belongs to its Set, so four Sets publishing
`twist` are four controls, where a signal name is one thing across the whole session.

#### Publishing decides what is *shown*, never what is *reachable*

A `param` record still addresses any control in any node, published or not. That is how a Set
file records the values its author froze, how `--param` works, and how MCP or an agent tunes
something the console does not show.

The distinction matters more than it looks. If publishing gated access, a Set's author could
lock an operator out of their own machine — and this project's standing position is the
opposite one everywhere it has come up: every measurement is shown and none is acted on,
nothing is hidden, and needing a hand is the craft rather than a failure. **A surface is a
choice about attention, not about authority.**

#### The parameter collision is gone, and an interface still decides what is shown

`SetError::ParamCollision` used to refuse a pair of procedures that both declare a param of
one name, because `Set::params` was one flat map keyed by name across the whole Set. Two
renderers over one geometry declare `exposure` almost every time — every L4 in `examples/`
does — so the check as written forbade exactly what M3 is for. **The engine now keys values
by the node that declares them and the error is deleted**, which is the internal half of the
address the graph needs anyway.

What that leaves for an interface is the external half, and it is the interesting one.
Internally a control is node-and-name. Externally it has the name the Set gave it — so two
`exposure`s can be published as two controls, or as one control driving both, and which of
those is right is an authoring decision rather than an error.

**A bare name still means every node that declares it**, which is the useful default for one
knob and is exactly the "one control driving both" case: `--param exposure=2.0` moves both
renderers. Setting two nodes' `exposure` *apart* is the address in the record vocabulary —
`layer` plus an `index` defaulting to 0, which keeps every existing session stream replaying
unchanged — and that is built, as is the interface that gives the two of them separate names.

#### A published range narrows, never redefines

The `.kir` declares `param exposure : float [0.0, 8.0] = 1.0`. A Set may publish it over
`[0.2, 0.8]` — *"on stage this only wants to go this far"* — and that is checkable as a
subset. A published range outside the declared one is refused rather than clamped, because
the declared range is the procedure's statement about where it still looks like itself.

#### An empty interface publishes everything

A Set with no interface declaration publishes all of it, which is exactly what happens today,
so the feature is additive and every Set file that predates it keeps working. The first
declaration makes the list the interface. That is one sentence of rule and it means an author
opts in by naming what they want rather than by hiding twenty-four things.

#### What the console shows that a Set did not publish

The mix controls — `gain`, `opacity`, `blend`, `mask` — belong to the **edge into an L5**,
not to the Set on the other end of it, so they are there whatever a Set publishes and even if
it publishes nothing. Same for `residency`, `transport` and `preview`, which are about a Set
being *played*. A Set that publishes nothing is still mixable; it just has no material
controls of its own.

And the case worth naming, because it is what the whole thing is for: a Set holding two
pipelines merged by a nested L5 can publish that L5's crossfade as **one control**. Two
scenes, one knob on the desk, and the twenty other numbers that made them stay in the file
where the author left them.

### L5 — built

L5 has been the deck's mix since M2 and has never been a `kind`. Under the node model it
becomes one, and **it has two roles rather than two implementations**:

- **The console.** The top-level L5 is what an operator sees and mixes on — gain, opacity,
  blend mode and mask per input, with a surface attached to every one of them. This is what
  L5 is *for*, and `crates/karakuri-engine/src/shaders/composite.wgsl` already is it.
- **A merge node, nested.** The same node inside a Set, folding several L4s into one
  `Texture`. Nothing an operator touches; it is there so that a Set can be *composed* of
  several rendering pipelines rather than of one.

One node kind, one shader, one set of per-input parameters. What differs between the two
roles is only whether a surface is wired to it.

**Built, and as a node kind rather than a `kind` line.** Those are two different words and
this document used one while meaning the other. A `.kir`'s `kind` says what a *procedure*
lowers to; L1 through L4 each have a block of code behind them and the two senses coincide.
An L5 has no code to lower — the compositing is fixed, `shaders/composite.wgsl` — so there is
nothing for a `kind L5` file to contain. What "under the node model it becomes one" asks for
is `crate::node::Merge`, and that is what exists: `crate::mix` holds the shader, the uniform
and the per-input controls, `Deck` mixes on it with a surface wired to every input, and a Set
folds its renderers with it and no surface at all.

A Set says which it wants with `karakuri_engine::set::Layering`, reached from the command
line as `--merge <slot>`. It is not in a Set file: the format records the *nodes* of a Set —
a `slot` per node, chain and camera included — and how the renderers meet each other is not
one of them. A saved Set therefore loads as overdraw, which is what every Set was before an
L5 could be nested.

**Which separates the mix from the deck**, and the separation is worth having because today
they are one type. `gain`, `opacity`, `blend` and `mask` are properties of an *edge into an
L5* and travel with it wherever it is nested. `residency`, `priming`, hot swap, budget
governance, `transport`, `preview` and metering are properties of **a Set being played** and
have nothing to do with mixing; they sit beside the top-level L5 in `Deck` because that is
where a performance happens, not because they belong to L5.

#### Overdraw and compositing are different operations, and the graph says which

This is what the presence of an L5 decides, and it is the difference between one render
target and several:

| Shape | What happens | Cost |
|---|---|---|
| Several L4s straight to the Set's output | **Overdraw**, in declaration order — the first pass clears, the rest load | One target |
| Several L4s into an L5 | **Compositing** — an L5 input is a texture, so each L4 needs its own | One target per input |

**They do not agree, and that is the reason the node is asked for rather than inferred.** For
renderers under `blend additive` they nearly do — addition is addition and the fold order is
the draw order either way, which `crates/karakuri-engine/tests/merge.rs` asserts in pixels.
Under `blend weighted` they do not: a weighted renderer resolves `over` onto whatever its
target holds, so overdraw composites it against the picture so far while an L5 composites it
against a clear and then folds the result. Neither is wrong, and the memory is the smaller
half of the difference.

The same cloud drawn as sprites *and* as strokes is the first shape and costs no memory at
all: additive's blend state accumulates into whatever is there, and a weighted renderer's
resolve composites `over` rather than replacing, which is the identical result on the cleared
target the first pass writes. **The first shape is built** — `Set::build_many` takes the
renderers in draw order and `crates/karakuri-engine/tests/overdraw.rs` asserts the
composition in pixels. Two scenes cross-fading is the second, and it costs 7.03 MB an input
at 1280x720 — which is what compositing costs, asked for explicitly by placing a node.

An earlier draft of `docs/roadmap.md` recommended the first shape for *every* case and said
so knowingly — *"it is also the answer that becomes wrong first: a real graph has nodes that
consume textures, and then the combine is a node"*. It was not wrong; it was the rule for one
of the two shapes, stated as if it were the rule for both.

#### What a Set is, exactly

**A named subgraph with exactly one `Texture` output.** That is the whole definition, and
everything else follows: a Set may hold several pipelines, may branch (one L1 read by several
L4s) and may merge (several L4s into an L5, several geometries into one node); a deck slot
takes one texture, so a Set has one output; and "grouping" is not a hierarchy level but a
name drawn around some nodes.

### Metadata file format — M4

Nothing writes or reads one. `karakuri-store`'s record vocabulary covers the Set file
— `set`, `slot`, `capacity`, `param`, `bind`, `camera`, `seed`, `edge`, `src` — and the
session stream, which adds `tick` and the mix and measurement records listed above, and
**none of the records below**. The store is content-addressed from
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
{"t":"perf","kind":"L1","ns_per_element":0.9}
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

**Unknown `t` values are ignored, and so is an unknown key inside a record whose `t` is
known.** The Set file section above states the first rule for its own vocabulary; this is a
separate list read by a separate decoder, so the rule is stated here rather than inherited by
being nearby. The second half is the one a *removed* key needs — `perf` carried a
`bytes_per_element` and does not any more, and a reader meeting one in a file written before
that should pass over it exactly as it passes over a `t` it does not recognise. Ignoring
costs nothing here because a metadata file is a derived artifact: the store regenerates it
from the `.kir` plus a compile pass, so a key that still means something comes back on the
next regeneration and a key that does not is gone on purpose.

#### On `perf`

The record takes a different shape per kind, because the two do not scale the same way.

**L1 is per element.** It is a compute pass over the live count, so the cost really is
linear and the cost at any `capacity` is a multiplication. That per-element figure is an
intrinsic property of the procedure, which is exactly what an artifact should record —
a total would be a property of a Set that happened to instantiate it.

**The record carries no `bytes_per_element`, and its absence is a decision rather than a
field nobody filled in.** How many bytes an element occupies is not a property of one
procedure, so a per-procedure record cannot carry it: an L2's element struct is built from
everything that reached it unioned with its own `emit`, the `copy` slot is contributed by
whichever amplifier sits *above* it in the chain, and a derivation's stored slot exists only
because something *downstream* named the attribute. None of the three is visible from a
single `.kir`. What such a record could hold is therefore a floor — `kaleidoscope`'s was 96
bytes an element against the 312 the engine allocates for it, and `swirl_warp`'s 8 against
48 — and a floor at a third of the value, published under a name that reads as a
measurement, is worse than nothing, because the record has no way to say which of the two it
is.

**The engine reports it instead, from the sizes of the buffers it created**: per node, and
totalled over a Set. That is also where the question lives. `capacity` differs per node and
an amplifier multiplies it downstream, so bytes resident is a property of the Set that
instantiated the procedures and never of any one of them — where `ns_per_element` really is
intrinsic to a procedure, which is what keeps it here.

**A record format is an authored thing in this project**, so this is a change to the
vocabulary and not a value going missing. Nothing reads `perf` yet — the metadata file is
M4 and does not exist — so there is nothing to migrate, and a decoder meeting the key in an
older file passes over it under the unknown-key rule stated above.

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

### Attribute derivation — built

When a consumer requires an attribute the producer does not emit, a rule synthesises it
instead of rejecting the Set — see [emit / consumes](#emit--consumes). Two attributes have
one, and nothing else does.

| Attribute | Derived from | Rule | What it costs |
|---|---|---|---|
| `age` | the spawn instant | `t - birth_t` | A `birth_t` slot, written once at spawn and carried. The subtraction happens where the attribute is read. |
| `velocity` | `position` | `(position - position last step) / dt` | A `velocity` slot, written by the **L1** against the step it already has. A reader sees an ordinary stored attribute. |

**The two land differently, and the reason is a decision this language already made.** `t`
is readable everywhere, so `age` can be the cheaper half stored and the subtraction done at
the read site — exact rather than accumulated, and nothing recomputed per frame. `velocity`
is a difference divided by a step, and **an L4 deliberately has no `dt`**: a renderer is not
given the simulation's step because it does not integrate. Synthesising it at the read site
would mean smuggling `dt` into every renderer's uniform to serve one rule, so the division
happens where the step already is.

**This document said `velocity` needed a third buffer. It does not.** The reasoning was
that double buffering retains only one previous frame — true — but the obstacle it points
at is worse than that and the answer is different from either: *compaction moves elements
between frames*, so index `i` is not the same element in the two buffers and differencing
them is differencing two strangers. A value carried **on** the element moves with it, which
is what `seed` and `birth_frac` have always done, and needs no map from an identity back to
a slot. One slot, no buffer, no pass.

**The always-on cost this waited for turned out to be avoidable too.** The concern was that
every procedure emitting `position` would carry the storage whether or not anything read the
derived value. It does not: the Set is the first point holding every procedure at once, so
it is the point that knows whether anything consumes the attribute, and the slot is
allocated there or not at all — the same conditional shape `copy` has.

**Nothing any node emits is ever derived**, wherever in a chain the emitter sits. An
attribute that was both would have a slot *and* a substitution for one name, and every
reader would have to know which applied where. A chain whose consumer sits above its emitter
is therefore still a composition error naming the position, which is the honest report.

**A derived `velocity` is the L1's motion and no deformation below it changes it**, which is
a consequence of where the division happens and is the reading to want: a `velocity` is how
the material is moving, and a warp is not motion but where the material is being put. An L2
that wants the other reading emits its own `velocity`, and then nothing is derived at all.

**An element's first update has no derived velocity.** A spawned element's first pass runs
over a fraction of a step and `dt` is scaled to that fraction — right for a body that
integrates, wrong for one that computes position from `t` and jumps a whole step regardless,
where the quotient is inflated by `1/birth_frac` and that is up to twice the spawn batch. An
element of a procedure with no `spawn` block has the same problem from the other end: it
starts wherever an unwritten buffer left it. Both are a difference against a state the
element was never in, so both answer zero until it has lived a whole step.

**A derivation is not readable in a `spawn` block**, and this is refused rather than
substituted: every rule reads state an element being allocated does not have yet — its spawn
instant is written after the block runs, and last step's position is a step it has not
lived.

There is no adapter *node*. Both rules are pure functions of the element and the clock once
one extra value rides along, so the synthesis is a substitution at the read site or a write
in the pass that already runs. A rule needing a reduction or a scan would need a node; these
do not, and inventing one for them would put a position in the chain that has to be
explained.

### Multiple L1 sources — built, and `source` — built

Merging several geometry sources into one Set raises the question the removal of `id` left
open: how two sources avoid colliding identities. `seed` is a monotone ordinal from a
counter that resets at Set start, so two sources either share it — and the second one's
`seed` no longer starts at zero, which breaks every structured layout — or hold one each
and collide immediately.

The resolution keeps `seed` zero-based per source and adds a per-source value that says
which source an element came from:

| Name | Type | Notes |
|---|---|---|
| `source` | `uint` | which source produced this element. A per-source uniform, never declared |

**Readable now.** The value and its home were settled before anything could read it; what
was missing was one arm in each of the three resolvers that have a uniform to read it out
of. See [Ambient values](#ambient-values-and-the-signal-rule) for where it is refused and
[A mask that names the source it applies to](#a-mask-that-names-the-source-it-applies-to)
for what it is compared against.

- Each source counts its own `seed` from zero, so `seed % side` and every other structured
  layout works identically in every source.
- The hash builtins' salt becomes **per source** rather than per layer, so `hash1(seed)`
  differs between sources automatically while `seed % 512u` stays the same in both. Two
  grids of identical shape in different colours is then the default, not something to
  arrange.
- `source` is what downstream layers mask on.

**It is a uniform, and not the fourth implicit attribute this section used to call it.** The
row above read *"implicit, carried, never declared"* — a `uint` sitting on the element beside
`seed` — and the code reached the other answer. What settles it was decided elsewhere and for
other reasons: the chain is instantiated **per source** rather than the geometries being
concatenated into one buffer, because two sources kill independently and so share no live
range to compact into, and because two sources need not agree on what they `emit`. A chain
instance therefore knows *statically* which source it is running over. A value that is the
same for every element a given instance will ever touch is a uniform by definition, and
carrying it on the element instead is a `u32` on every element of every merged Set — plus
whatever alignment it drags behind it — spent restating a constant. `Source` in
`crates/karakuri-engine/src/set.rs` keeps that reasoning beside the sources themselves.

The salt is the half of this that already exists, and it is a uniform for exactly that
reason. `Source::salt` holds one per geometry, `Set::prepare` writes it as `seed_salt` into
every L4's uniform block and `write_l2_uniforms` into every L2's, and every generated module
declares a `seed_salt: u32` field to receive it. So nothing is carried per element to make
two identical grids differ in colour, and nothing needs to be carried per element to let a
mask pick one of them out either: a comparison against a uniform is the same value in every
lane, which is the branch a GPU costs least rather than the one it costs most.

#### A `source` value is assigned and recorded, never derived

**The value is not computed from anything.** It is chosen once, when a source is added to a
Set, written into the record stream, and read back from there forever after. That is the
same footing the randomness in this language already stands on: the hash builtins are salted
from `{"t":"seed"}` and are *"not pure functions of their arguments across Sets; they are
deterministic given the record stream, which is what the invariant requires"*. `source` is
one more value of that kind, and `{"t":"seed"}` gaining a source index is most of the
mechanism.

Every derivation that looks cheaper fails on a case this system actually has:

| Derived from | Fails because |
|---|---|
| Position among the merge node's inputs | Inserting a merge upstream silently changes what `source == 1` selects, and a mask is where most of the expressive range lives |
| Position in the Set's source list | Better — the list is visible in the Set file — but reordering it still retargets every mask written against it |
| The `.kir`'s content hash | Editing one character changes it, so a save under `--watch` detaches every downstream mask. That is the loop this project is built around |
| The procedure's declared name | `proc drift_shell` is a *type* name. The same lattice twice at two scales is one name and two sources, and they collide |

An assigned value fails none of them: it does not move when the graph is edited, when the
list is reordered, when the `.kir` is rewritten, or when one procedure is used twice.

**And where it came from stops mattering once it is recorded** — a clock at the moment of
creation, a counter, a hash of anything at all. The record is the truth, so the generator is
free. That is the whole benefit of recording rather than deriving, stated as a simplification
rather than a constraint.

#### Naming a source, on the terms HTML gives an `id`

**Built, for every node and not only for a source.** `--set near=lattice_shell.kir,
far=sphere_shell.kir,morph.kir,soft_points.kir` names two geometries, and the same spelling
names an L2, an L3, a field or a renderer. What generalised it is that the argument below is
about *use* rather than about geometry: a procedure used twice is two nodes whatever layer it
sits on.

**Every node has a name whether or not one was written**, which is where this ended up
differing from the paragraph below. An unnamed *source* being unreferenceable is fine while
the only thing that wants a name is a mask; it stops being fine when the name is also what a
rebuild, an edit history and an MCP call address the node by — a node nothing can name is a
node nothing can edit. So a name nobody wrote is derived from the procedure, disambiguated
against what is already taken (`lattice_shell`, `lattice_shell-2`), and it is a real name from
the moment it is recorded — which the paragraph above already licenses: *where it came from
stops mattering once it is recorded*.

Derived in one place, `Set::build_many`, and nowhere else. A caller that derived as well would
be the second place one fact lives; `Set::node_names` is how anything else finds out what a
node ended up called. Written names are taken first and derived ones fill in around them, or a
derived name claims the one a written name further down the list asked for. Two *written*
names that collide are refused, where two derived ones are told apart — a written name is an
address somebody chose.

**And `source` is built too, which closes the gap this paragraph recorded.** It used to read
*"nothing supplies the value a mask would compare, so naming a source and masking on one are
still different distances away"* — the distance is gone, and the two halves met exactly where
this section said they would.

A value nobody can write is a value nobody can mask on: `source == 0x8a3f21c4` is not
something an author or a model produces. So a source that something wants to point at
carries a **name**, and the mask is written against the name, resolved where the Set is
built.

**Resolved into a slot, and that is the one thing the paragraph above did not predict.** The
name cannot be written in the `.kir` — a procedure that named a node would be coupled to one
Set — so what the mask compares against is a `uses only : Source` slot the Set binds by name,
exactly as it binds a geometry, a field or a camera. "Written against the name, resolved
where the Set is built" holds word for word; the notation it resolves *through* is the one
the three edges before it established.

The analogy is exact and settles two things at once. An `id` belongs to the *element*, not
to the tag — so the name is written where a source is **used**, in the Set, and not in the
`.kir`, which is why two uses of one procedure are two names rather than a collision. And an
element with no `id` is simply unreferenceable, which is fine: most sources are never masked,
and **a name is a cost you pay when you want to point at something**.

Names are unique within a Set, checked where every source is first in hand — beside the
composition check, where `ParamCollision` used to live.

**The name is an alias and the assigned value is the value.** The alternative is deriving
the value from the name, which cannot work: it covers only the named sources, and the unnamed
ones still need a salt, so the assigned value has to exist anyway — at which point the name
is an alias for it. This is the same shape "What a Set publishes" already gives parameters:
a stable internal address, and whatever the Set chose to call it on the outside.

#### Two choices here are preference rather than force

Stated so a later reader can tell them from the parts above, which are forced.

- **The value that identifies a source is the salt itself**, folded to 32 bits, rather than
  a dense index with the salt kept beside it. One value does both jobs, and a collision breaks
  both at once — cheap to refuse at build, since every source is in hand there. Where a
  human reads it, display the Set-local ordinal instead.

  **This bullet said "the *attribute* carries the assigned value", and the attribute turned
  out to be a uniform.** The preference survives the move; half of what argued for it does
  not. One value rather than two was four bytes off every element of every merged Set, which
  is the kind of saving worth stating a preference about. Two `u32`s in a uniform block cost
  nothing anyone could measure. What still holds it up is the other half — one value, one
  collision, one thing to refuse at build — which was always the better half, and the
  preference is now correspondingly cheap to reverse.
- **A name lives in a Set file. The command line can write one, and that is an auxiliary
  path.** The first half is the one that matters: a name belongs to the *use*, and a Set file
  is what a use is recorded as — so the record is where it lives, and the eventual GUI writes
  it there. The command line gets a spelling anyway (`--set near=lattice.kir,…`, and the same
  override rule `--param` already follows beside `--load-set`), because it is the only
  authoring surface that exists today and a name has to be writable before a file can be
  saved carrying one.

  **The reasoning this bullet used to carry was falsified by the code.** It said "anyone who
  needs to point at a source is already writing a Set file" — but a Set file at the time
  refused a chain and a second geometry by name, so the people with two geometries were
  exactly the people who *could not* write one. Getting a name required a file and writing the
  file required a name. (A Set file carries a chain now; the fix to the circle was this
  bullet, and the fix to the file came after it.)
  Marking this a preference rather than a force is what let it be revisited without a
  redesign, which is the whole reason that distinction is drawn.

  **`source` becoming a uniform does not reach this bullet.** Where a name is *written* is a
  question about authoring surfaces, and where a value is *carried* is a question about
  buffers. The two are independent, and a name resolved where the Set is built resolves to
  whatever the runtime holds the value in.

**Identity is a triple, and at most two thirds of it is on the element.** `source` from
here, `seed` from the source that produced the element, and `copy` from any amplifying node
above it — see [L2 amplification](#l2-amplification--built). But `source` is the uniform
above, read from the same block by every element the instance touches, and `copy`'s slot is
allocated only where something upstream amplified. The per-element cost is therefore one
`uint` on a plain chain and two on an amplified one, never three. That is also what the
amplification section has said all along — *"identity downstream is the pair of the parent
`seed` and that index"* — so the triple was the outlier, and it was the outlier because it
counted a value that was never going to be per element.

**Which retires the packing question rather than answering it.** This paragraph used to
offer packing `copy` alongside one of the others: three `uint`s, none needing a full range,
an engineering choice with no semantic consequence. Two of its three premises are gone.
There is no third value to make room for, and `copy` is already free in the case that would
have wanted the packing — `generate_element_layout` in `crates/karakuri-ir/src/layout.rs`
lays `seed` and `birth_frac` in the first eight bytes of a block that any `vec3` attribute
forces out to sixteen, so `copy` lands in padding the layout already had. A chain emitting
`position` has a stride of 32 whether or not it amplified. Packing would save nothing there
and would cost both values their plain reads. It would still save four bytes on a layout of
nothing but scalars, which has no padding to absorb the slot — a case that does not carry an
engineering choice on its own.

**The merging is built and `source` is not.** `--set a.kir,b.kir,renderer.kir` builds two
sources in one Set, each at its own capacity, each with its own salt, and the chain runs per
source. What does not exist is the value: nothing exposes a `source` uniform to a procedure,
so nothing downstream can mask on one, and the two ways to tell sources apart today are the
salt (which differs by construction) and writing different attributes in different chains.

**And the salt is assigned and recorded**, which was the provisional half and is no longer.
`--save-set` writes one `seed` record per geometry carrying the value that source was running
at, and `--load-set` hands each one back to the source its `index` names — so a Set that has
been saved keeps its colours whatever order its records arrive in, and reordering `--set`
after a save does not move them. This paragraph said assigning needed a stable *name* for a
source; it needed an *address*, which `seed`'s `index` already was. What is still derived is
a Set nothing has recorded: a bare `--set` salts each source from the Set's seed and the
source's ordinal, which the section above licenses outright — *where it came from stops
mattering once it is recorded*, and the first save is when it stops.

**The two salting rules compose, and are on different axes.** Amplification requires
`hash1(seed)` to give every copy of one element the *same* value, which is what makes eight
mirror images read as one object. Merging requires it to give *different* values in different
sources, so that two identical grids differ in colour without being arranged to. Both hold at
once: the salt varies with `source` and is constant across `copy`. A procedure that wants the
other behaviour on either axis writes it — `hash1(seed + copy * 8191u)` breaks copies apart, and
nothing breaks sources together because nothing should.

**Downstream treats sources differently by writing attributes, not by branching on
`source` in L4.** An L2 modulator masked to `source == 0` writes `tint`, and L4 renders
`tint` without learning that there was more than one source — so an L4 written against one
source works unchanged against five. An L4 that branches on `source` is coupled to a
particular Set's composition and stops being reusable.

### Combining two sources — built

Three things get called "mixing two sources" and only one of them needs anything new.

| | What it is | What it needs |
|---|---|---|
| Crossfade | draw both, blend opacity | Nothing here. Two Sets and the L5 mixer |
| Dissolve | hide one source's elements progressively | Nothing here. An L2 mask on `source` writing `size` or `tint` |
| Interpolation | pair elements and blend their attributes | **A cross-source read** — `uses <name> : Geometry` and `<name>.<attr>`, [above](#a-geometry-slot-l2-only) |

Interpolation is the only real addition, and it is not a count change — it is an
element-wise operation that needs to read *another source's* element at the corresponding
index. `seed` provides the correspondence: element 5 of source A pairs with element 5 of
source B, which is exactly what per-source zero-based seeds buy.

It fought compaction, and the restriction below is what shipped: two sources kill independently, so after compaction the
paired elements sit at different slot indices and the correspondence needs a seed-to-slot
lookup. **Restricted to sources that are static — no `spawn`, no `kill()`, therefore no
compaction, therefore `seed` is the slot index — it is a direct indexed read and costs
nothing.** That restriction is statically checkable, is what `Checked::is_static` answers,
and covers lattices and shells, which is most of what anyone wants to interpolate. Dynamic
sources can wait for a correspondence structure.

`examples/morph.kir` is the worked one: a lattice and a sphere of the same size, one fader
between them.

### L2 amplification — built

`L2 : Geometry -> Geometry` is an endomorphism, which is what makes L2 freely stackable and
also what made kaleidoscopes, instancing, trails, and subdivision inexpressible: **nothing
in the layer model could change the element count.**

The gap is filled by a second kind of L2 rather than a new layer number. Both take geometry
and return geometry, so they occupy the same slot position; what differs is the count mode,
which `Geometry` already declares.

```
kind    L2
amplify 6
```

- The factor is a compile-time constant, the same rule as loop bounds and `capacity`, so
  the output buffer is sized and the cost multiplied out statically — see
  [amplify](#amplify-l2-only) for the bounds and why there is no Set override.
- Amplifying stages are **not** freely stackable: counts multiply rather than compose, so
  each one's factor enters the estimate as a product. It is charged where the multiplication
  happens, on the node's own per-element figure, so a stage of factor 64 running an
  expensive body is refused for the reason it deserves rather than sailing through at a
  sixty-fourth of its true cost.
- Copies need distinguishing — six petals are at six positions — so a copy index is
  readable inside an amplifying block, and identity downstream is the pair of the parent
  `seed` and that index. **Stacked amplifiers compose the index rather than overwrite it**: a
  node of factor `n` turns a parent's `copy` into `copy * n + c`, which is the mixed-radix
  numbering of the whole chain. Overwriting would make two elements of one parent
  indistinguishable the moment a second amplifier ran, which is the whole of what `copy` is
  for.

Amplification is cheaper to build than its position suggests. **Its output is derived and
recomputed every frame**, so unlike an L1 buffer it needs neither double buffering nor
compaction: nothing reads its previous value and its liveness is decided upstream.

This is where the primitive-centric bet pays out on the geometry side, for the same reason
the spec already gives for drawing one point cloud several ways: one simulated element
producing six copies costs one simulation and six draws, not six simulations.
`examples/kaleidoscope.kir` is the picture.

**What it turned out to need, and none of it was in the paragraph above.** An amplifier
cannot share its input's alive buffer — its own is `factor` times as long — so it writes one,
and what it writes is each parent's flag repeated. That is not the layer deciding liveness,
which stays refused: it is the L1's decision re-indexed onto a longer buffer. **The flags are
written before the dead-slot return**, which is the whole of why a parent that dies leaves no
live copies: a killed element stays inside the live range until the next step's scan compacts
it, so for exactly one frame it is a dead slot in range, and copies whose flags were merely
left alone still draw.

It needs a `Counts` of its own for the same reason and one more. A `Counts` is three numbers
at once — the workgroups a compute pass is dispatched in, the range a pass bounds itself by,
and the instances a renderer draws — and all three multiply. **The range and the workgroup
count are separately load-bearing**, which is easy to miss: a stage below an amplifier bounds
itself by the range it was *built* against and is dispatched over the count it is *recorded*
with, so a chain can be right about one and wrong about the other, and a fixture with fewer
elements than one workgroup covers sees neither mistake.

---

## Resolved

Every decision this specification settled is in [docs/adr/](adr/), one record each, with the
alternatives that lost and the reasons they lost — element identity, compaction, `var`, spawn
quantization, substepping, sorting, runtime capacity, and the measured signals. The rules those
decisions left in force are one file each in [docs/principles/](principles/), where `ls` is the index.

What each decision produced is normative text in the sections above and is unchanged. This section
kept summaries of it until the records existed; they exist now, and a summary that restates a decision
is a second place for it to drift.

## Open questions

**None.** Which this section has been wrong about before — it said "None" while L2, L3 and
L5 were undecided, which was true of the questions someone had thought to write down and
false of the specification. So the sections above now state what is *decided* as well as what
is not, and a reader who finds no gaps here should read them rather than conclude there is
nothing to look at.

The last one standing was **how two merged geometries keep their identities apart**, carried
in `docs/roadmap.md` since before there was a compiler. It is answered above under "Multiple
L1 sources, and `source`": per-source zero-based `seed`, a per-source `source` value — a
uniform rather than the implicit attribute that section first called it — whose value is
**assigned and recorded rather than derived**, a per-source hash salt that is the same value,
and a name written where a source is used for anything that wants to mask on it.

Two things about how it was reached are worth more than the answer. Every derivation anyone
proposed — position among a node's inputs, position in the Set's list, the `.kir`'s content
hash, the procedure's declared name — failed on a case this system actually has, and the
list of those cases is in that section rather than in anyone's head. And the answer turned
out to be a mechanism the language already had: randomness here is *"deterministic given the
record stream"* rather than pure, so a recorded value was the native move and not an
addition.

The four that were open at v0.2 are recorded above with their decisions and their reasons.

The difference between a question that is open and one nobody has asked is invisible from a
list, which is why the sections above state what is decided as well as what is not. Two of
the questions this section has held were answered by *one sentence each* of what an author
expects to be able to do, and the last one by an author noticing that a content hash collides
for one procedure used twice — which is the argument for asking rather than for deriving,
made three times now.
