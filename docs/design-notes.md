# Karakuri Design Notes and Architecture History

Everything above this line is implemented and tested. **Everything below carries its own
state in its heading**, because this section has outlived the sentence that used to open it:
a heading ending in *"— built"* is implemented and tested like the rest of the document, one
ending in *"— not built"* is design that has been settled and that nothing implements — no
parser accepts it, no checker enforces it, no generator emits it — and one ending in
*"— partly built"* says in its opening line which half is which.

The unbuilt half is written down because later work depends on these shapes and because
deciding them now keeps V1 from foreclosing them. The built half stays here rather than
moving up because the reasoning is what makes each of them worth reading, and cutting a
section into its rules and its argument would put the two in different places.

Checked against a symbol rather than from memory, which is how the mismatch this note
replaces came to be: `Kind::L2`, `Kind::L3`, `set::Published` and `node::Merge` all exist,
so the four sections naming them are built.

### L2 and L3 — built

The [layer algebra](#kind) was settled before there was a compiler, and everything about L1
and L4 was decided by building them. L2 and L3 have been "later" for long enough that the
vagueness became load-bearing: the engine is being split so that `Ln` is a node, and the
shape of an L2 node and an L3 node constrains that split now rather than when they are
written. What follows is what is decided, why, and what is not.

#### L3 — the camera

**What crosses the edge is camera state, not a matrix**, and the reason is multiplicity.
Mixing two L3s is a weighted blend of trajectories — an orbit and a handheld rig at 0.3 —
and a lerp of two view-projection matrices is not a projection of anything. So an L3 node
produces the six numbers a camera *is*:

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
expects a camera to do: *"follow the first vertex"*. A camera that follows an element takes
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
record — *"jump on the beat at random while continuously facing the center"* — happens to be
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
  codegen time, the way a `Field` is spliced into whichever procedures evaluate it. Left
  unstated, fusion would rest on a reading of the language rather than on a rule — the same
  distinction `fullscreen`'s empty `consumes` draws.

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
which is the shape that survives a Set that is a graph. Fusion is then an optimisation a
graph compiler may apply, rather than the only implementation there is.

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

#### What an L3 can point at: an element by age, or a reduction

An L3 reads geometry through a `Geometry` slot, bound by an `edge` to an L1 exactly as an
L2's `far` is:

```
proc follow_eye {
  kind L3
  uses subject : Geometry
  consumes position

  camera {
    eye    = subject[0u].position;
    target = mix(subject[1u].position, subject.centroid, vec3(0.2));
  }
}
```

```
{"t":"edge","node":"follow_eye","slot":"subject","to":"ribbon_chase"}
--edge follow_eye.subject=ribbon_chase
```

| Read | Type | Cost per frame | What it is |
|---|---|---|---|
| `subject[i].<attr>` | the attribute's | one buffer read | the element `i` places after the oldest living one |
| `subject.centroid` | `vec3` | one reduction pass over `capacity` | mean `position` of the live elements |
| `subject.bounds_min`, `subject.bounds_max` | `vec3` | the same pass | per-axis min and max of `position` over the live elements |

**`i` is an age rank, not a seed.** Compaction is order preserving, so the live range is in
spawn order and index 0 is **the oldest living element** (ADR-0099); `[i]` is the `i`-th
after it. In a procedure with no `spawn` and no `kill()` nothing moves and `i` equals
`seed`. `i` is a `uint` expression and may change every frame. An `i` at or past the end of
the live range reads its last entry, the youngest element. The range is the one the step
left: an element killed in that step is still in it, and is read by index until the next
step compacts it away. The reductions skip it.

**Which element is the L1 author's decision, and nothing checks it.** An L1 that expects to
be watched gives its low indices meaning — spawns them first, never kills them, makes them
the leader the rest follow, or places invisible camera points there. One that does not care
still offers its oldest survivors.

**`consumes` lists what the camera reads through its slot**, as an L2's `consumes` covers
`far.<attr>`. `subject[i].<attr>` is refused unless `<attr>` is consumed, and a reduction
is refused unless `position` is. The Set refuses a camera whose source neither emits nor
synthesises what it consumes. Without a geometry slot, `consumes` on an L3 is refused.

**What is read is the L1's buffer as its step left it**, before any L2 in its chain. The
camera pass runs after every source's step and before any renderer draws, so an element
and the camera that follows it are from the same frame.

**The reduction pass runs only when the camera block reads a reduction.** It is one
workgroup striding over the live range, bounded by the source's `capacity`, and the L3's
estimate charges it as `REDUCTION_OPS_PER_ELEMENT` ops per element of that source. A camera
that reads only `subject[i]` pays one read per access.

**When the live range is empty, the camera block does not run** and the camera keeps the
state it had the frame before. A camera that reads geometry starts at the built-in orbit's
placement, so one whose subject has never had a living element shows what the orbit would.

**When element 0 dies, `subject[0u]` becomes the next oldest.** For a fountain that is
continuity; for a cloud with a deliberate anchor it is a jump, and the fix is upstream: do
not kill the anchor.

**An L3 declares at most one `Geometry` slot.** Reading a second source is not built.
**Several L3 nodes may bind the same source.** A camera that reads geometry is seekable
exactly when the source it reads is.

**`[ ]` is legal only after a `Geometry` slot name in an L3.** The language has no arrays;
this is element addressing, and it is spelled as indexing because that is what it is.

Decided in [ADR-0379](adr/0379-an-l3-reads-its-geometry-by-age-rank-and-by-reduction.md),
which extends ADR-0099's element zero to any age rank.

### What a Set publishes — built

A Set publishes every `param` every procedure in it declares, addressed by the node that
declares it. With one L1 and one renderer that is about nine controls and the addressing is
invisible — a bare name still reaches them, because it reaches every node that declares it.
With a graph — several pipelines, L2s, an L3, a nested L5 — it is twenty-five, most of
which are **authoring decisions the Set's author already made** rather than anything an
operator wants under their hands at two in the morning.

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
one per declaration. **A vector declaration is that many keys**, so `param glow : vec3` is
three controls — `glow.x`, `glow.y`, `glow.z`, in that order, adjacent in the list because
they are adjacent in the declaration. The order is an address: a MIDI control is learned
against its position in the published interface, so the components have to come back in the
same order every run. Per declaration would put two controls called `exposure` on a console,
which is a shape `publish` itself refuses. A wildcard control's range is the **intersection**
of what its nodes declare — one knob moving both must not offer a position only one of them
said it still looks like itself at.

It is not in a Set file yet, and it is now the only one of those four left out: a chain saves
as a `slot` per node, a camera as the `camera` record, and a layering as the `merge` record.
But it **is** in a `Request`, so it survives a hot swap. Leaving it out was worse than a lost
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
does — so the check as written forbade exactly what a Set of several nodes is for. **The
engine now keys values by the node that declares them and the error is deleted**, which is
the internal half of the address the graph needs anyway.

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
it publishes nothing. Same for `residency` and `transport`, which are about a Set
being *played*. A Set that publishes nothing is still mixable; it just has no material
controls of its own.

And the case worth naming, because it is what the whole thing is for: a Set holding two
pipelines merged by a nested L5 can publish that L5's crossfade as **one control**. Two
scenes, one knob on the desk, and the twenty other numbers that made them stay in the file
where the author left them.

### L5 — the built-in mix, built

**This section is about `crate::node::Merge` and not about the kind.** `kind L5` is built —
[The `frame` block (L5)](#the-frame-block-l5) is the specification, in the present tense above
the boundary — and the built-in mix is what keeps its place beside it, exactly where the
built-in orbit camera keeps its place beside `kind L3`. What is below is that node.

L5 is the deck's mix, and it was not a `kind` for as long as the compositing was fixed. Under
the node model it is a node, and **it has two roles rather than two implementations**:

- **The console.** The top-level L5 is what an operator sees and mixes on — gain, opacity,
  blend mode and mask per input, with a surface attached to every one of them. This is what
  L5 is *for*, and `crates/karakuri-engine/src/shaders/composite.wgsl` already is it.
- **A merge node, nested.** The same node inside a Set, folding several L4s into one
  `Texture`. Nothing an operator touches; it is there so that a Set can be *composed* of
  several rendering pipelines rather than of one.

One node kind, one shader, one set of per-input parameters. What differs between the two
roles is only whether a surface is wired to it.

**Built, and as a node rather than as a `kind` line.** Those are two different words and this
document used one while meaning the other. A `.kir`'s `kind` says what a *procedure* lowers
to, and `crate::node::Merge` is a node with **no procedure**: `crate::mix` holds the shader,
the uniform and the per-input controls, `Deck` mixes on it with a surface wired to every input,
and a Set folds its renderers with it and no surface at all. There is no `.kir` behind it and
there is not meant to be, which is why it gets no `slot` record and is described by a `merge`
record of its own — the arrangement `camera` already established for the built-in orbit.

**What that used to be a statement about was the fixed compositing**, and the compositing is
no longer the whole of L5. *The mixer is an L5, and so is a master effect* reads this node's
own signature, `[Texture] -> Texture`, as one kind with a built-in instance: **a master effect
is that signature with one input**, the mixer is it with several, and admitting frame effects
was *giving L5 a writable form* rather than extending the algebra. That form exists now: fan-in
is answered by `uses … : Texture` plus `edge`, and how many inputs an L5 folds is a property of
the procedure rather than of one built-in shader. **What did not change is this node**, which
is what that paragraph promised would survive: `L5` joins the kinds above, `crate::node::Merge`
stays exactly where the built-in orbit camera stays, and the `merge` record keeps describing it
for the reason the `camera` record describes that. **Feedback is the one effect that does not
follow**: reading the previous frame is a cycle, and which cut is read is answered where a pass
is instantiated rather than in the file that declares `retains` — see
[L5's chain](#l5s-chain--built-and-its-surface-is-not), which is the half that is still nobody's.

A Set says which it wants with `karakuri_engine::set::Layering`, reached from the command
line as `--merge <slot>`, **and a Set file records it** — the `merge` record, see [Set file
format](#set-file-format). It was kept out on the rule that the format records the *nodes* of
a Set — a `slot` per node, chain and camera included — and not how the renderers meet each
other. `edge` ended that rule by recording exactly how two nodes meet, and what stands in its
place is the reading `camera` already established: a node with no procedure gets no `slot`
record and is described by a record of its own, an L5 having no `kind` for the same reason it
has no code to lower. Until then a saved Set loaded as overdraw whatever it was built as —
which is what every Set was before an L5 could be nested, and which left a saved variant pool
unselectable, since a `select` needs an L5 to be about.

**Which separates the mix from the deck**, and the separation is worth having because today
they are one type. `gain`, `opacity`, `blend` and `mask` are properties of an *edge into an
L5* and travel with it wherever it is nested. `residency`, `priming`, hot swap, budget
governance, `transport` and metering are properties of **a Set being played** and
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

The first shape was once the recommendation for *every* case, with the objection to it
already attached: it is the answer that becomes wrong first, because a real graph has nodes
that consume textures, and then the combine is a node. That was not wrong; it was the rule
for one of the two shapes, stated as if it were the rule for both.

#### What a Set is, exactly

**A named subgraph with exactly one `Texture` output.** That is the whole definition, and
everything else follows: a Set may hold several pipelines, may branch (one L1 read by several
L4s) and may merge (several L4s into an L5, several geometries into one node); a deck slot
takes one texture, so a Set has one output; and "grouping" is not a hierarchy level but a
name drawn around some nodes.

### L5's chain — built, and its surface is not

**The list is built and its surface is not**, which is the whole of what this section now is.
As of M5.16's IR/codegen pass `kind L5` parses, checks, costs and lowers to a fullscreen pass —
that half moved above the boundary and lives at [The `frame` block (L5)](#the-frame-block-l5) —
and as of M5.16's engine pass the master chain **is** the ordered list below: slots, cuts,
retentions and the record. `examples/feedback.kir`, `examples/bloom.kir` and
`examples/rgb_shift.kir` are what runs there, `karakuri-engine`'s `master.rs` holds no
hand-written effect, and `shaders/master.wgsl` is down to the one pass that writes a retention.

**What is left is the surface**, and it stays here rather than moving up: the three named rows
of the Master bay, *Add* and *Remove*, the Library's drop onto the chain, and `+ add` becoming a
control. A **Set's** nested L5 is left with it — `compile::sort_compiled` still refuses one where
a slot is loaded, with the reason. The record is
[ADR-0340](adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md) and
what remains is [roadmap.md](roadmap.md)'s M5.16 second pass.

**Which cut a slot answers `retains` with** — `mix` or `exit` — is this section's and not the
kind's, which is [`retains`](#retains-l5-only)'s own point: the file declares, the place that
instantiates it answers, and the place that answers is a chain slot.

- **`mix`** — the frame as the mixer wrote it, before the chain touched it. One echo and not a
  trail.
- **`exit`** — the chain's own output, after the last slot and before the tone map. A trail,
  because what is read back already contains it.

**The engine allocates a retention only where a slot's answer names one**, which is ADR-0317's
own rule and the reason the declaration exists at all: a cut that is read has to be held, and
holding one nothing reads is the cost nobody would pay
([P-0091](principles/0091-cost-is-known-before-it-is-paid.md)). At most two are ever allocated —
one per cut — however many slots declare `retains`, because two slots reading `exit` read one
frame. A `cut` on a slot whose procedure does not declare `retains` is refused rather than
ignored, and a slot whose procedure declares `retains` and whose record carries no `cut` is
refused the way an unbound slot is.

**Both cuts are copied at the chain's end, and the position stopped mattering.** ADR-0317 copied
the `mix` cut *between* two passes, because the mix's own target was also the second pass's
destination and the copy had to be recorded while it was still true. A list cannot honour that:
the slot that reads the cut may sit anywhere. So the **entry** — what the mix writes into — is
held apart from the pair the rest of the chain ping-pongs between, the frame as the mix wrote it
survives the whole chain, and a slot reading `mix` reads the *previous* frame's mix wherever it
sits. The two pictures ADR-0317 named are the same two bytes for byte.

**A retention is written by a pass and not by a `copy_texture_to_texture`**, which is what
sanitising it *where it is written* costs: `master.wgsl`'s `keepable` is the whole of what is
left of that file, and it is why a `.kir` neither needs the guard nor can leave it out. The pass
is a `textureLoad` and a `select`, exact for every value that is not a NaN or an infinity, so a
retained frame is still bit for bit the frame it was taken from.

**A chain slot's L5 declares no `uses`.** The master chain is an ordered list and its only
fan-in is that order; there is no Set for an `edge` to be written in, so a procedure with a
Texture slot is refused *from a chain slot* rather than from the language. That refusal is
`karakuri_engine::master::Slot::build`'s, beside the two that pair `retains` with a cut, and each
names the fix rather than the rule.

#### The two roles

[ADR-0098](adr/0098-l5-is-one-node-kind-with-two-roles.md)'s split survives the kind intact, and
what differs between the two roles is still **only whether a surface is attached**:

- **A master chain slot.** What an operator rides. One input, no `edge`, its params on the Master
  bay's rows.
- **Nested in a Set.** Several `uses … : Texture` slots bound by `edge`s, folding renderers into
  the one `Texture` a Set outputs, with no surface at all.

`gain`, `opacity`, `blend` and `mask` remain properties of **an edge into an L5** and travel with
it; residency, priming, hot swap, budget governance, transport and metering remain properties of
**a Set being played**. A written L5 changes neither list.

#### The master chain is an ordered list of L5 slots

**A slot holds one L5 procedure, its params, and its `cut` where the procedure declares
`retains`.** The chain is the ordered list of them, `src` of the first slot is what the mix
wrote, `src` of every other slot is what the slot before it wrote, and what the last slot writes
is what the tone map reads.

**It is a master setting and not a Set's**, which is
[ADR-0227](adr/0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md)'s
reading unchanged: the chain sits one level out from every Set on the panel, so a Set that
carried it would reconfigure the master the moment it was loaded into any deck. It is not a deck
slot's either — it reads what the fold produced, after every deck's edge has been applied. There
is one chain.

**The default chain is empty**, and that is what keeps the default look free: with no slots the
mix writes straight into the target the present pass reads and the frame is bit for bit the frame
this program drew before any of this existed.

**An amount of zero is not a skip.** The three fixed passes were always loaded, so
ADR-0317's rule — an amount of zero records no pass at all — was the only way to stop paying for
one nobody was using. A list has a better spelling for *no pass*: no slot. So a slot that is in
the chain runs, at zero as at one, and what a chain costs is the slots it holds rather than a
function of the values in them. That is the shape P-0091 prefers on its own terms — a constant
cost rather than one that appears and disappears under a hand riding a fader through zero — and
it is what lets the estimate below be a number rather than a bound.

#### What the three shipped procedures cost, and what bloom gave up

`examples/feedback.kir`, `examples/bloom.kir` and `examples/rgb_shift.kir` are built, and their
figures are at [The cost of an L5](#the-cost-of-an-l5). **Two of the three are the hand-written
bodies term for term** — `fs_feedback` and `fs_rgb_shift` — and
`crates/karakuri-codegen/tests/naga_test.rs` asserts each expression against `master.wgsl`'s.

**Bloom is not, and the gap is structural rather than awkward.** The hand-written bloom is
**two** passes with an intermediate target between them — bright-and-blur-x, then blur-y-and-add
— and the second half needs **both** the bright buffer *and* the frame it was taken from. A
chain slot's output replaces the frame, so by the time a second slot ran the frame it needs
would be gone; letting it name the earlier value is a graph, and the chain is a list. Expressed
as one pass the separable 9-tap blur becomes a 9x9 kernel: **81 fetches per texel where the pair
costs 19**, at the same radius and the same picture. **M5.16 owes a fresh measurement** —
ADR-0317's 0.80 ms for bloom was taken on the two-pass form and does not describe this one. What
would close the gap is a procedure that can name a second output, which is not taken here and is
not owed by anything.

**What the language buys back on the same page**: bloom's knee and radius stop being constants
in a shader nobody can see and become **literals in a file anybody can read**. They are still
not controls, and ADR-0317's argument for that is untouched — the tap count is what a radius
costs — but a different radius is now a different procedure, priced when it is compiled instead
of refused as a knob.

**The measurement is taken and the answer is a fifth.** At 1280x720 on this machine (Metal,
Apple M4 Pro, 2026-09-10) the single 9x9 pass costs **0.92 ms** against ADR-0317's **0.80 ms**
for the separable pair — 1.2x for 4.3x the fetches, because eighty-one taps into one frame hit
cache where a second pass hits bandwidth. So nothing is owed a second output: the shape that
would close the gap costs more to specify than the gap is worth
([ADR-0303](adr/0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md),
and `examples/master_cost.rs` is what took it).

**What the same measurement says about the estimate is sharper.** `cost.rs` prices this pass at
**3889** ops per fragment against a ceiling of 4096, 95% of it, and prices the whole three-slot
chain at 3947 — and the GPU says the three together cost 0.98 ms, 5.9% of a 60 Hz frame. The
weights behind that figure (`texel` 2, `tap` 4, `frame_step` 2) over-predict a cached tap by
about four to one on this hardware, which is the question ADR-0340 said a GPU would have to
answer. Re-weighting is a change to `cost.rs` and to every artifact's price with it, so it is
named here and not taken.

#### What a chain costs, which is a chain's question and not a slot's

**A chain's cost is the sum over its slots, and that is not the addition ADR-0013 forbids.** The
rule there is that three *different* quantities must not be added; these are rates against
**one** quantity — the frame's texels, covered once by every slot — so the sum is what the chain
costs per texel and means exactly what one slot's figure means. It is stated here because it
looks like the forbidden thing.

**In milliseconds, at the output's size.** A frame is rendered once at the largest enabled
output's size and scaled into the rest (ADR-0247,
[ADR-0325](adr/0325-the-frame-follows-the-largest-enabled-output-and-a-resize-costs-0-145-ms.md)),
so the chain's price is its ops per texel against **that** area, and it moves when an output is
enabled. A measurement of it names which resolution it is about
([ADR-0303](adr/0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md)).
That is the number the governor spends where the estimate answers
([ADR-0296](adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)),
and the chain is charged against the frame beside the decks rather than against any one of them.

**Where each ceiling is checked is the [validation pipeline](#validation-pipeline)'s own
division**, extended and not amended: stages 1–5 are per artifact, so stage 4 rejects one L5
above the per-fragment ceiling and knows nothing about a chain, and stages 6–8 are per Set and
per frame, which is where a chain of slots has a total at all. The **total exists**:
`Present::chain_ops_per_fragment` is the sum over the slots and is read back off the running
chain. **What spends it does not**: nothing sets an estimate on this side of the frame today
(ADR-0325 recorded the same gap for a slot's), so the governor is handed nothing and the number
is owed a reader by M5.16's second pass.

**Memory is a chain property too.** The **entry** where the chain has any slot, plus one target
per slot that is not the last up to a ping-pong pair, plus one per retained cut some slot asks
for — so nothing at all for an empty chain, 7.03 MB for one slot at 1280x720, 21.1 MB for three,
and 7.03 MB more per cut. They are allocated when the list is installed and when the frame is
resized and never when a parameter moves, which is P-0091: a record whose *shape* is the shape
already running is applied as a `queue.write_buffer` per slot and allocates nothing, and only a
different list is a build.

#### Records

**The session record is `master_chain`, one record written whole, and it grows a list where it
had four scalars.**

```ndjson
{"t":"master_chain","slots":[
  {"proc":"sha256:a3f2c1…","cut":"exit","params":{"amount":0.5}},
  {"proc":"sha256:77b9e0…","params":{"amount":0.8}},
  {"proc":"sha256:1d4a8f…","params":{"amount":0.2}}]}
```

- **Whole, and ADR-0317's argument for that is unchanged**: a stream that moved one slot without
  saying where the others stood would describe a chain a replay could not put back. The place that
  turns an operation into this already knows the chain that is running and fills the rest in,
  exactly as `karakuri-cli`'s `set_exposure` fills in the operator
  ([ADR-0192](adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
- **`proc` is a content address**, the `slot` record's own spelling. So the *procedures* become
  part of the state a replay reproduces: a stream naming a procedure the store does not hold is
  refused with the address in the message rather than played with a different chain.
- **`cut` is present exactly where the procedure declares `retains`**, carrying `mix` or `exit` as
  a word for `Record::Blend`'s reason — a word an older build does not know is the engine's to
  diagnose against what it supports rather than a parse failure that takes the line with it.
- **The four-field form is not read**, and the refusal names the key it met.
  `{"t":"master_chain","feedback":…,"cut":…,"bloom":…,"rgb_shift":…}` is what streams recorded
  between 2026-09-09 and M5.16 carry, and a build that read both would be carrying two shapes of
  one record for the length of the project. Before v1 a compatibility cost is a bill rather than
  an argument
  ([P-0085](principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)); the bill
  here is those sessions. What was owed for it is paid as `#[serde(deny_unknown_fields)]` on this
  record and on no other — `unknown field 'feedback', expected 'slots'`, with the line number —
  because here an unrecognised key is not a field from the future, it is the old form, and
  dropping it the way this format drops every other would play an **empty chain** where the
  session had three passes.
- **The retained frame stays part of closed state**, which is ADR-0317's determinism paragraph
  with one word changed: a **pass** over a target this chain wrote, from a parameter a record
  carries, into a target that reads as zero until it is written (P-0092). The pass sanitises and
  does nothing else, so it is still the copy for every value that is a number.

**A Set file gains nothing for the chain and one line for a nested L5.** The chain is not Set
state and `Store::write_set` refuses it, on ADR-0227's argument. A **written** nested L5 is a node
with a procedure, so it gets a `slot` record like every other node —
`{"t":"slot","layer":"L5","proc":"sha256:…"}` — and the built-in mix keeps the `merge` record,
which is [Several cameras](#several-cameras)' arrangement exactly: a written camera has a `slot`
and the built-in orbit has a `camera` record.

**The library tier is the same list under a name.** ADR-0227 put a chain setting in two tiers and
left the file to *the record that has something to serialise*; this is that. What is saved is the
`slots` array above and nothing else, on
[ADR-0221](adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)'s
shape — a name the operator typed, one path component, a place of its own under the store root,
bytes the store hands over whole. The directory's name and the shipped tier's contents are
[roadmap.md](roadmap.md)'s *Mx — TODO* entry, which is where that half already sits.

#### What is forced and what is chosen

**Forced.**

- The chain runs in linear HDR, unclamped, upstream of the one tone map and the one sRGB encode,
  so an L5 runs **before** the tone map
  ([P-0064](principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
- Every length is a fraction of the frame's height and never a count of texels, which is why
  `frame_step` exists and why no ambient carries the render size (P-0086, ADR-0247).
- A procedure knows only what it declares: `src` is implicit, the cut is answered where the slot
  is, and a `.kir` names no node (P-0086).
- The retained frame is part of what a replay reproduces (P-0092).
- Targets are allocated when a list is installed and when the frame is resized, never when a
  parameter moves (P-0091) — which is why applying a whole record whose shape has not changed is
  a uniform write and only a different list is a build.
- The cost is computable before anything is built, which is what the literal loop bound is for
  (ADR-0252), and the three axes are not summed (ADR-0013).
- One kind with two roles, one implementation (ADR-0098).
- One record, written whole (ADR-0317).

**Chosen, and a wrong one here is a one-clause revision.** The block's name (`frame`) and the two
reserved names in it (`src`, `held`). The three builtins' names and shapes. `retains` as a bare
declaration with the cut answered by the slot, rather than `retains mix` in the file. No zero-skip.
A chain slot's L5 declaring no `uses`. `uses … : Field` refused on an L5. One chain, at the master,
rather than one per deck. The shipped `bloom.kir` as a single 81-tap pass, which is now measured
rather than feared. And that the four-field `master_chain` record is refused rather than read.

### Metadata file format — partly built

**Four of the nine records below are written and one tool reads them back; five have no
producer.** Both of the paths that store an artifact *from a compile* write a
`<hash>.meta.ndjson` beside its `.kir`, carrying `meta`, `param_decl`, `capacity_decl` and
`emit` — which is exactly what *"the `.kir` plus a compile pass"* can answer, because every
one of them is a declaration the check pass already resolved. `Store::put_artifact` itself
writes no card and is not one of those paths: the store holds bytes under their address and
does not compile, so the card is the caller's to write — `karakuri-cli`'s `put_meta`, which
`Placed::put` and `Sources::into_nodes` both go through. A `put_artifact` with no card
after it leaves a store that is uncarded rather than damaged — `Store::read_meta` says so,
and `crates/karakuri-store/tests/store.rs` both puts artifacts that get no card and writes a
card beside one by hand.

`karakuri-store` decodes all four, `Store::write_meta` and `Store::read_meta` are the file,
and `Store::write_set` refuses them from a Set file **on the same terms as a `tick` and for a
different reason, so with a sentence of its own**: a `tick` is refused because a Set file
carries no time, and these because a Set file records what a value *is* where a declaration
says what a procedure *declares*. The check is `Record::is_metadata` and not
`!Record::is_set_state` — the latter refuses them too, but under the message naming a tick,
and telling an operator a `capacity_decl` was rejected for carrying time sends them looking
in the wrong place.

**Two readers exist, and between them a library can be found rather than only opened.**
MCP's `read_set` resolves a saved Set's nodes to their cards and renders what each procedure
*declares*, so a model can weigh saved material without fetching and compiling every source;
`list_sets` is what makes that reachable, because `read_set` answers about an id its caller
already has and nothing could say which ids exist. It lists every Set the store holds, most
recently written first, filtered by what a node is called or by which layer a Set has a node
on — and `--list-sets` prints the same answer at a terminal. **What a node is called is one
function's answer**, `setfile::node_called`, so a name read in a listing is the name found on
opening that Set.

What is still missing is what the *cards* would let a library do and cannot yet. Search is
over node names because that is what a card can be searched on today: `origin` and `tag` have no
producer, so nothing finds a Set by the prompt that made it or by a word somebody filed it
under. Nothing groups, and nothing walks genealogy, which needs a `parent` that nothing
writes.

**The other five wait on a producer, and an empty one would be worse than none.** `perf`
is a measurement and wants the stage-7 probe — what a compile pass can answer is
`ops_per_element`, from `cost::estimate`, a different quantity in different units, and
publishing one under the other's name is the mistake this section already withdrew a
`bytes_per_element` for. Not `Checked::cost`, which is the field named for that number and
is unconditionally `None`: the estimator returns its answer to a caller rather than storing
it there, and its own doc says to ask `cost::estimate` instead of reading the field.
`origin`, `parent` and `tag` describe where an artifact came from, and nothing generates
procedures yet; `thumbnail` names a stored asset and nothing renders one. `parent` in
particular has to start being recorded with the first generated artifact or the genealogy
has a hole at its root — which is a demand on whatever writes the first one, not a line to
write empty now.

The store is content-addressed, which is half of what a library needs of it. This is the
producing half of the other, with a lookup over one Set and a listing over all of them;
what is still missing is grouping and genealogy, and a search over more than a node's name.

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
{"t":"thumbnail","path":"thumbnails/a3f2c1….mp4"}
```

The store regenerates metadata from the `.kir` plus a compile pass, so metadata files are
reproducible artifacts rather than hand-authored ones.

**A `param_decl` may omit `default`, and that never means the param has no default.** The
grammar makes `= <expr>` mandatory, so every declared param has one; the writer folds a
literal with an optional leading sign and states nothing it cannot fold. The alternative —
omitting the whole record — was rejected: it would tell a reader the artifact declares no
such param, which is false, where a missing key says only that the value is not known,
which is true. It also has to survive the unknown-key rule above. A decoder that skips keys
it does not recognise cannot tell an absent `default` from one it skipped, and under both
readings the answer is *"not known"*; under the other choice the two readings differ,
because a record that is not there cannot be skipped into existence.

**A vector `param` is one such omission today, and it is a gap rather than a rule.** `default`
is one number and the engine now loads one per component — `glow.x`, `glow.y`, `glow.z`
([ADR-0268](adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)) — so a card for
a `vec3` states no default while the run has three. The two cannot *disagree*, because both
folds are in `karakuri-ir` and share one literal reader; the card is silent rather than wrong.
Closing it is a question about this record's shape — whether `default` grows an array, or a
`param_decl` becomes one record per component — and nobody has taken it.

**`capacity_decl` and `emit` are absent where nothing was declared**, on the same rule: one
record per declaration, and a declaration nobody wrote produces none. Only an L1 declares a
capacity, and a renderer emits nothing — an `emit` with an empty list would read exactly as
its absence reads, and would be a line somebody has to explain.

**The number in a `param_decl` is the number the engine loads.** Both are the declared
expression folded, and they are folded by one function in `karakuri-ir` rather than one
apiece: a writer with a fold of its own agrees with the shader by coincidence, and the
coincidence breaks first on a negative default, which parses as a negation of a literal
rather than as one. A card claiming `0.0` where the run loaded `-0.35` would describe a
procedure nobody ran, with nothing downstream able to say which of the two was wrong.

`origin` records what produced *this* revision, not the original intent. When an artifact
came from revising another one, `parent` points at what it was revised from and `prompt`
holds the instruction that did it — "more aggressive, red", not the paragraph that started
the lineage. Walking `parent` recovers the whole path; storing only the final wording loses
it. Anything else fed to the generator belongs here too, for the same reason: two artifacts
from the same prompt that differ because different reference material was supplied are
otherwise unexplainable.

**Why this vocabulary is shaped as it is.** One Rust enum carries all three vocabularies
rather than one per file, because a separate enum makes a metadata record decode as
`Unknown` and the format promises to *pass those over* — so a Set file could not refuse one
([ADR-0135](adr/0135-one-record-enum-and-one-classifier-behind-both-predicates.md)). The
store keeps bytes and whoever compiled them writes the card, because `karakuri-store` has no
compiler and should not grow one
([ADR-0136](adr/0136-the-store-keeps-bytes-and-whoever-compiled-them-writes-the-card.md)). A
default that does not fold writes the key absent rather than the record absent, because a
missing record says *no such parameter*
([ADR-0137](adr/0137-an-unfoldable-default-writes-the-key-absent-not-the-record-absent.md)).
And `thumbnail` is not called `preview` because that name was a deck record with a
different shape ([ADR-0134](adr/0134-the-metadata-preview-becomes-thumbnail.md)); the deck
record has since been retired outright (ADR-0240) and the name stays `thumbnail` anyway.

**One `t` means one shape, across every file.** A metadata file describes what an artifact
*declares*; a Set file records what a value *is*. Those are different records, so they get
different names — `param_decl` and `capacity_decl` here, `param` and `capacity` there. The
alternative, reusing a `t` for two different shapes in two different files, cannot be read
by a decoder that dispatches on `t` alone, which every ndjson reader does. It is not enough
for the two vocabularies to be disjoint in practice; they have to be disjoint by name.

**The library asset is `thumbnail` and not `preview`, because the deck had a `preview`.**
This list carried `{"t":"preview","path":…}` while the session vocabulary carried
`{"t":"preview","slot":2}` — two shapes under one `t`, which is what the rule immediately
above forbids, and a *silent* collision rather than a loud one: the deck record's `slot` was
optional, so a decoder holding one vocabulary read the metadata line as the deck's and
dropped the `path` without a word. `thumbnail` is the name the stored asset carries
everywhere else — the store keeps one under `thumbnails/` — and it drew the distinction the
collision was hiding: the deck's `preview` rendered a running instance, a thumbnail is a
stored asset. They were two words for two things everywhere but here.
**The metadata name is the one that moved**, because
nothing wrote or read it and no file anywhere held that line, where the deck's `preview`
was written into session streams and renaming *that* would have broken reading them. A
suffix — `preview_path` — was the other candidate and was rejected: a suffix says two
versions of one concept, which is what `param_decl` beside `param` and `capacity_decl`
beside `capacity` are, and a stored asset beside a live audition is not that.
**The deck's record is gone now** (ADR-0240) and `preview` names nothing, which changes none
of this: the rename already happened, `thumbnail` is the right word on its own terms, and
reusing a freed name would put a third meaning on it.

**Unknown `t` values are ignored, and so is an unknown key inside a record whose `t` is
known.** The Set file section above states the first rule for its own vocabulary; this is a
separate list read by a separate decoder, so the rule is stated here rather than inherited by
being nearby. Both halves hold in the code, and the *decoder* being separate is not the same
question as the record type being separate: the four implemented records share one Rust enum
with the Set and session vocabularies, because that is what lets a `param_decl` met in a Set
file be **recognised and refused** rather than passed over as a `t` nobody knows. The rule
this section states — one `t`, one shape, across every file — is a rule about names, and the
names are disjoint across all three vocabularies: `param_decl` beside `param`,
`capacity_decl` beside `capacity`, `thumbnail` beside what was the deck's `preview`. The second half
is the one a *removed* key needs — `perf` carried a `bytes_per_element` and does not any
more, and a reader meeting one in a file written before that should pass over it exactly as
it passes over a `t` it does not recognise. Ignoring
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
vocabulary and not a value going missing. Nothing writes or reads `perf` yet — the metadata
file exists and no compile pass can produce this record, which needs the stage-7 probe — so
there is nothing to migrate, and a decoder meeting the key in an older file passes over it
under the unknown-key rule stated above.

**L4 is a measurement at reference conditions.** Point sprite cost is dominated by fill
rate: `point_rate` and resolution decide the overdraw, so it is not linear in element count
and a per-element figure would be a fiction. It becomes linear at the bottom, where the
one-pixel floor holds every sprite to a single fragment — a floor is exactly the shape a
per-element figure would have, and it is the one regime a reference measurement must not be
taken in.

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

**`source` is half built, and the half that is missing is the value.** The slot type exists
(`ast::SlotTy::Source`), the checker reads it, and `karakuri-codegen`'s `layout` reserves a
`u32` per declared Source slot and gives it a key the engine would look it up by
(`source_slot_key`). Nothing calls that function outside the module that defines it, so no
engine writes a value into the field. Naming a source and masking on one are still different
distances away, which is what this paragraph recorded before and still records.

An earlier revision of this paragraph said `source` was built and the gap closed. That was
true of the declaration and false of the value.

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
| Interpolation | pair elements and blend their attributes | **A cross-source read** — `uses <name> : Geometry` and `<name>.<attr>`, [above](#a-geometry-slot-on-an-l2) |

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

The last one standing was **how two merged geometries keep their identities apart**, open
since before there was a compiler. It is answered above under "Multiple L1 sources, and
`source`": per-source zero-based `seed`, a per-source `source` value — a
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
