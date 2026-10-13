---
id: KIR-E105-PARAM-SYNTAX
aliases: ["E105", "KIR-E105", "kir-e105-param-syntax"]
title: Invalid parameter syntax
summary: Parameter declaration syntax or range bounds specification is malformed.
---

# Invalid parameter syntax

## Behavior

The parameter definition does not match canonical `.kir` syntax. Header declarations do not use trailing semicolons. Parameters declare their type, bounds, and default value:
`param <name> : <type> [<min>, <max>] = <default>` where `min <= default <= max`. Supported types are `float` and `int`.

## Remediation

- Write parameter declarations without trailing semicolons: `param name : type [min, max] = default`.
- Ensure bounds satisfy `min <= max` and default value satisfies `min <= default <= max`.
- Match literal types to the declared type (e.g. `0.0` for `float`, `0` for `int`).
