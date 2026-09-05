---
id: 0262
title: A library filter field steps through what the store already holds, rather than taking letters
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0090]
tags: [console, ui, operations, library]
---

# A library filter field steps through what the store already holds, rather than taking letters

## Context

*List what the store holds* is one row on [every operation](../manual/operations.html), and its
variant is `Operation::ListSets { holds: Option<String>, layer: Option<Layer> }` — free text and a
layer. The MCP `list_sets` tool has been narrowing on both since it was written. The panel had not:
the Library bay took its listing as a `&[String]` of names from `Store::list_sets` and constructed
`ListSets` nowhere, so the row's panel badge was `plan`.

The mock has drawn two fields under the scope row since it was written —
`.lib-filters` in [console.html](../manual/console.html), `holds…` and `layer…` — and giving them a
route meant deciding what a press on one *is*.

**The console has exactly one letter-taking flow**, `Menu::Naming`, which
[ADR-0221](0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md) bounds to one
path component with no separators, for an arrangement's name. A filter is not a name: it is a
fragment matched against what a node is called, and nothing bounds it. So the field either gets a
second letter-taking flow — a decision about the console, not about this bay — or it gets an
affordance the console already has.

## Decision

**Both fields step, over a closed list, and the operation names where the step arrived.** `layer`
steps `LAYERS` — `L1`, `L2`, `L3`, `L4`, `FIELD` — the console's own curation of an enum the
vocabulary gives no list for. `holds` steps the node names the *unnarrowed* listing actually
contains, read off the store by the host and handed across the seam as `View::holds`; this crate
reads no store
([ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)). Both wrap **through
unset**, so every state either field can be in is one a press can leave, and a press with nothing to
step to still emits — it asks for the listing again, which is a real question and the one
`LibraryBay::chip` already answers for the chip that is already marked.

[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) is what permits this and what
bounds it: the affordance is the surface's, and what crosses into the vocabulary is
`ListSets { holds, layer }` naming the destination — never a step, because there is no step in the
vocabulary to name. `TransitionRow::shape` is the precedent three bays along
([ADR-0187](0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)): one pill
that cycles, emitting the value it arrived at.

**This is a first cut and not the finished control**, and the record exists to say so rather than to
close the question.

## What it costs

- **A filter cannot name a node the listing does not already contain.** `holds` is a walk over
  candidates, so *show me the Sets holding something called `bloom`* is unaskable on the panel while
  no Set holds one — which is the case an operator hits when they are looking for something they
  half-remember the name of. The MCP `list_sets` tool takes the text and the panel cannot;
  `--list-sets` does not narrow at all, and asks the unnarrowed question.
- **The cycle grows with the store.** The candidates are every distinct node name across every Set,
  sorted and deduplicated. A store of a few dozen Sets is a few presses; a store of a few hundred is
  a control nobody uses. There is no number at which it breaks — it degrades — which is exactly the
  kind of cost that is never noticed until somebody has lived with it.
- **A partial match is unreachable by pressing.** The narrowing is `contains`, matched
  case-insensitively, and the fields step through the names as the store spells them, so today the
  fold changes no answer. It is there because the tool's is, and the two must not come apart the day
  either field takes letters.

The `holds` half is what a text-entry model would fix; the `layer` half is a closed list of five and
stepping is the right control for it whatever happens to `holds`.

## Alternatives rejected

**A text-entry model for the console, and these two fields as its first users.** The alternative
that actually loses something. It is refused *here* rather than on its merits: the first control
that wants a thing is not where that thing is decided, which is `Chosen`'s rule one control up. A
console-wide text-entry flow needs a decision about focus, about what the keyboard means while it is
open — against
[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
where every letter is addressed to a bay — about how it is dismissed, and about what it looks like
in a mock that draws no cursor anywhere. Extending `Menu::Naming` instead would be worse: its bound
to one path component is ADR-0221's and is about a *name*, and a filter that silently refused a
space or a `/` would be a control refusing a question nobody could see the rule for.

**Take the letters in `Menu::Naming` unbounded, and let the filter be whatever was typed.** Cheaper
than a new flow, and it makes one flow mean two things — a name being recorded and a fragment being
matched — differing in what they will accept. A name meaning two things is what
[ADR-0051](0051-a-name-means-one-thing-so-the-buss-noise-entry-is-deleted.md) is about, and this
would be a control meaning two.

**Leave the two fields drawn and inert, and keep the `plan` badge.** What the tree already was. The
badge was honest rather than stale — the drawing existed and the route did not — but an operator
sees two fields whichever way the badge reads, presses one, and gets nothing; a field that is drawn
and claimed and inert is a defect this repository had already shipped once that day.

**Emit a step and let the host resolve it — `ListSets { step_holds: true }` or similar.** P-0090
forbids it in the plainest of its words: *an operation that says toggle, cycle or step* — a surface
that can only step has no way to arrive, and two surfaces stepping one control disagree about where
they are. The step is the console's arithmetic and stays in the console.

## Consequences

- `crates/karakuri-console/src/view.rs` carries `LAYERS`, `Field`, `Filters`, `stepped_holds`,
  `stepped_layer` and `LibraryBay::filter`, which is where the argument above is written in full.
  `crates/karakuri-console/src/input.rs` claims the row.
- `crates/karakuri/src/main.rs` performs the press: `narrowed` applies it through `View::narrow`,
  which is the one door into that state and refuses a `holds` the console cannot draw. The listing
  is re-read there, in the same branch `SelectScope` is re-read in.
- **The two fields are tipped in the mock**, under §5 step 3 — each saying that a press steps, what
  it steps through, that the wrap goes through unset, and that what is narrowed is `my sets`. The
  tips were owed the moment the fields became controls and are part of this record's work.
- *List what the store holds* carries `panel has library filters` on
  [operations.html](../manual/operations.html), and `docs/roadmap.md`'s M5.3 counts three `plan`
  rows where it counted four.
- **The `layer` cycle is exhaustive over `karakuri_operation::Layer` and nothing makes it stay so**:
  an array is not a match, so a sixth layer compiles. `karakuri-console/tests/library.rs` counts
  them, and `View::narrow` accepting any layer is only correct while every layer is in `LAYERS`.
- **Nothing here decides the console's text entry.** The question is open, and this record is the
  reason to expect it to be asked again.
