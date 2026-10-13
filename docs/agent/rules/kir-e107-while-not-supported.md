---
id: KIR-E107-WHILE-NOT-SUPPORTED
aliases: ["E107", "KIR-E107", "kir-e107-while-not-supported"]
title: While loops are not supported
summary: The procedure contains a while loop, which is disallowed in .kir shaders.
---

# While loops are not supported

## Behavior

Dynamic `while` loops are strictly prohibited in `.kir` to ensure execution determinism and prevent unbounded GPU execution on the render thread.

## Remediation

- Replace `while` loops with fixed-iteration `for` loops.
- Use `for i in 0..N { ... }` where `N` is an integer literal constant (note: no parentheses around `i in 0..N`).
