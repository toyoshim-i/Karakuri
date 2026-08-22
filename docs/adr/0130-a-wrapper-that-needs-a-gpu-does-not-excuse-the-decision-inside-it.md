---
id: 0130
title: A wrapper that needs a GPU does not excuse the decision inside it
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [testing, mcp, save]
---

# A wrapper that needs a GPU does not excuse the decision inside it

## Context

`save_set` answers a client in two messages: `Accepted(id)` when the render loop takes the
save, then `Settled(Ok|Err)` when it lands. The first exists so that a client which times
out can be told something true — *this is being written, and I cannot yet say whether it
worked* — rather than the flat *nothing was saved*.

Every test drove a stand-in for the render loop, and **the stand-in sent `accepted`
itself**. Deleting `reply.accepted(&said)` from the real loop left the whole suite green,
while a timed-out model was told *"Nothing was saved, and asking again is safe"* about a
save that was running and would land. The one message whose entire purpose is to prevent a
false claim to a model was the one nothing checked.

The reason it went untested was real and load-bearing-sounding: it lives in
`Live::save_set`, and `Live` owns a `winit` window and a GPU and cannot be built in a unit
test.

## Decision

**Split the decision out of the wrapper.** `accepted_save(slot, id, sources, root,
Option<&Reply>) -> String` stamps or takes the id, forms the sentence, prints it and sends
the acceptance. `Live::save_set` calls it *above* `playing_values`, which is the only line
in that function that needs a `Deck`. The test drives the real `awaited` over a real
channel, so it reads what a client reads.

The general form: **"it needs a GPU" is a claim about a line, not about a function.** Where
one line needs the device and the rest is a decision, the decision comes out. This change
had already used that seam four times — `nothing_to_save`, `no_such_slot`, `live_sources`
and `drained_saves` are free functions for exactly this reason — so the excuse was
inconsistent with the file it was made in.

## Alternatives

**Accept it as untestable and note it.** What was in place, and the cost is now measured:
a false claim to a model, undetected. An untested wrapper is tolerable; an untested
*decision* inside one is how a guarantee becomes decoration.

**Build a headless `Live`.** Rejected as far more than this owes. `Live` is the frame loop;
making it constructible without a window is a restructuring with its own risks, to test
five lines.

## Consequences

Two things remain unreached and are recorded rather than implied: `run_requests`' forwarding
of slot, id and reply, and the `saves_in_flight` increment for an MCP-initiated save. Both
need the thread spawn as well as the deck, and there is no seam that takes one without the
other.

One test in this area asserts *source order* over `include_str!` — that `finished_saves()`
sits above `Live::frame`'s early returns. That is the same reasoning taken to its limit:
the defect is a statement order in a function nothing can instantiate, so the test asserts
the order where the order is, rather than asserting nothing.
