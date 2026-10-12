---
id: SLOT_UNALLOCATED
aliases: ["slot-unallocated"]
title: Target slot is not allocated
summary: Requested slot index does not exist in the active session configuration.
---

# Target slot is not allocated

## Behavior

The specified slot number is outside the bounds of allocated slots for the current session. Active Karakuri sessions typically allocate between 1 and 4 slots (indices 0 to 3).

## Remediation

- Call `get_permissions` to query the list of valid, currently allocated slots.
- Restrict slot parameters in tool calls to indices reported by `get_permissions`.
