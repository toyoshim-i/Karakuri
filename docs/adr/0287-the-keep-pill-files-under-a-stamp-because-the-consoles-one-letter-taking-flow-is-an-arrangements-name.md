---
id: 0287
title: The keep pill files under a stamp, because the console's one letter-taking flow is an arrangement's name
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0090]
tags: [console, ui, operations, library, inspector]
---

# The keep pill files under a stamp, because the console's one letter-taking flow is an arrangement's name

## Context

*Keep what a deck is playing* is one row on [every operation](../manual/operations.html), and its
variant is `Operation::SaveSet { deck: u8, id: Option<String> }` — a deck, and *"what to file it
under, or a stamp"*. Three of its four columns were already `has`: the key `k`, the MCP `save_set`
tool, and the store behind both. The panel column read `plan`, and the mock has drawn the control
since it was written — `.half-head`'s `keep` capsule in the Inspector's pane head, one per pane.

Its tooltip in [console.html](../manual/console.html) reads: *"Keep deck A as a Set, exactly as it
is on screen — the versions running, with their parameters, capacities, salts, layering and fold. It
goes into the library under a name."* **Drawing the pill meant deciding what that last clause
is**: a name the operator types at the press, or the name the store gives a save that arrived
without one.

**The console has exactly one letter-taking flow**, `Menu::Naming`, which
[ADR-0221](0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md) bounds to one
path component with no separators, for an arrangement's name. A Set id is not that name: it is a
different thing kept in a different place, matched by `mcp::checked_id` against a different rule,
and refused rather than overwritten where it is already taken. Reaching for that flow means either
a second one, or one flow meaning two things.

## Decision

**The pill emits `Operation::SaveSet { deck, id: None }` — the same call the key `k` makes — and
the store names the file.** `karakuri_environment::filed_as` answers `None` from an operator with
`history::stamped_id()`, which is *"a Set filed under the time you saved it"* and is the convention
the whole save path already reads: the library lists most recent first
([ADR-0263](0263-the-library-bay-lists-most-recent-first-because-the-listing-is-the-operations-and-not-the-surfaces.md)),
so a Set kept a moment ago is the row at the top rather than a row an operator has to remember a
word for.

So the mock's *"under a name"* is honoured by the store rather than by a field: what a keep is filed
under is a name, and nobody typed it.

**Two smaller decisions were taken with it, and they belong to the same control.**

- **The pill keeps the deck its pane is showing, not the deck the selection is on.** `k` keeps the
  selection because a bare key press cannot say which deck; a pill drawn inside a pane can, and the
  deck head one row under it is already in that arrangement — `DeckHead::deck` carries `Pane::deck`
  so that a press answers with the deck it was measured for. Two panes therefore keep two different
  decks with the selection sitting on one of them, which is the whole point of a per-pane control.
- **One capsule treatment, where the mock draws two.** The mock's first pane carries `.pill.on` and
  its second a plain `.pill`, and neither page says what the lit one is reading. Deck A is on air,
  *and* holds the selection, *and* is the first pane; a console that picked one of those three would
  be drawing a state off a guess, which is
  [ADR-0200](0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)'s rule. A
  keep is a press and not a setting, so there is nothing here for a wash to be *on*.

## What it costs

- **An operator cannot name a Set from the panel at all.** Every Set kept from this console is a
  timestamp, and the only way to a chosen id is the MCP tool — which files into the sandbox rather
  than the library ([ADR-0261](0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md)),
  so *"keep this as `glass_shell`"* is unreachable from every surface a person has. That is the real
  loss and it is not small: a library read by time is a library an operator scrolls rather than
  recognises.
- **A stamp says when and not what.** The library bay's rows carry the id and a time; a store filled
  by this pill carries the same fact twice and nothing else. What would fix it is a favourite or a
  rename, and this workspace has neither — `console.html`'s *What keeps a favourite, and where it
  does not travel* already says no field on a Set listing holds one.
- **The pill and the key can drift.** They are one operation asked for from two ends, and nothing
  compiles them together: `main.rs` performs the press and the key press in two arms, and the second
  argument of each is what says they file the same way.

## Alternatives rejected

**Open a naming flow at the press, which is what the mock's tooltip reads like.** The alternative
that actually loses something, and it is refused here rather than on its merits: the first control
that wants a thing is not where that thing is decided — `Chosen`'s rule two bays along, and
[ADR-0262](0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)'s
against the same wall. A console-wide text-entry flow needs a decision about focus and about what
the keyboard means while it is open, against
[ADR-0259](0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
where every letter is addressed to a bay.

**Extend `Menu::Naming` to take a Set id.** Cheaper than a new flow, and it makes one flow mean two
things — an arrangement being named and a Set being named — differing in what each will accept and
in what happens when the name is already taken: an arrangement is overwritten by design (ADR-0221)
and a Set id already in the store is *refused*. A name meaning two things is what
[ADR-0051](0051-a-name-means-one-thing-so-the-buss-noise-entry-is-deleted.md) is about, and a flow
whose refusal rule changes with what opened it is worse than either half.

**Leave the pill drawn and inert, keeping the `plan` badge.** What the tree was. An operator sees
the capsule whichever way the badge reads, presses it, and gets nothing — and this repository has
shipped a drawn, claimed, inert control once already.

**Draw no pill until a name can be typed.** ADR-0200's rule pointed the other way here: the value
behind this control exists, the whole save path is built, and the thing that is missing is a
convenience over it. Omitting the control would leave the bay's one act unreachable in order to
protect a field nobody has designed.

**Light the pill on the deck that is live, the way the mock's first pane is drawn.** It would make
the capsule a readout of something the Mixer already says, one bay away, in the colour that bay uses
for *on air*; and it would say it about a press.

## Consequences

- `crates/karakuri-console/src/view.rs` carries `KeepPill`, `keep_pill` and `KEEP_LABEL`, and
  `inspector_into` paints one capsule per pane through `pill_at` — the plain `.pill`. The words to
  its left are clipped one `.half-head` gap short of it, which is what `.sep`'s `flex: 1` is.
- **The console emits `Operation::SaveSet` and the row's panel badge is what has to follow**
  (`docs/contributing.md` §5 step 2): `karakuri-console/tests/panel_column.rs` reads every
  `Operation::` this crate constructs and demands a `has` badge for it, and it panics on a variant
  its `sample` table has no arm for. Both are owed by this record and neither is in this crate.
- **The mock's tooltip is owed one sentence** (§5 step 3): *"It goes into the library under a name"*
  is true and says nothing about who writes it, and the page should say that the pill files under
  the time it was kept, as the key column's `k` already does on
  [operations.html](../manual/operations.html).
- The pill claims nothing until `karakuri-console/src/input.rs` carries a `Probe` row for it; until
  then it is painted and a press on it reaches `egui`.
- **Nothing here decides the console's text entry.** The question is open, it is now asked by two
  bays, and this record is the second reason to expect it to be asked again.
