---
id: 0240
title: The output shows the mix, and residency keys belong to the mixer
status: accepted
date: 2026-09-02
supersedes: []
superseded_by: []
principles: [0060, 0070]
tags: [operations, ui]
---

# The output shows the mix, and residency keys belong to the mixer

## Context

In closing sub-milestone M5.1 (Program Bay), two rows on
[every operation](../manual/operations.html) still owed implementations for their key column badges:

1. *Put a deck on air, prime it, or take it off* had a planned key shortcut (`space, w`).
2. *Choose what the output shows* had a planned key shortcut (`v`) and a backing vocabulary operation
   (`Operation::SetPreview`), allowing the central Program Picture to temporarily swap its canvas
   from the composited mix to a single auditioned deck.

Meanwhile, [ADR-0239](0239-the-program-bay-preserves-preview-size-when-arranging-beside.md) firmly
established the geometry and visual architecture of the Program Bay: the central Program Picture
presents the master output, while four dedicated Deck Preview cells (A–D) continuously display
per-deck audition frames beside or below the picture.

## Decision

1. **Retire *Choose what the output shows* (`Operation::SetPreview`) outright**:
   - The primary Program Picture strictly presents the master mixed output.
   - Per-deck auditioning is already continuously provided by the four Deck Preview cells; swapping
     the main output canvas away from the mix to show a single deck is redundant, confusing, and
     unnecessary during a live performance.
   - The operation is deleted from [`docs/manual/operations.html`](../manual/operations.html), the
     operation vocabulary (`crates/karakuri-operation`), and the MCP gate.

2. **Retire Program bay keyboard shortcuts for deck residency**:
   - Managing deck residency (Live, Priming, Allocated) is fundamentally the operational responsibility
     of the Mixer bay, operated via channel faders, mute controls, and residency tally chips.
     Binding global letter shortcuts (`space, w`) within the Program bay's scope was misplaced
     responsibility.
   - The key route for *Put a deck on air, prime it, or take it off* is marked `gap &mdash;` on
     [`docs/manual/operations.html`](../manual/operations.html).

## Consequences

- **Clear and Dedicated Roles**: The Program Picture is always the master output. Preview cells are
  always the channel monitors. Neither steals or swaps roles with the other.
- **Lean Vocabulary**: An unnecessary operation and its attendant audit rules are removed rather than
  maintained as dead specification.
- **M5.1 Exit Criteria Met**: The Program Bay rows in the panel and key columns of
  [`docs/manual/operations.html`](../manual/operations.html) carry no remaining `plan` badges.
