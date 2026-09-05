---
id: 0240
title: The output shows the mix, and residency keys belong to the mixer
status: accepted
date: 2026-09-02
supersedes: []
superseded_by: []
principles: [0080, 0087]
tags: [operations, ui]
---

# The output shows the mix, and residency keys belong to the mixer

> **Annotated 2026-09-02, later the same day.** This record rested on P-0070, *Auditioning is a
> prerequisite, not a convenience* — and the decision below made that file false as written: its
> rule was *one slot on the output in place of the mix*, and its *Where it holds* named
> `Deck::set_preview` and the `v` key. P-0070 is retired and the requirement re-recorded as the
> property,
> P-0080,
> *An operator can see a slot's own material without putting it on air*
> ([ADR-0241](0241-auditioning-survives-the-control-that-was-retired-and-is-re-recorded-as-a-property.md)).
> The front matter above is re-pointed to it, which is metadata rather than a change to what this
> record decided ([ADR-0059](0059-a-records-pointer-into-the-principles-registry-is-metadata.md)).
> P-0080 also records what this decision leaves owed: `karakuri-cli` can no longer audition a slot
> at all, and the four cells draw only Live decks, so an off-air candidate is visible on no
> surface.

> **Annotated 2026-09-02, again, after the maintainer read it.** **The reason below is not the
> reason.** This record retired *Choose what the output shows* as *redundant* — the four cells
> already audition, so swapping the main picture adds nothing. The retirement stands and the
> argument for it does not. The operation should never have existed: **once material is on the main
> output it is the broadcast of the final result, not a preview of it**, so putting a slot there was
> never a look at a candidate and redundancy was never the objection. How it got there is named in
> P-0080:
> a design made for the console, which has a picture and four cells, was bent to fit the command
> line, which has one window. What follows from the corrected reason is
> [ADR-0243](0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md) — the picture
> is an output and the four cells are monitors.
>
> The first annotation above also says this record left `karakuri-cli` unable to audition a slot,
> and calls that something P-0080 records as owed. **It is not owed.** The command line is test
> tooling and the instrument's principles do not bind it
> ([ADR-0242](0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)).
> The other half of that sentence — the four cells draw only Live decks — is a real gap and is the
> one entry left in P-0080's *Where it is not met*.

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
