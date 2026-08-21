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

Rust stable and a GPU. `.kir` files go in — at least one geometry and one renderer, and
optionally deformations, a camera and a field — and the CLI puts them
through parsing, type and contract checking, cost estimation, WGSL generation, and pipeline
creation. There is no hand-written shader anywhere in that path.

```sh
cargo run -p karakuri-cli                                    # a window
cargo run -p karakuri-cli -- --render out.png --frames 240   # one frame
cargo run -p karakuri-cli -- --seq frames/ --frames 420      # every frame
cargo run -p karakuri-cli -- --param turbulence=2.6          # a uniform write
cargo run -p karakuri-cli -- --capacity 65536                # a Set-level dial
cargo run -p karakuri-cli -- a.kir b.kir                     # a different pair
cargo run -p karakuri-cli -- --watch                         # edit a .kir, watch it swap
cargo run -p karakuri-cli -- --watch --budget-ms 0           # ...and watch it roll back
cargo run -p karakuri-cli -- --set a.kir,b.kir --set c.kir,d.kir   # two Sets, mixed
cargo run -p karakuri-cli -- --tonemap agx --exposure 1.5    # the output transform
cargo run -p karakuri-cli -- --bpm 128 \
  --bind layer=L1,key=turbulence,signal=beat,curve=pow2,range=0.1..2.4
```

A Set file can be saved and loaded, which is what `--bind` was a stand-in for:

```sh
cargo run -p karakuri-cli -- a.kir b.kir --save-set night01 --param turbulence=2.6
cargo run -p karakuri-cli -- --load-set night01
cargo run -p karakuri-cli -- --load-set night01 --record-session take1
cargo run -p karakuri-cli -- --replay take1 --seq frames/
```

`--save-set` puts the slot's `.kir` files in the store as content-addressed artifacts and writes
a Set file referencing them by hash, with the capacity and the salt each geometry *was
running at* — the procedure's own declared default where no `--capacity` overrode it, rather
than the flag's number — and the parameters, bindings, camera, node names and edges the run
was given. `--load-set` reads it back, and a run driven by the file renders
**the same frame** as the run whose flags wrote it. Anything the file could not be carried
across in full is printed rather than dropped — see the Status table. A slot holding a
chain — L2s, an L3, a field, several geometries — is saved as the chain it is: the format
records a `slot` per node, keyed by the layer the node's own `kind` declares and its index
within that layer, so a cube morphing into a sphere is a Set that can be kept.

`--record-session` writes the **timeline**: that Set file's records, then a `tick` a frame
and every edit between them, so each one lands at an exact frame position. `--replay`
renders it back, reading `tick` where a live run reads a clock and `audio` where it reads a
microphone — so what is replayed is the performance, not only the material. It renders
offscreen, deliberately: a window would put a clock back at the one place a replay must not
have one.

`--set` fills a deck slot and may be given up to four times; every Live slot renders into
its own target and one composite pass mixes them. In the window:

```
0-3  focus a slot     space  on air / off air     [ ]  gain      \  gain to 1.0
- =  exposure         `      exposure to 1.0      t    tone map  h  the rest
w    warm off air     y      slot sync mode       u i  scrub the slot
b    tap the beat     , .    halve / double it    o p  latency offset
```

The last row is `--audio-in` only.

Taking a slot off air holds it rather than stopping it: `t` only advances through a step,
so bringing it back resumes where it left off. `t` cycles the tone map operator live, which
is the only way to compare two of them on moving material.

`w` **asks** for a slot to warm off air; the governor grants it only if the frame budget
has room. A request it is still holding shows as `park` on the status line, apart from
`off` — the same residency, opposite situations, and a surface that showed them alike
would be telling an operator their request was discarded when it is reconsidered every
pass and takes effect by itself when a slot comes off air.

`y` cycles a slot's **transport**: `free` runs at wall time, `tempo` scales the rate by the
room's tempo, and `beat` locks the slot's clock to the room's musical position — tape-style,
so `u`/`i` scrub it back and forward a quarter beat a press. Modes the material cannot take
are skipped with the reason: **beat sync needs closed-form material**, because a position
lock has to be able to place the clock and an accumulating procedure can only be run
forward; and **tempo sync is refused on material that reads `beats`**, which already follows
the room and would otherwise follow it twice. Engaging a mode anchors the material at the
current tempo, so nothing jumps at the moment it is switched on.

`[`/`]` move a slot's **gain** — the level the material arrives at — and `;`/`'` move its
**opacity**, the fader across the blend. `m` cycles the blend mode: `add`, `over`, `max`.
Under `add` the two faders are the same dial twice; under `over` one dims a layer and the
other stops it hiding what is beneath, and an `over` layer at zero *gain* is a black card
that still covers. Opacity is the one that silences under every mode, so it is the one to
pull when material has gone bad — **except while that slot is being auditioned**, where both
faders are ignored on purpose and the way out is to end the audition.

`v` cycles what the output shows — the mix, then each slot in turn. Auditioning an off-air
slot draws it without stepping it, so looking at material never moves it; a slot that has
never run has nothing to draw, and priming is what gives it something. The status line says
`PVW<n>` while the output is one slot rather than the mix.

The status line carries each slot's residency, gain, simulation `t`, and its **level** —
mean and peak luminance, measured on the GPU and lagging a few frames because reading it
back synchronously would be a stall. Mean is what two Sets are matched on; peak warns which
one will dominate the mix wherever it lands regardless of its fader. Both are taken over the
texels whose luminance was a finite number, and `x` counts the ones that were not: a shader
dividing by a value that reaches zero produces a NaN, and one NaN admitted to a sum makes
the mean NaN. It is not a warning — that artifact is ordinary and usually shows as a
blown-out pixel — it is there so the two numbers beside it stay readable. Opacity, blend mode and
transport appear only when they are away from their defaults: the line is read in the dark,
and a column saying the same thing on every slot is four columns of nothing.

`--watch` recompiles in the background on a save and swaps the result in at a frame
boundary; a file that does not compile prints its diagnostics and changes nothing. See
Status below for what the swap does and does not do.

Defaults are `drift_shell` + `soft_points` at 262144 elements. [examples/](examples/) also
holds `spark_fountain`, an L1 that spawns and kills, and `beat_shell`, which reads the
`beats` ambient so the figure turns with the bar and swells on the beat rather than at
wall time. Pair either with the same L4:

```sh
cargo run -p karakuri-cli -- examples/spark_fountain.kir examples/soft_points.kir
```

A procedure that fails
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
- No UI for now. V1 is a CLI plus a window, and **the CLI is scaffolding rather than the
  destination** — the end state is a GUI application, so the command line is deliberately an
  auxiliary way to reach what the records already carry
- Audio input exists — spectrum, energy, onset, and a beat grid that corrects the local
  oscillator. External sync exists out of process: `--tempo-source` runs a separate program
  that reports where the beat is, and the first one is Ableton Link. See `docs/plugins.md`

---

**To play it rather than to judge it, read [`docs/manual.md`](docs/manual.md)** — a
walkthrough, every flag and key, what the status line means, and a list of the things it
deliberately will not do. That last section exists because this document is not where an
operator would find them: "the downbeat is arbitrary until Link" is a fact about playing,
not about design, and it had no home until the manual had one.

## Vocabulary

| Term | Meaning |
|---|---|
| Procedure | Code that runs every frame on the GPU, written in the IR |
| Artifact | A saved procedure. Content-addressed and immutable |
| Slot | Two things, and it is worth knowing which. A **layer slot** is a position within a Set (L1/L2/L3/L4). A **deck slot** is a position within the deck, holding a whole Set. `docs/roadmap.md` says *member* for the second; the code says `Deck::slot`, and that disagreement is recorded rather than resolved — renaming either is churn until something depends on telling them apart |
| Deck | Where prepared-but-not-showing material lives, at Set granularity. Up to four slots; one to four of them Live and composited, the rest resident |
| Residency | How ready a deck slot is, over three levels: **Live** (composited), **Priming** (stepped, and drawn only if it is being auditioned), **Allocated** (compiled, buffers held, not stepping, keeping its state; drawn only if it is being auditioned). It is **two facts, not one** — what the operator *requested*, which only they change, and what the engine is *effectively* doing, which the governor recomputes each pass |
| Parked | Requested Priming, effective Allocated: waiting for budget, **not cancelled**. It resumes by itself when there is room, because the request is never overwritten — every pass recomputes the effective level from it |
| Set | Filled slots forming one video source. The unit of compilation and of lifecycle |
| VideoSource | The interface L5 consumes. Set is one implementation of it |
| Signal bus | Input distributed to every layer. Always complete; values carry a confidence |
| Local oscillator | The single source of truth for phase and tempo. External input is only correction |
| Record stream | The path engine state is mutated through, so that a session replays. ndjson. **Everything an operator moves goes through it, and so does the material** — see Invariants |
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
  all write the same records. The engine cannot distinguish a human from an agent.

  **This one is a target, not a description, and the gap is worth stating exactly** —
  an invariant that reads like a report of the code is worse than one that admits it is
  not there yet.

  What goes through a record: `audio` and `tempo` every frame, `canvas` once at the head,
  and `gain`, `opacity`, `blend`, `mask`, `transition`, `preview`, `residency`, `look` and
  `transport` on every key that moves them. Each is built, read back, and only then applied,
  so what drives the engine is what a replay would decode rather than a second path that
  happens to agree with it.

  **A control surface is what put that under load**, because it is the first thing that is
  not the keyboard: every MIDI action ends in the method a key press ends in, so a session
  recorded from a controller replays with neither controller nor map attached. Building it
  found the one place the invariant was already false — the beat tap moved the grid and
  dropped the `tempo` record it had just built, so a session replayed on a different phase
  from the one it was played on. Two keys were doing it. That is the value of stating an
  invariant as a claim rather than keeping it as a habit.

  **The second hole was in the other direction: a record written for a frame that never
  happened.** `tick` is a promise that the deck advanced by that many steps, and the frame
  loop used to read the clock, write the `tick`, measure the audio, and only then find out
  the swapchain had no texture to draw into. An abandoned frame therefore told the stream it
  had simulated steps the deck never took, and a replay obeyed the record — so resizing a
  window during a recorded session was enough to make the replay diverge from the
  performance. The fix is an ordering: acquire first, and nothing below it runs unless the
  frame is going ahead. Not reading the clock is also what makes the skipped interval
  *survive*, since it is then carried into the next frame and simulated there; before, it
  was recorded, never simulated, and lost.

  The **material** joined them: `--load-set` builds a Set from `set`, `slot`, `capacity`,
  `param`, `bind`, `camera` and `seed` records, and `--bind` is now a way of *writing* a
  `bind` record rather than a second path beside it — one decoder, so a command line and a
  Set file cannot mean different things by the same fields.

  **Every record type has a writer**, and a session carries its own material:
  `--record-session` writes the Set at the head of the stream and then the timeline — a
  `tick` a frame and every edit between them — and `--replay` renders it back, reading
  `tick` where a live run reads a clock and `audio` where it reads a microphone. So a
  performance is replayed rather than only rebuilt.

  **What is still short is one thing, and it is the format's**: a session stream has no way
  to say what a *deck* held. A Set file describes one Set, so only slot 0's material reaches
  the head and a multi-slot session replays the rest of the performance against a deck of
  one, reporting what it skipped. Closing that is a format change.

  The frame path still writes nothing itself: serialising allocates, so a frame moves a
  record into a buffer that already has room and a writer thread does the rest. An `audio`
  record carries a `Vec`, so it is swapped for an empty shell rather than copied
- **The governor may lower a slot's effective residency and may never write its requested
  one.** A refusal is a deferral: the operator's request is what the next pass reads

### Signals

- **Consumers never branch on whether a provider exists.** They branch only on confidence.
  A parameter binding is the first consumer holding this up: what it writes is
  `lerp(the parameter's manual value, the mapped signal, confidence)`, so an invented
  signal moves a parameter a tenth as far as a measured one and no code tests for a
  microphone
- The signal bus is always complete. A signal with no provider returns a synthesized value
- **`noise` is not a bus name.** It is the `bind` record's, because a generator has a kind,
  a rate, a stream and an octave count, and `sample` takes a `&str`. Completeness is
  unaffected — an unknown name is still answered
- Rendering reads only the local oscillator, never an external clock directly

### Determinism

- **All randomness comes from an explicit seed stream**
- The session oscillator is part of what a record stream reproduces. Its whole input is the
  tick sequence and one explicit seed
- No implicit randomness, no time-derived seeds, no thread-ID-derived randomness
- `dt` is a fixed simulation step, not the real frame delta
- **Frame advance comes from a `tick` record, never from a measurement.** Live, the engine
  derives it from real time; on replay it reads it back and measures nothing. Today the
  derived value is passed as a number and no `tick` record is built — what already holds
  is the half that matters for determinism, that the engine advances by a **step count**
  and never by a duration, so the record is a serialisation of something that exists
  rather than a channel that has to be invented
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
geometry and L4 for rendering, nothing else.

**That assumption is proved and V1 is closed**, and two milestones have closed on top of it.
M2 made it playable — a deck, an L5 mix, tone mapping, audio and a beat grid, MIDI,
transitions, Set files and session replay. M3 made it deep: **every layer in the model is now
an IR `kind` with a node behind it**, so a Set is a chain rather than a pair — several
geometries, deformations that stack and mask and amplify and pair, a camera written as a
procedure, several renderers, and a signed distance field any of them can call.

Work has moved to M4, which is the library at scale, and it opened with the one thing M3
left: **the nodes in a Set had no names.** They have them now, and the first *edge* between
them is spelled — `uses far : Geometry` in a procedure and `--edge morph.far=sphere_shell`
in the Set, so `--set` order no longer decides which geometry a morph reads. A Set file, a
hot-swap rebuild and the MCP surface all reach past a slot's first geometry. The same
notation now carries a **field**: `uses shape : Field` and `shape(p)` replaced the reserved
word `field(p)`, so a procedure can name as many shapes as it wants and the Set says which
each one is. What is still capped at one apiece is the *number of fields a Set holds* and
the camera — a `Vec` and a notation, in that order.

The table below is what exists, part by part. What is still absent, and why, stays in
[docs/roadmap.md](docs/roadmap.md) rather than being listed here: agents, the library,
the node editor.

### Status

**All three clauses hold.** `.kir` text goes through every stage and 262144 elements come
out on a GPU with no hand-written shader in the path.

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

The third clause closed last. A `.kir` file that changes is reparsed, rechecked,
regenerated, and rebuilt into a whole new `Set` — shader modules, pipelines, buffers, bind
groups — on a worker thread, and the render loop collects it with a `try_recv` at the top
of a frame. There is no `recv` and no `device.poll(Wait)` anywhere on the frame path.
`--watch` is both the demo and the development loop. What a swap does and does not do is
worth stating exactly:

- **It lands between two frames, never inside one — by convention, not by construction.**
  The live Set is only ever replaced at the top of `begin_frame`, which returns it as a
  `&mut` borrow the caller holds for the whole frame body, so no single reference changes
  identity underneath a frame. That is weaker than it sounds and the difference is worth
  stating: the command encoder is the caller's and borrows nothing, so a caller that calls
  `begin_frame` twice inside one encoder gets two Sets in one frame, and it compiles. Both
  callers in this repository call it once per frame; nothing in the types says they have
  to. Making it structural means handing the encoder out from `begin_frame` too, behind a
  guard that submits on drop, and `crates/karakuri-engine/src/deck.rs` now does exactly
  that: `Deck::begin_frame` returns a guard owning both the `&mut Deck` and the encoder, so
  a second frame while one is open does not compile and two generations of Sets cannot
  reach one encoder. That is the shape a caller with more than one Set has to use. A
  `HotSwap` driven on its own — which is still what the CLI does — keeps the weaker
  property above.
- **It transfers no state.** A new procedure means new buffers, so the incoming Set starts
  cold: `t` at zero, nothing primed. Warming a Set out of sight before it is shown is
  priming, which the deck does and this does not.
- **It is reversible for a window.** The outgoing Set is kept alive — and unstepped, so its
  `t` stands still — through eight warmup frames and thirty measured ones. If the median
  frame interval over those thirty exceeds the budget, the candidate is dropped and the
  outgoing Set is live again at exactly the `t` it was parked at. Only after it passes is
  the old one released, and it is released on the worker thread: dropping a Set frees GPU
  resources, and a free on the render thread is the same invariant as an allocation on it.
- **Saves that land during a window are collapsed to the newest.** A judging window is
  about forty frames, which is long enough for two more saves to compile behind it. The
  channel is first-in-first-out, so the verdict frame drains it to the last finished build
  rather than taking the front of the queue; a superseded Set is never shown, and a
  superseded build that *failed* still prints its diagnostics.
- **A build that fails changes nothing.** A `.kir` that will not compile prints its
  diagnostics on the worker thread and never becomes a request at all; a pair `Set::build`
  refuses is reported and put down. Either way the running Set keeps its `t`, its element
  buffers, and its live count.

Measured across a swap at capacity 262144, 1280×720, **on a host clock** — the same caveat
as every other number in this file. Two more that are specific to these: it is a frame
*interval* rather than a GPU cost, and it was taken in the **headless** test harness, which
has no surface and therefore no vsync and no compositor. Its `device.poll(Wait)` per frame
stands in for the pacing a real window would get, so these are the cost of producing a
frame, not the cost of showing one:

| | median | worst |
|---|---|---|
| steady, before the request | 5.0 ms | 13.0 ms |
| the frame the swap landed on | 4.6 ms | — |
| steady, after the swap | 5.0 ms | 10.3 ms |

Three to five frames were rendered between the request going out and the swap landing,
which is what "does not block" means operationally — a blocking receive would make that
number zero. The swap frame has not, across runs, cost more than the worst ordinary frame
in the same run; but it is one sample of a noisy quantity, so the honest claim is
"indistinguishable from jitter", not "free".

The watchdog measures a median rather than a worst case, for the reason `Probe` does: on a
host clock a single sample carries whatever the OS scheduler was doing, and one hitch is
not a reason to throw away generated material. It also discards the first eight frames after
a swap, because a cold Set's first frames pay for pipeline first-use and for its own
whole-capacity buffer upload — a budget check that fired on frame one would roll back every
candidate that ever existed.

The budget defaults to 20 ms: one 60 Hz frame plus slack, 60 Hz being the rate `dt` sets.
**That default does not generalise to a faster display, and this one is faster.** On the
120 Hz panel it was developed on, steady state is 8.3 ms and a frame rate cut in half reads
as 16.7 ms, which the default does not catch. `--budget-ms` is the operator's answer; a
budget derived from the display rather than from a constant belongs with the budget
governor and is not built, alongside the decision about whether GPU timestamps can be trusted at all. That decision is a **ratio against a second measurement of the same submission**, not a constant: a fixed floor was what let an adapter's meaningless 0.095 ms reading through, and it is checked on every measurement rather than once, because the adapter that produced it passed calibration and lied afterwards.

| | |
|---|---|
| IR: parse, type and contract check, cost estimation | Works. Diagnostics carry a span, a hint, and every error at once. A `consumes` not covered by `emit` is rejected outright, except `age` and `velocity` which the engine synthesises — so a streak renderer pairs with any geometry emitting `position`, and everything else is still a refusal by name at build rather than a shader that comes up short at runtime |
| WGSL generation | Works, validated through naga. Generated names cannot be captured by anything a `.kir` can spell |
| Set, buffers, pipelines, compute and render | Works. Double buffering, Set-level `capacity`, parameters as uniform writes. Nothing on the frame path allocates: both per-frame uniform writes go through storage sized once at build time |
| Linear HDR end to end, sRGB once at output | Works |
| Store | Works, and the engine obeys it. `--save-set` writes a Set file and puts every node's `.kir` source in the store as a content-addressed artifact; `--load-set` reads it back and builds from it, resolving each procedure by hash or from inlined `src` when the file is bundled. **Every record type has a writer.** `audio` and `tempo` go through a record every frame, `gain`, `residency`, `look` and `transport` on every key that moves them, `set`/`slot`/`capacity`/`param`/`bind`/`edge`/`camera`/`seed` on every save and load, and `tick` once a frame into a session stream. `--record-session` writes the timeline as it happens and `--replay` renders it back, so a performance is replayed rather than only rebuilt |
| What a Set file cannot carry | Named rather than dropped, and printed on load. A `param` may be a vector and the engine's map holds `f32`; and a `camera` record carries two of `Orbit`'s six fields, so the other four come back as defaults. Each is a real disagreement between the format and the engine rather than an omission in the loader, and a Set file that half-applied in silence is the failure this repository keeps refusing. Two have left this list. `seed`, `capacity` and `param` keyed by layer against an engine holding one of each per Set: params are per node now, the salt is per source, and each source runs at its own capacity. And a `slot`'s **name**, which used to be recorded and reported because no surface pointed at a node by one — an `edge` does, so a name is carried back to the node it belongs to |
| One frame loop | Works. There were two — the window's and the offscreen PNG's — and the seam between them was meant to be a single line (a live run measures its step count from a clock, a replay reads it from a `tick`). It had become five, and **every replay defect this project has found was one of the four extras**: the look applied once instead of per frame, the governor running on one path only, the present pass skipped on unkept frames. Now `frame::compose` is the loop and a `Sink` is where it goes — a window, a PNG writer, and a test double. The **ordering is structural**: `compose` takes the committing work as a closure and calls it only after the sink has a target, so a frame with nowhere to draw cannot record a `tick` claiming steps the deck never took. That was two statements in the right order before, and getting it wrong was invisible. The test sink is what finally reaches `Live::frame`'s claims, which no test in this program had ever done |
| Replay fidelity | Works, and it is now checked end to end rather than argued: `crates/karakuri-cli/tests/replay.rs` runs the binary, replays two sessions differing in one record, and reads the pixels. Every assertion there is paired with a **control** — the same session replayed twice, byte for byte — because this material moves on `t`, so "the frames differ" is true whatever the code does. Building it closed a real hole: a `look` record moved nothing on replay. The offscreen renderer took a look and applied it once before the loop while the replay driver wrote every decoded one into a variable nothing read again, so a session replayed under the look it *started* with however many times the operator changed it — and with no automatic gain by design, the exposure is a control an operator is expected to ride. It also closed a second one of the same family: a `tick` is a **terminator, not a header** — `split` files each record into the frame of the next tick — and a frame used to push its `tick` before the `audio` and `tempo` it had just measured, so a replay showed frame N what frame N−1 heard, in the two signals every binding is driven by. Both bugs were in the record ordering of the one function that has no test reaching it, `Live::frame`, which needs a window; the splitter half is tested and the writer half is checked by reading, and that is said where the test is rather than left to assume |
| Session recording | Works, and it carries its own material: `--record-session` writes the Set at the head of the stream — from `--load-set` when given and saved under `ID-material` when not — so `--replay` needs nothing else and neither does this flag. It is now **refused** with `--render`, `--seq` and `--replay` rather than silently dropped: the recorder is only built on the path that opens a window, so those runs exited 0 having written no session and said nothing, which is the failure the recorder's own construction site carries a comment warning against. It used to write the `--load-set` file *or nothing*, and "or nothing" was a timeline of ticks with no material under it — a session that refused to replay long after the set was over, with nothing said at the time. What is still short: a session stream has no way to say what a **deck** held, so only slot 0's material is at the head and a multi-slot session replays the rest of the performance against a deck of one |
| Set file and session stream | The two projections are kept apart by `Record::is_set_state` and `project`. A Set file drops the mix as well as the ticks, and for a different reason: a gain is state, but the *session's*, and a Set file that restored one would pull down whatever fader it was next loaded under. A session folds *down* to a Set file — `Store::save_session_as_set`, ticks dropped and state folded — which is the projection the format specifies. What does not exist is the other one: folding a session to the deck state it ends at, so a run could resume where the last one stopped. Nothing needs it yet |
| Transport | Works, per slot. `free`, `tempo` (rate, scaled by the room's tempo against a per-slot anchor) and `beat` (position lock, jumps and reverses). **Two mechanisms, because there are two kinds of material**: closed-form state at `t` is a pure function of `t`, so a seek costs one element pass; accumulating material integrates, and reversing a sum is impossible rather than slow, so all it can be given is a rate. The anchor is a per-slot dial because **material has no intrinsic tempo** — a `.kir` declares parameters and a capacity, not a bar length — and it defaults to the tempo at the moment sync is engaged so that engaging it moves nothing. What is not built: a position source that can itself reverse. Audio only ever moves forward; a deck link (M7) does not, and arrives at the same entry point |
| `beats` | Works. The session's tempo grid, readable from IR as an ambient beside `t` — per substep, continuous across a tempo correction, and the same instant `t` names. **Without it the picture ran at wall time whatever the music did**: a tempo change moved the grid every binding was sampled on and moved nothing that was drawn, and a `bind` cannot close that, because what follows a tempo is not a parameter value but the passage of time. `beat` and `bar` stay bus names and stay different — phases in `[0, 1]` for driving a parameter, where `beats` is unbounded and monotone for driving a position |
| Signal binding | Works. A binding samples the bus, curves it, maps it onto a range, and blends into the parameter **by confidence** — so a binding to `beat` or `bar` takes full effect today, and one to `energy` moves fully when a microphone is open and a tenth as far when none is. The binding did not change when audio arrived; the same name started answering with a different confidence |
| Oscillator, synthesized bus, noise | On the frame path now. One oscillator per session, owned by the deck, advanced by the same `steps` the slots are |
| Element lifecycle | Works. `spawn` and `kill()` run, order-preserving compaction is wired into the L1 dispatch, and `element` and the draw are both indirect off one counts buffer. A procedure that can neither spawn nor kill skips the scan entirely and dispatches in place |
| Tone mapping | Works. Four operators — clamp, Reinhard, ACES, AgX — chosen by a uniform, so switching one mid-set is a buffer write. ACES by default, picked by rendering all four across five exposures and looking; `cargo run -p karakuri-engine --example tonemap_compare` regenerates that. Applied once, immediately before the single sRGB encode |
| The deck and the mix | Works. Up to four slots, each with its own `HotSwap` and its own HDR target, folded together in slot order with a per-slot **gain, opacity and blend mode**. Allocated holds its state, so a slot brought back resumes rather than restarts |
| Diagnostics | Works, and it turned out to have a second audience. Seventy-three diagnostics carry a `hint`, written to be kind to whoever typed the mistake. The first real MCP session showed they are also **the specification an LLM reads**: denied an example to copy, a model probed the checker instead — safely, because `write_procedure` checks before it writes, so a failed compile changes nothing. `hint: v0.2 defines 'points' only` closed off line primitives in one try — a hint that has since had to be rewritten, because the language grew the primitive it was refusing. And the hint that carried a *reason* — `loop bounds cannot reference a param… cost estimation needs them fixed at parse time` — changed the model's whole approach rather than its syntax: it concluded that integrating a streamline per element was incompatible with the cost model and switched to an analytic curve. **A hint that only corrects syntax produces a procedure that compiles and cannot be afforded.** Every hint should say why |
| Examples | Twenty-one procedures. **`drift_shell + swirl_warp + soft_points` is the first chain**: an L2 twisting the geometry between the simulation and the draw, which no `.kir` could reach before because a Set was a pair. `drift_shell + soft_points` is the default and is particles; **`drift_shell + drift_streaks` is the same L1 drawn as segments**; **`drift_shell + glass_shell` is the same L1 again, drawn as a solid** — the same cloud that is a lamp under `additive` is an opaque ball under `weighted`; **`drift_shell + field_march` draws no elements at all** and marches a distance field instead; **`drift_shell + melt_blob + field_lens` splits that in two** — `melt_blob` is a shape and nothing else, `field_lens` is a marcher containing no shape, `--edge field_lens.shape=melt_blob` says which shape it marches, and either can be swapped without touching the other. `drift_shell + kaleidoscope + soft_points` turns one point cloud into six; **`drift_shell + beat_jump + soft_points` is the camera written as a procedure** — `beat_jump` cuts to a new angle on the beat and keeps facing the centre, and does it by hashing the beat number, so it holds no state and stays seekable under a scrub; **`drift_shell + swirl_warp + late_bloom + soft_points` stacks two deformations** and `late_bloom` is where a `mask` block earns the layer its claim — three lines of `deform` that are a different effect masked on `age`, on `seed` or on distance, with a `weight` beside it as the fader; **`lattice_shell + sphere_shell + morph + soft_points` is a cube becoming a sphere on one fader** — two geometries in one Set, paired element by element, with `--edge morph.far=sphere_shell` saying which of them the morph reaches for. `beat_strands + beat_strokes + beat_bloom` are three a model wrote over MCP in one session, and what they do with `lines` is not what I would have written — `velocity` redefined as the offset to the next sample, so consecutive segments share an endpoint. **`strand_shell + strand_strokes` is line art made *without* a line primitive**, kept as written now that `lines` exists because it is the measurement of what one topology cost: roughly twice the elements. `beat_shell` reads `beats`, `spark_fountain` spawns and kills |
| **Rendering primitives** | **Three topologies and two blends.** **`lines` cost the engine nothing** — the same quad, six vertices per instance and a `TriangleList` pipeline, laid along the segment from `clip` to `clip_b` instead of around a point. Both ends of a segment belong to **one element**: no element links to another, deliberately, since an index into the element buffer is what compaction invalidates. **`fullscreen` cost rather more, and it is what the SDF builtins were for** — `sd_sphere`, `sd_box`, `sd_torus`, `sd_plane` and the four CSG operators passed the checker from M1 with nothing able to draw them. An L4 declares it by having **no `vertex` block**, gets `eye` and `ray`, and **consumes nothing** — which is a checked rule rather than a consequence, and is what makes skipping the paired L1's whole simulation provable instead of merely plausible. Its fragment budget is eight times a sprite renderer's, because a fullscreen pass covers the canvas exactly once where sprites overdraw an unknown number of times. **`blend weighted` is the second way fragments combine, and the first way material in this project has ever occluded anything** — weighted blended OIT: an accumulation target, an `R16Float` revealage whose blend state multiplies rather than adds, and a resolve pass. Order independent, so it does not fight compaction the way a depth sort would. It reads `color`'s alpha as *opacity* where `additive` reads it as emission strength, and clamps it, which is the one thing an author has to know. What leaves the resolve is exactly what the additive path leaves — premultiplied colour, coverage in alpha — so **L5 was not touched**: the mix reads a weighted slot without knowing the mode exists |
| Blend modes | Works: `add`, `over`, `max`. The set is what survives an **unbounded linear HDR** mix rather than what a VJ mixer usually lists — `screen` and `multiply` assume `[0, 1]` and nothing has tone mapped this far up the pipeline, so `screen` of two 2.0s is 0.0. `over` is the only mode in which one layer hides another, and what it hides with is coverage the L4 pass accumulates into alpha; thin material barely covers, which is correct rather than a defect. This is also what finally separates **gain from opacity** — gain is the level the material arrives at, opacity is the fader across the blend and the only control that silences a slot under every mode. What is not built: a layer stack that is anything other than slot order |
| Priming and the governor | Works. Priming steps a slot and does not draw it — L1 owns every piece of per-element state and L4 is stateless, so warming is the compute passes and nothing else. The governor computes each slot's effective residency from the operator's request and a budget of per-Set costs measured by the probe at build time. It **never demotes a Live slot**: an over-budget deck reports and suspends priming. An unmeasured Live slot means the committed cost is unknown, and unknown is not headroom, so priming is suspended until every slot has been measured |
| Audio | Works. Analysis runs in the driver's callback, not on the frame path, and the frame reads one small value through a `try_lock` on both sides so neither can block. Level is RMS mapped −60 to −6 dBFS: a mastered track's loud windows sit near the top of that, where a top at 0 dBFS would leave real music between 0.80 and 0.90 and a bound parameter barely moving |
| Beat tracking | Works. The grid is **predicted, not chased**: once locked it free-runs, takes a slow trim, and moves not at all for a single disagreeing estimate — re-acquiring takes eight consecutive consistent revisions. **The octave is folded, not judged**: every candidate period is halved or doubled into a one-octave window centred on the grid, which starts at `--bpm`. So a window centred an octave off tracks an octave off, `,` and `.` are the fix, and there is deliberately no automatic one — the alternative is a heuristic that can be confidently wrong, which is the failure that shows on stage. A 3:2 error is a different problem and is not touched. The correction leads by the analysis lag plus the output lag, so what is shown lands on the beat rather than behind it |
| Where the lead comes from | The measurable part is computed and the rest is a **signed operator offset**, on `o`/`p`. That is not a gap waiting for a better sensor: sound and picture leave by different paths, neither ends at this machine, and what has to line up is what a person in the room sees and hears. So the measured half aims at being *stable* rather than complete — a constant unknown costs one adjustment, a drifting one costs the whole night — and the offset goes negative, because a delayed PA makes the sound the late one |
| Per-slot metering | Works. Mean and peak linear Rec.709 luminance per drawn slot — Live, or being auditioned — reduced on the GPU and read back without ever waiting, so it lags a few frames and says by how many. A slot nobody is drawing reads nothing rather than reading what it last drew. Texels whose luminance is not a finite number are counted and left out of both figures: one NaN admitted to a sum makes the mean NaN, and dividing by a value that reaches zero is ordinary enough in a shader that a routine artifact would otherwise cost a slot the number its fader is set by |
| `kind Field` | Works. A signed distance function in a file of its own: handed a `point`, it assigns a `distance`, and it draws nothing and holds no elements. **It has a file and no node** — the L5 argument run backwards, since a `kind` says what a procedure lowers to and this has only code to lower — so it is spliced as one WGSL function per *slot* into whichever procedures declare one, and into no others. **A caller says what it takes and the Set says which** — `uses shape : Field` in the header, `shape(p)` in the body, `--edge field_lens.shape=melt_blob` on the command line — and a slot nothing binds is refused rather than filled in from whatever field is lying around. That replaced `field(p)`, a reserved word, which is a breaking change to the language and is exactly what capped a procedure at one field. Its cost is **per evaluation** and the caller pays it once per call, so a marcher and a field that each fit alone can still be refused together, with both figures in the message. Out: several *per Set* — the notation no longer caps it and the engine's `Option<Field>` still does |
| Masks | Works: a straight front at an angle, and an iris, per slot. A mask multiplies the layer's opacity per texel — everything the fader does, done to part of the frame — so it needed no new place in the composite. Both ends of the front are exact, 0 revealing nothing anywhere and 1 revealing everything everywhere, which is what lets a layer masked to nothing be **skipped**: a third escape from material that has gone NaN, beside residency and the fader. **A wipe is a mask and one scheduled move** (`c`), and neither half knows about the other. Out: a mask read from a texture — an arbitrary shape, or another layer's luminance. `kind Field` now exists and is what such a mask would be written as; what is missing is the deck's mask reading one, since `shaders/composite.wgsl` is a hand-written engine shader with a fixed set of mask kinds rather than a generated one |
| Transitions | Works. One scheduled move — a control, a destination, a musical duration, a curve — and a crossfade is two of them sharing a start and a length. `f`/`g` fade the focused slot out and in, `x` crossfades to the next slot, `n` and `j` choose where a fade starts and how long it lasts. **The first thing in the engine that schedules on the beat clock** — the transport already *follows* it — and a fade is a function of the session's beat count and of nothing else, so the same records reproduce it on a machine at a different frame rate and a tempo change mid-fade moves the fade with it. One record schedules the whole move and the values it produces are not recorded, which is `tick`'s shape from the other end. A hand on a control cancels whatever was moving it. A wipe is one of these carrying a mask's front, which is why there is no `wipe` record and no `crossfade` record — the first-class things are the shape and the move |
| Slot preview | Works. `v` cycles what the output shows: the mix, then each slot. An audition **adds a draw and never a step**, so an off-air slot is drawn while it is being looked at and nothing moves that would not have moved anyway — an allocated slot shows the still it stopped at, a priming one shows what it is warming into. Not a second pass: the mix runs as always with that slot's terms at unity and the others skipped, so what lands is its own texels through the same tone mapper. Shown ignoring its faders, and metered, because the number wanted before putting it on air is the level the material arrives at. What is not built is a default renderer per topology — there is no slot holding geometry with no L4 to draw it, so there is nothing yet for one to do |
| MIDI in | Works. `--midi-in` opens a port, `--midi-map` says what each knob and pad does, and every action ends in **the same record a key press writes** — so a surface can do nothing a key cannot, and a session recorded from one replays with neither attached. The map is a file and is deliberately **not** in the stream: which knob is which belongs to the hardware in the room. With no map, every message prints the line that would map it, which is how a surface is discovered until M5 has a UI to assign one in. 7-bit; the 14-bit MSB/LSB convention is not implemented, which is about 0.8% of a fader's range per step. **MIDI out is not built**, so a surface's LEDs and motorised faders do not follow the deck — which starts to matter the moment two things can move one fader, and a transition is now one of them |
| Window output | Works, and it is a **preview**. `--canvas` is what a run renders at (1920x1080 by default) and `--size` is only how big the window is; the canvas is fitted into the window and the leftover is black, so dragging one changes what an operator can see and nothing about what is drawn. `a` sizes the window to the canvas one texel to one texel, which is what a downstream window capture wants. Three things came out of splitting them: the canvas now goes through a **record**, so a replay is at the size the performance ran at rather than at whatever `--size` says; a window resize no longer reallocates every slot's target on the render thread, which was the last GPU allocation on the frame path; and `present_mode` is now chosen (`Fifo`) rather than taken from whatever order the driver listed, which had made frame pacing a property of the machine with nothing saying so. Not built: fullscreen and display selection, because anything past a preview is the output-routing seam's job |
| MCP | Works. `--mcp PORT` serves the Model Context Protocol on loopback, so a chat client can read a slot's procedure, rewrite it, and be told what the compiler and the frame budget made of it — **vibe live coding**, where "the one that's showing, a bit more vivid" is a small edit to a declarative file. It is the **third control surface** after the keyboard and MIDI and obeys the same rule, so **a set a model rewrote replays with no model attached** — which is not a feature anyone could retrofit but this invariant used for something it was not designed for. It was not true when this landed: a review found that a procedure change went through a *file* and not a record, so an AI-driven set replayed with the procedures it started with. `procedure` records close it, and the material is now the last thing a session was missing. Most of it never touches the frame — reading is a file, writing is a check and a file, and the compile, the frame-boundary swap and the rollback are `--watch`'s, so **a model that writes something too expensive is caught by the machinery that already catches a human who does**. `write_procedure` returns the checker's diagnostics rather than a bare failure, which is what closes the loop. Two resources come with it: the IR spec, and a built-in vocabulary **generated from the checker's own table**, because prose drifts from code and a generated list cannot. Loopback only, on purpose; `ssh -L` is the way in |
| Tempo source | Works, and the verification is the interesting part. `--tempo-source` runs a separate program that reports where the beat is, as an **anchor** — a beat, a tempo, and the source's own clock reading at which both were true — so the pipe's delay never becomes phase error. The first anchor aligns the grid and every one after it is trimmed, because the source is another program from another repository; a shared grid carries a beat *number*, which is the one thing a beat tracker structurally cannot find. **The first implementation is Ableton Link, and what it is for changed when it was tested.** rekordbox joins a Link session and its decks can be made to follow it, but it never publishes the deck's BPM — Pioneer's answer is that Link has no master for a tempo fader to be the master of — so with rekordbox the picture does not follow the music by itself. That is a smaller failure than it first reads: a set is not a rehearsed timeline, matching the timing on the night is the craft, and "set by hand" is the answer this milestone gave to gain, the tempo octave and the panic key alike. What Link adds even when set by hand is that the setting is **shared** — `b` taps a grid on this machine, a Link tap puts every peer on it at once. With a peer that *drives* Link it follows on its own |
| **A panic key** | Deliberately not built, and the reasoning is in `docs/roadmap.md`. Everything it would undo is already manually recoverable, and an anomaly is usually one frame and usually harmless — so a control that resets a performance in response to one does more damage than the thing it responds to. What was missing was a way to *see*, not a way to recover, which is the third time this milestone has landed on **show the number, act on nothing** |
| **Bloom** | Does not exist. Values above 1.0 are what would feed it |
| **Automatic gain** | Deliberately not built. The meter shows the number; nothing acts on it. An exposure that moves by itself is the worst thing that can happen on stage, and the honest order is to show the measurement first |
| Hot swap | Works. Built on a worker thread, installed at a frame boundary, watched for a window, rolled back automatically. `--watch` on the CLI. Starts cold — no state transfer |
| **Prompt-driven generation** | No in-app prompt. Generation happens through MCP from an outside client, and what reaches the engine is a file whoever wrote it — see the MCP row. What is not built is a control *here* that asks for material |

One thing known to be fragile rather than merely absent:

- **GPU timestamps.** See Working style below. Every performance number in this repository
  came from a host clock.

Simulation time used to be an f32 running sum (`t += dt * steps`), whose rounding depended
on how a frame's steps were grouped — twenty steps taken one at a time and ten taken two at
a time landed two ULP apart at `dt = 1/60`, which the built-in camera could turn into a
different last bit of a rendered pixel. It is now `steps_taken * dt` from an integer
counter, and `t` advances once per substep rather than once per frame: a frame of two steps
runs its two `element` passes at the two instants two frames of one step would.

The scan, measured because the spec asks for it rather than assuming it is free: **0.083 ms
per frame at capacity 262144** on this machine — a host clock, taken as the slope between
4 and 64 back-to-back scans in one submit so that submit overhead cancels. The same workload
under `TIMESTAMP_QUERY` resolves to 0.000 ms, which is the flakiness the Working style
section describes rather than a fast shader.

The hand-written `Points` pipeline is still present. It was the vertical slice that had to
keep working while everything else was built, and it can go once the generated path covers
what it covers.

---

## Working style

- **Vertical slices, not layers.** Not "build the whole signal bus" but "get a triangle on
  screen, then never break it"
- Keep it running. Do not commit a state that does not build
- **The gates are hooks in the repository, not habits.** `git config core.hooksPath
  .githooks` once per clone, and then a commit is refused if its Rust is not what
  `cargo fmt` writes, and a push is refused unless the whole workspace formats, lints under
  `-D warnings`, and passes every test. Formatting is on the commit because it costs a
  second; the rest is on the push because it costs minutes, and a gate that costs minutes
  gets skipped until it is not a gate
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
  karakuri-audio/     input device, analysis, tempo tracking, the beat lock
  karakuri-midi/      wire messages, and the operator's map of them
  karakuri-store/     content-addressed artifact store, ndjson I/O
  karakuri-cli/       V1 entry point
.githooks/            pre-commit: `cargo fmt --check` on what is staged
                      pre-push:   fmt, clippy and every test
                      enable with `git config core.hooksPath .githooks`
docs/
  ir-spec.md          the IR. Settled; open questions are empty
  manual.md           how to play it: flags, keys, and what each does
  plugins.md          out-of-process helpers, and why they are out of process
  roadmap.md          where this goes after V1
examples/             app presets: seven L1, four L2, one L3, one Field,
                      eight L4, and a control-surface map to copy
.karakuri/            the store: artifacts, sets, sessions, scratch and the
                      edit history (gitignored, and `--store` moves it)
```

---

## Documents

| | |
|---|---|
| This file | What exists, how to run it, and the rules that hold now |
| [docs/ir-spec.md](docs/ir-spec.md) | The IR: grammar, semantics, lowering, record formats. Its Resolved section records the decisions and why, including the ones implementation forced |
| [docs/roadmap.md](docs/roadmap.md) | Milestones after V1, and the **Demands on earlier work** each one places on code written now. That is the part worth reading before making a decision here |
