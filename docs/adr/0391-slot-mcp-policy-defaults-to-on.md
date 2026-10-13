---
id: 0391
title: Slot MCP policy defaults to on
status: accepted
date: 2026-10-13
supersedes: [0362]
superseded_by: []
principles: [0085, 0090, 0094]
tags: [mcp, console, inspector, slot-policy, safety]
---

# Slot MCP policy defaults to on

## Context

[ADR-0362](0362-a-slots-mcp-policy-is-auto-on-or-off-and-the-mixers-live-contribution-decides-auto.md) introduced a tri-state per-slot MCP policy (`Auto`, `On`, `Off`) with `Auto` as the default setting. Under `Auto`, any slot contributing live to the mix (`tally == Live && opacity > 0.0`) automatically write-protects itself against external MCP modifications (`SLOT_IN_MIX`).

While effective for guarded, high-stakes performances where an operator protects an on-air visual feed while an agent prepares off-air material, this default creates immediate confusion and friction for first-time performers, evaluators, and live-coders:

1. **Initial Exploration Roadblock:**
   On launching Karakuri, the initial default slot (Deck A) is live on air by default. When a new user or evaluator directs an agent to make its very first exploratory edit or shader tweak, the operation is immediately refused with `SLOT_IN_MIX`.
2. **Mental Model Misalignment on First Run:**
   Before understanding Karakuri's mixer faders, residency tallies, and off-air audition workflow, the operator perceives the system as blocked or broken, requiring manual intervention before experiencing basic live-coding capabilities.

## Decision

1. **Change Default Slot Policy from `Auto` to `On`:**
   - `SlotPolicy::default()` now returns `SlotPolicy::On`.
   - On session launch, all slots are immediately writable (`On`) by default.
   - First-time users and live-coders can direct an agent to modify, rebuild, or mutate shaders immediately without being blocked by initial `SLOT_IN_MIX` refusals.

2. **Updated Cycling Order:**
   - The interactive slot pill (`SlotMcpPill`) in the Inspector pane header cycles:
     ```
     On -> Auto -> Off -> On
     ```
   - When preparing for a performance where live on-air feeds must be shielded from unexpected autonomous mutations, the operator can click the pill once to engage `Auto` (protecting on-air faders) or twice for `Off` (complete lockdown).

3. **Supersession:**
   - This record supersedes [ADR-0362](0362-a-slots-mcp-policy-is-auto-on-or-off-and-the-mixers-live-contribution-decides-auto.md).

## Rejected Alternatives

- **Keeping `Auto` as the Default:**
  Rejected because beginner performers and evaluators should experience frictionless live-coding on their first attempt, rather than hitting security refusals before understanding the mixer architecture.
- **Removing Slot Write Protection Entirely:**
  Rejected because live performance safety ([Principle 0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)) requires an operator to have full authority to lock down on-air slots via `Auto` or `Off`.
