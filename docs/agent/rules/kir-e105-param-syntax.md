---
id: KIR-E105-PARAM-SYNTAX
aliases: ["E105", "KIR-E105", "kir-e105-param-syntax"]
title: Invalid parameter syntax
summary: Parameter declaration syntax or range bounds specification is malformed.
---

# Invalid parameter syntax

## Behavior

The parameter definition does not match the canonical `.kir` syntax. Parameters must declare their bounds before their default value: `param <name> : <type> [<min>, <max>] = <default>;`.

## Remediation

- Reorder the bounds and default value to `param name : type [min, max] = default;`.
- Ensure the default value lies within the declared `[min, max]` interval.
