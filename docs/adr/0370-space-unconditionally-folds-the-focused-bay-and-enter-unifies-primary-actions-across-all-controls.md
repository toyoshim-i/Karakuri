---
id: 0370
title: Space unconditionally folds the focused bay, and Enter unifies primary actions across all controls
status: accepted
date: 2026-09-26
supersedes: [0343, 0357]
superseded_by: []
principles: [0090, 0094]
tags: [console, grammar, keyboard, focus, m8]
---

# Space unconditionally folds the focused bay, and Enter unifies primary actions across all controls

## Context

In earlier keyboard grammar specifications ([ADR-0343](0343-the-grammar-reaches-all-nine-bays-and-space-on-a-bay-is-the-fold.md),
[ADR-0357](0357-a-lane-is-taken-out-by-a-glyph-on-its-row-and-enter-on-the-lane-is-the-key.md)),
the functions assigned to `Space` and `Enter` depended on cursor address depth within the focused bay.

At the bay root level (address depth 0, where the cursor's path was empty), `Space` folded or unfolded
the bay. However, once an operator navigated deeper into child controls (e.g. an inspector parameter,
a sequencer lane, or transport controls), `Space` became overloaded to cycle or toggle control states
(cycling tone map operators, toggling mute, cycling sync chips, or returning parameter values).
Concurrently, `Enter` was assigned inconsistently: in some bays it descended into rungs, in others it
executed specific actions (such as removing a sequencer lane in ADR-0357 or selecting an item), and
elsewhere it was refused with "nothing here performs".

In live performance environments—frequently operated in dark venues under severe time constraints
([P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md))—this
depth-dependent duality led to critical misoperations:
1. An operator intending to quickly collapse or expand a bay with `Space` would inadvertently alter an
   on-air parameter, toggle a lane mute, or change a composite mode because the focus cursor remained
   nested within a child control.
2. An operator intending to toggle a control state with `Space` would collapse the entire bay if the
   focus cursor had popped back to the bay level.
3. The cognitive load of tracking whether `Space` meant "fold bay" or "cycle control" violated
   [P-0090](../principles/0090-a-surface-offers-it-never-decides.md).

Commit `7db3497` resolved this by flattening the keyboard interaction model into a predictable,
unambiguous baseline.

## Decision

**1. `Space` unconditionally folds or unfolds the focused bay from any address depth:**
- In `karakuri_console::focus::press`:
  ```rust
  // Space always toggles folding for the focused bay unconditionally (ADR-0259, ADR-0343).
  if key == Press::Space {
      return fold(panel, bay);
  }
  ```
- Regardless of whether the cursor points to the bay root (`Addressed::Bay`), a header control
  (`Addressed::Head`), an item row (`Addressed::Item`), or a deeply nested control rung
  (`Addressed::Of`), pressing `Space` immediately toggles folding (`Op::Fold` / `Op::Unfold`) of the
  focused bay.
- Physical muscle memory is absolute: `Space` always folds/unfolds the focused bay.

**2. `Enter` is unified as the primary action across all controls:**
- `Enter` is established as the universal trigger for primary actions across the console:
  - **Toggling boolean states**: Starring in Library, soloing the picture in Program (`0 1 enter`),
    routing sinks in Outputs (`1 enter`).
  - **Cycling multi-state chips**: Cycling tone map operators in Transport (`9 enter`), cycling sync,
    composite, and authority chips in Inspector, cycling sequencer grid modes.
  - **Executing item actions**: Cycling and triggering candidate actions in Staging, removing lanes
    in Sequencer.
  - **Entering child rungs**: When pressed at bay root with items, `Enter` steps focus into the
    remembered item.

**3. `Alt+Enter` and `Ctrl+Enter` establish secondary actions and defaults:**
- New press variants `Press::AltEnter` and `Press::CtrlEnter` are introduced to the focus grammar.
- These modifiers trigger secondary, non-destructive resets (e.g. resetting level faders, trim
  parameters, and exposure offsets to their default declared values).
- Controls without secondary actions cleanly decline modifier presses (`Asked::Nothing("a bay has no secondary action")`).

## Alternatives rejected

- **Retaining depth-dependent `Space` (fold at root, cycle in child):**
  Rejected because subtle cursor positioning errors in dark live environments caused disruptive
  on-air parameter jumps or unwanted panel collapses.
- **Reserving `Space` exclusively for playback / transport toggle:**
  Karakuri is a real-time visual performance instrument where the render clock runs continuously.
  Unlike linear digital audio workstations (DAWs), a global stop/start spacebar mapping does not fit
  continuous generative visuals.
- **Assigning bay folding to a modifier combination (e.g. `Shift+Space` or `Ctrl+F`):**
  Bay folding is a frequent panel management operation. `Space` is the largest, most readily
  accessible key on the keyboard; keeping it as unconditional fold ensures fast and reliable layout
  control.

## Consequences

- The entire grammar test suite (`crates/karakuri-console/tests/grammar/`) was updated to assert
  `Press::Enter` for all control toggling, chip cycling, and action dispatches.
- The focus grammar eliminates ambiguity between navigation, control actuation, and bay folding.
