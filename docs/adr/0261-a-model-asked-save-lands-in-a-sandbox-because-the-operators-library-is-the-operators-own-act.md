---
id: 0261
title: A model-asked save lands in a sandbox, because the operator's library is the operator's own act
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0096, 0083, 0090, 0094]
tags: [store, mcp, operations, docs, principles]
---

# A model-asked save lands in a sandbox, because the operator's library is the operator's own act

## Context

P-0048 stated where this program's material lives and who writes each place. Its second row read:

> `<store>/sets/` is user presets and only `--save-set` writes it.

**That sentence names a CLI flag as the sole writer of a directory, and it was false.** Four
controls write `<store>/sets/` today and three of them are not `--save-set`: `karakuri-cli`'s `k`
key, the panel's own `Keeping::save_set`, and MCP's `save_set` tool. The row has been wrong since
the `k` key landed and it was copied into two module headers — `scratch.rs` and `places.rs` — which
is how it stayed wrong in three places at once. ADR-0231 had already counted that duplication as a
cost it paid, without resolving it.

The wording matters more than a stale row usually would, because of what P-0048 is for. Its *what
it rules out* records the day a session rewrote three tracked examples, one day after the manual
gained the sentence saying a write replaces a file with no backup, recoverable **only because of
git, which is not a property of the feature**. A rule written against a model destroying an
operator's files was, in its own text, protecting `examples/` and naming a flag on the row next to
it. Every control that can destroy a `.kbset` an operator made was outside the sentence.

**And one of those controls is a model's.** `save_set` takes an `id`, an id that already names a
set is overwritten (ADR-0128), and the id space is the operator's own preset names. A model asked
to *"save this as `night01`"* — by an operator who has a `night01` — destroys it. Nothing in the
tree stopped that, and nothing said it should not happen.

Two questions therefore arrived together, and they have to be answered together: what the rule
actually says, and what happens to a model that asks for a save.

## Decision

**The rule is the actor, and it is re-recorded as
[P-0096](../principles/0096-the-operators-library-is-written-by-an-operators-own-act.md).** The
operator's library is written by an operator's own act — `--save-set`, the `k` key, a control they
pressed, and any route their hands reach next. A rule stated as a flag goes stale the first time a
second control reaches the same code; a rule stated as an actor does not, and the new file carries
its own table so the statement and the addresses cannot separate. P-0048 is deleted and its number
is retired, per `docs/contributing.md` §4.

**A save asked for over MCP lands in `<store>/sandbox/`.** It is written by
`Store::write_sandbox_set`, refused on the same three terms `Store::write_set` refuses, and
established by `Store::open` beside the other directories. What lands there is **an explicit
edit-history snapshot** — the thing an operator goes looking for after a show when a model has been
editing live — which is why the answer is a second directory rather than a closed door.

**`Operation::SaveSet` stays `Standing::Open` and `gate.rs` is untouched.** An operation that is
open is open to every control channel. That is rule 01 — *one vocabulary, four ways in* — and this
decision does not narrow it: the model is not refused, is not told to fetch a person, and gets an
answer naming a file that exists.

**Nothing in the sandbox is overwritten.** `karakuri_environment::filed_as` puts the stamp in front
of whatever name a client chose, so a model's save is `<stamp>` or `<stamp>_<name>`, and the accept
and the outcome both name what was written. `<store>/sets/` still overwrites a name an operator
typed, which is ADR-0128 unchanged: an id an operator types is an instruction, and there is no such
instruction in the sandbox.

**Who asked is an argument, not an inference.** `karakuri_environment::Asked` is passed at the call
site by both surfaces, and both already distinguished the two: an MCP request arrives with
`Some(reply)` and a key press with `None`. `reply.is_some()` would answer correctly today and would
be answering a different question — *is anybody waiting who is not at the terminal* — so the next
control that waits for an answer without being a model would file into the sandbox with nothing
failing.

## Alternatives

### a. Close `SaveSet` behind an openable class, with a fifth gate class and a Library bay pill

Live today, and the shape ADR-0235 already built for: four classes, each opened at the head of a
bay, each refusal naming the pill a person has to press. A fifth class — the operator's library —
with its pill at the head of the Library bay is a small, consistent addition, and it hands the
operator the decision, which is P-0094's own instinct.

**It loses on rule 01 and it loses in the same words ADR-0235 refused a scope in.** Every class
ADR-0235 draws is drawn off P-0094's question — *what does this do at its worst, on the frame it
goes wrong, while the operator's attention is on the room?* — and a save's worst case is a file on a
disk. Nothing stops, nothing goes quiet, nothing leaves the operator's hands. Classing it would be
classing by the noun, which is exactly what `WriteProcedure` staying open exists to refuse: a
control that rewrites what a live deck is drawing is open, and a control that writes a file would
be shut.

**And it buys the safety with the operator's attention, which is the more expensive currency.** A
closed class is a refusal mid-session and a person to interrupt. The sandbox costs nobody anything
at the moment of the save and leaves more behind than the closed class would: a class that is never
opened is a session of live edits with nothing kept anywhere.

### b. Refuse a model the operation outright

Also live today, and it is the smallest possible change: `save_set` stops being a tool. Nothing can
overwrite what it cannot reach.

**It loses on rule 01 outright**, which is the contradiction ADR-0235 was written to end — an
operation reachable from three ways in and not the fourth. It loses on P-0083 in a way a refusal
cannot repair, because there is no next attempt to carry anything toward: the model is not blocked
on an opening, it is blocked permanently.

**And it takes the wrong thing away.** The loss P-0048 was written about is work destroyed with no
copy anywhere. A model that has been editing live for an hour and cannot keep any of it is that
same loss arriving by the other door — nothing is overwritten and nothing is kept. The sandbox is
the one answer that is not a trade between those two.

### c. Let the model write `sets/` and make the id space safe instead — refuse an id already taken

The bundle path already does this (`--take-in` refuses an id already in the store rather than
overwriting), so the precedent is in the tree and the sentence is written.

**It answers the collision and not the rule.** A model would still be writing the operator's
library, and *what is in my library* would stop being a question the operator can answer from what
they did. It also puts a model's names into the operator's own namespace permanently: every id a
model has ever used is an id the operator can no longer take.

## Consequences

- **`karakuri-store` grows one directory and one writer.** `Store::SANDBOX`, `Store::sandbox_path`
  and `Store::write_sandbox_set`, with `Store::open` creating `sandbox/` beside `sets/`,
  `sessions/`, `thumbnails/` and `arrangements/`. The three refusals `write_set` made are now
  `refuse_what_is_not_a_set`, a free function both writers call, so a `part`, a `tick` or an
  artifact's metadata is refused identically in both directories.
- **`karakuri_environment::Asked` is a new public two-armed enum**, threaded through
  `setfile::save`, `accepted_save` and both surfaces' `Save` and `Saved`. Every existing call site
  passes `Asked::Operator`; the two that pass `Asked::Model` are `Live::run_requests` in
  `karakuri-cli/src/main.rs` and `Keeping::requests` in `karakuri/src/main.rs`.
- **A model cannot read back what it saved.** `read_set`, `list_sets` and `--load-set` all read
  `sets/`, and none of them was widened. This is stated in each of those tools' descriptions and in
  the `read_set` failure sentence rather than left to be discovered, and it is the largest thing
  this decision leaves undone: whether the sandbox is readable at all is a question nobody has
  asked yet. What a model has instead is the id in the accept and in the outcome.
- **The panel's Library bay does not list a sandbox save**, and `Keeping::took_save` returns
  `false` for one, so the listing is not re-read. `my sets` is the operator's library, and a bay
  that listed the sandbox would be listing two things under one scope.
- **Both surfaces say two sentences where they said one.** A library save keeps the sentence it
  had; a sandbox save names the directory and says outright that `--load-set` — or `l`, on the
  panel — does not reach it.
- **`checked_id`'s *a name a client picks twice overwrites* paragraph is now the opposite**, and
  says so with the argument it used to make: the premise was that a name a caller typed is an
  instruction, and there is no operator's instruction in the sandbox. What it still refuses is
  unchanged — a path never crosses the protocol.
- **Two module headers stopped restating the table.** `scratch.rs` and `places.rs` point at P-0096
  instead, which is what ADR-0231 named as a cost and did not resolve. The rule now lives in one
  file.
- **`gate.rs` is untouched.** `Operation::SaveSet { .. } => Standing::Open` is the same line it was.
- **The manual moved first.** `docs/manual.md`'s *Where your work lives* table gains the row and
  loses the flag, its `save_set` and *Keep it* sections say where a tool call's save goes, and
  `docs/manual/operations.html`'s *Keep what a deck is playing* tooltip says it on the row itself.
  `console.html` says nothing about where a save lands and gains nothing.
