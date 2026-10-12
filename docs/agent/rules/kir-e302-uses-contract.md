---
id: KIR-E302-USES-CONTRACT
aliases: ["E302", "KIR-E302", "kir-e302-uses-contract"]
title: Invalid uses contract
summary: A uses declaration specifies an invalid or incompatible upstream slot or node.
---

# Invalid uses contract

## Behavior

The procedure's `consumes` or `uses` block references attributes that the upstream generator does not `emit`.

## Remediation

- Verify that the upstream L1 procedure emits all attributes consumed by this L4 procedure.
- Check attribute names and types for consistency across both procedures.
