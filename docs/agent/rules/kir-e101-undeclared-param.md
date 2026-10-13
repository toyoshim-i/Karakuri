---
id: KIR-E101-UNDECLARED-PARAM
aliases: ["E101-UNDECLARED-PARAM", "kir-e101-undeclared-param"]
title: Parameter contract or scoping violation
summary: Missing required parameter (such as spawn_rate), duplicate parameter declaration, or parameter name collision.
---

# Parameter contract or scoping violation

## Behavior

A parameter declaration violates interface contracts or scoping rules:
- An L1 procedure declares a `spawn` block but does not declare the mandatory `param spawn_rate : float [min, max] = default` in its header.
- A parameter is declared more than once in the header.
- A declared slot (`uses`) or local binding (`let`) shadows a declared parameter name.

## Remediation

- If using a `spawn` block, declare `param spawn_rate : float [0.0, 40000.0] = 8000.0` in the header.
- Ensure each parameter name is unique and does not collide with slot or variable names.
