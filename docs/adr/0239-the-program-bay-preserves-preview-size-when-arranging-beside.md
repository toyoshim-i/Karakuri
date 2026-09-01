---
id: 0239
title: The Program bay preserves preview cell size when arranging beside
status: accepted
date: 2026-09-02
supersedes: [0182]
superseded_by: []
principles: []
tags: [ui]
---

# The Program bay preserves preview cell size when arranging beside

## Context

[ADR-0182](0182-the-program-bays-body-arranges-itself-for-the-larger-picture.md) established the dynamic placement of the Program bay: choosing between *below* (previews in a bottom row) and *beside* (previews split into two side columns) based on whichever provides the larger central picture.

However, in ADR-0182, the side columns in the *beside* placement were sized as half of the bay's total body height (`(H - 6)/2`), causing the preview cells to expand dramatically (from 63px to ~163px tall, consuming nearly 600px of bay width). This caused two issues:
1. **Loss of sizing coherence**: When the operator sizes the preview row using the divider bar in below mode, transitioning to beside mode inflated the preview cells to an arbitrary height determined by the bay's vertical extent rather than the operator's chosen size.
2. **Loss of interactive sizing on launch**: If the console opened directly in beside mode, the divider bar was set aside, preventing the operator from adjusting preview sizes.

## Decision

**The preview cell size established or resized in the below arrangement is preserved in the beside arrangement.**

- **Preserved Cell Dimensions**:
  - The cell height `cell_h` is derived from the preview row's fixed sizing (`layout.sizing(row)`, default `size::PREVIEW_ROW_H = 63.0px`), and width is `(cell_h × 16/9).round()`.
  - When the operator drags the divider bar in Below mode to resize the preview row, this chosen height is persisted in the layout and carried over to the Beside mode.
  - In *beside* placement, column width equals `cell_w` (e.g. 112px at default height).
  - The two stacked cells per side are vertically centered within the bay's body.
- **Initial Layout & Pure Geometric Decider**:
  - The arrangement selection is governed purely by geometric area comparison: `area(beside.picture) > area(below.picture)`.
  - At default preview height (63px tall, 112px wide), the crossover occurs at body width **706px** (window width 1484px).
  - To ensure standard windows (e.g. default 1440×900) open in **Below mode** with the interactive preview resizing divider bar readily available, the outer panes are given comfortable default widths (Left pane: 340px, Right pane: 400px), placing the initial central body width at ~644px (< 706px).
  - When the operator drags the preview row larger in Below mode (e.g., increasing `cell_h`), Below's available vertical space decreases, causing the Beside arrangement to naturally win earlier and transition smoothly while preserving the operator's enlarged preview size.
- **Central Picture Space**:
  - In Beside mode, the central picture takes the remaining width (`body.width() - 2 × (cell_w + PROGRAM_DIVIDER)`) and full body height `body.height()`.
  - The compact columns give the central picture significantly more horizontal room.

## Consequences

- **Consistent Visual Hierarchy**: Preview thumbnails retain the operator's chosen size and aspect across both Below and Beside arrangements without abrupt scaling.
- **Interactive Resizing Preserved**: Standard windows open with Below placement, enabling divider drag; dragging immediately updates the size, which persists across arrangement transitions.
- **Maximized Program Picture**: The beside arrangement provides an even larger canvas for the mixed master output.
- **ADR-0182 Superseded**: ADR-0182 is annotated with `superseded_by: [0239]`.
