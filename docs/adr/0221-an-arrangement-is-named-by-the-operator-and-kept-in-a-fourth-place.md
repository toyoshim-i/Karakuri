---
id: 0221
title: An arrangement is named by the operator and kept in a fourth place under the store
status: accepted
date: 2026-08-29
supersedes: []
superseded_by: []
principles: [0096, 0085, 0087]
tags: [console, store, format]
---

# An arrangement is named by the operator and kept in a fourth place under the store

## Context

[ADR-0208](0208-resetting-is-the-default-case-of-restoring-an-arrangement.md) landed *Reset the
arrangement* as a row and said in the same breath what it left owed. `docs/roadmap.md`'s M5 says it
in the maintainer's own words:

> **What it needs that does not exist**: a name for an arrangement, a record carrying one, and two
> operations beside `Reset`.

This record takes the first of the three and the place that follows from it. The two operations are
named here and are deliberately not added — `karakuri-operation` and the manual's row are another
change's — and the sentence at the end of this record is written so that whoever adds them has no
decision left to take.

**The serialiser is not owed, and that was checked rather than inherited.** The roadmap's claim —
*"`karakuri-layout`'s `Layout` round-trips today, an unbounded maximum is written as `null`
deliberately, and a `NodeId` goes on the wire as the bare number it is"* — is itself a correction of
a wrong one, so all three halves were read off the code:

- `Layout` has a **hand-written** `Serialize` over a borrowed `WireOut<'a>` and a
  `#[serde(try_from = "Wire")]` `Deserialize`. `Arrangement` is private and derives `Debug, Clone`
  only; nothing about it is serialisable on its own.
- `Node::max` carries `#[serde(with = "unbounded")]`, whose module documentation states the reason:
  `serde_json` writes an infinity as `null` and then refuses to read a `null` back as an `f32`, so
  an explicit absence is written instead.
- `NodeId(usize)` is a newtype deriving `Serialize`/`Deserialize`, which serde writes as the inner
  value. `LoadError`'s own documentation depends on that — *"a `NodeId` is written on the wire as
  the bare number it is, so 'node 7' is the eighth entry of `nodes`"*.
- `TryFrom<Wire>` runs `check_structure` and `duplicate_name` before assembling anything, which is
  [ADR-0158](0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md)
  refusing a file that disagrees with itself rather than repairing it.

**What round-tripped was a `String`, though, and not a file.** `karakuri-console/tests/loading.rs`
and `karakuri-layout/tests/loading.rs` both go through `serde_json::to_string` and back in memory.
That is the format proved, not the path, and this repository has been bitten by exactly that
distance: `Record::Transport` had no round-trip test until the day before this record and was the
only record with a signed unbounded field. So the store half below is tested by putting a real
`Layout` through a real file, and §4 says what that found.

## Decision

### 1. An arrangement is named by a name the operator picked

A `String` the operator types, in the same namespace shape a Set id has: one path component,
letters, digits, `-` and `_`. Where no operator is there to type one — a key press cannot — the
fallback is a stamp from `karakuri-environment`'s `history::stamped_id`, which is the convention
`accepted_save` already follows for a Set with `id: Option<String>`. *(Annotated the day after: **nothing reaches that fallback and the
store's own documentation says it never should** — `ArrangementEntry`'s doc argues that a Set is
ordinarily filed under a stamp nobody chose and an arrangement never is, because it is saved by an
operator telling the console what to call this shape. The two are reconciled in favour of the store:
the fallback was written for a caller that does not exist, and it is half of why no key is bound —
a stamped save would keep arrangements nothing in this program can put back, since there is no
listing control to show an operator the stamp picked for them.)*

[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) is most of the argument:
the naming rule, the stamp fallback, the refusal that keeps a name to one path component
(`mcp::checked_id`) and the listing that skips a name the layout does not claim all exist and were
built for a Set. Nothing here is new machinery; it is the same handle on a different kind of file.

[P-0087](../principles/0087-name-the-property-never-the-shape.md) is the rest, and
it is what rules the alternatives out. A name is **recorded** — assigned once, by the person who
will look for it — and every alternative below derives one from where the thing currently sits.

### 2. A saved arrangement lives in a fourth place: `<store>/arrangements/<name>.arrangement.json`

`karakuri-store`'s own header states the layout, and this adds a fourth line to it beside
`sets/<id>.set.ndjson` and `sessions/<stamp>.ndjson`. The spelling is the Set file's, one component
at a time: the operator's name, what kind of thing it is, and the format it is in — which is what
lets a listing tell an arrangement from an editor's backup or a half-written `.tmp` without opening
either.

**It is a fourth thing rather than one of the three, and each of the three refuses it for its own
reason.** An **artifact** is content-addressed and immutable, which §3 shows is fatal. A **Set** is
a state projection of *material*, and `Store::write_set` refuses any line that is not Set state — an
arrangement is not a record at all, so it could only ride in a Set as something the Set file format
does not have. A **session** is the timeline, and ADR-0208 §4 already put every arrangement
operation in `Silent::Surface`: an arrangement is not something a replay reconstructs anything from,
which is precisely why it needs a file of its own instead of a place in the stream.

**Under the store, and that is
P-0048
rather than convenience.** The principle's three places have exact counterparts here, which is the
strongest evidence this is the right shelf: *what ships* is `karakuri_console::layout()`, the
built-in default, and **nobody writes it**; *what you saved* is `<store>/arrangements/`, and only a
save writes it; *what you are editing* is the running panel, which is the scratch. The principle's
own failure — a model writing to the files that ship — is unrepresentable here for the same reason
it is for `examples/`: there is no path from the store to the default.

**So the default is not a file, and there is no reserved name.** `ResetArrangement` reaches code;
`RestoreArrangement { name }` reaches a file. An operator may save an arrangement of their own
called `default` and it shadows nothing. This also means `read_arrangement` must never fall back to
the built-in: an operator who mistyped a name needs to be told the name, not to watch their console
reset.

### 3. What loses

**A content address, the way an artifact gets one.** The heaviest alternative and the one that
arrives first, because the store already has a `Hash` and a `put_artifact` that is idempotent, and
because it makes two identical arrangements one file for free.
[P-0087](../principles/0087-name-the-property-never-the-shape.md) names this exact
failure — *"a content hash moves on every character"*. Move one divider and save, and the
arrangement you have been calling `four_deck` all week is under a different address; there is
nothing to overwrite, so every save is a new file and the store fills with the arrangement's own
history under names nobody can read. Worse, the operator's handle would then have to be a **second**
thing — a name-to-hash table — and the moment that exists the hash has stopped being the identity
and is only where the bytes are kept. It also breaks ADR-0208's framing outright: the default
arrangement's address changes whenever `karakuri_console::layout()` changes, so *restoring the
default* would stop being the same operation as *restoring an arrangement*.

**A slot number, the way a synth has patch banks.** Genuinely attractive for the instrument this is:
a slot is reachable from a MIDI map's bare number, which is the one thing a map line cannot do with
a name ([ADR-0202](0202-the-map-reaches-the-masks-front-and-the-shape-has-no-spelling.md)), and
eight slots on a panel is a control anybody can draw. It loses on P-0053's second clause, the one
about position: *"a Set list position breaks on reordering"*. Delete slot 3 and everything after it
is a different arrangement than it was yesterday, and the operator's muscle memory now recalls
somebody else's console mid-set — silently, because a slot number is always valid. It also loses on
the maintainer's own sentence, which is what this whole family is being built from: *"the default is
one arrangement among the ones they could **name**"*.

The MIDI reach is a real cost and is stated rather than waved away: a name is not spellable by a map
line today, so `RestoreArrangement` arrives with the same gap the four other rows in *Arranging the
console* have. That is a fault in the map's grammar, already recorded twice, and not a reason to
give an operator a handle that moves.

**A slot number with a name beside it** — bank 3, *"four_deck"*. It has the reach and the readable
handle, and it loses for the reason ADR-0158 refused a repaired parent pointer: it is one fact
written twice, and the two disagree the first time something is deleted. The name is the fact; the
number would be a position derived from it.

**The path the operator saved to, i.e. no store at all.** `--save-arrangement ~/desk.json`, and the
name is wherever they put it. It is the cheapest thing that could work and it is what the panel
would grow if nobody decided. It loses on P-0048, which exists because this program has already
handed a writer a path and had it overwrite three shipped presets, and on portability: the store is
the one root the program is given, and an arrangement kept outside it does not travel with the
library it was arranged around.

**A config directory of the console's own** — `~/.config/karakuri/arrangements/`. The strongest of
the losers, and the argument for it is real: an arrangement is about the *console*, and the store is
a library of *material*, so this is a category of thing the store had not held. It loses on two
counts. The operator's saved things already live under the store — that is the whole of what
`<store>/sets/` is — and P-0048's three places are about who writes, not about what the bytes
describe. And a second root means a second `--store`-shaped flag, a second thing to back up, and an
operator who copies their library to another machine and finds the console reset.

**A record in the session stream.** Symmetrical with everything else this system keeps, and it is
what somebody who has read `record.rs` reaches for first. It loses on ADR-0208 §4, which decided the
other way for a stated reason and would have to be reopened: arrangement operations are
`Silent::Surface`, an arrangement is not what a replay reconstructs anything from, and a session
replayed on a different window would arrive carrying somebody else's panel.

**Owning this in `karakuri-environment` rather than in `karakuri-store`.** That crate's charter — *a
module belongs if what it deals with is outside this process* — fits a disk exactly, and it is where
`history` and `scratch` already are. It loses because the **store root's layout is
`karakuri-store`'s**, stated in that module's own header and enforced by `Store::open`: a fourth
directory established anywhere else would be a second answer to what is under the root, and
`list_arrangements` would be reading a directory whose existence another crate is responsible for.

### 4. The store's half is bytes, and the format stays `karakuri-layout`'s

`write_arrangement` takes `&[u8]` and `read_arrangement` hands back `Vec<u8>`, byte for byte with
nothing appended — the same contract `put_artifact` has with `.kir` source. The store keeps files it
does not have to understand, and the one place a broken arrangement is refused is the loader
ADR-0158 hardened. A check here would be a second answer to a question that already has one, and a
stricter one would refuse files this program writes.

**It overwrites**, unlike `put_artifact` and like `write_set`: a name is an instruction, and saving
over `four_deck` is what an operator who has just moved a divider means. Atomic all the same, so a
crash mid-write leaves the previous arrangement rather than nothing.

**Nothing in `src/` names `karakuri-layout`; the test suite does.** That is where the two halves
meet, and it is what makes the round trip a real one — a `Layout` is built, folded, soloed,
serialised, written to a real file, read back and compared as *what it draws*.

**And putting a real file under it found something the in-memory tests could not have.** Two of the
nine tests — the round trip and the solo — pass under a defect that files the arrangement in the
wrong directory entirely, because they write and read through the same wrong path. A format test is
not a location test. The test that catches it is the one that asserts the bytes at the path the
header claims, which is why that assertion is spelled out rather than left to the round trip.

## Consequences

- **`karakuri-store` gains three methods and one directory.** `write_arrangement`,
  `read_arrangement`, `list_arrangements`; `Store::open` establishes `arrangements/` beside the
  other three, so a store written by an older build gains it the first time this one opens it.
- **A fourth error variant, `StoreError::NoArrangement(String)`**, rather than reusing `NotFound`,
  which carries a `Hash` and could not say the name back.
- **`ArrangementEntry` says `name` where `SetEntry` says `id`**, and the difference is the decision:
  a Set is ordinarily filed under a stamp nobody chose, and an arrangement never is.
- **`karakuri-store` gains a dev-dependency on `karakuri-layout`.** It is in `cargo tree` and worth
  seeing; `src/` does not name it and must not.
- **Two operations are owed and their payloads are decided here**, so that adding them is
  transcription:
  - `Operation::SaveArrangement { name: String }` — file the running arrangement under `name`,
    overwriting what is there. A surface that cannot type a name passes a `history::stamped_id`
    stamp, as `accepted_save` does for a Set. Its route is `serde_json::to_vec(&layout)` into
    `Store::write_arrangement`.
  - `Operation::RestoreArrangement { name: String }` — put back the arrangement filed under `name`.
    Two refusals, both already written: `StoreError::NoArrangement` where nothing is filed under it,
    and the loader's own sentence where the file disagrees with itself (ADR-0158). The viewport the
    file carries is the one it was saved at; the caller sets the current one and solves, exactly as
    `Op::Reset` carries the viewport across today.
  - **`ResetArrangement` is unchanged and stays payload-free.** It is the built-in, it is not a
    name, and `RestoreArrangement` never reaches it.
- **A third operation is *not* owed by this record, and that is deliberate.** Listing the names is a
  question, and unlike `Report` its reply is sayable, so
  [ADR-0205](0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md) does not refuse
  it — but it is not part of save-and-restore, and a panel can read `list_arrangements` the way the
  Library bay reads `list_sets`. Whoever draws the control decides it, with the listing already
  built.
- **Nothing gains a route on any surface here.** *Reset the arrangement* still has four empty
  badges, and the two new rows are `docs/manual/operations.html`'s to write.

  *(Two corrections, both from the day after. **It did not have four empty badges when this was
  written**: [ADR-0220](0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)
  landed the same day and made that row's key badge `has r`, so the empty one this record meant is
  the **panel** badge. And the family found a home: all three rows carry `panel transport` now,
  beside the map pill, which is the only region on the console that is about the instrument rather
  than about material and whose own tooltip already offers save, load and start a new one.)*
- **`docs/adr/INDEX.md` owes a row for this record**, which is not added here because that file is
  shared with another session in flight.
- **The roadmap's M5 *Adds* bullet can lose two of the three things it says do not exist.** The name
  and the place are decided; the record carrying one is still owed, and this record argues it is
  owed to `arrangements/` rather than to `record.rs`.
