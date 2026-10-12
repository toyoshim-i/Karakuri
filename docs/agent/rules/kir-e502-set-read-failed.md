---
id: KIR-E502-SET-READ-FAILED
aliases: ["E502", "KIR-E502", "kir-e502-set-read-failed"]
title: Set read failed
summary: Unable to read or parse the requested Set configuration from the store.
---

# Set read failed

## Behavior

The requested `.kbset` file was not found in the store or contains corrupt record data.

## Remediation

- Call `list_sets` to view all valid, stored Set IDs.
- Verify that the set ID is spelled correctly without file extensions.
