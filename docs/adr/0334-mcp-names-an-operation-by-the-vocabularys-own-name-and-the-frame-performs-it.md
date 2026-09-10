---
id: 0334
title: MCP names an operation by the vocabulary's own name, and the frame performs it
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0083, 0087, 0090, 0094]
tags: [mcp, vocabulary, surfaces, operations, m5]
---

# MCP names an operation by the vocabulary's own name, and the frame performs it

## Context

[ADR-0235](0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md)
turned the MCP column of [the operations page](../manual/operations.html) from a column of `gap`
into a column of `plan`: every operation is meant to be connected, and what could stop a
performance is refused until the operator opens its class. It changed no code on the day it was
written. What it left is a backlog, and this record is that backlog met.

Counted on 2026-09-09, before this: **40 `plan`, 7 `has`, 17 `gap`.** The seventeen are
ADR-0315's twelve surface-state rows and the sequencer's five, which carry that record's one
sentence — *a model has no window*. The seven are the tools
[ADR-0199](0199-mcp-names-its-operations-and-performs-them-itself.md) built. **The forty are the
question**, and every one of them is an operation the vocabulary already names, with a payload a
surface can say.

### The seven exist because of what they do, not because a tool is how MCP works

ADR-0199 gave MCP a tool per operation for a reason that was true of those six (seven, after
`wire_input`): each performed something **only the server could** — reading a file, checking and
writing one, reading the store, draining a channel of swap reports. `Live::operate` would have
printed *no record* and done nothing for all of them, and there is no `Live` on a connection
thread to call anyway. So *"what routes is the naming"*, and the performing is that module's.

**None of that is true of the forty.** `SetGain` is not a file and not a listing; it is the thing
the panel's fader does, the thing `g` does, the thing a mapped knob does. What is missing is not a
tool. What is missing is a way to reach the place those three already end.

### Forty tools would be forty second spellings

[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) is that every surface routes into
one name, and [ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md) made that
name a crate with no dependencies precisely so that every surface can reach it. A tool per row
would put a second name beside each of the forty — `set_gain` beside *Gain* — and
[P-0087](../principles/0087-name-the-property-never-the-shape.md) is the rule that a shape invented
beside a property is the thing that goes stale. The page's own test would then be comparing two
hand-written lists, which is what `karakuri-operation` exists to abolish.

## Decision

**One tool, `operate`. It takes an operation of the vocabulary by its own heading and a payload,
is audited by `gate`, and is performed where every other surface's operations are performed — on
the main thread, at the next frame, through a channel the frame loop drains.**

The seven stay exactly as they are. `operate` refuses a name one of them takes, and names the
tool.

### The name on the wire is the manual's heading, and there is no second spelling

`{"operation": "Gain", "with": {"deck": 0, "gain": 0.8}}`. The heading is
`Operation::title`'s string, which is the `<h3>` of the row and is compared against the page by
`the_manual_and_the_vocabulary_agree`. **A model and an operator read the same words**, and there
is no table of wire names to keep in step with anything.

The headings are prose — *Put a node's previous version back* is a sentence — and that is the cost.
It buys the thing an identifier would have given up: nothing here can drift from the manual,
because it *is* the manual's word.

### The JSON spelling lives in `karakuri-environment`, not in the vocabulary

`karakuri-operation` has no dependencies and that is the whole point of it, so a serialiser there
would be hand-written over `std` — a `Display` and a parser per payload, in the crate every surface
depends on, for one surface's protocol. **The spelling is a fact about a wire, not about the
vocabulary**, so it is in `mcp.rs`, where `serde_json` already is and where `layer_of`/`kind_of`
already cross between the compiler's list and the vocabulary's.

**It is a table, and the table is the schema, the parser and the test.** `SPELLED` has one row per
operation, in the page's order. Each row carries a `sample` — one instance of the operation and
the smallest call that names it, written side by side — a `make` that reads a payload, and a
`shape` that is its JSON Schema. The title is not written in the table at all: it comes back from
`(sample)().title()`, so a row cannot name an operation that does not exist.

**Exhaustiveness is a `match` with no wildcard**, which is ADR-0235's own discipline in `gate` and
`written`'s before it: `sayable` answers, per variant, whether this surface can name the operation
and why not where it cannot, and a sixty-fifth operation stops the build there.
`the_table_and_the_classification_agree` is what keeps the table and that match from drifting, and
`every_operation_of_the_vocabulary_is_spelled_here` walks `Operation::TITLES` against the table
both ways.

### Paths never cross the protocol, and two payloads are refused for carrying one

`mcp.rs`'s rule — a client may be on another machine through an `ssh -L`, where a path means
nothing, and a tool that took one would invite a model to write anywhere on the render machine's
disk. Every free string in an accepted payload is a **name something in this run produced**: a Set
id (`checked_id`, one path component), a signal on the bus, a parameter a procedure declares, a
version by the name the store filed it under — which is the name `write_procedure` hands back. A
separator is refused in one sentence, in one function.

Two are refused for the path itself:

- **`AttachBeatSource`'s `Process` arm** is a command line for the render machine to run, which is
  a path with arguments after it. The `AudioInput` arm is taken and the `Process` arm is refused
  saying what it is and that `--tempo-source` is still where a process starts.
- **`TransferSet`'s `Take { file: PathBuf }`** carries one outright. That row is refused for a
  second reason as well — see below — and would be refused for this one if it were not.

### The audit is the one call, and a closed class is a built route

`gate::audit` is called in exactly one place, between `asked` and `perform`, and `operate` crosses
it like the seven. **Thirty of the thirty-one operations it takes are refused today** — every one but *Put a
node's previous version back*, and
that is the mechanism rather than a shortfall: ADR-0235's *"a closed class is reached and answered
with a refusal, which is a built route"*. The refusal is `gate::refusal`'s sentence, asserted by
equality and not by a `contains`, and it names the class and the pill that opens it, so a model can
hand it to the person sitting there ([P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).

**`LoadSet` is refused with the reading nobody took.** Its class is a predicate over its target, and
this server has read no residency — `Running::unread()`. That is the answer P-0094 asks for rather
than a guess that the deck is idle, and it is what `audited`'s own documentation predicted would
happen the day a tool named that operation.

### The frame performs it, and nothing on a connection thread does

`crates/karakuri`'s `App::operated` drains a third channel beside the two `Keeping::requests`
drains, and hands each operation to `App::performed` as an `Acted::Emitted` — the value a fader
hands it. **A model's `SetGain` and a hand on the strip are the same press from there on.**
`karakuri-cli`'s `Live::run_operations` is the same two lines against `Live::operate`, which is
where its keys and its MIDI messages end; it serves the same `mcp::serve`, so a channel it did not
drain would answer every call *the render loop had not taken this operation*.

The answer goes back at the frame it was performed on, and says where anything it *started* is
reported: `swap_outcome` for a rebuild, the grid for a scheduled move. Nothing holds a connection
open across a transition.

### The rows, and why the ones that stay stay

**31 rows go `has operate`; 9 stay `plan`; the 17 `gap` are untouched.** The column reads
**38 `has`, 9 `plan`, 17 `gap`.**

The nine are three groups and every one of them is work somewhere else:

- **Three carry `Undecided`** — *Walk the edit history*, *Edit the file instead*, *Move a boundary*.
  A surface can only say what the vocabulary has settled. (`SelectScope` carries `Undecided` too and
  is not among them: its row is `gap` for ADR-0315's reason, which was taken first and is the one a
  reader of the page meets.)
- **Five have no performer on the frame this drain lands on** — *Star a Set*, *Send a Set to
  somebody*, *Narrow the published interface*, *Choose where the frame goes*, *Record the session*.
  Four of the five have a performer sitting in `crates/karakuri`'s own press arm; *Choose where the
  frame goes* needs the event loop, which that frame does not hold; *Narrow the published interface*
  has none anywhere. **They are refused by the spelling rather than accepted**, because a call
  answered `ok` for work that did not happen is the plausible wrong answer P-0094 is written
  against, and it is worse than a refusal.
- **One is a defect** — *Set a deck's mask position*. `crates/karakuri`'s `reading` supplies the mask
  for `SetMaskShape` and for `Wipe` and not for `SetMaskPosition`, so `written` answers
  `Owed(NotRead(Mask))` and nothing moves. One arm, and it is named here so that it is fixed rather
  than worked around.

***Element capacity, seeds, the camera* was a tenth of these and left while this was being
written.** ADR-0328 gave the Inspector's deck head two chips, `resized` and `re_salted` went onto
the frame this drain lands on, and the row became `has operate` with nothing on this surface
changing. That is the shape each of the five leaves in, and it is why they are listed by name.

### The curriculum

`karakuri://operations` is a third resource beside the IR specification and the built-ins, rendered
from `SPELLED`: every name `operate` takes, the schema of its payload, and one call that names it.
[ADR-0092](0092-a-resource-listing-is-a-curriculum-not-an-index.md)'s rule, and generated for
`docs/contributing.md` §4's first reason — the same table accepts the call, so the page cannot say a
name exists that does not. The tool's own `operation` enum comes from the same list.

## Alternatives

### a. Forty tools, one per operation

The literal reading of *the MCP column is a backlog*. It loses on P-0087 and P-0090: forty second
spellings of names that already exist, forty schemas to keep in step with forty payloads, and a
page test reduced to comparing two hand-written lists. It also makes the tool list forty-seven
entries long, which is a list a model reads once and searches badly — where one tool with an
enumerated `operation` is a list it can be given whole.

### b. A tool per class

`mix`, `master`, `transport`, `program`. Fewer tools, and it looked tidy. It fails on what a class
*is*: `gate::Class` is a permission boundary drawn by P-0094's question, not a grouping of
arguments — `SetMasterOut` is filed with the mix faders and `SetResidency` with the live deck, and a
caller reading a tool called `mix` would look for neither. It also puts the audit's own taxonomy on
the wire, so the day a row moves class the tool it is called through changes, which is a rename a
model cannot see coming. And five of the sixty-four are in no class at all.

### c. Perform on the server's thread, as the seven do

The shortest change: `operate` would need no channel and no drain. It is refused by
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
and by arithmetic. There is no `Deck`, no `Look` and no `Chain` on a connection thread, and reaching
one would mean a lock the render thread also takes — putting a fader write behind a mutex a slow
disk can be holding, on the path that must not wait. It is also the second route P-0090 exists to
prevent: two places that turn `SetGain` into a `Record::Gain` are two places that can disagree.

### d. Accept everything the gate lets through, and let the panel drop what it cannot perform

It would have made five more rows `has` today. It is the plausible wrong answer: the client is told
`ok`, the terminal says *nothing moved, and nothing here decides it*, and the model reports success
to the person beside it. P-0094's register for a tool surface, and ADR-0301 already refused it for
one of these five in as many words — *"a success reported for an act that had no effect"*.

### e. An identifier per operation on the wire, rather than the heading

`set_gain` instead of *Gain*: shorter, and easier to type. It is a second spelling of a name that
already exists, and it would have to be written down somewhere and kept in step with the page by
hand — which is the drift `karakuri-operation` was built to end. The heading is long and it cannot
be wrong.

## Consequences

- **The MCP column reads 38 `has`, 9 `plan`, 17 `gap`.** M5.10's exit is not met and its *Blocked
  on* is empty: the nine are work in `crates/karakuri` and in the vocabulary, and each is named on
  the roadmap.
- **`mcp.rs` publishes eight tools.** `tools()` gains one generated entry;
  `every_tool_this_server_publishes_names_an_operation_the_gate_lets_through` drives it with the one
  operation the audit passes, and `every_operation_operate_takes_stands_where_the_page_says_it_does`
  is what checks the other thirty.
- **`Reporter` has a third channel**, bounded at `ASKED` like the other two and for the same reason:
  a deck being saved to a slow disk must not be able to fill the queue a fader moves through.
- **`crates/karakuri` gains one function and two call sites**, and touches no press arm.
  `karakuri-cli` gains one method and one line in `Live::run_requests`.
- **A defect is recorded rather than fixed here**: `reading` does not supply the mask for
  `SetMaskPosition`. It was found by asking, per row, whether an accepted call would be true.
- **No new principle.** This applies P-0090 and P-0087, and the closed half of it is ADR-0235's.
