---
id: 0371
title: Divider dragging cascades through constrained bays into flexible siblings
status: accepted
date: 2026-09-27
supersedes: []
superseded_by: []
principles: [0090]
tags: [layout, console, bays, prompt, m9]
---

# Divider dragging cascades through constrained bays into flexible siblings

## Context

With the introduction of the Prompt bay (M9 Phase 1, [ADR-0365](0365-the-prompt-bay-embeds-an-interactive-multi-session-terminal-multiplexer-with-cursor-anchored-input.md)),
the console left pane was reorganized into a three-way split container containing `Library` (top),
`Staging` (middle), and `Prompt` (bottom).

This arrangement introduced a challenging constraint profile:
- **`Library`**: Flexible (`Sizing::Flex`), absorbing remaining available pane height.
- **`Staging`**: Constrained within a tight bound: minimum 66.0 px (header and candidate status) and
  maximum 125.0 px (standard candidate row preview height).
- **`Prompt`**: Flexible (`Sizing::Flex`), designed to expand and provide space for terminal sessions.

Previously, `Layout::set_divider` ([ADR-0295](0295-the-grip-is-the-fold-and-a-panes-outer-edge-is-the-other-one.md))
operated exclusively on immediate adjacent pairs: dragging a divider adjusted only sibling `a`
immediately preceding the divider and sibling `b` immediately following it.

In a three-bay split with tight intermediate constraints, this local pair resizing caused severe
usability stalls:
1. When an operator dragged Divider 1 (between Staging and Prompt) upwards to expand the Prompt bay,
   the drag stopped dead after only 59 px of movement, as soon as Staging contracted to its 66 px
   minimum height. Prompt could not be expanded further, despite Library possessing ample flexible
   height above Staging.
2. Similarly, pulling Divider 0 (between Library and Staging) downward when Staging was compressed to
   its minimum bound stalled immediately, unable to push displacement down into Prompt.

Halting user input when accommodating flexible space exists violates direct manipulation affordances
([P-0090](../principles/0090-a-surface-offers-it-never-decides.md)).

Commits `cc68844` and `1d130e6` resolved this by allowing divider dragging displacement to cascade
through constrained intermediate bays into flexible siblings.

## Decision

**1. Cascading displacement across sibling chains (`Layout::set_divider`):**
- When an adjacent bay reaches its physical boundary limits (`min` or `max`), it acts as a rigid
  sliding spacer rather than a hard stop.
- Surplus drag displacement (`overflow delta`) propagates further along the split axis:
  - Dragging Divider 1 upward compresses Staging from 125 px down to its 66 px minimum; any further
    upward displacement cascades past Staging into Library, shrinking Library by the remaining
    amount while holding Staging at 66 px.
  - Dragging Divider 1 downward expands Staging from 66 px up to its 125 px maximum; any further
    downward displacement cascades into Library, expanding Library back to its previous height.
  - Pulling Divider 0 downward expands Library and compresses Staging to its 66 px minimum, cascading
    any remaining downward displacement into Prompt.

**2. Strict split container conservation:**
- Before applying displacement, `Layout::set_divider` evaluates total capacity across all placed
  siblings in the split:
  ```rust
  let allowed = req.min(total_shrink_before).min(total_grow_after);
  ```
- No divider drag can violate container boundaries or force any sibling below its minimum constraint.

**3. Respecting sizing semantics during cascade:**
- Non-adjacent bays are resized according to their flexible capacity.
- Constrained nodes maintain their strict bounds, preventing distant fixed or collapsed bays from
  distorting during multi-bay drags.

## Alternatives rejected

- **Local clamping (halting drag at immediate neighbour bounds):**
  Rejected because it blocked operators from sizing the Prompt terminal to a comfortable height
  whenever Staging was present between Library and Prompt.
- **Proportional global redistribution across all bays:**
  Resizing one divider would cause all bays in the container to simultaneously shift and resize. This
  destroys operator layout stability and violates direct manipulation expectations.

## Consequences

- Direct pointer dragging across multi-bay containers feels smooth, natural, and unhindered by
  intermediate constrained nodes.
- Full cascading behavior is verified by integration tests in `crates/karakuri-console/tests/prompt.rs`
  (`prompt_boundary_drag_cascades_through_staging_into_library` and
  `staging_top_boundary_drag_downward_cascades_through_staging_into_prompt`) and
  `crates/karakuri-layout/tests/operations.rs`.
