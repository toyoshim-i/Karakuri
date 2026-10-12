---
id: KIR-E108-FOR-BOUND
aliases: ["E108", "KIR-E108", "kir-e108-for-bound"]
title: For loop bound must be constant
summary: A for loop bound is not a static constant expression.
---

# For loop bound must be constant

## Behavior

The upper limit of a `for` loop evaluated to a non-constant or dynamic variable. GPU instruction cost estimation requires static bounds.

## Remediation

- Specify an integer constant literal for the loop range (e.g. `0..16`).
- If variable length is needed, loop over a static maximum and break or conditionally discard.
