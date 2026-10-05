# Set File Format

`.kbset`. One record per line. Concatenation is composition — the extension no longer spells the
encoding, and the line above is what says it.

**The extension says the file is already resolved**, which is the store's invariant rather than a
label on the format. Every `proc` here is a content address, so reading one of these resolves
nothing against the filesystem around it. `.kset` is the **authoring** form — a Set
file that names its `.kir` files by relative path and lives beside them
([ADR-0229](adr/0229-a-set-file-is-authored-beside-its-parts-and-travels-as-a-bundle.md)), and
[described below](#the-authoring-form). It is read and resolved; nothing writes one, because it
is a file a person authors. The distinction is load-bearing rather than
cosmetic: a swap happens on a frame boundary and an over-budget Set rolls back on its own
([P-0094](principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)),
which holds only because nothing is left to resolve at the moment of the swap. A form that walked
the filesystem while swapping could fail halfway — a neighbour missing, or changed since the file
was written — and a swap that can partially fail is not a swap. So the store holds `.kbset` and
only `.kbset`, and `Store::list_sets` derives an id by stripping that suffix, which is where the
check already lands
([ADR-0231](adr/0231-a-sets-two-forms-take-two-extensions-and-the-store-holds-only-the-resolved-one.md)).

**Bundled or not is a property within `.kbset` and not a third format.** The `src` records below
are appended to a file that was already resolved; what they save the receiver is a lookup in a
store that has never held the material, not a resolution pass.

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
{"t":"merge","live":1}
{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15,"height":2.0}
{"t":"seed","stream":"L1","value":19274}
{"t":"seed","stream":"L1","index":1,"value":48113}
```

**One `slot` record per node, in node order** — the geometries, the deformers, the cameras,
the renderers, then the fields. **The built-in camera has no `slot` record**, because it has
no procedure to reference: it is the last node of the L3 layer in every Set, and the `camera`
record below is what describes it. **The L5 has none for the same reason**, and the `merge`
record describes it — a node with no procedure is described by a record of its own, which is
one rule and not two. The chain above is two geometries, a morph between them and
two renderers over the result; a Set of one geometry and one renderer is the two `slot` lines
it always was. Only `near` was written down: the other four names are derived from their
procedures where the Set is built, which is why the `edge` can name `morph` and `sphere_shell`
without either appearing in the file.

- `proc` is a hash reference. When bundled, the source is inlined as a run of
  `{"t":"src","hash":"…","line":0,"s":"…"}` records — **one run per artifact**, however
  many `slot` records reference it, because a reader keys `src` by hash; and appended
  after the records that were already there, the file's own order otherwise untouched.
  **`--package ID` writes one**, to standard output, and refuses whole where the store
  cannot supply one of the sources: a bundle short of one procedure is a file that looks
  self-contained and is not. The run is the source `split` on `\n` and the reader
  rejoins it the same way, which is exactly invertible — so an artifact that travels
  through a bundle comes back byte for byte and keeps its address, where dropping a
  trailing newline would not. **`--take-in FILE` reads one back into a store**, and what
  it guarantees is that check: every inlined run must hash to the address its `slot`
  record names, or the file is refused with nothing stored. Without it a `src` run would
  be a way to file arbitrary text under an address the receiving operator recognises,
  which is the one thing content addressing is for.
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
- **`camera` carries the built-in's three placement numbers, and they are the whole of what a
  Set file says about it**: `radius`, `speed` and `height`. It carried two until 2026-09-09 —
  the height came back at its default, so a Set kept with a camera looking down and loaded
  again was looking along the equator, and nothing said so. **`height` is absent from every
  file written before that and reads as 2.0**, which is what those files did.
- **The built-in camera has no `param` records, and it is the `slot` rule one level down.** It
  has no `slot` record because it has no procedure to reference; it has no `param` records
  because the `camera` record already carries its values, and a node's three numbers written
  twice in one file is two spellings of one fact. Its three *are* parameters of that node —
  `--param L3:0:radius=12` writes one, a `bind` may drive one, and the Inspector draws three
  rows — so this is a rule about the file rather than about the engine, and it is the same
  rule the `slot` line follows: **a node with no procedure is described by the record that
  describes it, and by nothing else.** The lens three, `fov_y`, `near` and `far`, are not
  parameters and are not recorded either; they come back as declared.
- **`merge` says the Set composites its renderers** rather than overdrawing them, and it is
  the `camera` precedent rather than a new rule. A Set built with `--merge N` gives every
  renderer a cleared target of its own and folds them through an L5; a Set without one draws
  them over a single attachment, the first clearing and the rest loading. That was the one
  thing a Set knew about itself that this file could not say — so a composited Set saved and
  loaded back overdrew, and `select` had nothing left to be about, which is what made a saved
  variant pool an *unselectable* one.

  The rule that kept it out was "a Set file records the **nodes** of a Set, not how they meet
  each other", and `edge` ended that rule by recording exactly how two nodes meet. What holds
  instead is already above: **the built-in camera has no `slot` record, because it has no
  procedure to reference, and the `camera` record describes it instead.** An L5 has no `kind`
  for the same reason — the compositing is fixed, so a `kind L5` file would have nothing to
  contain — so a node with no procedure is described by a record of its own. What was chosen
  here is the second sentence over the first; what lost is the claim that a Set file says only
  what a Set holds.

  **Its presence is the whole statement, and there is no boolean field.** A record that could
  say `false` would be a second spelling of its own absence, and two spellings of one fact
  leave a reader asking what a writer meant by choosing the other. **And no `index`**:
  `camera` carries one because a Set may hold several cameras, where a Set holds exactly one
  L5 — it is the node the Set's single `Texture` output comes out of — so there is nothing
  here for an index to distinguish, and a field that could only ever be 0 invites a file to
  name a node that cannot exist.

  **`live` is which renderer is the only live one**, in draw order, the numbering `select`'s
  `renderer` and a `slot`'s `index` use. **Absent means every input is live**, which is the
  state a merge nobody has selected in is in — an L5 input is live by default — so a Set that
  was never selected in writes the bare `{"t":"merge"}` and reads back identically. Absent is
  emphatically *not* node 0 on the `slot` rule: read that way it would silence every renderer
  but the first in every composited Set ever saved without a selection.

  **`gain`, `opacity`, `blend` and `mask` are not fields here, and that is deliberate.** An L5
  input carries all four, and nothing outside `crates/karakuri-engine/tests/merge.rs` can set
  one — there is no flag, no key and no MCP tool — so a field for them would have no producer,
  and every composited Set would record four defaults nobody chose. That is the argument this
  document makes for keeping `parent` off a metadata card rather than writing it empty: a
  record whose producer does not exist waits for it. **The record grows a per-input row when
  something can set one** — one line per edge, on the terms `slot` and `capacity` are one line
  per node — and `live` is here in the meantime because a selection is the one input control
  an operator can actually reach.
- `param` and `bind` are keyed by `layer` in the record, and the engine holds one value per
  **node**. A name two procedures both declare is two values, each reaching the node that
  declared it — which it had to become, since every L4 in `examples/` declares `exposure`
  and a Set holding two renderers would otherwise have had one. A `param` record carrying a
  bare name reaches **every node that declares it**, which is the useful default: one knob,
  both renderers. Setting two of them *apart* is `layer` plus an optional `index`, which
  both records now carry the way `procedure` does — `--param L4:1:exposure=2.0`,
  `--bind index=1`.
- **A `param` value may be a vector, and it is expanded into components on load.** `{"t":"param",
  "layer":"L4","key":"glow","value":[0.4,0.7,1.0]}` against a `param glow : vec3` becomes three
  writes — `glow.x`, `glow.y`, `glow.z` — because a parameter is driven one component at a time
  (see [param](#param)). The wide value earns its keep on the line: a file, or a model, says the
  vector once and the reader expands it. What is written back out is components, one `param`
  record each, which is what lets a Set file record a single component an operator moved.

  Three shapes are reported rather than carried, and each names the component keys that would
  work: a single number against a vector declaration, which names no component; a `vec2` written
  against a `vec3`, which is not that parameter; and a `bind` on a bare vector key, since a
  binding resolves to one number and a `vec3` has three places to put it. Binding one component
  — `{"t":"bind","key":"glow.y",…}` — is ordinary.
- Unknown `t` values are ignored, for forward compatibility.
- **`gain`, `opacity`, `blend`, `mask`, `transition`, `select`, `residency`,
  `look`, `master_out`, `master_chain`, `canvas`, `procedure`, `authority` and `transport` are not in this list and must never be.** They are the session's
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

**One place this format is still finer than the engine**, reported on load rather than
dropped: `camera` carries two of the six fields the engine's orbit has. It is a disagreement
between the format and the engine rather than a gap in the loader, and settling it is the
format's business and the engine's, not the reader's.

There used to be three more. `seed`, `capacity` and `param` were keyed by layer against an
engine that held one of each per Set, and **the engine caught up**: params are per *node*, the
salt is per *source*, and each source runs at its own capacity. A vector `param` was the
fourth, and it closed the other way round — not by widening the engine's value channel to hold
three floats, but by **addressing the component**, so the wide value lives on the line and the
engine holds `glow.x`, `glow.y` and `glow.z`
([ADR-0268](adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md)).

The *session* records are further along: `audio` and `tempo` per frame, and `gain`,
`opacity`, `blend`, `residency`, `look`, `master_out` and `transport` per key press, are each
built by the CLI, decoded back,
and only then applied — and `karakuri-cli`'s `--record-session` writes them to a session
stream as they happen, `--replay` reading it back. The path the engine is driven through is
the record's, in both directions.

### The authoring form

`.kset`. The same file, one record per line, with **one record type changed**: a node is a
`part` naming its `.kir` by a **relative path**, where a resolved file's `slot` names it by
content address.

```ndjson
{"t":"set","id":"morph_01","v":1}
{"t":"part","layer":"L1","name":"near","path":"lattice_shell.kir"}
{"t":"part","layer":"L1","index":1,"path":"parts/sphere_shell.kir"}
{"t":"part","layer":"L2","path":"parts/morph.kir"}
{"t":"part","layer":"L4","path":"soft_points.kir"}
{"t":"capacity","layer":"L1","value":32768}
{"t":"edge","node":"morph","slot":"far","to":"sphere_shell"}
{"t":"camera","kind":"orbit","radius":8.0,"speed":0.15,"height":2.0}
```

**A `part` carries what a `slot` carries, with a path where the address is**: `layer` and
`index` are the node's address on exactly the `slot` rule — absent is 0 and 0 is not written —
**so a second `part` on the same layer must write `"index":1`**. Two parts claiming one index
are not refused: the later one replaces the earlier, and the only sign is the load note
``two L4 slots both claim index 0; the later one is used``. `--take-in` accepts such a file.
`name` is what this Set calls the node, on the same terms and surviving resolution
unchanged, because it is the same node and the `edge` above points at it by that name.
Everything else a Set file holds — `capacity`, `param`, `bind`, `camera`, `seed`, `edge`,
`merge` — is unchanged and means the same thing in both forms. **Only how a node's source is
named differs.**

**`part` is a `t` of its own and not a `slot` carrying a `path` instead of a `proc`.**
`docs/contributing.md` §4 rules out precisely that
— *"reusing a tag for a differently shaped record in a different file"* — and the bill would be
paid by every reader that dispatches on `t` alone: a decoder meeting `slot` would have to know
which file it came out of before it knew whether `proc` was there, and the one that forgot to
ask would build a Set with a node missing rather than refuse a file. With two tags a decoder
never meets a `slot` without an address, and **a `.kbset` that contains a `part` is a file
disagreeing with its own extension** — refused rather than skipped, by name and by the path it
wanted, because a `part` is a *node* and skipping one hands back a Set that is a geometry short.

**What a `.kset` may contain** is what a `.kbset` may, with `part` in place of `slot`: the
session's records and an artifact's declarations are out of it for the same reasons and with
the same sentences. **What it may not do is reach outside its own directory.** A `part`'s path
is resolved against the directory the `.kset` is in, and that directory is the whole of what it
may name — an absolute path, a `..` that climbs out, and a symlink pointing out are three
spellings of one escape, and each is refused out loud, naming the path and what it escaped.
Refused rather than repaired: an include quietly rewritten into one that reads is a rule an
operator can only find by experiment, and an authoring file is a thing you are *sent*. See
[ADR-0229](adr/0229-a-set-file-is-authored-beside-its-parts-and-travels-as-a-bundle.md), *The
wall the authoring form needs*.

**A `slot` in a `.kset` is legal and unusual rather than wrong.** Resolution has nothing to do
to one and passes it through, so what it names is material the receiving store must already
hold — which is the one thing this form exists not to require. The same goes for a `src` run:
a `.kset` that inlined a source has already carried it, and the resolved file carries it on.

**Resolution is a read, a hash and a store put — never a compile.** A `slot`'s `proc` is the
content address of the `.kir` *source*, so turning a `part` into one is: read the file, hash the
bytes, put the artifact in the store, and write a `slot` naming that address. Nothing parses a
procedure — the checker runs where a Set is *built* — and every path is put through the wall
before a single byte is read, so a file with one escape in it stores nothing at all.

**Resolving is packaging, and packaging is resolving done ahead of time** — *one operation, two
moments* (ADR-0229 part 4). It is what makes the store's invariant keepable: a `.kbset` is
resolved because something resolved it *before* it reached a store, rather than at the moment of
a swap, where a neighbour that has gone missing or changed would fail after the exchange had
begun. A swap that can partially fail is not a swap.

**Implemented.** `karakuri-environment`'s `setfile::resolve` is the resolver and its own wall;
`--package FILE.kset` is the route through the command line — the packaging step end to end,
resolution followed by the same inlining `--package ID` does, to standard output. `--take-in
FILE.kset` is the other moment of it: the same resolution, filed into this store rather than
written out. Both are flags that already existed rather than a third beside them, because *Send a
Set to somebody, and take one in* is the operation every one of these moments belongs to. `Store::write_set` refuses a `part` by
name, which is where a `.kset` and a store are held apart, and nothing in `sets/` can be one:
an id is the file name with `.kbset` stripped off it, so a `.kset` dropped in there has no id
and can be asked for by nobody.

**And the Library's route to one is built.** The bay's `presets` scope lists the `.kset` files in
the root the program was told about, and `l` over a row of it takes the file in and then loads it
in one press — ADR-0229 part 3's load that packages, performed at the second of its two moments.
What is refused there is `resolve`'s refusal and nothing new: a part named from outside the file's
own directory is not read, and an id the store already holds is refused rather than overwritten,
so a preset that will not come in loads nothing and leaves what is on air alone.

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
frame. There is at most one binding per (`layer`, `index`, `key`); a second replaces the
first, because two would be resolved in some order and the order would decide the value.

**A binding can be removed while the Set is playing, and removing it is the whole of
handing a knob back.** *Take a parameter back* is the operation, `source` with no attachment
is the record, and what the param is left at is its own value — which is what step 4 already
writes at confidence 0.0, so there is no fourth state for a binding to be in and nothing
suspended for anything to carry. A hand on the *value* of a bound param is not that: it
moves the base the binding blends from and leaves the attachment where it is, which is what
makes the pair order-independent above. See
[ADR-0319](adr/0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md).

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
