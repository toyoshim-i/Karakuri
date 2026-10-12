---
id: KIR-E309-CAMERA-CONTRACT
aliases: ["E309", "KIR-E309", "kir-e309-camera-contract"]
title: Camera contract error
summary: Camera projection or coordinate transform is malformed.
---

# Camera contract error

## Behavior

Camera transform uniforms are accessed incorrectly or with mismatched types.

## Remediation

- Use standard camera ambients: `view_proj` (mat4x4), `eye` (vec3), and `ray` (vec3).
