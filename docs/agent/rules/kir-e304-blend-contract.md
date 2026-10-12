---
id: KIR-E304-BLEND-CONTRACT
aliases: ["E304", "KIR-E304", "kir-e304-blend-contract"]
title: Incompatible blend configuration
summary: Blend configuration is incompatible with render target or output mode.
---

# Incompatible blend configuration

## Behavior

The declared blend mode cannot be satisfied by the active render surface target format.

## Remediation

- Change blend mode to `additive` or `weighted`.
