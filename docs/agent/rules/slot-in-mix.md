---
id: SLOT_IN_MIX
aliases: ["slot-in-mix"]
title: Slot is active in live mix
summary: Target slot is currently active in the live projection under Auto policy.
---

# Slot is active in live mix

## Behavior

The target slot is active in the live projection mix under policy `Auto`. Direct mutations to an on-air slot are rejected to protect performance stability and prevent visual artifacts while rendering live output.

## Remediation

- Target an off-air slot (e.g. Deck B, C, or D) to stage your changes.
- Notify the operator that modifications are staged and awaiting a deck transition.
- Alternatively, request the operator to lower the slot mixer fader to 0.0 or toggle its policy pill to `mcp · on` if live on-air mutation is desired.
