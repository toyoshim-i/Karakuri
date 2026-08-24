---
id: 0163
title: A boundary gets first refusal on a pointer
status: accepted
date: 2026-08-24
supersedes: []
superseded_by: []
principles: []
tags: [ui]
---

# A boundary gets first refusal on a pointer

## Context

Two things want the pointer. `egui` routes it to whatever widget is under it;
`karakuri-console`'s own model routes it to the boundary between two regions, which is not a widget
and never will be — [ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)
put the arrangement here rather than in the toolkit's memory.

They do not overlap today, because `egui` has nothing in the gaps. That is a fact about the current
drawing rather than a rule, and the failure it invites is the one where both believe they are
dragging.

## Decision

**A pointer on a boundary is the panel's, and `egui` is not told about it.** Otherwise the event
goes to `egui`.

**A drag in hand keeps its claim wherever the pointer goes.** A drag is a gesture, not a position:
once a boundary is held, moving the pointer over a bay, over a widget, or outside the window
entirely does not hand it over. Deciding the claim from the pointer's current position on every
event is the obvious implementation and it is wrong — the boundary is dropped the moment the
pointer leaves the gap, which is within one pixel of picking it up.

**One exception, and it is documented where it is implemented: the panel always learns where the
pointer is**, whoever the event was claimed by, because every keyboard operation is addressed to
the region under the cursor. Motion updates the cursor unconditionally; the claim decides only
whether `egui` is *also* told. Buttons and the wheel are exclusive.

**The claim is asked before the release is applied.** Asking after is a real bug and not an obvious
one: `Panel::released` takes the drag out of hand, so a claim decided afterwards sends the
button-up to `egui`, which never saw the button-down. The window loop has exactly one place it
routes a pointer event, so that ordering exists once.

## Alternatives

**`egui` first, and the panel takes what is left.** The conventional arrangement, and it reads
correctly today because `egui` has no widget in a gap. Rejected on what it depends on: it is
correct only while nothing is ever drawn in or near a divider, so the first bay that puts a control
near its own edge silently steals the boundary. The rule above depends on the arrangement, which is
this repository's, rather than on the toolkit's layout, which is not.

**Ask `egui` whether it wants the event, and fall back.** `egui` answers that for widgets it knows
about, and a boundary is not one, so the answer is always *no* — the fallback would be the whole
mechanism, expressed indirectly and a frame late.
