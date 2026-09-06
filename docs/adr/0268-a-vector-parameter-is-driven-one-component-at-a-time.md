---
id: 0268
title: A vector parameter is driven one component at a time
status: accepted
date: 2026-09-06
supersedes: []
superseded_by: []
principles: [0083]
tags: [ir, engine, set-file, parameters, spec]
---

# A vector parameter is driven one component at a time

## Context

`docs/ir-spec.md`'s `param` section lists three types — `float`, `vec2`, `vec3` — and its own
example is

```
param glow   : vec3  [0.0, 4.0] = vec3(0.4, 0.7, 1.0)
```

Every reader of that declaration discarded the three numbers. `karakuri_ir::Param::default_scalar`
folds a float literal and a negated one and answers `None` for anything else, so a `vec3` never
entered a node's value map; `karakuri-engine`'s `node::write_params` therefore packed `[0.0; 3]`
into the uniform field, and `karakuri-environment`'s `setfile.rs` refused a `Value::Vec2` or
`Value::Vec3` on a `param` record outright — *"param `{key}` was skipped: it is a vector and the
engine holds scalar parameter values only"*.

**Each of those was locally right.** The packer panics on a layout field its caller writes at the
wrong width, so `write_params` had to write *something* three-wide; the loader's *reported rather
than dropped* rule made the refusal a note instead of silence. What was wrong was the shape of the
gap: a declaration the language admits, the specification advertises and the checker accepts
reached the GPU as `vec3(0.0)` with nothing said anywhere.

`docs/contributing.md` §3 names this file's own version of it — *"the assertion built to stop a
silently-zero parameter is satisfied by writing one"* — and `uniforms.rs`'s `finish` carried that
sentence in its documentation as a description of the vector case.

## Decision

**A parameter is driven one component at a time.** A `.kir` declaring

```
param glow : vec3 [0.0, 4.0] = vec3(0.4, 0.7, 1.0)
```

gives the engine three keys — `glow.x`, `glow.y`, `glow.z` — each an `f32`, each over `[0.0, 4.0]`,
each with its own default. **The uniform stays one `vec3<f32>` field and codegen does not move**:
the four manglings in `karakuri-codegen` are `param_{name}`, `field_{slot}_{name}`,
`source_{slot}` and `usr_{name}`, none of which can produce or consume a `.`, and the two semantic
keys separate with `\u{1}`. So a component key never reaches a uniform layout field name.

`.` is the separator for three reasons, each checkable:

- **No `.kir` identifier can contain one.** `lexer.rs`'s `lex_ident` takes `is_alphanumeric() || c
  == '_'` and nothing else, so a component key collides with no declaration however it is spelled.
- **It is the language's own spelling for a component.** `check.rs`'s `check_swizzle` maps exactly
  `x`, `y`, `z`, `w`, and `docs/ir-spec.md`'s *Types* says *"Swizzles allowed (`v.xy`, `v.zyx`)"*.
- **The command line already spells *member of* this way.** `--edge <node>.<slot>=<node>`, whose
  parser splits on the last `.` because *"the slot is a `.kir` identifier, which cannot"* hold one
  (`crates/karakuri-cli/src/main.rs:1107-1114`).

### Why component addressing rather than widening the engine's value channel

**Every consumer of a parameter value is one number.** A binding resolves one `f32` per frame from
one `Sample`; a fader is one; a published control is one `[f32; 2]` range set through one `f32`; a
MIDI control change is one 7-bit value. A `HashMap<String, ParamValue>` would still have to answer
*which component* at each of those four, so the address arrives anyway and the wide type merely
defers it four times.

**The strongest objection is `setfile.rs`'s own opening**, which argues the opposite and had two
precedents to argue it from:

> Capacity was a second and `seed` a third, and both are closed the same way: each is keyed by
> node, the engine now holds one per geometry, and so each is carried rather than reported.

The answer is that capacity and seed are **one number per node**, and the format and the engine
disagreed only about *how many nodes*. A vector is **three numbers the pipeline touches one at a
time**, and the format and the engine disagree about the arity of one value. Widening the map would
carry it as far as the first consumer and no further.

### What becomes of the wide types already in the vocabulary

`karakuri_operation::ParamValue::{Scalar, Vec2, Vec3}` and
`karakuri_store::record::Value::{Scalar, Vec2, Vec3}` both stay, and neither is now dead weight:
**the wide value earns its keep on the line a person or a model writes, and the performer expands
it.** `{"t":"param","layer":"L4","key":"glow","value":[0.4,0.7,1.0]}` is one line saying the whole
colour, and `setfile::from_lines` turns it into three writes. `Operation::WriteParam { deck, param,
value: ParamValue }` is the same shape one call along: an MCP tool, when there is one, sets a
`vec3` in one call rather than three.

What is written *out* is components, one `param` record each. That is what lets a Set file record
the single component an operator moved, and it means a saved file and a hand-written one load to
the same list.

**Three shapes are reported rather than carried**, each with the component keys in the sentence,
which is [P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):

- a scalar written against a vector declaration, which names no component;
- a vector written against a declaration of another width, which is not that parameter;
- a `bind` on a bare vector key, since a binding resolves to one number and a `vec3` has three
  places to put it. Binding one component is ordinary.

The binding refusal is in `from_lines` rather than in `binding_from_record`, because whether `glow`
is a `vec3` is a fact about the *procedures*, which that function is not handed and a `--bind`
string does not carry.

### The two name lists

A node now keeps two, and telling them apart is the whole of what this costs.

- **`param_names`** is the declaration names — one `glow` — and is the uniform's own field names.
  `node::write_params` walks it, because the packer finds a field by name and no layout has a field
  called `glow.x`. It is read at that one site and there is deliberately no accessor for it.
- **`param_keys`** is `karakuri_ir::Param::keys` — `glow.x`, `glow.y`, `glow.z` — and is what
  `Set::declared_names` walks, which is what `Set::published` and `Set::bind` are built from, and
  how `Set::params` and `Set::ranges` are keyed.

**The order inside a declaration is `x`, `y`, `z`, and that order is a MIDI address.** A control is
learned against its position in the published interface rather than against a name —
`docs/manual/console.html:861-863`, *"A MIDI control is learned against **the deck and the position
in its published interface** — knob 3 is knob 3 whatever Set is loaded"* — and
`crates/karakuri/src/main.rs:6964-6970` is where that position becomes a number: `for (at, control)
in published.iter().enumerate()`, `ord: at + 1`. Components come back in the same order every run
or three knobs stop meaning what they meant.

## What it costs

**A `vec3` spends three of a procedure's recommended parameters.** `docs/ir-spec.md`'s `param`
section says *"3 to 8 parameters recommended"*, and an empty interface publishes every one of them,
so a Set whose renderer declares a `vec3` shows three rows where the author wrote one line.

That is paid on purpose. **Hiding the three behind one row would put a position in the published
interface on something no control change can set**: a MIDI CC carries seven bits and a knob learned
against position 4 would be a knob with nothing to write. The alternative was considered and lost —
see *One MIDI target for a whole vector* below.

**And no surface can write any parameter live today, scalar or vector, so this lands under a gap it
does not close.** `docs/manual/operations.html:515-524`, the *Write a parameter* row, is `plan` on
the panel, `gap` on the keyboard, `plan` on MIDI, `plan` on MCP, and `has` only on the CLI's
`--param`, which is a launch flag. Each of those was checked:

- **The panel's Inspector draws parameter rows and hit-tests none of them.** `view::param_into` is
  paint only; the Inspector's entries in `input.rs`'s `CLAIMS` are the deck head's controls — the
  mode chip, the anchor, the two scrub arrows and the fold — and there is no claim for a `.param`
  row.
- **MIDI's target enum has no param arm.** `crates/karakuri-midi/src/map.rs:117-125` is `Gain`,
  `Opacity`, `Exposure`, `MaskPosition`, `Residency`, `Blend`, `Tap`.
- **MCP has no param tool.** `crates/karakuri-environment/src/mcp.rs`'s `tools()` serves
  `read_procedure`, `write_procedure`, `wire_input`, `swap_outcome`, `save_set`, `read_set` and
  `list_sets`.

So what this decision changes is what a *declaration*, a *Set file* and a *published interface*
mean. What an operator's hands can reach is unchanged, and the three routes above are where the
component spelling will first be felt.

## Alternatives rejected

### a. Widen the engine's value channel to `ParamValue`

`Set::params` becomes `Vec<HashMap<String, ParamValue>>`, `write_params` reads one entry and packs
it at its own width, and a `param` record carries straight through. It is the smallest diff and it
has two precedents in this very file: `capacity` and `seed` were both format-finer-than-engine gaps
closed by the engine catching up.

**It loses because every consumer downstream of the map is one number, so the address arrives
anyway.** `Binding::resolve` produces one `f32` from one `Sample`; `Published` is a name, an address
and a `[f32; 2]`; `Set::set_published` clamps one number; a MIDI control change is one 7-bit value.
Each of those four would need *which component* added to it, which is the component key with the
type widened for nothing in between. And the precedent does not carry: a capacity and a salt are one
number per node, so the engine catching up meant holding more of one thing; a vector is three
numbers inside one value, so widening means holding a thing the pipeline immediately takes apart.

**What it had going for it, kept honest**: the vocabulary and the record format *already* have the
wide types, so this alternative needs no new spelling anywhere and cannot be got wrong by a
separator. That is real, and it is why `ParamValue` and `Value` keep their arms — the wide type is
right at the edge, where a file or a model says the whole vector at once, and wrong past it.

### b. One MIDI target for a whole vector

Publish `glow` as a single control and let one knob move all three components together, so a `vec3`
costs one of the 3-to-8 rather than three.

**It loses on what a control change carries.** A CC is one 7-bit number and a `vec3` is three, so
the row would either move the components in lockstep — which is a broadcast and not a colour — or
name a component after all, at which point it is three rows wearing one heading. It also breaks the
address: `published()`'s order *is* the MIDI address, so a row that no CC can set is a position in
that list that a learned knob resolves to nothing.

### c. Refuse `vec2` and `vec3` params in the checker outright

The honest way to pay nothing here. `check.rs` already says *"params may only be `float`, `vec2`, or
`vec3`"*; narrowing that to `float` would make the whole question go away, the packer would never
meet a vector field, and the value channel could stay an `Option<f32>` with no component anywhere.

**It loses to the specification's own example.** `param glow : vec3 [0.0, 4.0] = vec3(0.4, 0.7,
1.0)` is in `docs/ir-spec.md`'s `param` section as the illustration of what a `param` is, and a
colour is the obvious thing a renderer wants one for. Refusing it would be deleting a feature the
document teaches in order to avoid implementing it, and the language would then have no way to say
*this parameter is a colour* short of three declarations an author has to keep in step by hand —
which is what component addressing gives them, from one line, with the type still checked.

## Consequences

- **`crates/karakuri-ir/src/ast.rs` gains `Param::default_components`, `Param::keys`,
  `Ty::param_components`, `COMPONENTS` and `component_key`.** The literal fold is factored into one
  private `fold_literal` that `default_scalar` also calls, so the two widths cannot diverge —
  which is `default_scalar`'s own reason for living in `karakuri-ir` at all.
  `default_components` folds a literal, a negated literal, `vecN` of `N` such, and the broadcast
  `vecN(a)` that `docs/ir-spec.md`'s *Types* makes legal. **A nested constructor answers `None`**
  and the doc says so rather than leaving it to be found.
- **`crates/karakuri-engine/src/set.rs`**: `declared_defaults` inserts one entry per component;
  `declared_ranges` is lifted out of `Set::build`'s closure for `declared_defaults`' own reason —
  so the expansion can be asked without a device — and puts the declared pair under each component
  key; `declared_keys` is the shared expansion the four node kinds build `param_keys` from;
  `field_declared` is component keys where `field_params` stays declaration names.
- **`node::write_params` reads the component keys** and no longer packs zeroes. It still walks the
  *layout's* names, and the doc says why a component key reaching that loop would trip the packer's
  type assertion on the render thread.
- **`Set::bind` refuses a bare vector key with no new code.** `declared_names` carries the
  components and the value map is keyed the same way, so `bind(L4, "glow")` is `Bound::NoSuchParam`
  and `bind(L4, "glow.y")` lands on one number. The comment that said *"a node's map holds only its
  scalar params"* is corrected.
- **`crates/karakuri-environment/src/setfile.rs` expands rather than refuses.** The `param` records
  are collected in file order and turned into writes after the procedures are checked, because the
  width comes from the declaration. Its module doc's *One place the format is still finer than the
  engine* is rewritten; `docs/ir-spec.md`'s *Two places this format is finer than the engine* is now
  one place, `camera`.
- **`uniforms.rs`'s `finish` doc is corrected rather than deleted.** Its paragraph said the
  silently-zero guard was satisfied by `write_params` writing a zero for every vector param. That is
  now too strong: a component with nothing driving it resolves through the same `unwrap_or(0.0)` a
  `float` with nothing driving it does, which is the scalar case rather than a vector-shaped hole.
  The part that stays true — a write satisfies the scan — is stated as what is left.
- **`docs/ir-spec.md` moved with the code.** The `param` section now states the per-component range,
  which lived only in `ast.rs` and was in no specification at all, and the component addressing; the
  Set file section states the expansion and the three reported shapes; *What a Set publishes* states
  that a vector declaration is that many controls, in order.
- **Tests.** `karakuri-ir`'s `ast.rs` covers the fold, the broadcast, a negated component and the
  two `None` cases; `set.rs` covers the value and range maps without a device; a new
  `karakuri-engine/tests/vector_param.rs` renders a fullscreen L4 whose colour *is* the parameter
  and reads the texel back, writes one component and checks the other two did not move, and asserts
  the published order; `setfile.rs` covers the expansion, the round trip, the scalar-against-a-vector
  note and the binding refusal. `tests/camera.rs`'s
  `a_vector_param_is_left_undriven_rather_than_packed_as_a_scalar` is renamed and now asserts the
  components, keeping its record of the render-thread panic that came first.
- **This is a decision and not a principle.** Under
  [ADR-0249](0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)'s gate there is no
  question outside parameter addressing that reading it settles; the general rule it leans on — a
  refusal carries what the next attempt needs — is P-0083's and is not restated.

## What this does not decide

- **Whether any surface gains a route to writing a parameter, or in what order.** The *Write a
  parameter* row is `plan` on three surfaces and `gap` on the keyboard, and this record adds nothing
  to it beyond making the component the thing such a route would address.
- **What the panel's Inspector draws for a `vec3`.** Three rows is what `published()` hands it
  today; whether the page groups them, and what a `.param` row's name column shows for `glow.x`, is
  `docs/manual/console.html`'s under `docs/contributing.md` §5.
- **Whether `--param glow=0.4,0.7,1.0` is ever spelled on the command line.** The flag takes one
  number and a component key, which is enough; a vector spelling would be a second way to say what
  a Set file already says in one line.
- **What an artifact's metadata card says a vector default is.**
  `karakuri-environment/src/meta.rs` writes `default: p.default_scalar()`, which is `None` for a
  `vec3`, so a `param_decl` states no default while the run loads three numbers. The two cannot
  disagree — both folds are in `karakuri-ir` and share one literal reader — but the card is
  silent where it used to be silent about a value nothing had. Whether `Record::ParamDecl`'s
  `default` grows an array or a card carries one record per component is a question about that
  record's shape, and taking it here would be deciding the metadata format on the way past.
  `docs/ir-spec.md`'s *Metadata file format* says so beside the rule it qualifies.
- **`vec4` and matrix parameters.** `check.rs` refuses both and this changes nothing there;
  `Ty::param_components` answers `None` for them deliberately, so a fourth component key cannot be
  spelled by accident.
