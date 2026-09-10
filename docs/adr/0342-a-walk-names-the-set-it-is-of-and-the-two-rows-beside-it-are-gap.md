---
id: 0342
title: A walk names the Set it is of, and the two rows beside it are `gap`
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0083, 0087, 0090, 0091]
tags: [mcp, operations, vocabulary, history, library, m5]
---

# A walk names the Set it is of, and the two rows beside it are `gap`

## Context

[ADR-0334](0334-mcp-names-an-operation-by-the-vocabularys-own-name-and-the-frame-performs-it.md)
built `operate` and left nine rows `plan`;
[ADR-0341](0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md) closed six
of them and wrote the sentence this record is asked against: ***a `plan` badge in the MCP column now
means an unsettled payload and nothing else***. Three rows carry one — *Walk the edit history*,
*Edit the file instead* and *Move a boundary* — and `docs/roadmap.md`'s M5.10 says of all three that
*"what they wait on is the vocabulary settling what the operation acts on rather than a route"*.

**That is true of one of them.** Asking the other two what they are waiting for turns up a badge
that was wrong rather than pending, which is ADR-0341's own finding met a second time.

### The walk's payload was waiting for a surface, and the surface arrived

[ADR-0308](0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)
answered three of the four shapes a walk could be addressed by and left the fourth open in one
clause: *"What is left for a walk to carry is **which history**, and no surface spells it: the id
rides the aim, and the console holds a deck letter rather than a Set id."*

**A model spells one.** `read_set` takes a Set id, `list_sets` hands the ids out, and
`karakuri_environment::mcp`'s own rule — *"every free string in an accepted payload is a name
something in this run produced"* — names a Set id as the first example. So the sentence that kept
the payload `Undecided` was true about the console and false about the vocabulary's other surface,
and [ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md) is
about what *a* surface can say rather than what all four can.

### `history::list` is the reader, and it narrows nowhere

`karakuri_environment::history::list` walks `<store>/history/` newest first, stops entering day
directories once it has enough and **opens no file at all** — a row is a name. Its own head refuses
to hold the narrowing: *"which rows an operator is looking at is a question the surface asks, the
way `Operation::ListSets`' two filters are applied where they are answered and not inside
`Store::list_sets`."* And it says what a narrowing has to spell: **a row whose `set` is `None` is a
row no Set matches**, because those versions were written where there was no Set and folding them in
would invent a history for whichever Set was asked about (ADR-0276).

## Decision

**`Operation::WalkHistory` carries the Set it is a walk of. MCP reaches it through a ninth tool,
`walk_history`, beside `read_set` and `list_sets`. The other two rows are `gap`, each with its own
sentence, and `Sayable::Undecided` is deleted because nothing constructs it any more.**

### 1. `WalkHistory { set: Option<String> }`

`Some(id)` is that Set's history. **`None` is a walk that names no Set**, which is a state rather
than an absence: the deck a panel aims this at can be running the pair the run launched with, whose
versions are filed under no Set, and a narrowing to a Set matches none of them — so the walk lists
nothing and the surface says why. It is `history::Version::set`'s own `Option` read from the asking
side, and the panel's one derivation of it is the aim.

**Not `deck: u8`**, which ADR-0308 refused and this record honours: two decks running one Set have
one history between them, so a deck is how a console *arrives* at an id and never what the listing
is about.

**The panel says it, and the host is where the id is read.** The Library bay's chip press names the
row — that is ADR-0308 §1 and is untouched — and the id comes out of `view::View::aimed`, which
`crates/karakuri` writes per frame off the load pulldown's deck's `Aiming::at`, beside every other
reading it rebuilds per frame. `view::Chosen::asked(aimed)` is the one place it is read, so the
operation the press emits and the listing the press re-reads are narrowed by one value.

**`written` moves with it.** `karakuri_operation_record::written` answers `Silent(Question)`: a walk
is a listing of the store, it opens no file and moves no deck, and **landing** on one of its rows is
`RestoreProcedure`, which is a write and is `Silent(OnLanding)`.
[ADR-0281](0281-every-route-reaches-every-write-and-a-read-is-the-routes-own-interface-design.md)'s
consequence *"Walk the edit history is a write"* was reasoned from the payload being *a revision to
land on*; ADR-0308 moved that half to the landing's own row, and what is left is the question.

### 2. A ninth tool, and the property decides it rather than the shape

ADR-0334's rule for `operate` is that it names operations the **frame** performs, and ADR-0199's
rule for a tool is that it does something **only the server can** — a file, the store, a listing.
A walk is the second: it reads `<store>/history/`, which is exactly what `list_sets` does, and a
directory walk handed to the drain's frame would be a `read_dir` per day directory on the path that
must not wait ([P-0091](../principles/0091-cost-is-known-before-it-is-paid.md),
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).

**And a read's answer goes where the surface that asked puts answers**
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md),
[ADR-0260](0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)),
which for this surface is the reply. `operate`'s reply *can* carry text — `News::Settled` is a
`Result<String, String>` — so this was a choice and not a limit: what settles it is that the frame
would have had to do the reading, which is the thing the eight tools beside it exist to keep off
that frame.

**The reply is the listing**: the rows of `history::list` narrowed to that Set, most recent first,
each row the name the store filed the version under and the name `Revision::Picked` takes back. It
says out loud when the walk stopped with days unread, when it passed over entries the layout does
not claim, and when it rendered fewer rows than matched — three ways a listing could otherwise read
as the whole of something it is not, which is `list_sets`' own rule.

**One spelling of a row.** `history::Version::filed_as` is new and `crates/karakuri`'s
`version_row` now calls it: the bay draws that name and hands it back at the press, a model reads it
out of a walk and hands it back in `Revision::Picked`, and two `format!`s would be two answers to
*what is this row called* on a name the two surfaces hand to each other.

### 3. *Edit the file instead* is `gap`, because a model's edit is `write_procedure`

The row names an **event** — somebody saving a file in another program — and not an act a surface
performs. A model does not have another program; what it has is `write_procedure`, which **is** its
edit: checked, written, built on a worker, swapped at a frame boundary. There is nothing this row
would add to that, so **nothing is owed**, which is the difference between `gap` and `plan`
(ADR-0341). It is
[ADR-0205](0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md)'s kind of answer
one step along: that record refused a row whose *reply* the vocabulary cannot say, and this is a
route whose only content is a second spelling of one that exists.

**The payload stays `Undecided`**, and its doc now says what it is: the row's marker for `--watch`,
which takes no argument, cannot be turned off, and does not say which files it would take if it
could. Whether the row is an operation at all is a decision about the manual, and it is not taken
here.

### 4. *Move a boundary* is `gap`, with ADR-0315's own sentence

A divider's position is the arrangement's own state, exactly as a fold is.
[ADR-0315](0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md) made twelve
such rows `gap` and its consequences named this one as the thirteenth, *"not swept in"* only because
its payload is `Undecided` — *"its sentence is written and its badge is the day somebody settles the
payload"*. **That is the one sentence in that record this one disagrees with**: the badge is a claim
about what a surface can reach, the payload is a claim about what the operation acts on, and holding
the badge hostage to the payload made this row say `plan` — *a route is coming* — about a route
nobody has ever intended to build.

**The payload stays `Undecided`** for
[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)'s
reason as well as the pointer's: a key press cannot mean a viewport pixel, and half the boundaries in
the console's arrangement have no name any surface but the pointer could say. Settling it would not
have moved the badge, and the badge moving does not settle it.

### 5. `Sayable::Undecided` is deleted

`mcp.rs`'s classification has no wildcard arm, so every answer it carries is a group somebody has to
put a row in. The walk left this group by having a payload, and the other two left by being asked
what they can *reach* rather than what they *carry* — so the variant constructs nowhere.
ADR-0341 deleted `Sayable::Unperformed` on the day it emptied and wrote the argument: *"a
never-constructed variant is dead code that says something false about the program"*, and the
`match` with no wildcard is what makes the next arrival a decision rather than a default.

**So a `plan` badge in the MCP column now means nothing at all.** Every operation the vocabulary
names is performed on the drain's frame, has a tool of its own, is a window's, or is refused with a
sentence saying nothing is owed.

## Alternatives

### a. A walk through `operate`, with the frame answering the listing

The literal reading of ADR-0334 — *`operate` names operations by the vocabulary's own name* — and it
needs no ninth tool. It is refused because of **who would do the reading**: the drain's frame would
have to run `history::list`, which is a `read_dir` per day directory, on the loop that must not
wait. The reply could carry it (`News::Settled` is a string), so this is not a shape the protocol
forbids; it is the cost. ADR-0199's division is exactly this and is unchanged: a tool is what the
server does because only the server can.

### b. `WalkHistory { set: String }`, with no `None`

Tidier at the wire, where `set` is required anyway. It loses at the panel: the load pulldown's deck
can be running the pair the run launched with, and then there is no id to put in the payload — so
the chip press would have to invent one, name a different operation, or emit nothing. `None` is what
that deck's aim actually says (`Aiming::at.set` is an `Option`), and `history::Version::set` carries
the same `Option` for the same fact at the other end.

### c. `WalkHistory { set: Option<String> }` where `None` is *every Set's*

The other reading of the same shape, and the one a lister with a cap argues for: `history::list`
caps on **days opened** and the narrowing happens after, so an unnarrowed walk costs the same and
answers *what has been edited lately* across the store. It is refused because it makes one word mean
two things a step apart: `None` on a **row** means *filed under no Set* and would mean *any Set* in
the **payload**, which is the wildcard reading ADR-0308 refuses in as many words — *"a narrowing must
treat a `None` row as matching no Set rather than as a wildcard"*. No row on the page asks *what has
this store been editing*, and the row that would is `ListSets`' to grow rather than this one's.

### d. Leave the two rows `plan` until their payloads are settled

The consistent-looking move, and it is what M5.10 said all three were waiting for. It fails on what
`plan` promises: *a route is coming*. Nothing is coming for either — a model will not be handed a
window, and `write_procedure` is already the whole of what a model's edit is — so `plan` would be
this page telling a reader to wait for work nobody intends to do, which is the thing ADR-0341 wrote
`Sayable::Never` to stop saying about the send.

### e. Give *Walk the edit history* the page's `read` mark

It is `Silent(Question)` now, and ADR-0281 marks a read row so that an empty column reads as *that
route's own design* rather than as a debt. **Not taken here**, and it is left open rather than
refused: the mark's four rows are `Silent::Question`'s membership as of ADR-0281, `FilterLibrary`
joined that arm in ADR-0338 without the page moving, and whether the mark tracks the arm is a
question about the manual that this record would be answering in passing. Nothing on this page is
false today: the walk's key and MIDI columns are `gap`, which is what they were.

## Consequences

- **`Operation::WalkHistory { set: Option<String> }`**, and every construction site moved with it:
  `gate.rs`'s sample (its `Standing::Open` is unchanged), `mcp.rs`'s `SPELLED` row,
  `karakuri-console`'s `Chosen::asked`, and two console tests. `Operation::TITLES` and every title
  string are untouched, so `the_manual_and_the_vocabulary_agree` is unaffected.
- **`written` answers `Silent(Question)` for a walk**, and `Owed::Undecided`'s arm holds two rows
  rather than three. That arm's doc said *"at four variants"* and now says three, with the third —
  `SelectScope` — named as `Silent::Surface`'s, which it always was.
- **`mcp.rs` publishes nine tools.** `walk_history` is the eighth written out and `operate` the
  ninth, generated. `every_tool_this_server_publishes_names_an_operation_the_gate_lets_through`,
  `no_tool_writes_a_record_where_it_is_asked` and the page pair reach it by name because they read
  `tools()` rather than a list.
- **`view::View::aimed` is a new reading**, written per frame by `crates/karakuri` off the load
  pulldown's deck's aim and read in one place. It is the console being *told* an id rather than
  spelling one, which is what `View::starred` and `View::library` already are.
- **A defect is fixed on the way through**: the pointer's press on the `history` chip emitted
  `WalkHistory` and the window's re-listing branch matched `SelectScope | ListSets | SetFavourite`
  only, so a press on that chip marked the scope and left the bay drawing the listing it had. It is
  named here because it is this row's own route and the badge claims it.
- **A second defect is recorded rather than fixed.** The same branch does not match
  `Operation::FilterLibrary`, which ADR-0338's six kind chips emit, so a press on one of them
  narrows what the console draws and does not re-read the listing the narrowing is applied to. It
  belongs to that row's own pass.
- **`history::Version::filed_as` is the one spelling of a row's name**, and
  `crates/karakuri`'s `version_row` delegates to it. `history::list` is unchanged and still narrows
  nothing.
- **M5.10's exit is met and is not closed here**: `grep -c 'rt plan">MCP' docs/manual/operations.html`
  answers 0. Closing a sub-milestone is the maintainer's.
- **Three rows of `docs/manual/operations.html` changed cells**, one to `has walk_history` and two to
  `gap`, each tip carrying its own sentence — one refusal per mistake, worded once
  ([ADR-0131](0131-one-refusal-sentence-per-mistake-across-the-surfaces-that-face-a-person.md)).
- **No new principle.** This applies P-0087 (the property decides, not the shape), P-0090 (a read's
  answer goes where the surface that asked puts answers), P-0091 and P-0083.
