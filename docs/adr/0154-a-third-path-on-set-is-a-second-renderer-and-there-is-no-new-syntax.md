---
id: 0154
title: A third path on `--set` is a second renderer, and there is no new syntax
status: accepted
date: 2026-08-16
supersedes: []
superseded_by: []
principles: [0085]
tags: [cli, format]
---

# A third path on `--set` is a second renderer, and there is no new syntax

## Context

A slot had begun holding several renderers over one geometry, and the command line had to be able
to say so. `--set` already took a comma-separated list, and a **third path in it was an error**:
while a Set was a pair, `b.kir,c.kir` had been silently accepted as one literal L4 filename, so a
stray comma got blamed on a missing file, and refusing the third part was the fix.

## Decision

**One comma-separated list, read as one L1 and however many L4s, in draw order.** `--set
a.kir,b.kir,c.kir` is one geometry drawn twice. The refusal is lifted rather than respelled: there
is **no new syntax at all** — no second flag, no second separator, no repetition of `--set`.

The reasoning that had made a third path an error was sound and is now spent. It rested on a Set
being a pair, so that a third part could only be a typo; a Set holds a list, so a third part is the
thing the list is for.

**The typo it guarded against is still caught, by a narrower rule.** An empty part is not a
filename, so `a.kir,b.kir,` is refused for naming a renderer with no name.

## Alternatives

**A flag of its own** — `--renderer b.kir --renderer c.kir`, or a second separator between the
geometry and its renderers. Rejected: it introduces grammar for something the existing grammar
already reads, and the same list has to exist in the Set file and the session stream anyway, where
it needed no new record either. This is
[P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) applied to a command line.

**Keep the refusal and require a Set file for a stack.** Rejected for the reason
[ADR-0111](0111-a-name-lives-in-the-set-file-and-may-be-written-on-the-command-line.md) had already
found in the other direction: the command line is the only authoring surface that exists today, so
routing a capability through a file makes it unreachable to the person who has not written one yet.

## Consequences

- **Two formats absorbed it without a new record.** A Set file says a stack as several `slot`
  records on `L4` in draw order; a session stream says it as several `procedure` records carrying
  an `index`, absent meaning 0 and 0 not written — so a stream recorded before stacks existed
  replays byte for byte.
- **`--demo lines` became what it had been demonstrating.** It was two slots running `drift_shell`
  twice so that a second L4 could read the same cloud; it is one slot with two renderers, the same
  picture, and it no longer costs the simulation twice.
- The address a stack needs — `layer` plus an `index`, on the params, the edit history and MCP — is
  [ADR-0102](0102-a-renderers-address-is-layer-and-index.md), decided the same day; the two are
  halves of one capability.

## Evidence

Commit `4340da3` (2026-08-16). The rule is pinned by
`set_with_three_paths_is_one_geometry_and_two_renderers` and its neighbour
`set_with_a_trailing_comma_fails` in `karakuri-cli`.
