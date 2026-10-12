---
id: KIR-E201-UNDECLARED-NAME
aliases: ["E201", "KIR-E201", "kir-e201-undeclared-name"]
title: Undeclared variable or identifier
summary: An identifier is used without prior declaration in the current scope.
---

# Undeclared variable or identifier

## Behavior

The referenced identifier is not defined as a parameter, local variable, ambient input, or built-in function.

## Remediation

- Declare the variable with `let <name> = <expr>;` before first use.
- Check `karakuri://doc/ir-vocabulary` for the correct names of built-in functions.
