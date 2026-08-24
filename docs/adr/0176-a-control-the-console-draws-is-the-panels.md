---
id: 0176
title: A control the console draws is the panel's, and it clears the grab
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# A control the console draws is the panel's, and it clears the grab

## Context

`karakuri-console`'s `input` module decides who gets a pointer event: a boundary in hand, then a
boundary within `GRAB`, then `egui`. Its documentation has carried a warning since it was written:

> It is true only while the gaps stay empty. `GRAB` widens every boundary by six pixels either side
> … and those twelve pixels are *inside the bays*, over whatever the bay draws at its edge. **The
> first control placed near a bay's edge is under a boundary's grab, and then both do think they
> are dragging.**

The Outputs row's one sink is that first control.

## Decision

**Rule 3: a point on a control the console draws is the panel's** — because the console *paints*,
and `egui` owns no widget anywhere in it, so a press routed to `egui` there reaches nothing at all.
`egui` buys text and input plumbing (ADR-0155); it is not holding this panel's controls.

**Rule 2 stays ahead of rule 3.** *First refusal* is meant literally: a boundary keeps it, and a
control under a boundary's grab would simply be dead.

**What keeps that ordering from ever costing anything is measured, not asserted in prose.** The
Outputs row is 34 tall and the chip is 18.5, centred, so there are **7.75** pixels of row above the
chip and 7.75 below, against a `GRAB` of **6** — the boundary's band ends 1.75 pixels clear of the
control.

**That is a fact about two constants and a rectangle, so it is a test.** It fails if the control
moves up, if the row gets shorter, or if `GRAB` widens, and it carries a guard on itself: it also
asserts that the row's own top edge *is* still inside the boundary's grab, so it cannot pass by the
grab having gone missing. Widening the grab past 7.75 makes the sink unclickable along its top edge,
and then **rule 2 is what has to change, deliberately** — rather than the control being nudged until
the test goes quiet.

## Where the control's rectangle comes from, and why `claim` now needs a text shaper

One derivation — `view::outputs(ctx, layout)` — is read by both `View::draw` and `claim`. Two copies
of that arithmetic is a control drawn where it cannot be clicked, which is the same argument
`preview_cells` is written for.

It takes an `egui::Context` because **the chip is as wide as the name in it**, and laying that name
out is `egui`'s job and nobody else's — it is `egui` that will paint the same run. That is a real
widening of what *who gets this event* depends on, and the two alternatives are worse: storing the
rectangle from the last frame is state that drifts, and stating the chip's width as a constant is a
box the drawn text may overflow.

**Before the first frame there are no fonts and no drawn control**, so `outputs` answers `None` —
*a control that has not been drawn cannot be clicked*. That is also what stops the window loop
aborting on a pointer event that arrives before the first pass, since `Context::fonts` panics until
then, and on macOS a panic inside a `winit` callback aborts the process rather than unwinding.

## Consequences

- `claim` takes the context: `claim(panel, ctx, p)`.
- **`claim` says *the panel's*; it does not say *the dot's*.** What a press on the control does is
  `Outputs::op` — one named operation — and the caller asks `view::outputs` for it. The same
  derivation, asked a second time rather than copied, so the chip that claims a press and the chip
  that acts on it cannot come apart.
- **The row's own arithmetic does not divide.** `.outputs` is 8px of padding around an 18.5 chip,
  which is 34.5, and the arrangement rounds to 34; `align-items: center` spends the half pixel as a
  quarter either side, and that quarter is inside the 1.75 of clearance this whole rule rests on.
- **The mock's row wraps and this one does not** (`flex-wrap: wrap`, against one sink 172 wide in a
  990-wide panel). Owed with the second sink rather than written for one.
- **No tooltip.** Every `.sink` in the mock carries a `data-tip`, and a tooltip needs `egui` to own
  a widget. That is its own decision; nothing here draws half of one.
