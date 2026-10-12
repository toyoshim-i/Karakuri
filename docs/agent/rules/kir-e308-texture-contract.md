---
id: KIR-E308-TEXTURE-CONTRACT
aliases: ["E308", "KIR-E308", "kir-e308-texture-contract"]
title: Texture contract violation
summary: Texture binding or sampling mode is not supported by stage.
---

# Texture contract violation

## Behavior

Texture sampling was attempted in a compute or vertex stage where texture access is restricted.

## Remediation

- Move texture sampling operations to the `fragment` block.
