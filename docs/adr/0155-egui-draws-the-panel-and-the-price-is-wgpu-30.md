---
id: 0155
title: egui draws the panel, and the price is wgpu 30
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: []
tags: [ui, dependencies]
---

# egui draws the panel, and the price is wgpu 30

## Context

Nothing in this tree drew a user interface. The dependencies were `winit` and `wgpu`, with no
text rendering and no toolkit, and everything an operator saw was either the rendered output or
a line printed to a terminal. The console is the whole of the application from where a person
stands, so something had to draw it.

**The choice looked like a preference and turned out to be a version constraint.** A toolkit that
renders through `wgpu` shares the engine's `Device`, and sharing a `Device` means linking one
`wgpu`. So the question "which toolkit" is answered at the same moment as "which `wgpu`", and
the two cannot be separated.

There is no `egui` release that binds to `wgpu` 26. Read off the registry on 2026-08-23:
`egui-wgpu` 0.32 wants `wgpu` 25, 0.33 wants 27, 0.34 and 0.35 want 29, and 0.36 wants 30.
**Version 26 was skipped entirely**, which is the version this workspace was on. Adopting `egui`
at any version was therefore a `wgpu` migration, and the only open question was how far.

That question was measured rather than estimated, by copying the workspace, changing the version
strings and running `cargo check --workspace`:

- **`wgpu` 27** — 6 errors in 2 kinds. `DeviceDescriptor` gained `experimental_features`, and
  `PollType::Wait` became a struct variant.
- **`wgpu` 30** — 54 errors in 12 kinds. Ten are fields added to or removed from descriptors and
  one signature widened to `Option<&BindGroupLayout>`. The twelfth is the only one that is not a
  rename: `Device::pop_error_scope` is gone, and `push_error_scope` now returns an
  `ErrorScopeGuard` whose `pop()` yields the error. That is a guard that cannot be forgotten,
  which is an improvement on what it replaces.

## Decision

**`egui` draws the panel, and the workspace moves to `wgpu` 30 and `naga` 30 to get it.**

`egui` earns its place on text and input, not on widgets. This panel's controls are faders,
meters, step grids and lanes, and no widget library supplies any of them — they are custom
painting whatever we choose. What a toolkit supplies that would otherwise be written here is
font loading, shaping, a glyph atlas, and the input and hit-testing plumbing underneath it.
That is the job being bought.

**30 rather than 27**, even though 27 costs six errors and 30 costs fifty-four. The 54 are
mechanical and were read in full before this was decided; they are an afternoon. What 27 buys is
starting an interface milestone a major version behind on the day it opens, and paying the
27-to-30 bill later anyway, at a moment when it arrives bundled with whatever `egui` breaks in
the same upgrade. Churn is cheap before v1 and expensive after it, and this is before v1.

**The `egui` dependency is not added by the migration itself.** The version move lands on its own,
with no consumer, because an unused dependency is not what that change is.

## Alternatives

**`wgpu` 27 with `egui` 0.33.** Six errors, compiling the same day. Rejected for the reason above:
it is the same migration paid twice, and the second payment is the expensive one.

**No toolkit — stay on `wgpu` 26 and paint the panel here.** Zero dependency movement, and it is
not as unreasonable as it sounds, since the controls are custom painting either way. Rejected on
what it leaves: text rendering in full — font loading, shaping, a glyph atlas, wrapping — plus
the input and focus plumbing. That is a project of its own standing between here and the first
control a person can touch, and none of it is what this instrument is about.

**A page served over the loopback MCP port.** Buys layout freedom and a tablet as a second
surface, costs a second language and a state round trip on every interaction. This had already
been costed and set aside before the version constraint was found, and the constraint does not
revive it.

**A separate window with its own `Device` on its own `wgpu`.** Two versions of `wgpu` can coexist
in one binary, so the panel could have stayed independent of the engine's version entirely.
Rejected on the previews: the console shows what each deck is rendering, and a texture cannot
cross from one `Device` to another. Every preview would become a readback and an upload each
frame, which buys a version number with a stall on the frame path.
