---
id: 0364
title: Every bay head renders the menu dice, and double-clicking the header toggles folding
status: accepted
date: 2026-09-27
supersedes: [0295]
superseded_by: []
principles: [0090, 0094]
tags: [console, bay, folding, interaction, m8]
---

# Every bay head renders the menu dice, and double-clicking the header toggles folding

## Context

In Karakuri Console, bays across the left, centre, and right panes organize visual generation
tools (`library`, `staging`, `program`, `inspector`, `mixer`, `master`, and `sequencer`).

Previously ([ADR-0295](0295-the-grip-is-the-fold-and-a-panes-outer-edge-is-the-other-one.md)),
bay folding was initiated by single-clicking a 6-dot grip icon rendered on select bays. This
presented two usability limitations as the console expanded:
1. **Affordance conflict**: The 6-dot icon (`⋮⋮` / dice) across standard graphical environments
   denotes a drag handle or context menu trigger, rather than a toggle switch. Conflating it with
   immediate bay collapse conflicted with planned bay-level operations (resizing, resets, docking/routing,
   and layout presets).
2. **Inconsistent head presentation**: Grip icons were rendered on only four bays, leaving remaining
   bays without a unified visual anchor or mouse affordance for folding.
3. **Accidental folding in live performance**: A single pointer press near the header margin could
   abruptly fold an active bay during performance.

Furthermore, with retained header bars ([ADR-0343](0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md)),
a folded bay retains a 27px title bar in-place, making the header persistently visible and
hit-testable even when collapsed.

## Decision

**1. Uniform 6-dot menu dice across all 7 bays:**
- `karakuri_console::view::widgets::head::head_grip` returns `true` for all 7 bays (`Kind::Bay`,
  `Kind::Library`, `Kind::Master`, `Kind::Mixer`, `Kind::Staging`, and `Kind::Sequencer`).
- `BAY_GRIPS` dynamically evaluates to 7.

**2. Reserve the dice grip for the upcoming Bay Context Menu:**
- Clicking the 6-dot dice grip is dispatched by `dispatch_head_press` and returns `Some(Acted::Nothing)`,
  claiming the pointer on `Claim::Panel` and reserving the affordance for the upcoming Bay Context
  Menu popup without triggering folding.

**3. Double-clicking the bay header bar toggles folding:**
- Double-clicking anywhere on a bay header bar (outside interactive pills such as the program solo pill,
  MCP badges, and the reserved dice grip) toggles folding (`Op::Fold` when expanded, `Op::Unfold`
  when collapsed).
- Double-clicks are detected within a 400ms interval and 5.0px spatial tolerance in `App::handle_left_mouse_input`,
  dispatched via `Pointer::DoubleDown`.
- Single clicks on a bay header continue to set focus (M8.4) without toggling folding.
- Keyboard folding via `Space` on any focused bay remains unchanged.

## Alternatives rejected

- **Retain folding on single grip click**:
  Conflates context menu activation with window collapse, creating surprise layout shifts.
- **Dedicated minimize/expand button (chevron / `[-]` / `[+]`)**:
  Consumes horizontal header space alongside titles, status pills, and performance controls.
  Double-clicking title bars matches standard windowing OS behavior.
