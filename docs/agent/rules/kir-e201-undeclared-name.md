---
id: KIR-E201-UNDECLARED-NAME
aliases: ["E201", "KIR-E201", "kir-e201-undeclared-name"]
title: Undeclared variable or identifier
summary: An identifier is used without prior declaration in the current scope.
---

# Undeclared variable or identifier

## Behavior

The referenced identifier is not defined as a parameter (`param`), local variable (`let`), consumed attribute (`consumes`), ambient signal, slot (`uses`), or built-in function in the current block scope.

## Remediation

- Declare local variables before first use: `let <name> = <expr>;`.
- For attributes emitted by an upstream procedure, declare `consumes <attr>` in the header.
- Check `karakuri://doc/ir-vocabulary` for the correct names of built-in functions and signals.
