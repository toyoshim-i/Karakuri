---
id: 0232
title: A control is thrown because the request is asynchronous, and a curve is a helper where the control map already sits
status: accepted
date: 2026-08-31
supersedes: []
superseded_by: []
principles: [0085, 0087, 0090, 0091, 0092, 0094]
tags: [operations, console, transitions, automation, architecture]
---

# A control is thrown because the request is asynchronous, and a curve is a helper where the control map already sits

## Context

**Three panel rows could not be built, and the same function refuses all three.**
`karakuri_operation_record::written` is *one exhaustive match over every operation in the
vocabulary*, and one of its arms holds the rows in question:

```rust
Operation::FadeDeck { .. }
| Operation::Crossfade { .. }
| Operation::Wipe { .. }
| Operation::SelectRenderer { .. }
| Operation::TapBeat
| Operation::ScaleGrid { .. } => Written::Owed(Owed::NotSettled),
```

On [the operations page](../manual/operations.html) those are *Fade a deck out or in*
(`panel strip`, `key f g`), *Crossfade to the next deck* (`panel crossfader`, `key x`) and *Choose
which renderer of a deck is live* (`panel renderer chips`, `key r`) — every one of them badged
**on the grid**, and every panel route on them still a plan rather than a build. The panel program
already refuses to fake it: `crates/karakuri/src/main.rs`'s `unwritten` says of this arm that
*"[`Written::Owed`] is **a gap nobody has closed yet**. A fade owes a record and no build can make
it, so a press that reads as *nothing happened* is exactly the wrong reading"*, and the same file
records that **nothing on the panel reaches that arm on purpose any more** — its ten emitting
controls all write records, and a row that could only print a question was therefore not drawn.

**`Owed::NotSettled` is not an error and says exactly what is missing.** Its own definition:

> **Its record is not a function of values alone, and who supplies the rest is undecided.** Six
> operations, and they divide cleanly: scheduling one (a fade, a crossfade, a wipe, a renderer
> selection) needs the grid's position quantised onto a musical instant, plus the quantum and the
> length that `Operation::SetTransition` sets and no record carries; and moving the grid (a tap, an
> octave shift) needs the beat tracker rather than a value, and may be refused by it.

`Owed::why` prints it in one sentence — *"the record it writes is not a function of values alone,
and who supplies the rest is undecided"* — which is the shape of the whole problem: **nothing was
wrong, and nobody had said whose the missing values were.**

### The wall, in `Current`'s own words

[`Current`] is *what is running, at the instant the operation arrives* — a plain struct of
readings the caller hands in — and its doc names the gap and its own way out in the same paragraph:

> A plain struct of values rather than a trait the caller implements, and the reason is the survey
> rather than taste: part of what a record needs is *not readable from anything*. The quantum, the
> length of a fade and the wipe shape are `Operation::SetTransition`'s, and that operation **writes
> no record at all** — it is a surface's own setting deciding what the *next* move means. A trait
> over "the deck" could not answer for them; **a value handed in can, the day somebody decides whose
> they are.**

That day is this record. The crate's header states the same wall from the other side, and states
the rule that resolves it: **No engine** — *"Anything an operation's record needs that is arithmetic
rather than a value has to arrive inside [`Current`], and the six operations whose record needs the
grid's position or the beat tracker are [`Owed::NotSettled`] until somebody decides who computes
it."*

**And the precedent for paying it is in the same file, one paragraph down.** `Operation::SetSync`
was the seventh member of that arm and left it: *"What was actually missing was a reading, and
[`Current::tempo`] is it."* The panel's own account of that day is worth keeping, because it is this
one in miniature — the sync chip was *"a reachable affordance over an unwritable record"*, the
refusal to route around it kept the cost low
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)), and *"what
changed is the conversion, not this file's authority."*

### The two functions that say they are temporary

`karakuri-cli` builds two of these records by hand and each says so at its own definition.
`Live::fade_slot`:

> **Not through [`Live::operate`], and the reason is the record rather than the route.** This is
> what `f`, `g`, `x` and `c` write, and the operations they name — [`Operation::FadeDeck`],
> [`Operation::Crossfade`] and [`Operation::Wipe`] — are all `Owed::NotSettled`: `written` cannot
> build a `transition` record, because the start above is a quantisation against the grid and the
> length is `Operation::SetTransition`'s, which writes no record at all. Routing them through
> `operate` would print that gap where a fade used to happen, which is the one thing a route change
> may not do. **This function is what goes the day the conversion lands.**

And `Live::cycle_renderer`: *"Its record is written here rather than through [`Live::operate`],
because [`Operation::SelectRenderer`] is `Owed::NotSettled` for [`Live::fade_slot`]'s reason: a
selection lands on the grid, and the `start` below is a quantisation `written` has no engine to
compute."* Both bodies are three lines and the same three lines — read `beats` off the oscillator,
`karakuri_engine::transition::quantise` it, hand the result to `mix::transition_record` or
`mix::select_record` — and both are named in `OWED_RECORD_PATHS`, the table a test walks so that
**a record built beside the conversion cannot be added by accident.**

### The maintainer settled it in conversation, and the argument is not the one an outside reader would guess

Reading the three rows as a missing envelope feature — *the panel needs fade curves and the engine
does not have them* — is the obvious reconstruction and it is wrong. On 2026-08-31:

> そもそもそういったエンベロープ制御みたいなのは想定してないよ。
>
> 投げるものってなってるのは、そこが非同期だから。レンダリングエンジンの制約で同期処理は禁止して
> るから、基本的にはASAPを前提に非同期で投げてるだけ。
>
> そういうの、入れるとしたらオートメーションとして今の制御マップの位置にヘルパーとして入れるのか
> な。
>
> あとその手のカーブ制御を入れるってのは、逆にタイミング保証をするなら同期になるって意味でもある。
> 一方で、その制御自体をレンダリングのクリティカルパスから外す、つまりマップの中のヘルパーにして
> あげることで、それは自然と実現できるようになるね。

— *envelope control of that kind was never the intention. The reason a control is described as
something you **throw** is that it is asynchronous there: the render engine's constraint forbids
synchronous work, so it is simply dispatched asynchronously on the assumption of ASAP. If that sort
of thing were added, it would go in as **automation**, as a helper at the position the control map
occupies now. And adding curve control of that kind conversely means that guaranteeing timing means
becoming synchronous — while taking the control itself off the rendering critical path, that is,
making it a helper inside the map, is what makes it achievable naturally.*

**The panel is real-time control and it authors no envelopes.** The console page's own word for the
crossfader is his: *"It is a readout and a **throw**, and it is not a blend you hold… Throwing it to
an end asks for a crossfade."* A control is thrown not because a fader is flung but because the
request is **asynchronous** — an operation is dispatched ASAP and lands later. **Curve control is
the opposite thing**, and its cost is stated exactly: a guaranteed shape over a guaranteed length
would need synchrony *if it lived on the render path*. It does not have to live there.

### The two principles that already covered this, because he was right and they do

P-0001 is the
constraint from the engine's end, and the clause is the one that matters here: it rules out sizing a
buffer from what a record asks for at the moment it asks, because **"the record schedules the work,
and the work lands later."** That sentence is the whole of *why a control is thrown*, written down
about buffers before anyone asked it about faders.

[P-0087](../principles/0087-name-the-property-never-the-shape.md)
is its surface consequence: *a control whose request has been made and not yet granted has three
things to say and must say all three* — the state it is in, the state it was asked for, and that the
second has not happened. **Its second user is already built.** *"The mixer strip's two faders… A
transition is armed on the beat grid — `Deck::transitions_on` is what a surface asks, and with the
default quantum a fade is due up to a bar after the key"* — `view::Strip` carries `gain_to` and
`opacity_to`, `Strip::gain_pending` and `Strip::opacity_pending` derive the third clause every
frame, and `crates/karakuri-console/tests/armed.rs` asserts each of them. **So the drawing of
*thrown and not landed* exists.** What was missing was never the picture; it was the conversion
under it.

### P-0069 is the frame that makes the helper obviously right

[The three clocks](0255-three-clocks-run-at-once-and-a-slower-ones-work-never-lands-on-a-faster-one.md) fixes what
may happen on each: the frame clock is *"GPU execution and parameter evaluation. Nothing else"*, and
the beat/bar clock is *"Variant switching, parameter morphs, transitions — **selection** among
options already prepared."* An automation helper runs on the slower clock and **emits operations**;
the frame clock only evaluates what they scheduled. Timing guarantees therefore belong to the slower
clock, and moving a curve there costs the render path nothing — which is the maintainer's last
sentence with the principle's names attached. ADR-0222 has already walked this exact ground for a
step grid: *"A step grid is **the beat clock subdivided**, not a fourth clock… No new state, no
scheduler, no event queue."*

### P-0094 is the precedent for the shape, and the control map is the thing already standing in it

[Nothing external enters the render process](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
settles the shape by worked example — *a GPL library runs as a **separate program** behind a generic
interface* — and the generic interface, here, is the operation vocabulary. `karakuri-midi` is what
already sits in that position and its header says so: *"It produces a
[`karakuri_operation::Operation`] — 'the operator asked for gain 0.7 on deck 2' — and stops"*, with
`examples/surface.map` as the table an operator writes and `Map` as the pure function from a
hardware message to an operation. Its own account of why this is safe is the sentence automation
inherits whole: *"the reason an automatic writer is safe to run is that it can do nothing a human
could not do through the same interface."*

## Decision

The maintainer's, on 2026-08-31, in five parts.

**1. The panel is real-time control and authors no envelopes, and *thrown* means asynchronous.**
This is the framing every other part follows from, and it is recorded because the alternative
reading is the natural one: the three rows are not blocked on a missing curve feature. They are
ordinary asynchronous requests whose records nobody had finished describing. The render path forbids
synchronous work (P-0091), so a control dispatches an operation ASAP and it lands later — and
P-0075's *asked for, and waiting* is the panel's whole vocabulary for that, already drawn on two
faders.

**2. `Current` grows what a scheduled move means, and that answers *whose they are*.** The three
things `Operation::SetTransition` sets — the quantum, the length, the wipe shape — are **the
surface's**, and they arrive at the conversion the way the tempo and the transport already arrive:
as a reading handed in, optional like every other field, with `Current::default()` still meaning *I
read nothing*. What the reading carries is **the instant a scheduled move lands on, the length it
lasts, and the curve it takes**: the first is the quantum resolved against the grid by whoever holds
one, the second is `SetTransition`'s value unchanged, and the third is on no surface at all — one
constant in `karakuri-cli` that says of itself it is *"Not on a key"*, carried for
`Look::white_point`'s reason, because the record is written whole and a conversion that dropped it
would have to invent a shape for a move somebody else chose.

**The wipe shape is the one of the three settings that does not come**, and leaving it out is part
of the decision rather than an omission in it. A shape is a `Record::Mask`'s and already reaches a
record through `Operation::SetMaskShape`, which converts today; a field for it here would be a value
no arm reads.

**The resolved instant rather than the quantum, and that is the crate's own rule rather than a new
one.** *"Anything an operation's record needs that is arithmetic rather than a value has to arrive
inside [`Current`]"* — `quantise` is arithmetic and lives in `karakuri-engine`, which this crate may
not reach and must not. It is `Current::tempo`'s arrangement exactly: the engine's policy stays the
engine's, the conversion takes a value, and the agreement between them is held by a test rather than
by a dependency.

**3. ASAP is already spelled, so nothing is invented for a caller with no opinion about the
grid.**
`karakuri_engine::transition::quantise`'s own doc: *"`quantum` of 0 is 'now' and returns `beats` —
an operator who wants a cut does not want to wait for the bar."* So for a panel with no quantum
control the resolved instant **is** the grid's position, read off the oscillator exactly as
`mix::current_tempo` is read off it today, and the window computes nothing and decides nothing —
P-0090 in the form the panel already stated it: *"computing the anchor here would have been a window
binary taking a decision about a file format."* A caller that wants the next bar calls `quantise`,
and a caller that wants the next bar is a caller that holds an engine.
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) is passed rather than
dodged: the mechanism existed and was asked what it was made of.

**And the panel hands in nothing at all today, which is this decision working rather than an
exception to it.** `crates/karakuri` draws no control that asks for a fade, a crossfade or a
selection, so its reading is `None` and any of the three would answer *the caller's to fix*. Handing
in a start it had no control for would be the window choosing a setting nobody chose — which is what
the field being the surface's means, read from the other end. The day the panel draws one of those
controls it hands in what that control says, and until then the honest answer is that it has none.

**4. `karakuri-cli`'s two hand-built records are deleted by this rather than carried, and the wipe's
is not.** `Live::fade_slot` said *"this function is what goes the day the conversion lands"* and this
is that day; `cycle_renderer` keeps only the part that was always its own — *"Which renderer to move
to is the keyboard's own translation and stays here whatever happens to the record"* — and gives up
its `mix::select_record` call. [P-0090](../principles/0090-a-surface-offers-it-never-decides.md)
is what that is worth: the records stop being derived beside the conversion, which is *"exactly the
drift `karakuri-operation-record` exists to end"* in the words of the test that guards it.

**`Live::wipe` keeps its own path, and that is the boundary of what this settles.** A wipe is four
operations' worth of gesture and only *contains* a scheduled move: `Operation::Wipe` is six records,
and two of the values in them — the shape the front takes and how soft its edge is — are neither the
operation's nor this reading's. The shape is `SetTransition`'s third setting and the softness is one
more constant on no surface, and both are a `Record::Mask`'s. **What a wipe still owes is who says
they are its**, which is the same question this record answered for the quantum and the length,
asked about a different pair and not answered here.

**5. Curve control, if it is ever wanted, is automation: a helper where the control map already
sits, never a feature of the engine.** A trigger becomes a timed series of the same operations,
emitted on the beat clock, off the render path — one more writer standing where `karakuri-midi`
stands, depending on `karakuri-operation` and nothing else, unable to say anything a hand could not
say. **That is the vocabulary gaining a way in rather than growing a feature**, which is rule 01 of
[the manual](../manual/index.html) — *one vocabulary, four ways in* — extended by the same move
[ADR-0222](0222-a-sequencer-lane-is-a-fifth-route-and-not-a-binding.md) made for a step lane.
**The count in that phrase is already out of date and this record does not restate it wrongly**:
[the console page](../manual/console.html) draws five routes, `sequencer → command` beside the other
four, and says *"The sequencer is a fifth route, and it only works for the same reason."* Automation
is the sixth by that drawing — or, read more usefully, it is the fifth one generalised: a lane
writes a value per step and a helper writes a curve over a length, and both are operations emitted
on the slower clock.

> **Annotated 2026-08-31, later the same day: what *"where the control map already sits"* points at
> now has a name.**
> [ADR-0236](0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md)
> decides that a map is **the layer between a surface and the vocabulary**, holding three things —
> which surface gesture names which operation, whether a surface may reach a class of operations at
> all (the audit ADR-0235 requires), and helpers that carry time. **The position this record named by
> pointing at `karakuri-midi` is the third of those**, and the pointing is why it could not be named
> here: that crate was, and still is, the layer's only built instance. Nothing above changes — the
> helper still does not exist, and where curve control goes is still what this record decided. That
> record also takes up the question left open below about a helper and a sequencer lane, and answers
> it only this far: they are the same job of the same layer, with steps where the helper has a curve,
> and whether they are one mechanism or two is still open.
> `docs/contributing.md` §4.

## Alternatives

### a. The operation carries the quantum and the length itself

`FadeDeck { deck, to, quantum, length }`, and the conversion needs no reading at all. It is the
smallest change on this page — three struct fields, one arm moves from `Owed` to `Records`, no new
concept anywhere — and it makes a fade self-describing on every route: a MIDI map line or an MCP
call could ask for *this deck, to 0.8, over four beats from the next bar* without any surface state
in the middle.

**It loses because it contradicts what `SetTransition` is for, and the manual is where that is
said.** The operations page states the row plainly: *"These change nothing you can see and write
nothing to the stream. They decide what the **next** fade, crossfade or wipe means."* A setting that
decides what the next gesture means is not a parameter of the gesture; putting it in the payload
deletes the row's reason to exist while leaving the row on the page, which is a manual change
dressed as a struct change and
[ADR-0226](0226-m5-closes-when-the-manual-is-implemented-and-the-meter-is-progress-rather-than-completion.md)
is why that order is not available.

**And it makes every press read three values where a hand names one.** The vocabulary's rule is that
*an operation says what it wants*
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)) — a
destination, and the affordance belongs to whoever draws the control. `f` is one key and it would
have to name a quantum and a length it did not ask about; the pad on `examples/surface.map` that
would map to a fade cannot say three numbers at all, because *a note is a press*. The three settings
would then have to be carried by every surface anyway, to fill in the two the gesture did not name —
which is the reading this record takes, arrived at the long way with the vocabulary widened on the
way past.

### b. `SetTransition` writes a record

Let the setting into the session stream, and the conversion reads the quantum and the length back
out of what the stream last said rather than out of a reading. It removes the *whose are they*
question outright — they are the session's — and it makes a replay reproduce a fade's length without
anyone handing anything in, which is the property a record exists for.

**It loses on the same page and one line earlier.** *Writes nothing to the stream* is the manual's
sentence about this row, so this is a manual change first and a code change second. It is also a
change with a real cost rather than a formality: a console setting in the session stream means every
`z`, `n` or `j` press is a line in a performance's history, and a replay of that history reconstructs
**a console** as well as a picture. The nearest thing already decided points the other way —
`karakuri-midi`'s header on why the map is not in the stream: *"which knob is which is a property of
the hardware in the room, and replaying one room's wiring in another is not replaying a
performance."* A quantum is that kind of fact.

**And the stream does not need it, because the record it feeds is absolute.** `fade_slot`'s doc says
why: *"the record carries an absolute beat count, because 'at the next bar' is a different instant
depending on when it is read and a beat count is the same one on every run."* `Record::Transition`
already carries the start and the length, so a replay reproduces the fade exactly without ever
learning what quantum produced it. A `SetTransition` record would be the cause written into the
stream beside every one of its consequences — which is
[ADR-0227](0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md)'s argument
against a pattern record, in a smaller case.

### c. Build the envelope into the engine

Give `Transition` an envelope with authored segments, and let the panel draw and edit one. It is what
every other instrument in this shape has, the engine already owns `Curve`, `Transition::value_at`
and the exactness tests at both ends, and it would make the three rows a drawing problem rather than
a conversion problem.

**It loses on the one sentence the maintainer put it in: guaranteeing timing on the render path
means becoming synchronous.** A transition today is *a function of `beats` and nothing else* —
P-0069's own words for why the middle clock is statable at all, *"so the same records give the same
fade frame for frame on a machine running at a different rate, and a tempo correction mid-fade is
correct rather than a glitch."* An authored envelope that must hit its points is either that same
pure function of `beats`, in which case it is a `Curve` and not an envelope, or it is a thing with
state that has to be advanced at a moment somebody promised — and the frame clock's charter is *GPU
execution and parameter evaluation. Nothing else.* The rule this breaks is P-0091 from the other end,
which is exactly how P-0069 described their relationship.

**And it puts the feature in the one place it cannot be taken back out of.** A helper on the control
map can be deleted, replaced, or run in a second process; a scheduler inside `deck.rs` is on the
critical path of every frame whether or not anybody is automating anything. P-0094's worked example
is the general form of the answer — a thing that does not need to be in the render process is a
separate program behind a generic interface — and the generic interface already exists and is the
operation vocabulary.

### d. Let the conversion call `quantise` itself

`karakuri-operation-record` takes a dependency on `karakuri-engine`, calls `quantise(beats, quantum)`
with a `beats` reading, and the quantum stays a setting handed in. It is one function, it is pure, it
takes two `f64` and returns one, and it is the piece of arithmetic actually in question.

**It loses because the dependency is the whole cost and the function is not.** The crate's header
states the constraint as a charter rather than a preference: *"a crate that pulled `wgpu` in would be
unreachable from every surface again"*, and `karakuri-engine` is the crate that has `wgpu` in it. The
survey that produced `Owed::NotSettled` was taken **after** that was decided, which is why the answer
was *who supplies the rest* rather than *add the dependency*.

**And it would settle nothing, because the quantum still has no owner.** Handing in a quantum to be
quantised and handing in the instant it resolved to are the same information from the same caller;
the second needs no dependency and the first needs one, and neither of them decides whose the setting
is. The question this record answers would still be open, with a `wgpu` in the map's dependency tree
to show for it.

## Consequences

- **The three rows become buildable and the arm keeps three members.** `FadeDeck`, `Crossfade` and
  `SelectRenderer` leave it. `Wipe` stays, for the reason under D4 rather than for the one that held
  all four before — the instant and the length are settled for it too, and what is not is whose the
  shape and the softness are. `TapBeat` and `ScaleGrid` stay for the reason they always had: they
  need *the beat tracker rather than a value, and may be refused by it*, which is a different
  question with a different owner. **So the arm's own sentence needs rewriting rather than shortening**
  — it currently says *scheduling one … needs the grid's position quantised onto a musical instant,
  plus the quantum and the length*, and after this the scheduling half is one operation and a
  different sentence.
- **The three move to `Owed::NotRead`, not to `Written::Records`, and that is the point.** A caller
  that hands in no transition reading gets *the caller's to fix* rather than *nobody has decided
  this* — the only one of the three `Owed` reasons that names something a caller can do. It is
  `SetSync`'s path exactly, and the panel's mask reading already demonstrates the shape being worth
  having: *"a reading that did not arrive answers `Owed(NotRead(Reading::Mask))`, so a harness that
  stopped handing one in would say so out loud instead of moving nothing."*
- **Two tests in `karakuri-cli` fail on purpose and are the record of the landing.**
  `the_keys_that_keep_their_own_path_are_the_ones_whose_record_is_not_settled` asserts `f g`, `x`,
  `c`, `r`, `b` and `, .` all answer `NotSettled` against `Current::default()`, and it exists to fail
  this way — *"the day somebody settles one of these conversions, this fails and names the key that is
  now due to move."* Three of its six rows come out — `f g`, `x` and `r` — leaving `c`, `b` and
  `, .`. And `every_record_written_outside_operate_is_a_path_whose_conversion_is_owed` has a floor of
  four reached methods, already lowered once from five with the reason written at it; with
  `fade_slot` and `cycle_renderer` deleted from `OWED_RECORD_PATHS`, `operate` and `wipe` are what is
  left, so the floor moves again or it fails as the dead scan it was written to catch.
- **P-0075's second user gets something to be pending about on the panel.** The strip's armed fade is
  drawn, tested and, today, reachable only from `karakuri-cli`'s `f`. With the conversion landed the
  panel's own strip can ask for one, which is the first time the drawing and the gesture are on the
  same surface.
- **The next automatic writer will be the second one, and
  [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) already
  ruled it.** *"The transition
  is the only automatic writer that exists. A follower, a sequencer lane and a signal binding are all
  named in the plan and none of them writes a mix control yet. This is written down now rather than
  when the second one arrives, because the second one is the change that would otherwise settle it by
  accident."* An automation helper is that second writer, it emits operations, and a hand on a control
  cancels what it scheduled at `Deck::set_gain` and its neighbours — no new rule, and nothing for the
  helper to opt into.
- **The word *automation* enters the vocabulary here and appears nowhere else in the workspace.**
  `grep -ri automation docs/ crates/ examples/` finds nothing outside this record. That is worth
  stating rather than leaving to be discovered: nothing is being renamed, and nothing that exists is
  being described.
- **This began landing while it was being written**, in another session, and the shape it took is the
  one recorded above: `karakuri_operation_record::Transition` carrying a `start`, a `beats` and a
  `curve`, the crate header's *No engine* paragraph rewritten around *"the arithmetic stays in the
  crate that owns the grid and what crosses the seam is its result"*, and `Current`'s *"the day
  somebody decides whose they are"* answered in place — *"what settled it was deciding **whose** the
  setting is rather than finding somewhere to read it from."* The `written` arm, the `Reading`
  variant and `karakuri-cli`'s two functions had not moved when this record was closed, which is why
  the arm is quoted above as it stood on the morning of the decision.

### What this leaves undone

- **The helper does not exist, and neither does what triggers it.** This record decides where curve
  control goes if it is wanted and refuses the two places it could have gone; it builds nothing. A
  helper needs a clock reading, a shape, a length, an emitter and something to start it, and none of
  those is chosen here. The maintainer's own sentence is conditional — *if that sort of thing were
  added* — and this record keeps it conditional.
- **Whether an automation helper and a sequencer lane are one thing is not settled.** ADR-0222 put a
  lane on the beat clock emitting operations, off the render path, for P-0094's reason; the shape
  described here is the same shape with a curve where the lane has steps. Whether that is one
  mechanism with two front ends or two mechanisms is a question for whoever builds the first of them,
  and it is a real one — that record's *"the largest thing left is not the sequencer's"* is this
  question seen from the bay.
- **Whose control the mock's transition row is, is not settled here.** The console draws
  `wipe · iris · next bar · 8 beats` beside the crossfader, and the operations page gives that row to
  `SetTransition` with keys `z n j`. Under this record those four pills are the panel's own settings
  and feed the reading; under a helper they could as easily be the helper's authoring controls. Both
  readings fit the drawing, nothing in the mock distinguishes them, and this record does not choose.
- ***Composite a deck's renderers* is not unblocked by any of this**, and it is named rather than
  assumed. `Operation::SetCompositing` is not in the `NotSettled` arm at all — it is
  `Silent::NoRecord`, beside `LoadSet`, because *"`Record::Merge` is what a Set says about its
  layering"* and *"nothing in a stream says that the Set in slot 3 composites."* Layering is fixed
  when the Set is built: `--merge N` gives every renderer a cleared target of its own and folds them
  through an L5, `Set`'s `merge` field is set at construction from `Layering::Composite`, and the row
  wears a **launch** badge on the operations page for exactly that reason. Turning it on in a running
  deck is an engine feature and a record that does not exist, not a conversion.
- **The wipe's two values have no owner, and that is the residue of this decision rather than a new
  question.** `Operation::Wipe` stays owed until somebody says whether the shape a front takes and
  how soft its edge is belong to the surface — as the quantum and the length now do — or to the mask
  the deck is already wearing, which `Current::mask` reads. Both readings exist in the code today,
  which is why it is a decision and not a lookup.
