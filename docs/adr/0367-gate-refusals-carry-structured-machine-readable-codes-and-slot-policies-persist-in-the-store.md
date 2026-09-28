---
id: 0367
title: Gate refusals carry structured machine-readable codes, and slot policies persist in the store
status: accepted
date: 2026-09-21
supersedes: []
superseded_by: []
principles: [0083, 0085, 0090]
tags: [mcp, gate, refusals, policy, store, m7]
---

# Gate refusals carry structured machine-readable codes, and slot policies persist in the store

## Context

Karakuri's operator gate protects live performances by vetting external operations before they touch
the running engine ([ADR-0235](0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md)).
When an MCP agent attempts to mutate an on-air slot, write to a locked resource, or modify a closed
bay, the gate rejects the request.

Previously, gate refusals returned only arbitrary, human-readable English strings (e.g.
`"slot 0 is on air: lower fader or change policy to force"` or `"slot 2 policy is off: open in inspector head"`).
This design presented critical limitations for autonomous co-performance:
1. **Ambiguous error recovery for AI agents**: An LLM agent receiving an unstructured prose string
   had to rely on brittle regex matching or fuzzy prompt heuristics to understand why an operation
   failed. The agent could not distinguish whether a failure was transient because a channel was
   in the mix (`SLOT_IN_MIX`), administrative because the operator explicitly locked the slot
   (`SLOT_POLICY_OFF`), or structural because the target slot was unallocated (`SLOT_UNALLOCATED`).
   This violated principle [P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
   *"a refusal carries what the next attempt needs"*.
2. **Volatile slot policies**: While [ADR-0362](0362-a-slots-mcp-policy-is-auto-on-or-off-and-the-mixers-live-contribution-decides-auto.md)
   introduced three-way slot policies (`Auto`, `On`, `Off`), these settings lived solely in transient
   process memory (`Arc<RwLock<[SlotAccess; 4]>>`). Restarting the console reset all slot policies
   back to `Auto`, silently erasing operator lockouts and creating dangerous security lapses during
   show rehearsals and restarts.

## Decision

**1. Machine-readable refusal codes (`RefusalCode`).**
In `karakuri-operation::types::refusal`, refusal reasons are codified into a dedicated enum:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    SlotInMix,        // "SLOT_IN_MIX"
    SlotPolicyOff,    // "SLOT_POLICY_OFF"
    SlotUnallocated,  // "SLOT_UNALLOCATED"
    BayClosed,        // "BAY_CLOSED"
    LaneHeld,         // "LANE_HELD"
}
```
Each variant exposes a stable, uppercase string representation via `as_str()`.

**2. Structured refusal payload (`RefusalDetail`).**
Refusal errors are wrapped in a comprehensive payload carrying the operational context:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalDetail {
    pub code: RefusalCode,
    pub message: String,
    pub slot: Option<usize>,
    pub deck: Option<u8>,
    pub lane: Option<usize>,
    pub class: Option<crate::gate::Class>,
    pub policy: Option<SlotPolicy>,
    pub in_mix: Option<bool>,
}
```
In `karakuri-mcp`, when a gate refusal occurs, tool results return standard JSON envelopes flagged
with `isError: true`:
```json
{
  "content": [
    {
      "type": "text",
      "text": "[SLOT_IN_MIX] slot 0 is on air: lower fader or change policy to force"
    }
  ],
  "isError": true,
  "result": {
    "refusal": {
      "code": "SLOT_IN_MIX",
      "message": "slot 0 is on air: lower fader or change policy to force",
      "slot": 0,
      "policy": "Auto",
      "in_mix": true
    }
  }
}
```
This preserves full backwards compatibility for human operators viewing the MCP console log while
enabling programmatic agents to autonomously adjust their strategy (e.g. pivoting to an available
off-air slot upon receiving `SLOT_IN_MIX`, or requesting operator unlock upon `SLOT_POLICY_OFF`).

**3. Persistent slot policies in the store (`<store>/policies.json`).**
- `karakuri-store` provides `Store::policies()` and `Store::write_policies()` for disk persistence.
- Any change to `SlotMcpPill` in the console immediately flushes updated slot policies to disk.
- On launch, `App::new` reads `<store>/policies.json`, ensuring operator lockouts survive process
  restarts and crashes.

**4. Session journal tracking (`Record::Policy`).**
- Added `Record::Policy { slot: DeckSlot, policy: String }` to the session record grammar.
- `session::head` emits `Record::Policy` for all active slots upon session recording start, guaranteeing
  that session journals record the exact safety configuration active during performance.

## Alternatives rejected

- **Transport-level JSON-RPC error codes (HTTP 500 / -32000 series)**:
  Standard MCP host implementations (Claude Desktop, Goose, Antigravity) interpret JSON-RPC error
  frames as unrecoverable protocol exceptions. This terminates the agent conversation loop rather
  than returning the error response to the model, preventing autonomous error recovery and retry.
  Refusals are expected domain outcomes and must arrive within tool response payloads marked `isError: true`.
- **In-memory volatile policies**:
  Forcing the operator to remember to re-lock sensitive slots after every console launch is error-prone
  and risks catastrophic live overrides mid-show, violating
  [P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md) and
  [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).
