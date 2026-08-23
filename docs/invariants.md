# Invariants

The rules in force, for whoever is about to change engine code. Several of them name the
test that enforces them, because an invariant that can be tested is tested rather than
reviewed. `README.md` is the front door and [roadmap.md](roadmap.md) is where the project is
going; this is what must still be true when it gets there.

These are the skeleton of the design. Breaking them means rebuilding later. When in doubt
during implementation, these win.

## Render thread

- **Never allocate on the render thread**
- **Never compile shaders on the render thread**
- Pipelines are double-buffered; swaps happen only on frame boundaries
- If a new pipeline exceeds the frame budget, roll back automatically

## State mutation

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
  and `gain`, `opacity`, `blend`, `mask`, `transition`, `select`, `preview`, `residency`,
  `look` and `transport` on every key that moves them. Each is built, read back, and only then applied,
  so what drives the engine is what a replay would decode rather than a second path that
  happens to agree with it.

  **`procedure` and `save` are in the stream on different terms, and the difference is
  deliberate**: they *describe* a change that has already happened rather than asking for
  one, so they are written and not applied. A `procedure` says what a slot became when a
  swap landed. A `save` says a Set file exists — the first record whose subject is outside
  the stream at all, and therefore the first one a replay is defined to skip rather than
  obey. It says which id it skipped. See "Records with an effect outside the stream" in
  `ir-spec.md`.

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

## Signals

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

## Determinism

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

## IR

- IR blocks cannot read the signal bus. External values arrive only through `param`
- Loop bounds are compile-time constants. There is no `while` and no recursion
- **IR cannot observe a buffer slot index.** Element identity is `seed`, a monotone spawn
  ordinal that travels with the element through compaction
- `capacity` is a Set-level dial, not part of a procedure's identity. Artifacts record cost
  per element, not total cost
- See `ir-spec.md` for the full specification

## Color

- The pipeline is linear and HDR end to end. Internal targets are `Rgba16Float`
- sRGB encoding happens once, at final output
- Values above 1.0 are expected; they are what feeds bloom

