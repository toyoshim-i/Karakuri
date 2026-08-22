---
id: 0037
title: Tone mapping is a uniform, and the default is chosen by looking
status: accepted
date: 2026-07-30
supersedes: []
superseded_by: []
principles: [0012]
tags: [render, engine]
---

# Tone mapping is a uniform, and the default is chosen by looking

## Context

Tone mapping had to land somewhere, and a default operator had to be picked for material nobody
has generated yet.

## Decision

**One call site**: the `Present` fragment shader, immediately before the sRGB encode — the same
place, and the same reasoning, as encoding once at the end.

**All four operators are compiled in and selected by a uniform field** — `Clamp`, extended
Reinhard, the Narkowicz ACES fit, and an approximated AgX. Switching is a `queue.write_buffer`
and **never a pipeline rebuild**, so the choice is changeable mid-performance. The default is
`Clamp` at exposure 1.0, so a caller that never sets it sees exactly the previous behaviour, and
a test asserts that path is bit-for-bit unchanged.

AgX was included for a reason specific to this material: stacking additive, emissive,
high-saturation point sprites drives individual channels far past 1.0, and Reinhard and ACES both
compress **per channel**, so a bright blue core desaturates toward white before the other channels
catch up — read by a VJ as "it loses its colour when you turn it up". AgX shares one curve across
a rotated log-encoded space instead.

## The disagreement, which is the point of this record

The implementing agent recommended AgX and justified it by saying it keeps more magenta identity
in the overflowing core. **Looking at the comparison renders, it does not.**

| | |
| --- | --- |
| Clamp | Hard white disc at the core. Surrounding magenta is the most saturated. Punchy, no core detail |
| Reinhard | Gentlest. Magenta survives nearest the core. Lowest contrast |
| ACES | Most punchy. White core smaller than Clamp's, colour stays strong |
| AgX | Palest. No hard edge, but **the whole image desaturates** and magenta drifts to pale pink |

What AgX actually preserves is **hue, not saturation**. Magenta never shifts to cyan and there is
no hard clip edge, but by design its highlights desaturate toward white. "Losing colour when it
gets loud" still happens; it happens differently. The agent's observation was not wrong so much as
its **restatement** of AgX's property was.

The trade that remains is real and is not settled by measurement: **ACES works on stage** (strong
colour, clear contrast, a look people are used to), **AgX is kind to material** (nothing breaks
however much is piled on, which matters when the material is generated and its exposure is
unknowable in advance).

## Consequences

- Because it is a uniform, this decision is only about **what happens when nobody chooses**, and
  can be changed on the night.
- A reported measurement is checked by looking at it. Same posture as
  [ADR-0015](0015-a-measurement-carries-how-it-was-taken.md): the number, or the claim, carries
  how it was arrived at, and the reviewer goes and sees.

## Evidence

Session 2026-07-30T16:42Z, 2026-07-30T19:19Z.
