---
id: 0128
title: A Set saved under a name the caller chose overwrites
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [mcp, store, save]
---

# A Set saved under a name the caller chose overwrites

## Context

`save_set` gives a model the control the `k` key gives an operator. A key press cannot
type a name, so it gets a stamped one; a tool call may pass an `id`.

Two calls of `save_set {"slot":0,"id":"keeper"}` therefore land on one filename. The
second replaces the first, both return `isError:false` and *"saved as set `keeper`"*, and
two `save` records go into the session stream for one file. The caller here is a machine
that reuses names, which is not the shape a human at a keyboard has.

[ADR-0125](0125-a-client-named-id-is-an-allow-list.md) had already established that a
client-supplied id is validated as one path component. That stops an id escaping
`<store>/sets/`; it says nothing about an id landing on a file that is already there.

## Decision

**A named save overwrites, and the tool says so.**

`--save-set ID` has always overwritten, and a tool that behaved differently from the flag
would make the same word mean two things depending on which door it came through. What was
missing was not the behaviour but the statement of it: the tool description, the manual and
the roadmap now all say a named id replaces what is under it.

## Alternatives

**Route the id through `history::unused`, which renames a collision.** Rejected on two
counts, and the second is the one that decides it.

It renames — `keeper` becomes `keeper-1` — and returning a name the caller did not ask for
is the behaviour `checked_id`'s own doc refuses for the same surface: a client that names
something and gets something else back has to be told, and then it has to decide, which is
a conversation a save should not need.

More seriously, it would not work. `history::unused` is a private function over an
in-memory set of the ids **this process** has issued, reachable only through `stamped_id`.
It is not a store lookup. A repeated `save_set {"id":"keeper"}` across two sessions — which
is the case a model actually produces — sees an empty set and overwrites anyway. So the
guard renames on the run where a collision is harmless and lets it through on the run where
it is not: a protection that fires in exactly the wrong half of the cases is worse than
none, because it is read as coverage.

**Refuse an id that already exists.** Not taken. It makes "save my work again under the
same name" an error, which is the ordinary thing a caller wants, and it needs a store
lookup on the render thread's side of the channel to answer.

## Consequences

A model that wants to keep several versions must vary the name or omit `id` and take the
stamp. That is now stated where a model reads rather than discovered by losing a file.
