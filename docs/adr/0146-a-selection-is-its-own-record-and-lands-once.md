---
id: 0146
title: A selection is its own record, and lands once
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: []
tags: [engine, deck, records]
---

# A selection is its own record, and lands once

## Context

`Set::set_input(at, Input { live, .. })` skips a renderer in the merge fold. It was built,
tested, and its only caller in the workspace was its own test — the same shape as the
storage figure with an owner and no reader.

Selecting among a Set's renderers on the beat grid is the half of M4's "variant pools" that
is true today. The passage describing pools makes four cost claims and **they cannot all
hold**: *three alternatives differing only in L4 share one simulation* is true only inside
one Set, where `Set::step` walks sources and `Set::draw` walks renderers; *selection is cheap
for closed-form and dear for accumulating* is true only across deck slots, where priming
lives. This builds the first and names the second as unbuilt.

## Decision

**A `select` record of its own, not a `Control` case on `transition`.**

Selection is a cut, and a cut is a fade of zero beats — so being a cut did not settle it. Two
things did. `Control`'s own doc excludes choices ("choices rather than positions"), and a
transition is addressed by `(slot, control)` and cancels on that pair, while a selection
carries **a second index inside the Set** that no `to: f32` can hold without spelling an
index as a float.

**It lands once and is dropped.** A transition writes its control every frame of its length;
a selection that did the same would be a permanent second writer of the merge edges, fighting
whatever else set them.

**The renderer index is checked where the record is applied, not where it is decoded.** The
decoder knows how many slots the deck has and nothing about what is in them, and a hot swap
can change how many renderers a Set draws with between the schedule and the beat. The slot is
still checked at decode, because that answer cannot move.

## Alternatives

**`Control::Renderer` with `beats: 0`.** Rejected above — the address, not the shape of the
move.

**Make it effective under `Overdraw` too.** Rejected: `Set::set_input` writes merge edges and
an overdrawing Set has none. It is silently ineffective at the engine and the record level,
following `set_input`, and the *key* refuses with a sentence naming `--merge` — a control an
operator presses says why nothing happened; a record replayed on a Set built differently
does not get to fail.

## Consequences, stated rather than discovered

**An unselected renderer still costs a draw pass and a target** — about 7 MB at 1280x720.
`Set::draw` draws every renderer into its own merge target and only the fold is skipped. Not
free; cheap.

**L4 only.** An L1 or L2 alternative carries state, and selecting among those needs the deck,
priming, and geometry shared across Sets — none of which exist.

**It cannot be saved.** `Layering` is deliberately not in the Set file, so a composited Set
loads back as overdraw. The selection is a property of the run, like `gain`.

**It is one-way.** Nothing folds every renderer back in; a Set comes up with all of them live
and the first press leaves that state for the run. Restoring the fold is a different
statement and wants its own spelling — `Record::Preview`'s null-for-the-mix is the shape it
would take.

**`no_such_renderer` is not a second spelling of `mcp.rs`'s index refusal**, and was judged
against [P-0090](../principles/0090-a-surface-offers-it-never-decides.md)
rather than assumed. They answer different questions — one is *does this slot's file list
hold an L4 at index n*, the other is *does the Set now playing draw with n+1 renderers* — and
a hot swap can make them legitimately disagree. Only one surface reaches the second.
