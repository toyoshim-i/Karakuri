---
id: 0260
title: Sending a Set is a read, and a read's answer goes where the surface that asked puts answers
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: []
tags: [vocabulary, operations, surfaces, ui, docs]
---

# Sending a Set is a read, and a read's answer goes where the surface that asked puts answers

## Context

*Send a Set to somebody, and take one in* is one row on
[every operation](../manual/operations.html) — panel `plan` **library**, key `gap`, MIDI `gap`, MCP
`plan`, CLI `has` `--package, --take-in` — and one variant,
`Operation::TransferSet { transfer: SetTransfer }`, with two arms in
`crates/karakuri-operation/src/lib.rs`:

```rust
pub enum SetTransfer {
    Send { id: String },
    Take { file: PathBuf },
}
```

Half of it is built and reached. `karakuri-cli --package night01` writes the package to standard
output and `> night01.kbset` is the shell's; the taking-in half is a press on a `presets` row. The
sending half is not drawn anywhere, and `view.rs`'s `LibraryBay` says so deliberately rather than
leaving a gap that reads like a decision: the send is *"drawn nowhere"*, and what it waits on is
*"one sentence nobody has written — where does a package go when no shell redirected it"*, which
that note hands to `docs/manual/operations.html` and `karakuri-operation` rather than settling in
the first bay that wanted it. `docs/roadmap.md` M5.3 carries the same blocker in the same words:
the row *"waits on a destination the vocabulary can carry."*

**The question was hard because it was asked of the wrong noun.** *Where does a package go*
presupposes a place and asks the vocabulary to name one. Read what the vocabulary already does with
the things most like a package and it names no place for any of them.

### A package is the answer to a read, and the vocabulary already holds three of those

Nothing about sending is a write. `packaged_set` in `crates/karakuri-cli/src/main.rs` is explicit —
*"The id half is still a pure read and still leaves nothing behind."* `setfile::bundle` in
`crates/karakuri-environment/src/setfile.rs` is `store.read_set(id)` plus inlining, and its own doc
declines the destination: *"**Lines back rather than a file written.** Where a bundle goes is the
caller's."* Nothing is compiled, no device is opened, and `crates/karakuri-operation-record`
answers `Written::Silent(Silent::NoRecord)` for the whole variant. Sending is a read of the store
whose answer happens to be a file's worth of text.

The vocabulary holds three reads whose answers are handed back to whoever asked, and **not one of
them names a destination**:

- `ReadSet { id }` — *"Every knob with its range and default … without fetching a source or
  compiling it."* MCP returns it in the tool result; the panel is `plan` **library**; MIDI `gap`.
- `ListSets { holds, layer }` — *"it says how many it did not show."* `--list-sets` prints it, and
  `parse_args` says why there: *"On stdout, because it is the answer to the question that was asked
  rather than a note about a run."*
- `ReadProcedure { deck, node }` — MCP `read_procedure`, panel `plan` **inspector**, MIDI `gap`.

The `--package` arm sits in the match immediately below that `--list-sets` comment and gives a
different-sounding reason for the same act — *"To standard output, so a shell can redirect it"* —
before arguing against the store as a destination. **They are the same rule stated twice with two
different reasons.** The package is printed for the reason a listing is printed: it is the answer.

**So the sentence nobody had written is not a place. It is that sending has no place in it.**

## Decision

**Sending a Set is a read. Its answer is the package, and a read's answer goes where the surface
that asked puts answers. The operation names no destination, and none is owed, because a package is
the same bytes wherever it lands and nothing has to reconstruct where it went.**

Three things follow.

**1. `SetTransfer::Send { id }` is already the right shape; what was missing was its doc.** A
destination on the operation would be a payload the read does not need: it changes nothing about
what is asked — the Set filed under `id`, self-contained — it is dead on the surfaces that cannot
spell one, and no record reads it, which is
[P-0087](../principles/0087-name-the-property-never-the-shape.md)'s free variable. The property is
*one self-contained file of this Set*; the destination is presentation.

**2. Each surface does with the answer what it does with every other read's answer.** The command
line prints it and the shell redirects it. A model over MCP is handed it in the tool result,
exactly as `read_set` hands back declarations. The panel has no sink that can show a file's worth
of text, so its control hands the bytes to whatever the platform's own ask for a place is — *which
control that is, is not decided here* (below). MIDI is `gap`, and chosen: a read's answer has
nowhere to land on a controller, which is why `ReadSet`, `ListSets` and `ReadProcedure` are `gap`
there too, and `crates/karakuri-midi/src/map.rs`'s grammar — *"a slot number, a value word out of a
list, or a trailing `[lo, hi]`"* — can spell neither an id nor a file.

**3. No surface spells a path in a control of its own, in either direction.** The shell's `>` and
whatever the panel's control asks with are the operator's filesystem asked by the operator's own
tools; the wall [ADR-0221](0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
put on a *name* — one path component of letters, digits, `-` and `_` — is untouched, because no
name enters the store. And a model over MCP never names a place on the operator's disk: it receives
text and, if the Take side ever takes text, hands text over. That keeps `named_set`'s sentence in
`crates/karakuri-environment/src/mcp.rs` true from both ends — *"A read is not the harmless half of
that rule: it is the half that hands a file's contents back to the caller"*
([ADR-0229](0229-a-set-file-is-authored-beside-its-parts-and-travels-as-a-bundle.md)).

`gate.rs`'s `Operation::TransferSet { .. } => Standing::Open` stands, and it stands *because* this
is a read; alternative (a) below is what would have reopened it.

### One clause moves off `Send`

Today's `Send` doc reads *"`--package ID` — or `--package FILE.kset`, which resolves an authoring
file's parts into the store first and packages that"*, which makes `id` sometimes a path. It is
not: `--package night01.kset` is this row performed at both of its moments in one flag — a take-in
that resolves and stores, then a send of the id the file carried, which is ADR-0229 part 4's *"one
operation, two moments"*. The clause belongs on the CLI's `ParseOutcome::Package`, which already
argues *"one flag and two moments rather than two flags"*, and `Send { id }` keeps `id` meaning an
id. Nothing compiles differently — `crates/karakuri-cli/src/main.rs` constructs no
`Operation::TransferSet` at all.

## Alternatives rejected

### a. A destination on the operation — `Send { id, to: PathBuf }`

**What it had going for it.** The smallest change; it mirrors `Take { file }`; it lets
`--package ID FILE` exist; and it is literally what the roadmap asked for — *"a destination the
vocabulary can carry."*

**It loses four ways.** Only two surfaces could ever fill it — the shell and a model — and the
panel's one letter-taking flow takes a *name*, ADR-0221's one path component, which ADR-0229 says
in as many words cannot stretch to a path; a field dead on two of four surfaces is P-0087's free
variable. Over MCP it is a **write-anywhere primitive**: a model naming where on the operator's
disk a file lands, through the program running the show — `named_set`'s sentence is the mild form
of that hazard and this is the severe one, the reverse direction of which ADR-0229 spent a section
refusing. It would reopen `gate.rs`, since a variant that writes to a caller-chosen path is not
`Standing::Open`. And it copies `Take`'s justification without reading it: *"a path because a file
is what the only existing route takes"* is a narrow, dated reason that leaves bytes open, not a
shape to mirror.

### b. A fixed place in the store — `<store>/out/<id>.kbset`, or beside the Set

**What it had going for it.** The panel could write there with no path and no dialog; `<id>.kbset`
is the store's own naming rule, so nothing is invented; and the operator can be told the path at
startup as the store root already is.

**It loses on the sentence `packaged_set` already wrote**: *"a second copy of a Set sitting beside
the Set is a second answer to which of them is the file."* Under P-0087 that is worse than untidy —
a save over `night01` leaves `out/night01.kbset` saying something the Set does not, with no
assertion anywhere to catch it. And it does not answer the question: the file still has to leave
the machine, so the panel would have to show the operator a path to fetch it from, which is the
control it does not have wearing a different hat. Every `pub fn` on `Store` takes an id, a hash, a
stamp or a name — the root is the only path it knows — and this record adds none.

### c. A sum the vocabulary names — `Send { id, to: Destination }`

`Stdout`, `File(PathBuf)`, `Caller`, so each surface picks its arm. **What it had going for it**:
the per-surface behaviour becomes visible in the type rather than in prose.

**It loses because a sink is a surface's transport and the vocabulary has no surface in it.**
`karakuri-operation` has no dependencies by charter
([ADR-0180](0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md)) and names
destinations a *record* can carry, not the pipes a program prints into; `Stdout` in that crate is
`karakuri-cli` leaking upward, and `File` is alternative (a) again. A map line could not carry the
field, and it takes from the surface exactly what
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) leaves with it.

### d. The Library's `folder` scope as the outbox

**What it had going for it, and it is the closest loser.** The scope already exists as a drawing
and a chip; a directory the operator pointed the bay at is a destination the operator named;
writing `<id>.kbset` there would let the listing point at the file the moment it exists, which
answers `view.rs`'s asymmetry — *taking in names a file that exists and sending names one that does
not yet* — head-on; and it needs no dialog.

**It loses three ways.** The folder scope has no directory: `Scope::Folder` in
`crates/karakuri/src/main.rs` answers that it *"waits on an operation"*, because `ListSets` has
nowhere to put a directory — so the send would wait on the larger question rather than the smaller.
`console.html` settled the scope's direction: *"a folder is a way in, and my sets is where things
are."* And it makes the destination a property of console state — which scope the bay happens to be
showing — that a map cannot see and that means a different thing per session, where the shell's
precedent is per invocation.

### e. The store holds bundles, and sending is copying a file with the OS

Inline every source into `sets/<id>.kbset` on write; the package then already exists at a path the
program prints, the send half leaves the vocabulary, and the operator drags the file out.
**What it had going for it**: it kills the question outright, and it is honest that the operator's
messaging app is the real transport.

**It loses because the store's Set file is a projection and the artifacts are the material**
([ADR-0007](0007-a-set-file-is-a-projection-and-a-session-stream-is-the-timeline.md), ADR-0229 part
5). A `src` run inside the store's own file is a second derivation of bytes sitting beside it under
their hash — P-0087 in the store's own layout — and `unbundle`'s guarantee that a run hashes to the
address it names would have to be re-checked on every read of the store's own file. It also never
reaches the panel: an operator dragging out of `<store>/sets/` is the shell in another coat.

### f. Split the row, or retire the sending half

`TransitionSetting`'s doc is the precedent for a row the crate thinks is several operations and
carries as one *"because the manual is the specification … and the page is not this crate's to
change"*. The same holds here, and the split buys nothing: the destination question is identical
after it. Retiring the send half would leave an instrument that can take a Set in from anyone and
hand one to nobody, against a row the manual has carried since the vocabulary landed.

### The `Crossfade` precedent, judged

`Crossfade { from, to }` — *"**Both decks named.** The next deck is the keyboard's translation of
this, not the operation"* — is a different kind of thing and does not apply. There the operation
**carries** `to`, a record needs it, and the keyboard fills in a value the operation still names.
Here the operation carries no destination and never will, because no record needs one and the bytes
do not change with it. What the two share is the direction of
[ADR-0192](0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md): do not
put into the ask what no surface asking for it has. **The closer precedent is one row up on the
page** — *Keep what a deck is playing*, whose tip reads *"The tool waits for the disk and reports;
the key does not."* One operation, surfaces differing in what they do with its outcome, and nothing
about that difference in the payload.

## Consequences

- **`SetTransfer` keeps its two arms and their fields.** `Send`'s doc says it is a read and names
  no destination; the `.kset` clause moves to `ParseOutcome::Package`. No type changes and no
  behaviour changes.
- **The CLI is already this rule.** The one edit owed is that the `ListSets` and `Package` comments
  in `parse_args` say one thing rather than two.
  [ADR-0242](0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)
  makes the command line test tooling, so its `has` proves the operation exists and proves nothing
  about the instrument — which is why the panel column is the one that matters here.
- **The MCP tool's shape is now sayable**: `id` through `checked_id`, the `.kbset` text back as the
  tool result, no path in either direction, and it stays in the open class — a read handing back
  what the store already holds under an id the model was already allowed to `read_set`.
- **The panel stays `plan` **library**, and the destination is the surface's, not the operation's.**
  Which control the bay draws is the console page's under `docs/contributing.md` §5 step 3; the
  constraint this record puts on it is that the panel names no path itself.
- **MIDI stays `gap`, and the row's tip should now say why**, as *Choose which scope the library
  shows* already says why its MIDI cell is empty.
- **`view::LibraryBay`'s *"one sentence nobody has written"* paragraph is answered** and should
  point at this record rather than restate the wait.
- **M5.3's *Blocked on* is no longer a vocabulary question.** What remains is a control on the
  console page and a mechanism in the panel to carry the bytes out, which is a build item.
- **[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)'s
  Library walk acquires a second act on a row**: that record gives `enter` to *"the act the
  addressed control is for"* and says *"where an item has two acts they are two controls and a
  digit chooses between them"*, so once a send is drawn, a library row's load and send become `n 1`
  and `n 2` — or that record grows a rule that a row's first act keeps `enter` — and it is that
  record's grammar and the page's, not this one's.
- **This is a decision and not a principle.** A plausible alternative lost — (a) will be
  re-proposed by anyone who reads `Take { file }` first — and the rule decides one row. Under
  [ADR-0249](0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)'s gate there is no
  question outside this row that reading it settles.

## What this does not decide

### The form of the panel's control, and the two candidates

The panel's destination is the surface's; **what the surface asks with is open**, and the two
candidates cost different things.

**A save sheet — the platform's own ask for a place.** It is the GUI's `>`, it names a file that
does not exist yet, which is the one thing `view.rs` correctly says no listing can do, and it takes
no rule from ADR-0221 because it is the operator's filesystem asked by the operator's own tool.
It costs a dependency this workspace does not have: `Cargo.lock` contains no `rfd`, and the panel
is `egui` over `winit`, neither of which opens a save sheet on its own. That bill is
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)'s to pay now
rather than design around. **It is also a modal operating-system window over a live instrument.**
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
asks of every mechanism *"what does this do at its worst, on the frame it goes wrong, while the
operator's attention is on the room?"* and admits three answers, the third being *"it is loud, and
the operator's hands still work"* — and while a modal sheet is up, the operator's hands do not work
on the instrument. **The counter-reading, stated fairly**: sending is not a performance act, the
sheet appears only because somebody asked for it, and that rule is about mechanisms that *run
during* a performance — so P-0094 may not reach this at all. Nobody has argued that out.

**The clipboard.** It costs no dependency: `arboard` 3.6.1 is already in `Cargo.lock` —
**transitively, through `egui-winit` 0.36.1, and named as a direct dependency in no crate's
manifest in this workspace**, so using it still means adding it to one. Nothing under `crates/`
writes to a clipboard today. It names no file at all, which sidesteps the asymmetry `view.rs`
identifies rather than answering it, and `SetTransfer::Take`'s own doc already floats the pairing:
*"whether a route that has no filesystem — a model handing over the text — takes bytes instead is
open."* Against it: the one built receiving route, `--take-in FILE` and a `folder` row, takes a
**file**, and bytes on a clipboard are not one.

**Neither is chosen here.** The console page's, under `docs/contributing.md` §5 step 3.

### The rest

- **Whether `Take` takes text over MCP.** Its own doc calls that *"a smaller question than the
  row's own"* and leaves it open. This decision makes the answer easier — a model that receives
  text from one Karakuri's send has text to hand to another's take-in — but the shape is that doc's
  and `mcp.rs`'s, and the panel's take stays a file from a listing either way.
- **What a save sheet's default filename and starting directory would be**, if that is the control
  drawn. `<id>.kbset` is what the store calls the Set; the folder scope's directory, once it has
  one, is the obvious starting place. Both are the page's.
- **The MCP tool's name, and whether it is a tool beside `read_set` or a flag on it.** `mcp.rs`'s.
- **The Library walk's second act under ADR-0259.** Named above; that record's and the page's.
- **The `.kset` moment of `--package`.** This record says it is take-in then send in one flag and
  moves a clause; whether the CLI should instead grow `--take-in` to package is the CLI's, and
  ADR-0242 says the instrument's rules do not reach it.
