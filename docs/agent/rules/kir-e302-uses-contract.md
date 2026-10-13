---
id: KIR-E302-USES-CONTRACT
aliases: ["E302", "KIR-E302", "kir-e302-uses-contract"]
title: Invalid uses contract
summary: A uses declaration specifies an invalid or incompatible upstream slot, type, or access.
---

# Invalid uses contract

## Behavior

A slot declaration or usage violates interface contracts:
- Declaring duplicate slot names in `uses`.
- Using slot types not permitted for the procedure kind (e.g. `uses ... : Geometry` is only allowed in L2/L3; `uses ... : Field` cannot be taken by a Field; `uses ... : Camera` is only allowed in L4; `uses ... : Texture` is only allowed in L5).
- Declaring a slot name that collides with a built-in function or type constructor.
- Accessing far attributes (`slot.attr`) when the attribute was not declared or consumed.

## Remediation

- Ensure `uses <name> : <Type>` declares a type supported by the procedure's `kind`.
- Use unique slot names that do not clash with params, builtins, or types.
- Ensure consumed attributes are emitted by upstream procedures before accessing them.
