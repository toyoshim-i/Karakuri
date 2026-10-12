---
id: KIR-E307-SEED-CONTRACT
aliases: ["E307", "KIR-E307", "kir-e307-seed-contract"]
title: Seed contract error
summary: Random seed configuration is invalid or missing required salt.
---

# Seed contract error

## Behavior

Seed expression is invalid or attempts non-deterministic initialization.

## Remediation

- Use a 32-bit unsigned integer literal for the seed or rely on runtime slot seed derivation.
