---
id: KIR-E100-PARSE
aliases: ["E100", "KIR-E100", "kir-e100-parse"]
title: Syntax parse error
summary: The .kir procedure source contains a syntax error and cannot be parsed.
---

# Syntax parse error

## Behavior

The source code violates `.kir` grammar rules. The parser encountered an unexpected token, invalid keyword, or missing punctuation mark.

## Remediation

- Inspect the reported line and column numbers in the diagnostic output.
- Verify that statements terminate with semicolons `;`.
- Consult `karakuri://doc/ir-spec` for the complete language grammar specification.
