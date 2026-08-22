---
id: 0132
title: A rebuild restates the camera it was aimed with
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [engine, swap, camera, save]
---

# A rebuild restates the camera it was aimed with

## Context

`swap::Request` carried the layering, the salts, the params, the published controls, the
bindings and the edges, and did not carry the camera. `Set::build_inner` starts every built
Set from `Orbit::default()`, and only the startup path assigned a loaded one. So under
`--load-set X --watch`, editing any `.kir` put the camera back to its defaults and nothing
said so.

That was a display accident for as long as it lasted. It stopped being one when the `k` key
landed ([ADR-0122](0122-a-save-writes-the-bytes-that-are-on-screen.md)): a save reads
`set.camera` faithfully, so the first save after any rebuild wrote the **default** camera
into the operator's new preset. A wrong picture became a wrong file.

## Decision

`Request` carries the camera and `Watch` restates it on every rebuild.

**Restated, not read off the outgoing Set.** This is the rule the neighbouring fields
already follow and give the symptom for — `Request::bindings`' doc is where it is argued.
The sentence that carries: *a rebuild that lets a value be re-derived is a rebuild that
quietly discards what was loaded.* The camera was simply missing from a list it belonged in.

One `recorded_camera(args, slot)` feeds both the Set and its watcher, beside the
`recorded_salts` and `recorded_capacities` it is modelled on, so the two cannot be given
different answers.

## Alternatives

**`Option<Orbit>` rather than `Orbit`**, mirroring `FromSet::camera` and `build`'s own
parameter. Rejected: `Set::build_inner` starts at `Orbit::default()` unconditionally, so
`None` and `Some(Orbit::default())` build the same Set — the option would be a distinction
nothing downstream could act on. A slot that loaded no `camera` record carries
`Orbit::default()` and states it.

This puts the camera on **`salts`' side** of a question those two fields answer differently
on purpose: a salt is resolved once because nothing in a `.kir` declares it, where a
capacity stays deferred because the *new* file may declare one. Six camera numbers are
nothing a `.kir` declares, so they resolve once.

**Applying the camera after the params rather than before.** Equally correct today, because
the built-in orbit declares no parameters and no `ParamWrite` can address it. Rejected
anyway, so that the rebuild path and the startup path put the value in the same place: two
orderings that agree only by accident diverge silently on the day the orbit becomes
addressable.

## Consequences

One exposure is recorded rather than implied. `build_deck`'s wiring of the camera to
`Watch::new` has no test — replacing it with `Orbit::default()` there breaks nothing —
because a deck build needs `Args`, a GPU and four slots. `salts` has exactly the same
exposure and the same absent coverage. What guards both is structural: one function feeds
the Set and the watcher from one call.
