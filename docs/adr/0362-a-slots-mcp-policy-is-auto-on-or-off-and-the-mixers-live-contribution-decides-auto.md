---
id: 0362
title: A slot's MCP policy is auto, on, or off, and the mixer's live contribution decides auto
status: accepted
date: 2026-09-16
supersedes: []
superseded_by: []
principles: [0085, 0090, 0094]
tags: [mcp, console, mixer, policy, m7]
---

# A slot's MCP policy is auto, on, or off, and the mixer's live contribution decides auto

## Context

The initial architectural roadmap for milestone M7 envisioned an autonomous agent system featuring
a fine-grained "tri-state node authority protocol" (`Autonomous`, `Suggest`, `Takeover`) and an
asynchronous background generation queue. That proposal presumed an AI agent would continuously
modulate parameters and audition candidate cards in real time to the musical beat.

In actual live VJ performance, this model is fundamentally misaligned with reality:
1. **Network round-trips over MCP cannot match musical tempo**: Real-time parameter modulation to
   the beat belongs either in procedural shaders (using audio/tempo uniforms like `beats`) or under
   direct MIDI / hardware control, not in low-frequency LLM remote tool calls.
2. **The operator already has a mixer and auditioning surface**: The console's Mixer and Program
   bays already provide faders, solo, mute, and per-slot live previews. Replicating candidate
   staging through separate "promote/demote" queues creates redundant cognitive load.
3. **Co-performance centers on macro-level creation**: An operator typically directs an agent to
   duplicate, evolve, or reimagine an entire visual slot (e.g. *"copy slot 1 to slot 3 and make it
   cosmic"*), inspects the candidate off-air, and then transitions the faders live when ready.

While bays already featured coarse-grained MCP toggle switches, mutating shaders in Inspector
required slot-level write protection. Without it, an agent could inadvertently overwrite an on-air
shader mid-performance, violating principle [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).

## Decision

**1. A three-way slot MCP policy in the Inspector header: `Auto`, `On`, and `Off`.**
Beside the Inspector header's `keep` pill sits a new interactive pill (`SlotMcpPill`) displaying:
- `mcp · auto` (default): Automatically protects the slot while on air; permits agent writes whenever
  the slot is off air.
- `mcp · on`: Always permits agent writes to this slot, enabling intentional live hot-reloads.
- `mcp · off`: Strictly locks the slot against any MCP modifications.

Clicking the pill cycles the policy (`Auto` -> `On` -> `Off` -> `Auto`), immediately updating both
the console view and a thread-safe `SlotPolicies` handle shared with the MCP server.

**2. Live mixer contribution decides `Auto` write permission.**
A slot contributes to the live mix if and only if its tally is `Live` and its mixer opacity is greater
than zero:
```rust
let in_mix = strip.tally == Tally::Live && strip.opacity > 0.0;
```
When an operator lowers a fader to 0.0 (or mutes the channel), the slot leaves the live mix. Under
`SlotPolicy::Auto`, the slot immediately becomes writable by MCP without requiring the operator to
manually toggle permission flags. Bringing the fader back up immediately re-arms protection.

**3. Standardized Bay and Slot MCP terminology.**
Bay header switches and slot pills are standardized around uniform wording:
- Bay headers: `mcp · on` (open to MCP) / `mcp · off` (blocked).
- Slot headers: `mcp · auto` / `mcp · on` / `mcp · off`.

## Alternatives rejected

- **Tri-State Node Authority (Autonomous / Suggest / Takeover)**:
  Operating authority at the individual AST node level creates an explosion of state, makes session
  replay journal verification brittle, and solves an imaginary problem. The operator controls the
  channel fader and the slot policy; the agent writes procedures.
- **Separate candidate staging queues**:
  Adding a secondary staging lane for proposed variants forces the operator to learn a second review
  paradigm. Off-air deck slots (auditioned through the console's existing preview cells) already
  serve as natural candidate workspaces.
- **Binary On/Off slot policy**:
  A simple toggle forces the operator to remember to manually unlock a slot before issuing an agent
  prompt and manually relock it before fading it in. `Auto` eliminates this friction by binding
  permission directly to physical fader position.

