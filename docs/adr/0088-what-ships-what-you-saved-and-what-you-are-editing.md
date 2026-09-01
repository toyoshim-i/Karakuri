---
id: 0088
title: What ships, what you saved, and what you are editing are three places
status: accepted
date: 2026-08-15
supersedes: []
superseded_by: [0237]
principles: [0048]
tags: [store, mcp]
---

# What ships, what you saved, and what you are editing are three places

## Context

MCP writes straight to the file, and it has to: the hot-swap path begins with `--watch` noticing a
file change, so a procedure held only in memory never reaches the screen. Writing to a file is
**why an editor and a model take the same route**.

The cost arrived the same day: a model's session rewrote three tracked files in `examples/`.

## Decision

Three places, from the operator's framing:

| Layer | Where | Who writes it |
| --- | --- | --- |
| **App presets** | `examples/` | **Nobody.** They ship with the program |
| **User presets** | `<store>/sets/<id>.set.ndjson` | `--save-set` only |
| **Scratch** | `<store>/scratch/` | `--watch`, MCP, the operator's editor |

At launch, material from anywhere — `examples/`, an arbitrary path, `--load-set` — is **materialised
into scratch**, and the deck runs from there. The location is printed at startup, so an editor knows
where to point.

**Only a run with something writable copies.** `--render`, `--seq` and `--replay` do not, because an
offscreen render should be a function of its arguments and must not leave a directory behind as a
side effect.

**Two slots naming one file share one scratch file**, deliberately: copying per slot would silently
break the sharing `watch.rs` says it supports, and would make MCP's report that *this write also
reached slot N* untrue.

## Consequences

- **It dissolved an existing restriction.** `--mcp` and `--load-set` were mutually exclusive, because
  a Set built from the store has no `.kir` on disk for a model to read. With scratch, materialise it —
  and that closes the **user preset → edit → save** loop the three layers exist for. Material that
  existed only as a hash became named files anyone can open.
- **It also decided a question that had been left to taste.** The three rewritten examples had been
  left in the working tree for the user to judge. If `examples/` is what ships, a model's output
  living there is wrong by definition: restore the originals, keep the generated ones under their own
  names. **The option I preferred came out of the specification rather than out of preference.**
- Found while closing the loop and written down rather than fixed: **`--save-set` is a one-shot at
  launch**, so there is no way to save *this, now* from a running session. The remaining half of the
  loop needs one control that reaches from a key, from MCP and from the eventual GUI — the shape every
  other control in this system already has.

## Evidence

Session 2026-08-15T14:59Z–18:47Z, commits `6422f72`, and the `--load-set` follow-up. Standing rule:
[P-0048](../principles/0048-what-ships-what-you-saved-and-what-you-are-editing-are-three-places.md).
