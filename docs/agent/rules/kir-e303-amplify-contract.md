---
id: KIR-E303-AMPLIFY-CONTRACT
aliases: ["E303", "KIR-E303", "kir-e303-amplify-contract"]
title: Invalid amplify factor
summary: Geometry amplification factor exceeds maximum limits or is non-constant.
---

# Invalid amplify factor

## Behavior

The amplification multiplier exceeds hardware expansion limits (typically 64x) or is non-constant.

## Remediation

- Set amplification factor to an integer constant within `[1, 64]`.
