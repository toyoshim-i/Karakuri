---
id: 0366
title: Mix participation is decoupled from residency, and solo and mute are destination operations
status: accepted
date: 2026-09-19
supersedes: []
superseded_by: []
principles: [0085, 0090, 0094]
tags: [engine, mixer, residency, operations, m7]
---

# Mix participation is decoupled from residency, and solo and mute are destination operations

## Context

In earlier iterations of the engine, whether a slot contributed to the final composite output
was inferred directly from its allocation residency:
```rust
let in_mix = slot.residency == Residency::Live && slot.opacity > 0.0;
```
This conflated two entirely separate concerns:
1. **Resource lifecycle vs. audio/visual mix contribution**: `Residency` represents pipeline compilation,
   GPU texture allocations, and compute budget enforcement managed by the governor
   ([ADR-0061](0061-residency-is-requested-by-the-operator-and-effective-by-the-governor.md)).
   Tying on-air audio and compositing directly to `Residency::Live` prevented keeping off-air slots
   warm or staged without leaking their outputs into the mix bus. Conversely, taking a slot off the
   mix momentarily (e.g. through muting or auditioning another slot via solo) could inadvertently
   trigger residency transitions or deallocation.
2. **Non-idempotent toggle operations**: The initial mixer implementation provided `ToggleSolo` and
   `ToggleMute` operations. While simple for an isolated keyboard shortcut, toggle semantics are
   fundamentally fragile in concurrent, multi-surface environments where commands arrive over MIDI,
   MCP tool invocations, or rapid key sequences. Two consecutive or interleaved calls to `ToggleMute`
   invert state rather than enforcing mute, leading to desynchronization between physical hardware,
   MCP clients, and the visual console.
3. **Coarse-grained view invalidation**: Mutating mixer states lacked a dedicated revision tracking
   mechanism, causing the application either to trigger wasteful full-surface redraws or to miss
   updating the Mixer bay promptly during rapid fader and button sweeps.

## Decision

**1. Decouple mix participation from `Residency::Live` via an explicit `online: bool` slot flag.**
In `karakuri-engine::deck`, each slot maintains an independent `online: bool` state. Mix participation
is evaluated strictly by:
```rust
pub fn is_in_mix(&self, slot: DeckSlot) -> bool {
    if let Some(s) = self.slots.get(slot.index()) {
        s.online && s.opacity > 0.0
    } else {
        false
    }
}
```
Composite shader passes and session meter updates in `Frame::render` sample `is_in_mix` rather than
checking `Residency::Live`.

**2. Arbitrate slot lifecycle during residency transitions (`set_effective`).**
- **Demotion from `Live`**: When a slot's effective residency transitions away from `Residency::Live`
  (e.g. to `Allocated` or `Cold`), the engine unconditionally forces `slot.online = false` and retires
  active meters.
- **Promotion to `Live`**: When promoted to `Residency::Live`, the slot's online status is automatically
  re-arbitrated against active solo and mute states:
  ```rust
  self.slots[slot.index()].online = match self.solo {
      Some(s) => slot.index() == s,
      None => !self.muted.get(slot.index()).copied().unwrap_or(false),
  };
  ```

**3. Two-layer mixer state model for Solo and Mute.**
- **Low layer**: Slot-level `online: bool` flag indicating instantaneous mix contribution.
- **High layer**: Exclusive solo (`Deck::solo: Option<usize>`) and per-deck mute (`Deck::muted: Vec<bool>`).
  - Solo is mutually exclusive: at most one slot may hold solo at any given time.
  - Mute is per-deck: muting a deck silences its contribution when no solo is active.
  - Activating solo on slot $S$ forces slot $S$ online and all other slots offline, without erasing their underlying mute flags.
  - Clearing solo restores every slot's online status according to its recorded mute flag (`online = !muted[i]`).

**4. Purge toggle operations in favor of absolute destination operations.**
`ToggleSolo` and `ToggleMute` are completely eliminated across `karakuri-operation`, `karakuri-mcp`,
and keybindings. All operations require explicit target values:
- `Operation::SetSolo { slot: DeckSlot, solo: bool }`
- `Operation::SetMute { deck: u8, mute: bool }`
- `Operation::ClearSolo`
- `Operation::SetOnline { slot: DeckSlot, online: bool }`

Console mixer strip buttons ([S] for sun solo, [M] for pink mute) read the current strip state,
compute the inverted destination value, and dispatch explicit destination operations:
```rust
Operation::SetSolo { slot, solo: !strip.solo }
Operation::SetMute { deck, mute: !strip.muted }
```
This adheres to [P-0090](../principles/0090-a-surface-offers-it-never-decides.md): the surface offers an
explicit target state, leaving decision and enforcement to the engine.

**5. Mixer revision counter and bay-local dirty repaints.**
The engine increments a monotonically increasing counter (`revision: u64`) on every solo, mute, or
online state change (`mixer_revision() -> u64`). The application event loop polls `mixer_revision()`:
when changed, it marks only the `"mixer"` bay dirty with `Duration::ZERO`, triggering an immediate,
localized console redraw without invalidating pipeline shaders, preview textures, or unrelated bays.

**6. Session recording and Setfile codecs.**
State transitions are captured in session streams and Setfiles:
- Added `Record::Solo { slot: Option<DeckSlot> }`, `Record::Mute { deck: u8, muted: bool }`, and
  `Record::Online { slot: DeckSlot, online: bool }` to `karakuri-store`.
- Updated `karakuri-environment::setfile::codec` to serialize and deserialize mixer solo, mute, and
  online states faithfully.

## Alternatives rejected

- **Single-layer toggle state machine**:
  Toggling a single boolean in-place conflates mute intent with solo override. Un-soloing a channel
  would erase prior mute settings, forcing the operator to re-mute channels manually.
- **Binding mix participation directly to `Residency::Live`**:
  Coupling audio/visual mix participation to the GPU residency governor prevented preparing warm slots
  off-air and risked unintended deallocations during rapid muting operations.
- **Global console repaint on mixer events**:
  Redrawing all seven console bays upon every mixer button press or fader nudge wastes CPU/GPU cycles
  and introduces render jitter during live performance, violating
  [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).
