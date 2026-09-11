---
id: 0346
title: The GUI and CLI keymaps diverged in scaffolding and converge on the operations page
status: accepted
date: 2026-09-11
supersedes: [0220]
superseded_by: []
principles: [0087, 0090]
tags: [cli, docs, ui, keymap]
---

# The GUI and CLI keymaps diverged in scaffolding and converge on the operations page

## Context

[ADR-0220](0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md) recorded a collision between `crates/karakuri` and `crates/karakuri-cli` on eight letters — `f g n p r s u z` — and resolved what the **key** column of [every operation](../manual/operations.html) measures by assigning it solely to the GUI instrument (`karakuri`), declaring the CLI's keys to be "its own."

At the time, this was justified as a way to unblock honest measurement without splitting the manual into a fifth column. However, treating the two keymaps as permanently independent programs contradicted the codebase's own trajectory: ADR-0220 itself acknowledged that *"karakuri-cli is scaffolding rather than the destination, and its keys are what the instrument's keyboard is being built towards rather than a surface with a permanent claim."*

In practice, the key divergence was a temporary development drift (*一時的な開発中のズレ*). Having divergent keybindings across the terminal tool and the GUI instrument costs an operator muscle memory, violates [P-0087](../principles/0087-name-the-property-never-the-shape.md) (*Name the property, never the shape*), and compromises [P-0090](../principles/0090-a-surface-offers-it-never-decides.md) (*Every control ends in the same record*). An operator reaching for a control should press the same key regardless of whether they are sitting in front of the GUI surface or the terminal instrument.

An earlier refactoring attempted to force convergence prematurely by adding runtime `eprintln!` deprecation warnings to `karakuri-cli` whenever colliding keys were pressed. This caused terminal stderr noise during live performance (violating [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)), and was rightly reverted in Phase 2A. But rolling back the noisy warnings did not mean the permanent separation of keymaps was desirable; it meant the convergence required deliberate design and planned migration.

## Decision

1. **The CLI and GUI keyboard mappings will converge onto the unified specification in [`docs/manual/operations.html`](../manual/operations.html).** The operations page's key column is the canonical definition for keyboard controls across all Karakuri programs.
2. **ADR-0220 is superseded.** The doctrine that `karakuri-cli`'s keyboard is permanently separate and exempt from the unified operations page is retired.
3. **Keymap Convergence is scheduled as Phase 2B Task P20 in `docs/refactoring.md`**:
   - Standard instrument operations shared by both programs (slot focus, faders, mute, transport, save) will share identical bindings.
   - The eight colliding keys (`f g n p r s u z`) currently used by `karakuri-cli` for quick adjustments (fade, quantum, latency offset, renderer, status, scrub, mask) will be migrated to non-colliding modifier chords or aligned with the panel vocabulary (fold, solo, unfold, reset, report).
   - In-app CLI help (`BINDINGS`) and manual tables will be updated synchronously when the migration lands, without runtime deprecation spam.

## Consequences

- An operator uses identical muscle memory across `karakuri` and `karakuri-cli`.
- `docs/refactoring.md` tracks P20 as an explicit milestone in Phase 2B.
- `docs/adr/INDEX.md` and ADR-0220 are updated to reflect the superseded status.
