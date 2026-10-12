---
id: KIR-E101-UNDECLARED-PARAM
aliases: ["E101-UNDECLARED-PARAM", "kir-e101-undeclared-param"]
title: Undeclared parameter in contract
summary: Contract or header references a parameter not declared in the procedure body.
---

# Undeclared parameter in contract

## Behavior

A contract attribute or capacity expression references a parameter name that was never declared in a `param` statement.

## Remediation

- Declare the parameter in the procedure body: `param <name> : <type> [<min>, <max>] = <default>;`.
