---
id: 0280
title: A parameter written to a live Set is a session record, and the rebuild question is not settled with it
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0092]
tags: [records, vocabulary, engine, params, session, m5.4]
---

# A parameter written to a live Set is a session record, and the rebuild question is not settled with it

## Context

**A model can rewrite a whole procedure and cannot turn one knob.** `Operation::WriteParam` has been
in the vocabulary since it was written and nothing emits it;
`karakuri_operation_record::written` answered `Silent(NoRecord)` for it, and
[ADR-0199](0199-mcp-names-its-operations-and-performs-them-itself.md) recorded the reason in as many
words: *"`Record::Param` is a Set file's and has nowhere to put a slot."* The engine's writer,
`Set::write_param`, had exactly three callers — `karakuri-cli`'s launch-time fold, the restatement
inside `swap.rs`, and a test — every one of them at a **build**. There was no path in the workspace
that wrote a parameter to a Set that was playing.

[docs/roadmap.md](../roadmap.md) names the hole in the Inspector bay's *Blocked on*: *"There is no
public route from a `&mut Deck` to a live `Set`… That one hole stops* Write a parameter, Attach a
signal to a parameter *and* Set a node's authority*, and it is what makes this the bay where an
operator turns a knob and the bay with no way to turn one."*

**What was proposed first was a `Deck` writer that mutates the live Set and emits nothing.** That is
the obvious shape: `HotSwap::live_mut` exists, `Deck::schedule_selection` already reaches it, and a
knob is not a build. It is also named outright by
[P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md), under *What it rules out*:

> Mutating a live Set in place, which opens a hole in the record stream and loses replay, undo, A/B
> comparison and session recording together — the fork **is** the edit mechanism, not a fallback for
> expensive edits.

So *should a knob turn be recorded* was never open. What was open is **what the record is**.

### The two streams, established before choosing

A Set file (`.kbset`) is a **state projection** — what is loaded and what every value currently is.
A session stream is a **timeline** — a Set file's records, then `tick`s and the edits between them,
at `<store>/sessions/<stamp>.ndjson`. One record type serves both, and
`karakuri_store::record::Record::vocabulary` is the one place any record is classified into which.
A Set file is the session stream with the ticks dropped and the state folded down, and
`karakuri_store::project::project` is that fold.

**The vocabulary has already paid for a session twin of a Set file's record, twice.** `Record::Slot`
says what a Set *is* — this node runs this procedure — and carries no `slot`. `Record::Procedure`
says what a deck slot *became*, at a point in time, and carries one. `Record::Authority` is the
second, and [ADR-0211](0211-authority-is-set-per-node-and-the-record-is-the-sessions.md) states the
rule the third one inherits: every record naming a node of a Set goes in a Set file and carries no
deck slot, *"because what a Set is does not depend on which deck slot it is playing in"*.

## Decision

### 1. The record is `Record::Ride`, and it is the session's

`{"t":"ride","slot":0,"at":{"layer":"L4","index":1},"key":"glow.x","value":0.4}`.
`Vocabulary::Session`, so `is_set_state` is false, `Store::write_set` refuses it, and
`setfile::from_lines` skips it with the sentence every session record gets.

**`ride` and not a `param` grown a `slot`.** Three things break under the growth, and the third is
the one that decides it.

- **The classification stops being a property of the variant.** `Record::vocabulary` is an
  exhaustive match over variants and is *the one place any record is classified*; a `param` that is
  a Set file's when `slot` is absent and a session's when it is present is a record whose file
  depends on a field value. The exhaustive match that makes a new record impossible to forget stops
  being able to answer for this one.
- **The projection stops being a function of the record type.** `project::key_for` folds a `param`
  onto `Key::Param(layer, index, key)`. A `param` carrying a deck slot must **not** fold — see the
  next point — so one `t` would sometimes fold and sometimes not.
- **`written` would have to invent an address.** `ParamAt::node` is an `Option<NodeAt>`: a bare key
  names no node, and therefore names no layer. `Record::Param` requires a `layer`, so a conversion
  filling one in for a wildcard would be writing a placeholder — which is exactly what
  `Record::Param`'s `layer` already is (*"a placeholder that the loader ignored — the writer put
  `L1` on everything and said so"*), and that wart is unremovable because honouring the field now
  would silently retarget every Set file ever written. Growing the record would have been
  reproducing a defect the format is already stuck with.

`Record::Ride` therefore carries the address as **one optional field**, `at: Option<NodeAt>`, where
`karakuri_store::record::NodeAt` is `{layer, index}` and `index` absent is 0 on `Record::Procedure`'s
terms. *Present or absent as a unit* stops being prose the reader has to be told and becomes
something nothing can write down wrong — `docs/contributing.md` §4's structural tier.

### 2. It is dropped from the Set-file projection, and that is `select`'s reason

`project::key_for` answers `None`. This is the sharpest of that function's drops: a `ride` names a
layer, an index, a key and a value, so `Key::Param` would take it and the output would look right.
What it also names is a **deck slot**, and nothing in a stream says which deck slot the Set at the
head of it was played in — the format's own *"What no record says is what the deck held"*. Folding
one in would be guessing that a write on slot 3 was about the Set being written.

**Nothing is lost by the path an operator actually saves through.** A live save reads the live `Set`,
which already holds every ridden value. What cannot see them is the session-to-Set-file fold, which
already cannot see a `gain`, a `select` or an `authority` for the same reason, and the arm says so.

### 3. Two spellings of one act is the cost, and it is priced

A reader now meets `param` and `ride` and has to learn that they are not one thing written twice.
That cost is real and it is the cost `slot`/`procedure` already charges. What makes them two facts
is the projection: a `param` folds and a `ride` cannot, and no amount of prose would have made one
record able to do both. `karakuri-operation-record`'s rule about a second spelling being *"exactly
the drift this crate exists to end"* is about **one crate building one record two ways** — the
reason a crossfade is two calls to `fade` — and is not violated by two records for two files.

### 4. The write goes into the live Set through the deck, and it compiles nothing

`Deck::write_param(slot, &ParamWrite) -> Result<usize, CrossesAuthority>`, reaching
`HotSwap::live_mut()` internally, which is what `Deck::advance_selections` already does.

**`live_mut`'s refusal to be public does not cover this.** Its stated reason is that handing out the
live `Set` would be *"a second way to render a Set"* and that *"which Sets is this frame made of"*
would stop having one answer. Nothing here renders and nothing here replaces: the answer to that
question is unchanged by the call. `Deck::schedule_selection` is the public writer of exactly this
shape and reaches `live_mut` by exactly this road.

**And it is not a compile.** `Set::write_param` writes a number into a `HashMap<String, f32>`;
`Set::prepare` packs that map into a uniform on every frame of every slot. So the write is on screen
at the next frame with no worker, no candidate and no swap. `tests/live_param.rs` asserts that at
the texel, because a map that is right and a slot still rendering what it was built with are
indistinguishable anywhere else.

**The refusal survives.** `Set::write_param` is the one entry point and the place
[ADR-0223](0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md)'s
`CrossesAuthority` is decided, so a bare key over nodes that are not under one authority is refused
whole through this route as it is through a `--param` and a `param` record.

### 5. Vectors need nothing new, and the expansion is at the decoder

`glow.x` is a `ParamWrite::key`, because
[ADR-0268](0268-a-vector-parameter-is-driven-one-component-at-a-time.md) made a component a key.
A wide `value` stays legal on `ride` for the reason it is legal on `param` — one line a person or a
model writes — and `mix::change` expands it into one write per component with
`karakuri_ir::component_key`, which is the one spelling of that address in the workspace.

**Blind, where `setfile::from_lines` expands against the declarations.** That reader has just read
the `slot` records and holds the procedures; this one is looking at a Set already on air and holds
none of them. A component key nothing declares lands as `Ok(0)` and is said out loud in
`no_such_param`'s words, which is what a name a rebuild no longer declares has to do rather than
take the show down.

### 6. What a rebuild does with it is **not** settled here, and is a bigger record

**Settled, and smaller than this section makes it look —
[ADR-0282](0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md).**
The diagnosis below is right and the conclusion drawn from it is not: telling a declared value from
a moved one *does* need a record of which parameters were written, and `Set` can keep that record
itself — one set of keys per node, marked where a value is written, which is one field beside
`params` rather than a channel or a change to where a request is applied. The two costs weighed
against the engine's home did not survive contact: the request still states what it states and is
still reproducible from a record stream, and what the install adds is an inheritance the worker had
no way to state. The one thing this section names that the answer really does give up is the last
of them — an override for a dropped name re-landing when the name comes back — and 0282 gives it up
on purpose. Left as written below, because what it says happened is what happened.

A `--watch` rebuild restates `swap::Request::params`, which comes from `Watch::overrides`, which only
a re-point writes. So a knob moved today is walked back to where the slot was loaded on the next
save of any `.kir`, silently.

**The proposal weighed was that a rebuild should inherit by name from the outgoing live Set** — a
declared default read from the code because that is what the author just wrote, a live value carried
over because that is what the operator's hands are on — which would make the write into the live Set
the whole of it. **It is not implementable as stated, and the reason is a fact about the engine:**
`Set` holds `params` (the current values) and `ranges` (the declared bounds) and **no declared
defaults**. A value a knob moved and a value nobody has touched are the same entry in the same map.
So inheritance by name would carry the *outgoing* declared default forward too, and an author who
edits `param radius = 2.0` to `5.0` and saves would see nothing change — which is the one thing
`--watch` exists to do.

Telling the two apart needs a record of *which parameters were written*, and that is what
`Watch::overrides` is: it means the values that are not the code's. The question is only where that
list lives, and both homes cost more than this record does.

- **In the watcher**, fed by a live write. `Watch` is **moved into the build worker thread** by
  `HotSwap::new`, so the frame loop cannot call it — the only route is a channel, the way `Aim` is,
  which means a new message type and a send per ridden control per frame on the render thread.
- **In the engine**, as a *held* set on `Set` that `Set::write_param` marks and the install carries
  across at the frame boundary. One call, and it contradicts the argument written on eight
  `Request` fields — *"a request that depends on what happens to be live is not reproducible from a
  record stream"* — and moves where a rebuild's parameters are applied from the worker to the
  install. *(The second half of that sentence was wrong when it was written: a request's parameters
  are still applied at the worker, and what the install adds is an inheritance beside them —
  [ADR-0282](0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md)
  §3.)* It also loses a case the list keeps: an override for a name a rebuild dropped survives in
  `overrides` and re-lands when the name comes back, where a Set that never held it has nothing to
  carry.

That is a change to what a rebuild **is** rather than to where a value is stored, and it is left
open deliberately. Nothing above depends on the answer.

### 7. What this does and does not buy under P-0092

**A replay is exact and needs none of the above.** A rebuild is a live-only event: a session carries
it as `procedure` records, a replay builds afresh from source, and the `ride` records that follow
land at the frames they were made at. So the knob is where the operator left it, on replay, whether
or not a live rebuild would have walked it back.

**Recording the write is also what would make inheritance safe if it is ever built.** "The rebuild
reads live state" is the sentence that sounds like it breaks determinism. It does not, provided what
the live Set holds is itself determined by the records — which is exactly what this record
establishes, and was not true before it.

**One hole is left and it is older than this record.** A rebuild's restatement writes no record at
all: `Live::record_procedure` emits `procedure` and nothing else, so the camera, the salts, the
bindings, the interface, the edges and the params a request restates are invisible in the stream.
A session containing a `--watch` rebuild therefore already does not replay exactly. Naming it here
because this is the record that makes it visible; it belongs to `Request`, not to `ride`.

## Alternatives

### a. A `Deck` writer that mutates the live Set and emits nothing

What was proposed before P-0092 was read, and what the principle names in the sentence quoted above.
It is the cheapest thing that works this frame and it is the thing that cannot be replayed, undone,
A/B'd or recorded. Rejected by the principle rather than argued out here — the alternative is listed
so that it is not re-proposed as an obvious simplification, which is exactly what it looks like.

### b. `Record::Param` grows an optional `slot`

One record serves both files. Rejected on the three counts in §1, and the question it could not
answer is the one the maintainer put: **what does a slot mean inside a Set file, where there is
none?** Nothing — so `is_set_state`, `write_set` and the projection would each have to branch on
whether the field is present, and the classification that a new record cannot be added without
answering would stop being able to answer for this one.

### c. A `ride` carrying `layer` beside `index`, mirroring `param` exactly

One spelling of a node address across both records, learned once. Rejected because `param`'s
spelling cannot express *no node*: its `layer` is a required field that a wildcard fills with a
placeholder, and the format is stuck with it only because changing it now would retarget files. A
new record copying that shape would be reproducing a defect for symmetry with it.

### d. Expand a wide value in `written`, so the record is always scalar

`Record::Ride` would carry an `f32` and a `vec3` would be three records. Rejected because
`karakuri-operation-record` depends on `karakuri-operation` and `karakuri-store` **only**, by
charter — `karakuri_ir::component_key` is unreachable from it, and spelling `glow.x` by hand there
would be the second spelling of an address the workspace keeps in one place.

## Consequences

- **`Operation::WriteParam` writes a record**, and the `Silent(NoRecord)` group loses the one member
  whose reason was a missing record rather than a missing answer. The `Silent::NoRecord`
  documentation keeps the example, marked as having left, because it is what that answer *means*.
- **`mix::Change` is no longer `Copy`.** Every other variant moves the deck around a Set and is
  numbers and small enums; this one reaches inside a Set, and a parameter is addressed by name.
  `Clone` is kept.
- **A frame that carries a ride allocates**, on `mix.rs`'s existing terms: the record's `String` key
  and a one-to-three `Vec` of writes, once per ridden control per frame, because
  [ADR-0207](0207-a-continuous-control-says-one-thing-per-frame.md)'s router coalesces a continuous
  control to one operation a frame. That is the class `look` and `mask` are already in.
- **The panel is not wired here.** `crates/karakuri`'s `App::performed` gains the same arm
  `karakuri-cli`'s `Live::apply` has: `Change::Ride { slot, writes }`, a loop over
  `Deck::write_param(slot, write)`, `Ok(0)` reported with `no_such_param` and `Err` printed as it
  stands. That is the Inspector bay's row and another bay's commit.
- **The Inspector bay's *Blocked on* shrinks by one and not by three.** *Write a parameter* has a
  public route from a `&mut Deck` to a live `Set` now. *Attach a signal to a parameter* and *Set a
  node's authority* still have none: `Deck` has no `bind` and no `set_authority`, and both are the
  same one-line shape as `Deck::write_param` once somebody decides what each records.
- **A ridden knob still does not survive a rebuild**, by §6, and that is now a written question
  rather than an unnoticed one. *(Answered on the same day by
  [ADR-0282](0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md):
  it survives.)*
