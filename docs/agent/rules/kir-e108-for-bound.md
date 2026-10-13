---
id: KIR-E108-FOR-BOUND
aliases: ["E108", "KIR-E108", "kir-e108-for-bound"]
title: For loop bound must be constant
summary: A for loop bound is not a static constant expression.
---

# For loop bound must be constant

## Behavior

The lower or upper limit of a `for` loop evaluated to a non-constant expression, or the lower bound is greater than or equal to the upper bound. GPU instruction cost estimation and shader lowering require static bounds.

## Remediation

- Specify integer literal constants for loop bounds (e.g. `0..16`).
- If variable iteration counts are required, loop over a static maximum and conditionally `break` or skip.
