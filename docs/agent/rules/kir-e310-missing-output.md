---
id: KIR-E310-MISSING-OUTPUT
aliases: ["E310", "KIR-E310", "kir-e310-missing-output"]
title: Missing required stage output
summary: The procedure stage block does not assign a required output attribute.
---

# Missing required stage output

## Behavior

A stage block completed without assigning values to mandatory output variables. For example, an L4 `vertex` block must assign `clip`, and a `fragment` block must assign `color`.

## Remediation

- Ensure the `vertex` block assigns `clip = vec4(...);`.
- Ensure the `fragment` block assigns `color = vec4(...);`.
