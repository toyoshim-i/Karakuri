---
id: 0089
title: History is gated on compiling, not on landing
status: accepted
date: 2026-08-15
supersedes: []
superseded_by: []
principles: [0096]
tags: [store]
---

# History is gated on compiling, not on landing

## Context

Undo, in the operator's framing: snapshot every version that compiles, let a GUI walk the history,
and save the ones worth keeping as user presets. History may be discarded on restart.

## Decision

A snapshot is taken **at the watcher's compile-success point**, which is the one place a hand edit
and a model's write both pass through, so both ride a single chain.

`<store>/history/YYYY/MM/DD/HHMMSS-mmm_slotN_LAYER_procname.kir`, deduplicated per (slot, layer) so
what accumulates is **edits, not rebuilds**.

**The gate is *did it compile*, not *did it reach the screen*** — and that is the difference from
[ADR-0084](0084-a-procedure-change-goes-into-the-record-stream.md)'s record. **A version rolled back
for exceeding the budget never appears in the session stream at all**, and is often exactly the one
you want back.

**The launch version has to be seeded.** The first implementation recorded only the *post-edit*
version on the first edit, so there was nowhere to go back to — and **an undo that cannot undo the
first edit is not an undo.** Seed and watcher share one `Snapshots`, or the first rebuild writes an
unedited layer a second time.

## On the date dependency

Directory names need a date, and this workspace had **no time dependency at all**. Computing UTC by
hand would have kept it that way, but the point of the feature is that an operator can clean up after
themselves, and a directory named a different day than they mean defeats that.

`chrono` over `time`, for a specific reason: **`time`'s local-offset lookup is soundness-gated and
returns nothing from a multi-threaded process** — and this application always has an audio thread and
a build worker, so it would silently fall back to UTC. That is the one thing a directory name must
never do.

Local time was chosen **not** to avoid dates splitting — as pointed out, a late-night set splits more
readily under local time, and events crossing midnight are ordinary — but so that **the directory name
matches the day the operator means**. The stated reason was corrected to that.

## Evidence

Session 2026-08-15T15:28Z–15:43Z, commit `6422f72`.
