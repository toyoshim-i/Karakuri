---
id: 0199
title: MCP names its operations and performs them itself, because all six are `Silent`
status: accepted
date: 2026-08-27
supersedes: []
superseded_by: []
principles: [0028, 0031]
tags: [cli, mcp, vocabulary, surfaces]
---

# MCP names its operations and performs them itself, because all six are `Silent`

## Context

`docs/roadmap.md`'s item 3 is *every surface routes into the named operation*, and MCP is the fourth
surface. The three before it each answered differently, and none of the three answers transfers:

- **the MIDI map moved wholesale** — message becomes `Operation`, `Live::operate` performs it, and
  `Action` was deleted
  ([ADR-0196](0196-a-map-line-names-a-state-and-an-old-line-is-refused.md));
- **the CLI's key handler had mostly moved already**, and what was left divided into gestures that
  convert in the decided parts and twelve keys that **cannot** route, because the operations they
  name write no record
  ([ADR-0198](0198-a-gesture-converts-in-the-parts-that-are-decided.md));
- **the console's `panel::Op` did not move at all**, on purpose: routing there would have meant the
  panel *accepting* a named region, which is an inbound adapter, and there is no caller for one
  ([ADR-0197](0197-the-consoles-op-stays-and-what-blocks-it-is-the-page-rather-than-the-code.md)).

### The six tools, and what each of them writes

`crates/karakuri-cli/src/mcp.rs` publishes six, and every one names an operation the manual already
specifies. `karakuri-operation-record`'s `written` answers for all six:

| tool | operation | `written` |
|---|---|---|
| `read_procedure` | `ReadProcedure { deck, node }` | `Silent(Question)` |
| `write_procedure` | `WriteProcedure { deck, node, source }` | `Silent(OnLanding)` |
| `swap_outcome` | `SwapOutcome` | `Silent(Question)` |
| `save_set` | `SaveSet { deck, id }` | `Silent(OnLanding)` |
| `read_set` | `ReadSet { id }` | `Silent(Question)` |
| `list_sets` | `ListSets { holds, layer }` | `Silent(Question)` |

**Not one of them writes a record where it is asked.** Four ask rather than change, and a question
changes no performance. The other two owe a record that is written where the *work* lands:
`Record::Procedure` when a swap lands or is rolled back, `Record::Save` at the frame the save
landed — because a record written at the ask would claim a swap the budget went on to reject and a
file the disk went on to refuse.

### So the route MIDI took is not available, for two reasons rather than one

`Live::operate` turns an operation into the records it writes and applies those. An operation it is
handed that writes none prints *no record* and does nothing — ADR-0198's finding about twelve keys,
holding here for all six tools. Routing `save_set` through it would stop saving.

And there is no `Live` on these threads to route into. The server is a listener thread and a thread
per connection over one `State`; reading a procedure is reading a file, writing one is checking it
and writing a file, and `swap_outcome` drains a channel. **One tool reaches the render loop** —
`save_set` sends a `SaveRequest` and waits with the state unlocked — and it already ends where the
`k` key ends, in `Live::save_set`. There is one save path in this program and this was never a
second copy of it.

### What was left over, and it is not nothing

MCP is the only surface that can say a node address. The manual's own gap section says MIDI *"cannot
express a node address, a parameter name or an id"*; a key press has nothing to say one with; the
panel does not reach inside a Set. `karakuri_operation::NodeAt` and `karakuri_operation::Layer` had
**no caller anywhere in the workspace** — a payload the vocabulary carried and nobody used.

## Decision

**The tool call becomes the named `Operation` at the wire, and the tool is performed from that
value.** `asked` is the one place a tool name and a JSON object become an operation; `perform`
dispatches on the operation rather than on the string; the tool functions take the operation's
payload. Nothing is handed to `Live::operate`, and the two `OnLanding` records go on being written
where the work lands.

That is a smaller claim than the MIDI path's and it is the one that is true here: **the vocabulary
is what this surface names things in, and performing them is this module's** — which is the same
sentence ADR-0198 wrote for a key that writes no record, arriving at a surface that has a caller
where ADR-0197's did not.

**The page's MCP column is now checkable, and it is checked both ways round.** The vocabulary's own
manual test says of the routes: *"None of the surfaces route through this type yet, so such a test
could only read the page against itself. It becomes checkable one surface at a time as they
migrate."* This is the first of the four to become checkable. Three tests in `mcp.rs`:

1. every published tool's operation has a row on the page, that row's MCP badge is `has`, and the
   badge names *that* tool;
2. every row marked `has` for MCP is a tool this server publishes, naming the same operation;
3. all six operations are still `Silent`, asserted against `karakuri-operation-record` rather than
   against this file — so the day one of them starts writing a record where it is asked, the
   failure names the tool that is due to move.

Both scans carry a floor, for `tests/vocabulary.rs`'s reason: a scan that matched nothing would pass
every assertion after it.

**Two spellings of the layer meet here and are checked against each other.** `layer_of` and
`kind_of` convert between `karakuri_ir::Kind` and `karakuri_operation::Layer`, exhaustive both ways,
which is what `karakuri-operation`'s module documentation prescribes for every list it copies and
what `mix::blend_mode` and its neighbours already are for the mix. A sixth of either does not
compile until somebody has said what the other one calls it.

**Nothing a tool does changed.** No tool was added or removed, no schema moved, and the order
arguments are refused in is the order they were refused in before — the slot is checked where each
tool checked it, `checked_id` runs where each tool ran it, and nothing new is decided in front of
anything old. A change of route may not change what a tool answers, and a model that is told which
mistake it made is this surface's product.

## Alternatives rejected

**Route the six through `Live::operate`, as MIDI does.** The literal reading of the roadmap's
sentence, and it fails twice: `operate` would print *no record* for all six and do nothing, and
there is no `Live` on an MCP connection thread to call it on. Reaching one would mean sending every
tool through the render loop's request channel — putting a file read behind a frame, and putting the
`Live` mutex in front of a surface built so that a slow disk cannot stall another connection.

**Leave `mcp.rs` alone and write only the page-versus-tools test.** ADR-0197's answer, and the
argument for it is real: the operations are all `Silent`, so no record, no `operate` and no
exhaustiveness is anywhere on this path, and the constructed `Operation` is destructured again a few
lines later. It loses on the test. Without the naming in the code, the mapping from a tool to its
row is a table in a test file — two lists agreeing, which is the thing the vocabulary exists to
abolish, and the guard would have been checking the page against a copy of itself. With it, the
title comes back from `asked`, which is the path a real call takes.

**Give `Operation` a `Question`-shaped answer type so a tool could route through one call.** That
is `operate` growing a second job — *the operation, done* rather than *the operation, as the records
it writes* — which ADR-0198 rejected for the keyboard and is worse here, since four of these six
answer with a page of prose written for a model to read.

**Fold `Slots::holds` into `deck_named` and check it first for every tool.** Tidier, and it changes
which of two simultaneous mistakes a caller is told about: a `write_procedure` with a bad slot and
no `source` is told about the source today. The refusals are the product; a change of route may not
reword them.

**Add a row for the tools' arguments the page does not describe.** There are none — all six rows
exist and all six badges were already right. The page needed no edit, which is worth recording
because the other three surfaces each found one.

## Consequences

- **All four surfaces have an answer now**, and only one of the four was the wholesale move the
  roadmap's sentence described. `docs/roadmap.md` item 3 says what each answer was.
- **`NodeAt` and `karakuri_operation::Layer` have their first caller**, which is the payload only
  this surface can say.
- **The manual's MCP column is enforced**, in both directions and by badge text as well as by title.
  A tool renamed on the wire, a badge flipped to `gap`, or a row renamed now fails a test in the
  crate that publishes the tool.
- **The sentence *"a model can rewrite a whole procedure and cannot turn one knob"* is still true**,
  and now for a structural reason a reader can check: the six operations MCP names are three
  procedure rows, three library rows and nothing in *Mixing*, *Transport and tempo* or *Decks*.
  `WriteParam` is `Silent(NoRecord)` — `Record::Param` is a Set file's and has nowhere to put a
  deck — so the gap is in the session record vocabulary and not in this server.
- **No new principle.** P-0028 is what this applies, and P-0031 is why `karakuri_operation::Layer`
  is spelled in full in a file where `Layer` already means the record's.
