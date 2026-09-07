---
id: 0270
title: The reference workload is a named Set rather than whatever the default pair is
status: accepted
date: 2026-09-07
supersedes: []
superseded_by: []
principles: [0095]
tags: [docs, performance, measurement, engine, panel, examples]
---

# The reference workload is a named Set rather than whatever the default pair is

## Context

`docs/contributing.md` §1 has carried one rule about measurement since 2026-08-24: **one
reference workload, 262144 elements at 1280x720**, and every host-clock figure in the repository
is taken there. The bullet said what the workload *was* in the same sentence: *"it is the `.kir`
default capacity and the default canvas, and it earns its place by being what everything else was
measured at."*

**By the rule's own words that is not a requirement of anything.** It is not an application
requirement — nothing has to run at 262144 elements, and the bullet says so twice, *neither a
target nor a limit*. It is not a design requirement — no component is built to that number. What
it is, and all it is, is a **comparability convention**: the reason a figure written in July and a
figure written in September can be subtracted at all. The failure it prevents is silent, which is
why it is worth a rule: a reader puts two numbers side by side that were never about the same
thing and gets a result that looks like a finding.

**A convention that moves whenever the demo moves is not a convention.** As written, the workload
was whatever the shipped default pair happened to be, and that coupling was enforced in code.
`crates/karakuri/src/main.rs`'s `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none`
reads the shipped pair off the disk and asserts its L1 declares 262144, failing with *"the pair
this program plays by default is no longer the workspace's reference workload, and the reading
printed after three quiet seconds says it is"*. `Sources::under`, which is the two names a run
with no paths opens on, documents them as *"what the reference workload is measured on"* and
points at that test. So changing the default pair does not merely change a demo; it turns a test
red and, until the prose is rewritten, makes the panel's startup legend lie.

**The anchor was never chosen for what it measures.** 262144 is `examples/drift_shell.kir`'s
declared default, and it has been that since the file was added on 2026-07-26 in *Drive the whole
pipeline from the CLI, and add a pair to drive it with* — one canvas, no panel, no deck; the panel
binary arrived on 2026-08-29, over a month later. Nobody picked that number as the workload to
compare against. It became the workload because it was the number that happened to be there when
somebody first needed two figures to be comparable.

**It is an accident twice over.** 262144 is also `karakuri_ir::DEFAULT_CAPACITY`, the language's
default for a `.kir` that declares no capacity. So *the default pair's capacity* and *the language
default* are the same number, and a reader cannot tell from the number which of the two a figure
was anchored to. The pinning test already knows this: it checks a second file,
`examples/strand_shell.kir`, which declares 131072 and says why in the file (512 strands x 256
samples), specifically so the test can tell a real per-file read from a constant that ignored the
file — *"the second procedure declares the language default, so this test can no longer tell a
per-file read from a constant — pick another `.kir`"*.

**The coupling has already stopped a change.** `A demo that reads at ten thousand rather than at a
quarter of a million` (3307c0e, 2026-09-07) added `examples/star_vortex.kset`, argued that 262144
alpha-blended sprites *"was tuned when the panel drew one canvas"* and reads as fog, and then did
not make it the default — closing with *"the default pair is also what `docs/contributing.md`
names as the one reference workload, and splitting those two wants a record of its own."* This is
that record. The rule was written to protect a measurement and it is now holding a demo in place.

**A figure taken at the reference workload is no longer one kind of thing.** §1 named a capacity
and a canvas and nothing else, and that was enough while the only thing measuring was one Set.
It is not now. The panel is a deck of four slots; every slot is stepped and drawn on every frame
([ADR-0269](0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)), each
into a cell of its own, with a present pass per cell and an `egui` pass over the lot. ADR-0269's
own table is the size of the gap, headless and on one machine: a bare Set is 9.9–10.6 ms and a
deck of four is 21.0–22.4 ms, at the identical nominal workload. Before that record the same
comparison read 207.5 ms, because three unstepped slots each drew their whole capacity at one clip
position and the raster back end serialises order-dependent blending at one address. **The
pathology is gone and the distinction is not**: a panel figure is four simulations and a panel, a
headless figure is one Set, and §1 gave a reader no way to know which one they were holding.

## Decision

**The reference workload is `examples/drift_cloud.kset` — `drift_shell.kir` at the 262144 elements
it declares, with `soft_points.kir` — rendered at 1280x720.** It is named, not derived. The Set
file is the name: it is exactly the pair the figures were taken on, written in the repository's own
vocabulary for a pair.

**The panel's default pair is free to change without moving it.** What a bare `cargo run -p
karakuri` opens on is a demo decision — what is worth looking at — and it stops being a
measurement decision. The two questions were one question by accident and are now two.

**Nothing already measured is re-taken.** Every host-clock figure written in this repository stays
valid, because the thing they were measured on has not moved: `drift_shell.kir` + `soft_points.kir`
at 262144 and 1280x720 is what it was. Only the *definition* moved, from *the default* to *this
named Set*. A re-measurement out of caution would cost days and answer nothing.

**A figure says whether it is a panel figure or a headless one.** That is not a new rule — §1
already required a number taken anywhere else to say so beside itself — it is the naming of a case
that was being missed because it did not look like "anywhere else". Same capacity, same canvas,
different measurement.

## Alternatives rejected

**Leave it coupled, and let the default pair define the workload.** The status quo. It costs
nothing to keep and it is enforced by a test that works, which is the strongest thing in its
favour. It loses because it makes the default un-improvable: the shipped demo is 262144
alpha-blended sprites tuned for a program that drew one canvas, everyone who has looked at it
recently agrees it reads as fog, and the only thing standing between it and a better default is a
convention about arithmetic. That is a measurement rule deciding what the instrument looks like on
first run, which is not a thing it should be able to decide. It also loses on its own terms: a
convention whose value is that it does not move is stored in the one file most likely to move.

**Re-measure everything against a new anchor.** Pick the workload deliberately — a lighter pair, a
cheaper capacity — and re-take every figure at it. It is the honest version of the change and it
would leave a better anchor behind. It loses on arithmetic: the figures span from 2026-07-25 to
now, they are spread across `docs/`, `crates/`, ADRs and commit messages, several were taken on
machines nobody has any more, and some of them cost a windowed run with three quiet seconds and a
device. Nobody will do it, and a half-done re-measurement is worse than none — it produces exactly
the two-figures-about-different-things subtraction the convention exists to prevent, with both
figures looking current. **This decision is chosen partly because it is free**: no figure changes,
so nothing can be half-done.

**Drop the convention.** Let each figure carry its own workload and let readers compare what they
like. It is the option that admits the convention was never a requirement. It loses to the failure
§1 names, which is why the bullet exists: the failure is *silent*. Two numbers with no shared
anchor still subtract, and the difference still looks like a finding. Requiring each figure to
carry its conditions does not fix that, because a reader who does not look for the conditions is
the reader the rule is for.

**Name the pair rather than a Set — `drift_shell.kir` + `soft_points.kir` at 262144.** The
closest loser, and a real option: it is what the panel actually loads, since `Sources::under`
hands the run two paths and not a `.kset`. It loses because two paths and a capacity is three
things a reader has to keep together, and this repository already has one thing that means exactly
that triple — a Set file. `examples/drift_cloud.kset` is that pair and nothing else, and naming it
means a reader can open one file and see the whole workload. That the panel gets there by another
route is the panel's business.

## Consequences

- **`docs/contributing.md` §1's reference-workload bullet is rewritten** and now names the Set, the
  capacity, the canvas and the date the definition changed. It keeps the comparability argument,
  *neither a target nor a limit*, *a number taken anywhere else says so beside itself*, the
  `PROBE_RESOLUTION` reason and the pointer to
  [P-0095](../principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md);
  it adds the panel-versus-headless distinction and the sentence that no figure is invalidated.
- **`examples/drift_cloud.kset` is now load-bearing** in a way a `.kset` in `examples/` was not
  before: it is cited by name from `docs/contributing.md` §1. It is `drift_shell.kir` +
  `soft_points.kir` and its identity is the pair rather than the file name, so renaming or
  repointing it is a change to the convention and needs a record.
- **The panel's pinning test is now asserting the wrong thing, and it is not corrected here.**
  `the_capacity_is_the_l1s_own_declaration_and_the_l4_declares_none` pins the *shipped default
  pair* to 262144 with a message that says the reference workload moved. Under this decision the
  default pair moving is not the reference workload moving. What the test should pin is the Set the
  convention names; what it should still assert about the default pair is that it is read per file
  rather than from a constant, which is the defect the `strand_shell.kir` half already catches.
  `crates/karakuri/src/main.rs` is being edited in another session and the change is reported
  rather than made.
- **The panel's startup legend claims the coupling out loud**, and that sentence is now wrong in
  two ways rather than one. It says the run *"is the workspace's reference workload
  (docs/contributing.md §1) only while nobody passed a pair on the command line"* — which was the
  coupling — and, a few lines above, that *"an allocated slot neither steps nor draws, so none of
  them is in these numbers"*, which ADR-0269 made false. Reported rather than corrected, for the
  same reason.
- **`Sources::under`'s documentation is stale.** It says of the default pair *"these two are what
  the reference workload is measured on"*. They are the pair the reference workload is measured on
  because the workload names them, not the other way round, and if the default pair changes the
  sentence is simply wrong. Reported rather than corrected.
- **`karakuri-engine/tests/deck.rs`'s cost benchmark does not use the reference workload's
  material**, which is worth writing down because it is the confusion this record is about.
  `the_cost_of_a_slot_and_of_the_composite_are_measured_and_reported` builds its own `static_shell`
  L1 fixture at `CAP = 262_144`; its neighbour
  `the_cost_of_filling_a_preview_cell_is_measured_and_reported` `include_str!`s
  `examples/drift_shell.kir` and says why — *"the panel's own material, rather than this file's
  fixtures"*. ADR-0269's table describes the first as *"the reference workload —
  `examples/drift_shell.kir` + `soft_points.kir` at capacity 262144"*, which is the second test's
  material attributed to the first. That is what a workload defined as a capacity number rather
  than as a Set makes easy to do, and it is a description to correct rather than a measurement to
  discard: the figures are real and the material is a shell of points either way. Named, not fixed.
- **What is still not decided is what the default pair should become.** This record only frees it.
  `examples/star_vortex.kset` is a candidate with an argument already written (3307c0e) and
  `examples/light_cloud.kset` is another (eed3580); choosing one is a demo decision and belongs to
  whoever makes it.
