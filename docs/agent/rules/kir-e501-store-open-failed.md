---
id: KIR-E501-STORE-OPEN-FAILED
aliases: ["E501", "KIR-E501", "kir-e501-store-open-failed"]
title: Store directory open failed
summary: Unable to open or access the persistent artifact store directory.
---

# Store directory open failed

## Behavior

The runtime store directory path cannot be opened. The path may not exist or lacks read/write filesystem permissions.

## Remediation

- Check the store directory path provided in command-line arguments.
- Ensure the target directory exists and the process has write permissions.
