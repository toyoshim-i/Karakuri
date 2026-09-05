---
id: 0218
title: Re-anchoring is SetSync naming the mode the deck is in, and a cycle cannot say it
status: accepted
date: 2026-08-29
supersedes: []
superseded_by: []
principles: [0090]
tags: [ui, vocabulary]
---

# Re-anchoring is `SetSync` naming the mode the deck is in, and a cycle cannot say it

## Context

Writing the deck head into [the console page](../manual/console.html) turned up a control whose
sentence could not be written, which is the test that page exists to apply. Three facts, each read
off the source:

- **`Transport::engage`'s own documentation says re-engaging is the mechanism.** *"Re-engaging the
  mode a slot is already in re-anchors it, which is how an operator says 'call **this** the
  reference tempo' without a second control."* It is not incidental: `Transport::engaged` recomputes
  `anchor_bpm` from the session tempo every time, and clears the scrub with it.
- **No surface can ask for it.** `cycle_sync` starts its candidate loop at `at + 1`, so it can only
  land on the current mode when both others are refused — and the one case where that happens is
  material that accumulates *and* reads `beats`, where only `Free` is allowed and `Free` never reads
  the anchor. **The one reachable re-anchor is the one mode where re-anchoring does nothing.**
- **`Transport::engage` has no caller outside its own tests.** Eleven call sites, all in
  `transport.rs`'s `mod tests`. `Operation::SetSync` converts to `Owed::NotSettled`, and its MIDI and
  MCP badges are gaps.

So the page wrote the hole down rather than closing it: the anchor chip's tooltip reads *"A readout
and not a control. Re-anchoring is asking for the mode the deck is already in, and a chip that
cycles has no way to say that."*

## Decision

**Re-anchoring is `SetSync { deck, sync }` naming the mode the deck is already in. No new operation,
and the fault is in the two cycling affordances rather than in the vocabulary.**

[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) is the rule
and it predicted this exactly: *"a surface that can only step has no way to arrive."* `SetSync`
already names a destination; a cycle is an affordance built over it, and this is the one case where
the affordance is **structurally unable to express** something the operation can say. That is a
defect of the control, not a gap in the vocabulary.

**What changes is two documents and nothing else.** The anchor chip stops being a readout: a press
on it emits `SetSync` with the mode the deck is in, which re-anchors. And the *Set a deck's sync
mode* row gains a sentence saying that naming the mode a deck is already in re-anchors it at the
session tempo — so **every absolute route gets re-anchoring for free the day it exists**: a MIDI pad
naming a mode, an MCP call naming one. Only a cycle cannot offer it, and now the page says why.

The keyboard's translation — a key that re-emits the current mode rather than stepping — is a later
`karakuri-cli` change that this specification now covers.

## Alternatives rejected

**A `ReAnchor { deck }` variant.** The obvious shape, and the reason to record this at all, because
somebody will re-propose it. It names no destination `SetSync` cannot already name, and it costs a
variant, a row on [every operation](../manual/operations.html), a `TITLES` entry and an update to
the both-ways test that holds the page and the type together. Worse, it would be an operation whose
*meaning* is "again", which is the shape
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md) rules out —
two surfaces asking for "again" disagree about what state they are re-asking for, in the same way
two surfaces stepping one control disagree about where they are.

**Leave the anchor chip a readout and give re-anchoring to the keyboard alone.** It fails the
manual's first rule outright: every operation is reachable from the panel, from the keyboard alone,
from a mapped control and from MCP. It also leaves the sentence unwritable, which is what started
this.

## Consequences

- **A cycling affordance can never offer re-anchoring**, and that is now a stated property of the
  control rather than a surprise. Any future cycle drawn over a destination-naming operation
  inherits it: the state you are in is the one thing a cycle cannot ask for.
- **`Transport::engage` still has no production caller**, and this record does not add one. What is
  owed is the route: `SetSync` is `Owed::NotSettled` in
  [`karakuri-operation-record`](../../crates/karakuri-operation-record) because the engine's anchor
  clamp makes it a decision about the bytes — whether the record carries the anchor that was asked
  for or the one that was clamped — and that decision is untouched here.
- **The `y` key keeps stepping** until `karakuri-cli` changes, so re-anchoring is specified and
  unreachable in exactly one more place than it was. The difference is that the page now says which
  routes will have it and which never can.
