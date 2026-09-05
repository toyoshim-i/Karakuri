---
id: 0167
title: The panel keeps re-uploading what did not change
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [ui, performance]
---

# The panel keeps re-uploading what did not change

## Context

`egui` is immediate mode: it rebuilds every shape on the panel each time it draws, and uploads the
resulting vertices. An obvious answer is to cache each bay's drawing into a texture and re-upload
only what changed. It was designed in some detail before it was measured.

**The case for it rested on a number that turned out to be something else.** On the machine it was
written on, the buffer upload was **0.166 ms** — the largest part of the frame after the
submission — and that machine has unified memory. If an upload is that expensive where the CPU and
GPU share a pool, it should be worse where the bytes cross a bus, which no machine here could
check. So `docs/experiments/frame-cost-off-this-machine.md` was written and run on two more.

| | memory | backend | texture uploads | **buffer uploads** | record the pass | submit |
|---|---|---|---|---|---|---|
| Apple M4 Pro | unified | Metal | 0.000 | **0.166 ms** | 0.014 | 1.001 |
| AMD Radeon 780M | unified | Vulkan | 0.000 | **0.010 ms** | 0.011 | 0.273 |
| NVIDIA RTX 2070 SUPER | **separate** | Vulkan | 0.000 | **0.022 ms** | 0.015 | 0.298 |
| NVIDIA RTX 2070 SUPER | **separate** | DX12 | 0.000 | **0.064 ms** | 0.016 | 1.395 |

## Decision

**Do not take the optimisation. The panel goes on re-uploading what did not change.**

**The row that was supposed to be the large one is the second smallest.** The machine that moves
bytes across a PCIe bus pays 0.022 ms; the APU that moves them across nothing pays 0.010; the M4
Pro, which also moves them across nothing, pays 0.166 — **seven times the machine with the bus.**
Whatever that 0.166 ms is, the ordering rules out the only thing it was ever suspected of being.

Read the other columns and the reason is plain. Recording the pass costs 0.014, 0.011 and 0.015 on
three backends — everyone agrees about the work. `submit` on a discrete GPU is 0.298 against an
APU's 0.273, machines of entirely different classes, within 10%. **These are properties of `wgpu`
and the CPU rather than of the GPU or of what lies between them.** The last two rows settle it:
same GPU, same bus, same binary, and the upload still triples between Vulkan and DX12.

So the optimisation buys at most 0.022 ms of a 0.562 ms frame — 3.9% of the frame, 0.13% of a 60 Hz
second — and buys it by adding dirty-tracking to a panel that has none, which is the bookkeeping
immediate mode exists to remove.

**What this does not retire, and the distinction matters.** Caching a bay into a texture was
serving two masters. As a way to upload less, it is dead. As **the unit a scheduler defers**, it is
untouched: [ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)
puts the unit of deferral at a region, a whole-panel cache has no unit at all, and none of that is
about uploads — it is about not *building* shapes for a bay nobody is looking at. That cost is the
`egui` pass, which this table does not measure and this record does not settle.

## Alternatives

**Take it anyway, for the machines not yet measured.** Three machines, two memory topologies and
three backends now disagree, and they disagree in the direction opposite to the hypothesis. A
fourth would have to be strange to change it, and the work is a permanent mechanism against a
speculative row.

**Wait for more data.** The question was well posed and is answered: an upload that does not scale
with the bus is not paid for by the bus. Holding the design open costs more than the answer buys.
