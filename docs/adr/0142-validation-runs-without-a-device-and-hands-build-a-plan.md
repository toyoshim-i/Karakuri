---
id: 0142
title: Validation runs without a device, and hands `build` a plan
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [engine, testing, api]
---

# Validation runs without a device, and hands `build` a plan

## Context

Seventeen tests asserted only that a bad Set is refused, and each had to acquire a GPU to
reach the refusal, because `Set::build_many` took `&wgpu::Device` and there was no other
door. `build_inner` was 1,220 lines and did not touch the device for the first ~690 of
them: name derivation, all four slot-type edge walks, every `WrongKind`, the field cost
pass, and the per-source composition checks all ran before it. Two more — `NoCapacity` and
`Capacity` — sat further in still, inside `Simulation::build`, which is why the capacity
refusal was the slowest of them.

## Decision

`Set::validate` takes `build_many`'s arguments **minus the device and queue** and holds the
whole check pass. `build_inner` calls it and is the device half only.

**It returns a `Plan`, not `Result<(), SetError>`.** A validate that returned nothing would
leave `build_inner` re-deriving the names, the bindings, the salts and the per-head
attribute lists to build against — and a re-derivation is one edit away from being a
re-check. The `Plan`'s fields are private to the module.

**The checks moved; they were not copied.** `Simulation::build` is now infallible, which is
the type system holding the line: a second copy of the capacity rule would have nowhere to
put its `Err`. The evidence is an injection — one line changed inside `validate` fails both
the device-free test and a test that reaches the same rule through `build_many`.

## Alternatives

**Let `validate` take a `wgpu::Limits` so the device-limit check could move too.** Rejected.
`TooManyElements` compares an element buffer against `max_storage_buffer_binding_size`, and
its whole content is *this machine*. A caller supplying a limits value could ask about a
machine nobody has, which makes the refusal a function of a fiction. It also needs the
element stride, which only exists after codegen runs — so moving it means either a second
copy of the generated layout or running codegen twice. It stays on the device side, with
`Invalid` (the captured wgpu error) and `Panicked` (the swap worker's `catch_unwind`).

**Leave the tests slow.** The measured saving is smaller than it looks — libtest runs a
file's tests in parallel, so removing seven refusals from a file of thirty-one took almost
nothing off its wall clock. What the change actually buys is that those seventeen run in
the `--skip gpu::` set, and **on a machine with no adapter at all**.

## Consequences

**An ordering change.** `validate` now runs every head's checks before `build_inner` builds
any head; previously head 0 could return `TooManyElements` or `Invalid` before head 1's
composition checks ran. For a Set that is over a device limit on one source *and*
mis-composed on another, the variant reported changes. No rule fires differently on its own,
and no test covered the tie-break — preserving it would have meant interleaving `validate`
back into the build loop, which is the thing this undoes.

Each moved test's file now spells its Set fixture a second time beside the GPU helper it
mirrors. That is a duplicated fixture rather than a duplicated rule, and it is the one place
drift is newly possible in those files.
