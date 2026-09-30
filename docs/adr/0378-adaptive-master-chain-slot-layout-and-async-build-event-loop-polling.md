---
id: 0378
title: Adaptive master chain slot layout and async compilation event loop polling
status: accepted
date: 2026-09-30
supersedes: []
superseded_by: []
principles: [0083, 0090, 0091, 0094]
tags: [console, master, event-loop, winit, layout, m10]
---

# Adaptive master chain slot layout and async compilation event loop polling

## Context

In Milestone 10, the Master bay gained full interactive control over the post-processing effects chain: adding L5 procedures, adjusting live parameter faders, cycling cut chips, and removing slots.

During interactive testing on varied window dimensions and resolutions, two issues emerged:

1. **Entire slot drop on compact viewports:**
   Each slot's vertical well height was calculated assuming all declared parameters were visible: `well_height(slot.params.len())`. For effects with numerous parameters (e.g. `sepia_film` with 8 parameters requiring 188.5px), on standard laptop displays (such as 1280x800 or 1024x768) the available height in the Master bay was smaller than the required well height. The boundary assertion `if !region.contains_rect(well) { break; }` broke out of the slot rendering loop entirely. As a result, zero slots were displayed—hiding not only the parameters, but also the effect's name, status dot, and removal button.

2. **Event loop starvation during background shader compilation:**
   To guarantee [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) and [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md), master chain compilation is offloaded to a background worker (`ChainSwap`). However, when an effect was added while all decks were stopped (`live == false`), the application frame completed without scheduling a redraw (`if live { gfx.window.request_redraw(); }`). The event loop entered `ControlFlow::Wait`. Consequently, even though the worker finished compilation within 10–20ms, `begin_frame` was never called to install the compiled pipeline onto `Present` until an external window event (such as a mouse move) woke the loop.

## Decision

**1. Adaptive Bottom-Up Parameter Row Dropping (ADR-0340):**
- In `crates/karakuri-console/src/view/master.rs`, dynamically calculate available vertical space per slot: `available = region.max.y - size::MASTER_PAD_X - top`.
- Derive the maximum number of parameter rows that can fit: `max_params = ((available - min_h) / line_h).floor() as usize`.
- Size the slot well to `well_height(num_params)` where `num_params = slot.params.len().min(max_params)`.
- If vertical space is constrained, parameter rows drop progressively from the bottom up. As long as the minimum header height (`min_h = 24.5px`) is available, the slot header—displaying the effect name, status indicator, cut chip, and remove `−` button—remains visible and operable.

**2. Event Loop Polling while Worker is Building:**
- In `crates/karakuri/src/app/handler/redraw.rs`:
  - Request an immediate window redraw when `apply_chain` initiates a build.
  - At frame completion, request a continuous redraw if `live || gfx.engine.chain_swap.building().is_some()`.
- In `crates/karakuri/src/app/handler/mod.rs`:
  - In `about_to_wait`, include a 16ms deadline (`Instant::now() + Duration::from_millis(16)`) in the control flow deadline whenever `chain_swap.building().is_some()`.
  - In `new_events`, request a window redraw when `chain_swap.building().is_some()`.

## Consequences

- Master chain slots are reliably displayed and operable regardless of viewport dimensions or parameter counts.
- Background effect compilation completes and installs onto the screen immediately without requiring deck playback or manual mouse movement.
- The UI adheres strictly to the contract established in ADR-0340: *Rows drop from bottom up if space is short*.
