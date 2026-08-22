---
id: 0033
title: Freeing on the render thread is the same invariant as allocating
status: accepted
date: 2026-07-27
supersedes: []
superseded_by: []
principles: [0001]
tags: [engine]
---

# Freeing on the render thread is the same invariant as allocating

## Context

Hot swap builds the replacement `Set` on a worker thread, which satisfies the letter of "never
allocate on the render thread". What happens to the **outgoing** `Set` was not considered.

## Decision

Dropping a `Set` frees GPU resources, and **a deallocation on the render thread is the same
invariant as an allocation**. Retired Sets go to a shared vector via `try_lock` — **never
`lock`**, because blocking the render thread on a mutex is the stall the invariant exists to
prevent — and the worker drains and drops them outside the lock.

Two more things were found in the same pass and fixed for the same reason:

- **`Set::build` left the whole-capacity upload staged on the queue** — 12 MB at 262144
  elements. Left there, it is flushed by the **render thread's** next submit, so the hitch lands
  precisely on the frame the swap was supposed to be invisible to. The worker now submits and
  waits before handing over.
- **`Set::prepare` allocated two `Vec<u8>` per frame**, on the render path.

## Consequences

- `Set` is `Send` with no workaround — every field is a wgpu handle, a `Vec`, a `HashMap` or a
  number — and a static assertion sits beside the explanation, so a future non-`Send` field
  fails to compile at the line saying why it matters.
- The general shape: **work moved off the render thread is not off it until its side effects
  are too.** A queued upload and a deferred drop are both work that lands later, on whoever
  submits next.

## Evidence

Session 2026-07-27T14:47Z, 2026-07-27T09:22Z. Sharpens
[P-0001](../principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md).
