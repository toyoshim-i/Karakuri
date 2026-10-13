---
id: KIR-E310-MISSING-OUTPUT
aliases: ["E310", "KIR-E310", "kir-e310-missing-output"]
title: Missing required stage output
summary: The procedure stage block does not assign a required output attribute.
---

# Missing required stage output

## Behavior

A required stage block is missing, or an execution path through a stage block completes without assigning a mandatory output:
- L4 requires a `fragment` block assigning `color = vec4(...)`.
- L4 `vertex` block (when present) must assign `clip = vec4(...)` (and `clip_b` if `lines` topology).
- L1 `element` block must assign `position = vec3(...)`.
- `Field` block must assign `distance = float(...)`.
- L5 `frame` block must assign `color = vec4(...)`.
- Conditional branches (`if`/`else`) must guarantee coverage for the required outputs on all paths.

## Remediation

- Ensure the required output variable is assigned on all execution paths.
- If using `if` statements, ensure both `then` and `else` branches assign the required output.
