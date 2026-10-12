---
id: SLOT_POLICY_OFF
aliases: ["slot-policy-off"]
title: Slot policy is locked against modifications
summary: Target slot policy is explicitly set to Off, locking it against MCP modifications.
---

# Slot policy is locked against modifications

## Behavior

The operator has explicitly locked the target slot by setting its policy to `Off`. All write, rewiring, and state mutations targeted at this slot are refused.

## Remediation

- Do not retry mutations on this slot.
- Call `get_permissions` to discover available, unlocked slots in the current session.
- Request the operator to toggle the slot policy header pill in the Inspector to `Auto` or `On`.
