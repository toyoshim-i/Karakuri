---
id: KIR-E308-TEXTURE-CONTRACT
aliases: ["E308", "KIR-E308", "kir-e308-texture-contract"]
title: Texture contract violation
summary: Texture binding, retention, or sampling mode is not supported by stage.
---

# Texture contract violation

## Behavior

Texture declarations or sampling operations violate stage contracts:
- Declaring texture slots (`uses ... : Texture`) or `retains` is only permitted in L5 post-processing procedures.
- The `texel` and `tap` functions require a valid texture identifier (`src`, `held`, or a declared Texture slot).
- Sampling `held` requires declaring `retains` in the L5 header.
- Naming a slot `src` or `held` in L5 conflicts with reserved built-in textures.

## Remediation

- Use texture sampling and slot declarations only in L5 procedures.
- Add `retains` to the L5 header when sampling the previous frame via `held`.
- Use distinct slot names that do not collide with `src` or `held`.
