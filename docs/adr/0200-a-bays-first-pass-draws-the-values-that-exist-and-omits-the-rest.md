---
id: 0200
title: A bay's first pass draws the values that exist and omits the rest, rather than drawing an empty case the mock never drew
status: accepted
date: 2026-08-27
supersedes: []
superseded_by: []
principles: [0089]
tags: [ui, console]
---

# A bay's first pass draws the values that exist and omits the rest, rather than drawing an empty case the mock never drew

## Context

Five bays of the console were a head and nothing else — Library, Staging, Inspector, Master,
Sequencer. Which of the five to build first is not a decision anybody could reasonably re-propose:
the Library is the only one of the five whose values exist. `Store::list_sets` answers *what does
this library hold* with an id and an mtime per Set, and the other four are each waiting on machinery
rather than on work. That survey is in [roadmap.md](../roadmap.md), where the open questions live,
and it is not what this record is about.

**What is a decision is what a bay does with the parts of the mock it cannot fill.** The mock's
`.lib-row` is three things — a star, a name and a dim time — and the mock's Library bay has a scope
row, a path, two filter fields and a foot with a `load → C` pill in it. Of all of that, one thing
has a value behind it: the name.

- A **favourite** is a fact nothing in this workspace keeps. There is no such field on `SetEntry`,
  no record that carries one, and no metadata card that mentions one.
- A **scope** — `favourites`, `my sets`, `presets`, `folder` — is four collections of which one
  exists, and what a folder scope even reads is a question `console.html` itself lists as still
  open: *"A directory of `.kir` files is a different thing from a directory of Set files, and a
  bundle is a third."*
- The **filters** have an operation in the vocabulary — `Operation::ListSets { holds, layer }` — and
  nothing that answers it: `list_sets` reads names off a directory, and no index anywhere says what
  a Set holds.
- The **time** is the odd one out and is why this record exists rather than being obvious. The
  *value* is there: `SetEntry::written` is the Set file's own mtime, handed over with the id. What
  is not there is a **spelling**. The one answer in this workspace is `karakuri-cli`'s
  `setfile::written_at` — *"local, because the answer has to be the one the person would say out
  loud"*, to the second — and it lives in a package with no library target, so nothing can call it.
  The mock's own column is `16:09`, which is a third format again.
- **`load → C`** is *"How a Set gets from the library to a deck"*, the second of `console.html`'s
  two still-open questions and the manual's largest named gap.

The ones with no value at all are already settled, and
[ADR-0191](0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md) is
where: *a panel showing a parked chip that no `Deck` ever parked is a drawing of a state the engine
never entered*, with
[P-0089](../principles/0089-a-check-you-have-not-watched-fail-is-guessing.md) as the same
argument from the other side — a test owns its inputs, and this window's input is the program. A
star nothing can make true is that chip.

**What it does not settle is the time, or whether the bay may be drawn at all.** The time has a
value and no spelling, which is neither of ADR-0191's cases; and the largest thing missing from this
bay is a *route* rather than a reading.

## Decision

**A bay's first pass draws every part of the mock that has a value behind it and omits the rest
outright — no placeholder, and no empty case the mock did not itself draw.** For the Library that
is a row per Set, its name and nothing else, and a foot reading `n of m`.

**The listing is not blocked by the load.** *Nothing loads a Set into a running deck* stops anybody
**playing** from this bay; it does not stop the bay **saying what is there**, which is the whole of
what a readout is. The `load → C` pill is a control and this pass adds none.

**The time waits for its one spelling rather than getting a second.** The column is not drawn, and
the reason is recorded at `view::library` where the next person to want it will be standing: the
value exists, the format does not, and writing one here would be a second answer to *how a listing
spells a time* — which is the kind of thing this repository deletes rather than adds
([ADR-0198](0198-a-gesture-converts-in-the-parts-that-are-decided.md) deleted the last three records
this program derived twice).

**The empty case is the crate's existing one**, and it is not new here: with no store behind it the
bay draws its card and its head and nothing at all, exactly as the Mixer bay does with no deck and
the transport row with no engine. `tests/library.rs` asserts it by counting shapes against the
Master bay, which is the other bay in the mock with a title, no pill and a grip and has no body at
all — so the two draw the same shapes or the Library is drawing something a library nobody opened
does not have.

## Alternatives

**Draw the star hollow and the time blank, so the row is the mock's shape.** Rejected. A hollow star
on every row asserts that nothing is a favourite, which is a reading nobody took — there is no
favourite anywhere to be false. It is ADR-0177's row of zeroes with a different glyph, and the
`.strip.empty` the Mixer bay already declines to draw.

**Transcribe a time format into the console and draw the column.** Rejected, and it is the closest
of the three. It would have cost one `chrono` line in the example's dev-dependencies and a
`"%H:%M"`, and the column is the second most useful thing in a library after the name. What it buys
is a spelling that agrees with neither of the two that exist: not `written_at`'s local
`%Y-%m-%d %H:%M:%S`, and not the mock's bare `16:09`, which drops the day and is a fiction of a
single evening. The day `written_at` moves somewhere both callers can reach, this column is one
field and one line; until then the third format is the thing that would have to be kept agreeing
with the other two.

**Wait for the load decision before drawing the bay at all.** Rejected. It confuses a readout with a
route: the bay's job is to say what the store holds, and it can. Waiting would also leave the one
bay of the five whose values exist looking exactly like the four whose values do not, which is the
opposite of what the roadmap's survey is for.

**Put the foot under the last row, as the mock's flow does.** Rejected, and this is the smallest of
the four. The mock's bay is a flow: its children stack from the top and the leftover is under the
last of them. The console's bay is a rectangle, and where the leftover goes is a choice — under the
foot, or above it. It goes above: the Library is the bay carrying `style="flex:1"` in its column and
a `.grip` in its head, which is *"the bay that absorbs its column's height"*, and what absorbs it is
the list rather than a foot of one line at a fixed type size. A foot left floating would also put
its `border-top` between the list and bare card, which is a rule separating something from nothing.

## Consequences

- The Library bay lists what the store holds and says how many of how many, at every window the
  console is claimed to work at. `examples/panel.rs` reads `.karakuri` once at startup — a directory
  read is not a frame path's, and a Set saved while the window is up does not appear until the next
  run, which is the example's limitation and not the console's.
- **Six of the mock's Library controls stay undrawn**, and each is now named with what it is waiting
  on, in `view::library` and in the roadmap.
- **A store that cannot be read and a store that is empty are the same bay.** That is honest — a
  listing this run could not take names nothing it can name — and the example says which of the two
  it met, on standard output, rather than leaving the bay to say it.
- The pattern generalises to the other four bays and is meant to: **draw what has a value, omit what
  does not, and say at the code which is which.** The Inspector is the next one it applies to, and
  it is where it will be hardest — most of its rows have values and its authority row has none at
  all.
