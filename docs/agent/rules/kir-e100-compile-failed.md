---
id: KIR-E100-COMPILE-FAILED
aliases: ["E100-COMPILE-FAILED", "kir-e100-compile-failed"]
title: Shader compilation failed
summary: The procedure passed initial checking but failed native WGSL compilation or pipeline creation.
---

# Shader compilation failed

## Behavior

The `.kir` AST was validated by the frontend checker, but backend shader translation or GPU pipeline creation failed during translation to WGSL (e.g. Naga validation error or WebGPU pipeline layout error).

## Remediation

- Review the compiler error message and Naga validation details in the diagnostic report.
- Verify that arithmetic expressions do not generate invalid shader constructs.
- Check that the target device supports the required texture or buffer bindings.
