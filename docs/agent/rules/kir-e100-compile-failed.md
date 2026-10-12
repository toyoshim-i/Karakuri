---
id: KIR-E100-COMPILE-FAILED
aliases: ["E100-COMPILE-FAILED", "kir-e100-compile-failed"]
title: Shader compilation failed
summary: The procedure passed initial checking but failed native WGSL compilation or pipeline creation.
---

# Shader compilation failed

## Behavior

The `.kir` AST was validated by the frontend checker, but backend shader translation or GPU pipeline creation failed during translation to WGSL.

## Remediation

- Review the compiler error message in the diagnostic report.
- Verify that variable expressions do not produce division by zero or invalid shader constructs.
- Check that the GPU target supports all features and limits referenced by the procedure.
