---
id: 0341
title: A route that answers is built, and a send that ends in a dialog is `gap`
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0083, 0090, 0094, 0096]
tags: [mcp, operations, surfaces, m5]
---

# A route that answers is built, and a send that ends in a dialog is `gap`

## Context

[ADR-0334](0334-mcp-names-an-operation-by-the-vocabularys-own-name-and-the-frame-performs-it.md)
built `operate` and left nine rows `plan`. Six of the nine are this record: five it filed under
*no performer on the frame this drain lands on* — *Star a Set*, *Send a Set to somebody*, *Narrow
the published interface*, *Choose where the frame goes*, *Record the session* — and one it filed as
a defect, *Set a deck's mask position*. The other three carry `Undecided` payloads and are the
vocabulary's.

**Five of the six needed no performer written.** That is what asking, row by row, turned up:

- *Narrow the published interface* is performed by `attended`, which `App::performed` has called
  since ADR-0329 in the same arm as the fold, the capacity and the salt. The drain has been reaching
  it all along. ADR-0334's clause *"has none anywhere"* was wrong about this row.
- *Choose where the frame goes* is performed by `routed`, which needs an `ActiveEventLoop`. The
  drain is called from two `winit` handlers — `new_events`, on the wake `SERVED` asks for, and
  `window_event` — and **both are handed one**. The row was `plan` for want of an argument, not of a
  loop; `new_events` took its as `_event_loop` and the underscore came off.
- *Record the session* is performed by `Sessions::asked`, which needs the store as well as the
  engine. One more field of `App`.
- *Star a Set* is performed by `favourite`, and what it performs for a model is a **refusal** —
  [ADR-0301](0301-a-models-star-is-refused-because-a-favourite-has-no-sandbox-to-land-in.md) wrote
  that arm before there was a route to it.
- *Set a deck's mask position* is one arm of `crates/karakuri`'s `reading`. `karakuri-cli`'s
  equivalent has always named all three mask operations, which is what makes it a slip in one file.

**One is not waiting for anything**, and that is the decision this record is mostly about.

## Decision

**Five rows go `has operate`. `App::operated` calls the window's own press arms for the three that
`App::performed` does not reach, and `reading` grows the mask arm it was missing.** *Send a Set to
somebody, and take one in* goes **`gap`**.

### A route that answers is a built route, even when the answer is a refusal

`operate` takes *Star a Set*, the audit passes it — `Standing::Open`, and `gate.rs` is untouched —
and `favourite` refuses it with ADR-0301's sentence, which names the id, says `my sets` is the list
of Sets the operator chose, and says where the Set actually is. **The refusal goes back as `Err`**,
so the call fails rather than reporting a star that landed nowhere.

That is the same shape as ADR-0235's *"a closed class is reached and answered with a refusal, which
is a built route"*, one door along: there the refusal is the gate's, here it is the performer's, and
in both the badge describes **what a model can reach** rather than what it is allowed. ADR-0301 said
so itself — *"the row is owed a tool; what that tool answers is this record"* — and its `plan` was
the absence of the tool, which is now spent.

### The send is `gap`, because nothing here is owed

Both halves of *Send a Set to somebody, and take one in* are outside what this protocol carries, and
no performer moved onto the drain's frame would change either:

- **A send names no destination and never will.** [ADR-0260](0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)
  settled that sending is a *read* and that a read's answer goes where the surface that asked puts
  answers. On the panel that place is the system's own save dialog
  ([ADR-0311](0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md),
  which superseded the folder scope whole), and **a model cannot answer a dialog**.
- **A take names a file.** `SetTransfer::Take { file: PathBuf }` carries a path, and paths never
  cross this protocol — a client may be on another machine through an `ssh -L`, where one means
  nothing. ADR-0334 refused this arm for that reason already.

So the badge is `gap` and not `plan`: `plan` is a promise that a route is coming, and this one is
not. The refusal names the route a model's operator does have — `--package ID > FILE.kbset` and
`--take-in FILE`.

### `Sayable` grows the variant that says it, and loses the one it was drawn against

`mcp.rs`'s classification has no wildcard arm, so the distinction has to be a variant or it is not
checked: `Sayable::Never(&'static str)` went in beside `Sayable::Unperformed`, the two reading `gap`
against `plan`, and the difference between them is *whether anybody is owed anything*.

**Then `Unperformed` emptied, and it is deleted rather than kept.** ADR-0338's two rows moved in
another session on this same day — *Keep a node's procedure* gained an arm in `App::operated`, and
*Load a procedure over a layer* turned out never to have needed one, its `overlaid` sitting in
`App::performed` beside the fold and the Set load exactly as *Narrow the published interface*'s did.
That took the last row out of the group, and a never-constructed variant is dead code that says
something false about the program.

**So a `plan` badge in the MCP column is now only an unsettled payload.** Every operation is
performed, has a tool, is a window's, is `Undecided`, or is `Never`. The day a row is added that a
surface can say and the frame cannot perform, the `match` has no wildcard and stops the build until
somebody puts the answer back — which is the same one-line edit adding it was, and is why keeping an
empty variant against that day buys nothing.

### The recording's id is not on the wire

`Recording::Start` carries `Option<String>` and `operate` always sends `None`. Each start takes a
fresh stamp ([ADR-0289](0289-the-rec-pill-is-a-record-stop-toggle-and-each-start-takes-a-fresh-stamp.md))
— a second head under one id is read back as edits — and `Sessions::begin` refuses a named one in as
many words. A key this table accepted and the frame then refused would be an `ok` for work that did
not happen, which is what P-0094 rules out here.

## Alternatives

### a. Leave *Star a Set* `plan`, or make it `gap`

`plan` reads *a route is coming*, and none is: the answer a model gets is final and is written down.
`gap` reads *this surface cannot reach it at all*, which ADR-0301 already refused in as many words —
a model can name a Set id, so `gap` would be a lie about what is reachable rather than a statement
about what is allowed. The badge that describes the row is `has`.

### b. A ninth tool that hands a model the bundle text

*Send a Set to somebody* as bytes over the wire, rather than a file. It answers the wrong question:
sending is putting a Set **somewhere for somebody**, and for a model that somewhere is a path on the
render machine's disk. What a model wants of a Set it can already ask for — `read_set` says what one
holds and declares — and a whole `.kbset` on a connection is `--package`'s output with the
destination taken out, which is the half that made it a send. It also fails ADR-0334's own rule that
`operate` names operations and does not grow tools beside them.

### c. Accept a named recording and let the performer refuse it

One more key in the schema, and every call carrying it answered `ok` for a recording that did not
start. The plausible wrong answer, and cheaper to not offer the key.

### d. Keep `Sayable::Unperformed` for the next row that needs it

An `#[allow(dead_code)]` and a comment saying *the group is empty today*. It makes the compiler stop
asking the question this classification exists to force: the no-wildcard `match` is what makes
somebody say where a new operation stands, and a variant sitting there unused is a default they can
take without deciding. The record above is where the shape is kept, and the code says what is true.

### e. A second performer on the drain's frame, rather than calling the press arm

It would have made `App::operated` self-contained. It is the second route
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) exists to prevent: two places that
turn a `RouteFrame` into a window are two places that can disagree, and the star's refusal would
then be written twice.

## Consequences

- **Seven rows leave the `plan` column** — six as `has operate` and the send as `gap`. The seventh
  is ADR-0338's *Load a procedure over a layer*, folded in here on the day rather than left for a
  record of its own: its performer was never missing and the sentence refusing it had stopped being
  true, which is this record's first finding met a second time. M5.10's exit
  is not met and its *Blocked on* is still empty. **No count is written here**: ADR-0338's two rows
  were being given performers in another session while this was, so the column is what M5.10's own
  `grep` says it is and what stays is the three `Undecided` payloads.
- **`App::operated` takes an `ActiveEventLoop` and the store**, and calls `favourite`, `routed` and
  `Sessions::asked` in the order and by the call the pointer's button-up arm calls them. Nine
  arguments, which is two past clippy's line and is allowed with the reason written there.
- **A star answers `Err` and performs nothing.** It is the one arm of the drain that does not fall
  through to `App::performed`, because nothing was written and there is no record to write.
- **`reading` names three mask operations.** The defect ADR-0334 recorded is closed, and it was a
  hole under a mapped knob before it was a hole under a call.
- **ADR-0334's own reading of two rows was wrong**, and both are corrected here rather than in that
  record: *Narrow the published interface* had a performer, and *Choose where the frame goes* had a
  loop. Its Consequences carry a dated pointer to this.
- **`Sayable::Unperformed` is gone**, with its arm in `operated`'s refusal and its case in
  `a_name_the_vocabulary_does_not_carry_is_refused_naming_the_nearest`. The variant lasted one day.
- **No new principle.** This applies P-0083, P-0090 and ADR-0235's *a closed class is a built
  route* to a refusal that is the performer's rather than the gate's.

**2026-09-10, the other surface.** The five performers above are `crates/karakuri`'s, and `operate`
is served by `karakuri-cli --mcp` too — the same `mcp::serve`, the same audit, a different drain.
**That program has none of them.** `Live::run_operations` handed every operation to `Live::operate`,
which converts and applies the records and has no second arm, and then answered `Ok` regardless: a
model was told *"`Star a Set` was performed"* when nothing was starred, and the same for *Choose
where the frame goes*, *Narrow the published interface* and *Record the session*. `Element capacity,
seeds, the camera` and the other rows that write no record had been answered that way since
ADR-0334. **It is now refused there**, in one sentence naming the operation, saying this program has
no control for it and naming the instrument as the surface that answers it — `karakuri-cli`'s
`answered`, which is where `written`'s three answers become the one a caller gets. **No list is
written here and none is written there either**: what is refused is every operation `written`
answers `Silent` or `Owed` for, decided per call, so a row that gains a record stops being refused
without anybody editing a table — this record's own reason for writing no count. The exception is
the one arm that was already there: *Tap the beat*, which `Live::tap` performs.
*Nudge the latency offset*, *Halve or double the grid* and *Quit* have keys on that keyboard and are
refused all the same. A key that nudges is not a performer for an operation that names a value, and
what the drain reaches is `Live::operate`'s conversion and not the key handler.

**The column does not move.** A badge is a claim about `operate`, `operate` is performed on the
frame the panel drains, and every row this record made `has` is performed there. What would have
moved it is teaching the CLI the five performers, which is the second route *(d)* above refused. A
refusal naming the program that cannot perform it is this record's own rule read from the other
side: a route that answers is built, and on this surface the answer is that it has no control.
