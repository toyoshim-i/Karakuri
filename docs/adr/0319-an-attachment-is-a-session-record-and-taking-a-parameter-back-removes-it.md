---
id: 0319
title: An attachment is a session record, taking a parameter back removes it, and an authority chip is a destination
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0084, 0090, 0092, 0094]
tags: [console, inspector, engine, records, vocabulary, signal, agents, m5.5]
---

# An attachment is a session record, taking a parameter back removes it, and an authority chip is a destination

## Context

Three rows of [every operation](../manual/operations.html) waited on one sentence, and it was
written down as a sentence rather than as a question. `docs/roadmap.md`'s M5.5 said of *Attach a
signal to a parameter* and *Set a node's authority* that both are

> the same one-line shape as `Deck::write_param` once somebody decides **what each records**

and of the third:

> ***Take a parameter back* has nothing to call even once that route exists**: `Set::bind` has no
> inverse anywhere in the workspace and `Binding` carries no suspended state.

Everything under all three was built.
[ADR-0280](0280-a-parameter-written-to-a-live-set-is-a-session-record.md) laid the road —
operation → `Record` → `apply` → a `Deck` setter, with `Deck::write_param` reaching `live_mut` and
`Record::Ride` under it — and said in its own consequences that the road generalises and that only
the record was open. [ADR-0211](0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)
had already written `Operation::SetAuthority` and `Record::Authority` and said the engine work was
owed; `swap::Request::authorities` landed with it, so a grant survives a rebuild.
[ADR-0286](0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)
made a parameter row a control and left two things for this record by name:

> **What a `.param.bound` row's fader does.** … Whether a hand may move a knob a signal is holding
> is *Take a parameter back*'s question.

And the console had been drawing two of the three affordances with nothing behind them: the
`man / sug / auto` chips since 2026-08-29, *"the affordance and this pass does not claim a press on
them"*, and no `.sens` row at all — omitted on
[ADR-0191](0191-a-panel-draws-what-the-engine-can-be-in.md)'s terms, because nothing in
`crates/karakuri` bound anything and a bound row was a state the program could not enter.

**So what was open was three things and not one**: what an attachment records, what a take-back
*is*, and what a hand on a bound knob does.

## Decision

### 1. `Record::Source` is the attachment, and the take-back is the same record with nothing in it

```ndjson
{"t":"source","slot":0,"layer":"L1","key":"turbulence","source":{"signal":"energy","curve":"pow2","range":[0.1,2.4]}}
{"t":"source","slot":0,"layer":"L1","key":"turbulence"}
```

`Vocabulary::Session`, so `is_set_state` is false, `Store::write_set` refuses it,
`setfile::from_lines` skips it with the sentence every session record gets, and
`project::key_for` drops it. It is `Record::Bind`'s session twin on exactly `Record::Ride`'s terms
against `Record::Param`: a `bind` says what a Set *is* and carries no deck slot, and this says what
an operator *did*, to one deck slot, at one instant.

**Named for what it answers rather than for the act.** *Source* is the manual's own word — a bound
row shows its source instead of a number, `.pval.src` — so the record, the readout and the note
*Who is holding a control* are one word. `attach` was the alternative and it reads wrong at the
take-back, where the record says `attach` with nothing to attach.

**One record for both acts, and `source` present or absent is which.** They are one fact — *what is
driving this parameter* — so two `t`s would be two things the projection drops for one reason and
the applier has to keep in step by care. Present or absent as a unit is a thing nothing can write
down half detached, which is `karakuri_store::record::NodeAt`'s own argument one record along and
`docs/contributing.md` §4's structural tier.

**Its address is `bind`'s and not `ride`'s, and that is not a drift.** A value lands wherever the
name is declared, so `ParamAt`'s wildcard names no node and therefore no layer — which is why
`Record::Ride` carries one `Option<NodeAt>`. A binding is resolved through the nodes of **one**
layer: `karakuri_engine::binding::Binding` carries a required `layer` and an optional `index`, and
`Set::bind` walks `nodes_of(layer)`. There has never been a binding meaning *every layer*, so
`layer` here is always said and is **read** — which is the one thing `Record::Param`'s `layer`
never was.

**So `Operation::AttachSignal` and `Operation::TakeParamBack` carry a `BindAt` and not a
`ParamAt`.** That is a payload change to two variants nothing constructed, and it is the same
argument at the vocabulary: an address that can ask for something the engine cannot do is an
address refused at the far end for a reason the vocabulary already knew, and
`karakuri_operation_record::written` would have had to invent a layer to write the record with —
writing down the placeholder ADR-0280 §1 refused to reproduce.

**Its payload is `Record::Bind`'s four fields, restated and not shared.** One `#[serde(flatten)]`ed
struct across both would make a Set file's record and a session's one shape, which is what ADR-0280
refused for `param` and `ride`. What *is* shared is the semantics:
`setfile::binding_from_source` builds the `bind` those fields spell and hands it to
`binding_from_record`, so the three diagnostics that decoder owns — `signal=bpm`, `octaves` without
`fbm`, a generator on a binding that is not to `noise` — are said in its words on every route, and
there is one decoder rather than two.

### 2. Taking a parameter back **removes** the attachment

`Set::unbind(layer, index, key)` is `Set::bind`'s inverse, addressed by exactly the triple
`Set::bind` keys *at most one binding per (layer, index, param)* on — so a wildcard attachment and
an addressed one on the same name are two attachments and a take-back removes the one it names.

`Binding` gains no suspended state. **A suspended binding is a fourth thing to be** — beside
attached, absent, and blended at a low confidence — and it would have to be drawn, recorded,
restated on a rebuild and reasoned about at every confidence, where the arithmetic for *not driving
this parameter* is already written and is the **absence**:
[P-0084](../principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)'s
blend writes the param's own value at confidence 0.0, which is exactly what a hand asking for its
knob back is asking for.

What is given up is *hand it back* in one press, which
`docs/manual/console.html` promised until 2026-09-09 (*"The binding is kept and stops driving, so
you can hand it back"*) and which nothing anywhere could have done. What replaces it is that the
session stream carries the source, the curve and the range on the line that attached it, so
re-attaching is a thing a record can say.

### 3. A ride on a bound key writes, and the binding stays

**The operation is not refused and nothing cancels.** `Operation::WriteParam` on a bound parameter
moves the value the binding blends *from* and leaves the attachment where it is —
order-independent by construction, which is
[ADR-0047](0047-a-binding-blends-on-confidence.md)'s *"`--param` on a bound parameter is not a
competing writer"* — so nothing an operator does to a knob detaches a signal by accident. Only
`Record::Source` attaches one and only `Record::Source` detaches one.

**`Deck::cancel` is not the precedent it looks like.**
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)'s
*a hand cancels whatever automatic thing was writing the control it lands on* is enforced by
`Deck::set_gain`, `set_opacity` and `set_mask_position` each calling `Deck::cancel`, and what they
cancel is a **scheduled move** — a one-shot with an end, which an operator is overriding. An
attachment is a standing arrangement a Set file declares. Cancelling one on a knob nudge would
destroy the source, the curve and the range with no undo anywhere in this system, which is a much
larger thing to lose than a fade.

**The sequencer already decided this shape**, one bay along and for the same reason:
*"A lane's take-back is the mute rather than the hand. A hand's write lands at once and the lane
writes again at the next step, so the way to keep what a hand did is the label beside the row."*
A binding is that read continuously.

**What the console does about it is a different answer, and it is the console's.** A bound row's
fader is **drawn and not taken hold of** — `Param::movable` is `false` — because the number a hand
would write is not the number the row is showing: at a measurement's full confidence the manual
value carries no weight, so the handle would move under the hand and the picture would not. That is
ADR-0286's *"a handle that jumped to the pointer would be a lie about what a handle is"* read on
the value axis instead of on the track. It is **not** a refusal of the write —
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md): a `--param` still lands, and the
console is saying which affordance is on this row, not what may be asked. The affordance that is
there is `take back`, and after it the row is a handle again.

### 4. The sensitivity row is four chips, two of them controls

`.sens` under a `.param.bound` row: the source, the curve, the range, and `take back`.

- **The source is a readout.** The signal bus is **open by design** — `SignalBus::sample` cannot
  fail and a name nobody provides comes back at confidence 0.0 — so there is no list of sources
  anywhere for a chooser to be built over, and a console inventing one would be the surface
  deciding what may be asked for, which is P-0090 exactly inverted. A source is named where a
  source can be named: a Set file's `bind` line, or `--bind`.
- **The curve chip is a control**, and it is the blend chip's shape: four destinations
  (`karakuri_operation::Curve::ALL`, which arrives here with its first reader, on `Authority::ALL`'s
  terms), a cycle drawn over them in the console, and `AttachSignal` naming the one it arrives at.
  It **restates** the attachment — the signal and the range go back unchanged — so a press for a
  different shape cannot silently re-map the signal.
- **The range is a readout**, because a range is the procedure's declaration and not an operator's
  to write, which is `ParamValue`'s own sentence. **And there is no confidence on this row**: a
  value arrives with how well it is known, so a number an operator could set there would be telling
  the system how much to trust its own measurement — P-0084 inverted.
- **`take back` is the other control.**

**`view::Source::range` is not `view::Param::range`, and the two are two facts.** The row's range is
what the control was *published* over — the span the fader rides — and the attachment's is what the
signal is *mapped onto*, which a `bind` may narrow again within it. One range on the row would mean
drawing one and writing the other.

### 5. The authority chips are three destinations, and all three are claimed

`InspectorPane::set_authority` emits `Operation::SetAuthority { deck, node, authority }` — a
destination and never a step (P-0090), so pressing the word a node is already on asks for what it
already has, which is the renderer row's rule and the anchor's. `auth_chips` is the one derivation,
painted and pressed, replacing the running sum inside the painter — the shape a press had nowhere
to ask about, which is `param_rect`'s own correction one row down.

**A head that folds more than one node draws none and claims none**, which is `Node::authority`
being `None`. That field now carries the node's **address** beside the level —
`view::NodeAuthority` — because a press has to say which node it is about and the two are absent
together; two `Option`s would be *present or absent as a unit* held true by prose.

**And what an authority does is said on the page rather than implied.** Nothing in this workspace
writes a parameter on an agent's behalf, so no addressed write is refused *because of* a level, and
M5.5's own sentence stands: *whether an addressed write by an agent onto a node the operator kept is
refused cannot be decided until something writes a parameter on an agent's behalf.* What a level
**does** reach is
[ADR-0223](0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md)'s wildcard
refusal — a bare-name control over nodes that are not all under one authority is refused whole — so
granting one renderer and keeping the other narrows what one knob may do from the next press. The
record is kept so that an agent's write can be refused against it when something writes one.

### 6. The engine writers are three lines, as ADR-0280 predicted

`Deck::bind`, `Deck::unbind` and `Deck::set_authority`, each reaching `HotSwap::live_mut()` by
`Deck::write_param`'s road and for its argument: nothing here renders and nothing here replaces, so
*which Sets is this frame made of* has the answer it had before the call. None of the three
compiles anything — a binding is resolved by the next `Set::prepare` and an authority is a flag —
so all three are on screen at the next frame. `Deck::bind` **allocates** (`Set::bind` pushes into a
`Vec` and asks `Set::published`), which is why it belongs where a press is handled and not inside
`Frame::render`; that is where every record in this system is applied already.

## Alternatives rejected

### a. A suspended binding — *stop driving without losing the attachment*

The console's own tip, and the operation's own documentation, both promised it. Rejected in §2: it
is a fourth state with nothing behind it, and the thing it would buy — re-attaching in one press —
is bought instead by the record that attached it being in the stream. It is also the alternative
that would have had to be **restated on every rebuild** to survive one, so the cheap-looking
version of it is a `Request` field and a `Watch` field as well as a `Binding` field.

### b. A ride silently overriding a binding — or cancelling it

Two shapes of the same mistake and they fail in opposite directions. *Overriding* means a knob that
moves and a picture that does not, at any confidence above zero, with nothing saying why —
P-0094's plausible wrong picture. *Cancelling*, on `Deck::set_gain`'s precedent, means an
accidental nudge destroys a source, a curve and a range with no undo. §3 keeps the write legal and
puts the affordance where the manual's rule 02 already put it: *take back sits right beside it.*

### c. A per-deck authority, or a `Record::Bind` grown a `slot`

The first is ADR-0211's own rejected alternative and is not re-argued here. The second is
ADR-0280's alternative (b) one record along, and loses on the same three counts: the classification
would stop being a property of the variant, the projection would stop being a function of the
record type, and *what does a deck slot mean inside a Set file* has no answer.

### d. The record in the Set file only — `save` writes the bindings anyway

A live save already reads the live `Set` and writes its bindings as `bind` records, so the *state*
an operator ends on is not lost without this record. Rejected by
[P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md) in the same words ADR-0280
was: a session that recorded an operator attaching a signal at the second chorus and replayed it
attached from the first frame is not the same performance, and one that replayed it never attached
is a hole in the stream. A record is what a replay reconstructs a performance from.

### e. A source chooser on the sensitivity row

The pill the mock draws `armed` looks like the audio-in pill, which opens a list read at the press.
Rejected in §4: there is no list. Adding one in the console would be this crate deciding which of
an open bus's names may be asked for, and the day a `band9` or a second measured signal exists the
console would be the thing that has to be edited for it to be reachable.

## Consequences

- **`karakuri-operation-record`'s `Silent(NoRecord)` group loses two more members**, and the
  paragraph keeps both as examples of what that answer means. `Operation::AttachSignal` and
  `Operation::TakeParamBack` write `Record::Source`; `WireInput`, `SetProperty`, `SetCompositing`
  and the rest are still one record short in exactly the old way.
- **`Operation::AttachSignal` and `Operation::TakeParamBack` changed payload**, from `ParamAt` to
  `BindAt`. Nothing in the workspace constructed either outside `gate.rs`'s vocabulary walk, so the
  change is one fixture; a third address type in the vocabulary is the cost, and §1 is why it is
  two facts rather than two spellings.
- **`karakuri_operation::Curve::ALL` and `karakuri_console::view::AUTHORITIES` both got their first
  reader**, which is what ADR-0211 said would bring `ALL` in — *"it arrives with the first reader"*.
  `AUTHORITIES` is public now because `input::PROBES` counts it, and a `3` written there would be a
  second answer.
- **`crate::input::PROBES` goes to thirty rows** and `CONTROLS` up by five: three authority chips
  and the sensitivity row's two controls. The two readouts are drawn and counted by nothing, which
  is this file's rule that a control claims what it acts on and no more.
- **A node group is no longer *n* rows of one height.** `group_h`, `param_rect` and `sens_rect` all
  walk `rows_h`, because a bound row carries a sensitivity row under it — the same correction
  `InspectorPane::group` made one level out, and the reason a running sum is not good enough once
  something is pressed.
- **`crates/karakuri`'s pane reads `Set::bindings` now**, which was the one of the seven readings it
  did not take. ADR-0191's reason for leaving it out has gone: a press can attach one.
- **The panes are re-read on a `ride`, a `source` and an `authority`**, beside the `transport` that
  was already there, because each moves what a pane draws and `Set::published` allocates. It is off
  the record rather than off the operation, on that line's own terms.
- **A live attachment and a live grant do not survive a re-point, and an attachment does not survive
  a rebuild.** `swap::Request::bindings` and `Request::authorities` are restated from `Watch`, which
  only a re-point writes — so a save of any `.kir` in a slot walks a live attachment back, exactly
  as it walked a ride back before
  [ADR-0282](0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md).
  That record's answer is the shape this one would take — `Set::carry_moved_from` at the install,
  carrying what somebody *did* and letting a build state its own — and it is deliberately not taken
  here, on ADR-0280 §6's terms: it is a change to what a rebuild **is**. **A replay is exact
  regardless**, because a replay builds afresh from the stream and meets these records at the frames
  they were made at.
  **The attachment half was closed the next day** — a rebuild inherits it, by
  `Set::carry_bound_from` beside the values' own carry, after the maintainer found the panel's
  picture had stopped answering to the room
  ([ADR-0339](0339-a-rebuild-inherits-the-attachments-somebody-made.md)). The grant half stands as
  written here, and that record says why.
- **The sequencer's `seq 1` and the mock's `midi 21` are still states this program cannot enter.**
  A `.sens` row is drawn only where `Set::bindings` holds one, so the mock's two non-binding sources
  and the `step` curve beside one of them stay undrawn — ADR-0191, unchanged.
- **`docs/manual/console.html` lost a promise it could not keep.** The `take back` tip said the
  binding was kept; it says what happens now, and *Who is holding a control* gained the two
  paragraphs §3 argues.
