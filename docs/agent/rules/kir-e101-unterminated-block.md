---
id: KIR-E101-UNTERMINATED-BLOCK
aliases: ["E101-UNTERMINATED-BLOCK", "kir-e101-unterminated-block"]
title: Unterminated block
summary: A block opening with '{' is not closed by a matching '}'.
---

# Unterminated block

## Behavior

The parser reached the end of the file or encountered an unexpected outer token while attempting to match an opened block brace `{`.

## Remediation

- Add the matching closing brace `}` to terminate the block.
- Check indentation and ensure all blocks (`element`, `vertex`, `fragment`, `spawn`, `deform`, `camera`, `field`, `frame`) and control structures (`for`, `if`) are properly closed.
