---
id: 0395
title: Mix participation is arbitrated internally by solo and mute operations
status: accepted
date: 2026-10-17
supersedes: [0366]
superseded_by: []
principles: [0085, 0090, 0094]
tags: [engine, mixer, operations, journal, mcp]
---

# Mix participation is arbitrated internally by solo and mute operations

## Context

[ADR-0366](0366-mix-participation-is-decoupled-from-residency-and-solo-and-mute-are-destination-operations.md) established the decoupling of mix participation from GPU residency and introduced high-level destination operations for solo and mute (`SetSolo`, `SetMute`, `ClearSolo`). Under ADR-0366, low-level slot participation in the final composite (`slot.online`) was derived via arbitration:
- If a solo is active on slot `s`, slot `i` participates in the mix if and only if `i == s`.
- When no solo is active, slot `i` participates if and only if `!muted[i]`.

However, ADR-0366 also exposed `Operation::SetOnline { deck, online }` and `Record::Online { slot, online }` as surface operations and journal records. In practice, exposing direct mutation of `slot.online` created architectural inconsistency:
1. **Bypassing Mixer Arbitration**: Callers (such as external MCP `operate` or session playback) could directly mutate low-level `slot.online` without updating high-level `deck.muted` or `deck.solo`.
2. **Visual Inconsistency**: A deck could have its pink mute button `[M]` lit on the console mixer strip (`deck.muted[i] == true`), while simultaneously rendering its output in the Program view because `slot.online` had been forced to `true` by a direct `SetOnline` call.
3. **Journal Ambiguity**: Recording `Record::Online` alongside `Record::Solo` and `Record::Mute` was redundant and caused replay desynchronization when reconstructing mixer state.

## Decision

1. **Eliminate `Operation::SetOnline`**:
   - `SetOnline` is revoked from the `Operation` enum, the operation gate rules (`gate::rules`), and the MCP tool tables (`SPELLED`).
   - All external control surfaces (GUI buttons, keyboard shortcuts, MIDI CC, MCP tools) must control deck mix participation strictly through `SetSolo`, `ClearSolo`, and `SetMute`.

2. **`set_online` is strictly internal**:
   - `Deck::set_online` remains solely an internal, low-layer engine method invoked by mixer arbitration (`arbitrate_mix`) in response to solo and mute state transitions.

3. **Eliminate `Record::Online`**:
   - `Record::Online` is removed from `karakuri-store` and `karakuri-environment::setfile::codec`.
   - Session recording and replay track mixer state solely through `Record::Solo` and `Record::Mute`. Replaying these records automatically re-computes `slot.online` via internal arbitration.

## Consequences

- The mixer display (solo [S] and mute [M] buttons) is guaranteed to stay synchronized with the active composite render participation.
- The external operation vocabulary and session record schema are simplified and unified under intentional operator actions.
- Supersedes ADR-0366 regarding the exposure of `SetOnline` and `Record::Online`.
