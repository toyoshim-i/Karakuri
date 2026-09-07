---
id: 0271
title: The panel opens on the demo rather than on the reference workload's pair
status: accepted
date: 2026-09-07
supersedes: []
superseded_by: []
principles: []
tags: [panel, cli, examples, demo, docs]
---

# The panel opens on the demo rather than on the reference workload's pair

## Context

A run that names no material opens on two files this program chooses. Since `drift_shell.kir`
arrived on 2026-07-26 those two have been `examples/drift_shell.kir` + `examples/soft_points.kir`
— 262144 alpha-blended sprites at the capacity the L1 declares.

**That count was tuned for a program that drew one canvas.** The panel is now a deck of four
slots, each drawn into a preview cell and the mix into a picture ([ADR-0269](0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)),
and at a quarter of a million additive sprites no single element is visible in any of them: the
picture is a volume of fog. That is a real look and nothing is being taken away from anyone who
wants it — it is not what the first thing a person sees should be.

**A demo written to replace it already exists and was not allowed to.** `3307c0e`
(2026-09-07) added `examples/coil_vortex.kir`, `examples/star_flares.kir` and the
`examples/star_vortex.kset` that names them: a funnel of braided coils at 10240 elements, each
one a four-point flare turned to its own axis, with a heavy-tailed size distribution so a few
large ones sit among many small. Every decision in it was made by rendering frames and looking
at them, including two that are facts about this instrument rather than about the demo — the
built-in orbit is 14 degrees of elevation and revolving, so a disc has no tilt that survives a
full revolution, and the orbit looks down, so a vertical axis reads as a funnel rather than an
hourglass. That commit closed by declining to make it the default: *"the default pair is also
what `docs/contributing.md` names as the one reference workload, and splitting those two wants
a record of its own."*

**The coupling that blocked it is gone.** [ADR-0270](0270-the-reference-workload-is-a-named-set-rather-than-whatever-the-default-pair-is.md)
named the reference workload `examples/drift_cloud.kset` at 1280x720 — a Set, by name, rather
than whatever a bare run happens to open on — and said in its own consequences that what the
default pair should become *"is a demo decision and belongs to whoever makes it"*. `eb27392`
rewrote `docs/contributing.md` §1 and `a6f9065` took the coupling out of the code and the
startup legend. `Sources::under`'s doc has said since then that changing these two names *"is a
demo decision and takes no figure with it"*.

**This record is what ADR-0270 was for.** The order matters and is worth writing down: this
change is available because that one happened, not because it was always free. Before ADR-0270
the same edit turned a pinning test red and made the panel's startup legend lie, and the agent
that wrote the demo read that and stopped, correctly.

## Decision

**A run that names no material opens on `examples/star_vortex.kset`'s two parts —
`examples/coil_vortex.kir` and `examples/star_flares.kir`.** That is `Sources::under` in
`crates/karakuri/src/main.rs` for the panel and the no-argument default in
`crates/karakuri-cli/src/main.rs` for the CLI, and the two say the same thing because an
operator who runs one and then the other should see the same picture.

**It is a demo decision and it carries no figure.** No host-clock number in this repository was
taken on this pair, none moves because of this record, and the reference workload is unchanged:
`examples/drift_cloud.kset` at 1280x720.

**Two paths and not the `.kset`.** `Sources` is a pair, so a bare run gets the two parts and not
the Set's two `bind` records — `energy` on the funnel's `scale`, `band0` on its `ripple`. The
funnel turns and the pulse climbs it either way, because both are the L1's own motion; what a
bare run does not show is the material answering to sound. Loading `star_vortex` off the Library
bay is where that is, and the same has always been true of every other shipped Set.

## Alternatives rejected

**Keep `drift_shell.kir` + `soft_points.kir`.** The status quo, and it costs nothing to keep.
Its case was the reference-workload coupling — a change to the default was a change to the
comparability convention, so the demo was held in place by an arithmetic rule. That case no
longer exists: ADR-0270 moved the convention onto a named Set, and what is left in favour of the
status quo is that it is the status quo. Against it is what everyone who has looked at it
recently says, which is that a quarter of a million additive sprites reads as fog and that the
instrument's first impression is the one thing a demo is for.

**Ship the new pair and leave the default alone.** What `3307c0e` did, and it was right on the
day. It leaves the first thing anyone sees as the thing the demo was written to replace, and it
puts the better picture behind knowing to type two paths — which is exactly the operator who
does not have them yet. A demo nobody is shown is a file in a directory.

**Make the default the `.kset` rather than the pair, so the bindings come with it.**
`Sources` is two paths and `Sources::under` returns two paths; changing that means the type that
means *a Set's shape* becomes a file path, and every caller that asks *what two files is this
slot watching* has to resolve a Set first. It is a change to what a slot is in order to give a
demo two bindings it degrades gracefully without — with no audio input both signals are invented
at confidence 0.10, so what is lost is a tenth of the movement. Not worth the type.

**Pick a different demo.** `examples/light_cloud.kset` (`eed3580`) is the other candidate ADR-0270
named, and it is deliberately cheap — it exists so a cost can be compared against something,
which is a measurement job rather than a first-impression one. `star_vortex` is the one with an
argument written for being looked at.

## Consequences

- **`Sources::under` and the CLI's no-argument default name the new pair**, and both docs say
  what they were until today and why they moved. `karakuri-cli`'s `USAGE` line under
  `(nothing)` says the new pair.
- **`--demo lines` is untouched.** It supplies `drift_shell` + `soft_points` +
  `drift_streaks` on purpose — one geometry drawn two ways, with a script that fades one slot
  at a time — and `star_flares` consumes `size` and `uv`, which `drift_shell` does not emit. A
  demonstration brings the scene it is about; this record is about the scene nobody asked for.
- **`the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none` needed no change**, which
  is ADR-0270 having already done its work: the reference workload is pinned there by name and
  the shipped pair is checked only for reading its capacity out of its own file. `coil_vortex.kir`
  declares `capacity [1024, 1048576] = 10240` and passes that check.
- **A GPU test went quiet rather than red, and is now pinned to the workload.**
  `gpu::the_budget_parks_a_deck_and_the_strip_carries_both_residencies` built its four slots from
  the shipped pair and asserts deck B is parked for `NoHeadroom` — the budget refusing a second
  Live slot. On the new pair the governor measured 1.82 ms a slot against 2.66 ms of headroom and
  answered `NoPrimingNeeded` instead: `coil_vortex` is closed-form, so there is nothing to warm
  and the park is not the budget's. The test now builds from a `reference()` helper naming
  `drift_cloud.kset`'s pair, because whether the budget refuses a second slot is a claim about a
  cost and must not move when the demo does. That is the same split as the capacity pin, one
  test later.
- **A bare run still shows a parked deck, and the park has a different reason.** Deck B is still
  asked to prime at startup and still refused, so the two residencies still disagree and the
  rolling parked chip ([ADR-0190](0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md))
  is still on screen the first time anybody opens the panel. What changed is the word: a release
  run on an Apple M4 Pro reports `governor: 1.61 / 2.34 ms live` and parks deck B for
  `NoPrimingNeeded`, against `10.76 / 15.46` and `NoHeadroom` on the old pair. Both refusals hold
  numerically — warming was measured at 1.472 ms against 0.736 ms of headroom — but `closed_form`
  is checked before the budget, so the reason reported is the closed-form one.
- **The startup legend was asserting the old reason in a fixed string.** It said *"the budget has
  no room"* one line above the `Report` line printing the governor's own word, so a bare run
  printed `the budget has no room` and `NoPrimingNeeded` about the same park. The sentence now
  reads the reason off the report, which is what the paragraph below it about the resting slots
  already did. This is the same class of defect `a6f9065` cleaned up two commits earlier and the
  reason it is worth naming: a legend that narrates a decision it did not read goes wrong the
  first time the decision changes.
- **`CANVAS`'s doc no longer calls itself the reference workload.** 1280x720 is the workload's
  canvas and stays; the element count is the material's, and a bare run is 10240. A figure taken
  on a bare run is not a reference-workload figure, and the startup legend already prints the
  material and capacity it actually ran.
- **`docs/manual.md`'s *Your first set* names the new pair** and keeps the old one, since
  `drift_shell` + `soft_points` at 262144 is still in `examples/` and is still what
  `docs/contributing.md` §1 names.
