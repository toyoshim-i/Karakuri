---
id: 0131
title: One refusal sentence per mistake, across the surfaces that face a person
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: [0061]
tags: [mcp, midi, mix, messages]
---

# One refusal sentence per mistake, across the surfaces that face a person

## Context

Naming a slot the deck does not hold was refused in four different wordings:

| where | text |
|---|---|
| `main.rs` | `no slot {n}: this deck holds slots 0-{k}` |
| `mcp.rs` | `no slot {n}: this deck holds 0-{k}` |
| `midi.rs` | `no slot {n} — this deck holds slots 0-{k}` |
| `mix.rs` | `slot {n}: this deck holds slots 0-{k}` |

The duplication predates the save control. What the save control added was a **claim of
uniqueness on top of it** — a doc comment reading *"No such slot, in the words every
surface says it in"* — and it added the claim on the first control with two front doors.
A model calling `save_set {"slot":9}` and an operator pressing `9` then `k` made the same
mistake on the same control and were answered in two different sentences.

`docs/roadmap.md` had also begun offering *"the refusals are the same sentences whoever
meets them"* as something M5's surface would inherit.

Every existing test asserted `contains("no slot 9")`, which passes under all four.

## Decision

**One free function, `no_such_slot`, called by every surface a person or a model reaches**
— the key handler, the MCP tool, the MIDI router and the mix path — and every assertion
changed from `contains` to equality against it, so the next divergence fails a test rather
than being discovered by reading four files.

The claim in the roadmap is left standing because it became true; the alternative was to
weaken it, and a claim that is cheap to make true should not be weakened.

## Alternatives

**Weaken the doc comment and the roadmap sentence instead.** Cheaper, and it was the
explicit fallback offered when this was scoped. Rejected because the unification turned out
to be four call sites, and because the sentence describes the property M5 inherits: a GUI
that grows a fifth wording is the failure this prevents, and it is easier to prevent now
than to notice later.

**Unify the engine's two as well.** `karakuri-engine/src/deck.rs` has two more spellings.
Deliberately left alone: they are `assert!` messages on a call that should never have been
made, addressed to whoever is holding a debugger rather than to an operator, and they are
in a crate that cannot see `karakuri-cli`. Making them share would mean either moving the
sentence into the engine — where the deck's own vocabulary, not the operator's, is the
register — or a dependency inversion for a string. The line drawn is *surfaces that face a
person*, and it is written into `no_such_slot`'s doc so the exclusion reads as a decision.

## Consequences

A fifth surface gets the sentence by calling one function. The engine's asserts remain a
separate register, on purpose.
