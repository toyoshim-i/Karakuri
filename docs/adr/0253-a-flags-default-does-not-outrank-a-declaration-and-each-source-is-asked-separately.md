---
id: 0253
title: A flag's default does not outrank a declaration, and each source is asked separately
status: accepted
date: 2026-09-05
supersedes: []
superseded_by: []
principles: [0091]
tags: [cli, ir]
---

# A flag's default does not outrank a declaration, and each source is asked separately

## Context

Written now because P-0049 — *A general default must
not beat a specific declaration* — is retiring into
[P-0091](../principles/0091-cost-is-known-before-it-is-paid.md), and only one of its two halves
had a record. [ADR-0090](0090-a-ceiling-calibrated-for-one-shape-rejects-the-next.md) carries the
ceiling half — a limit calibrated for one shape rejects the next — and P-0049's title, its opening
paragraph and the defect it was written from are about the other half, which is recorded nowhere.

**A `.kir` declares `capacity [min, max] = default` and the default was never used.** The range was
enforced. The declared default lost, every time, to `--capacity`'s own default of 262144 — because
the flag's value was read whether or not the flag had been typed. So a procedure written for 131072
ran at 262144 unless somebody who already knew said so on the command line, and an example whose
point is only visible at the count it was written for did not show its point. Nothing was printed,
because nothing was wrong from the program's side: the number it used was inside the declared range.

## Decision

**A general option's fallback is what to do when nobody said. A declaration *is* somebody saying, so
the fallback does not outrank it.**

`Args` carries `capacity_given` beside `capacity`, and `capacity_for` reads the flag only when it
was **typed**; otherwise the procedure's own declared default wins, and the flag's default is used
only where the procedure declared nothing at all.

**And the question is asked once per source rather than once per run.** `capacities_for` answers it
for each `.kir` in the Set, which is the only form `Set::build_many` accepts — it takes
`(procedure, capacity)` pairs precisely because each source declares its own range. The build used
to resolve `capacity_for(args, &l1[0])` and hand that one number to everybody, so a grid written for
512 × 256 samples and declared at 131072 drew 32768 of them because it had been loaded beside a
cube. **Nothing refused that either**, and for the same reason as above: the number came from a
declaration, so it was inside somebody's range — just not the range of the procedure it was applied
to. A recorded capacity in a Set file wins over both, because the file is where that geometry's
count was decided and a Set that comes back at a different size is a Set that was not saved.

## Alternatives rejected

**Keep reading the flag's value unconditionally and set its default to *unset*.** The same fix
spelled as a sentinel rather than as a second field. It loses on what a sentinel does to every other
reader: `capacity` is a `u32` that several call sites use as a number, and an in-band *nobody said*
is a value each of them has to remember to test for. `capacity_given` is a fact about the command
line, which is what the question actually is.

**Refuse when a flag and a declaration disagree.** Considered and wrong in the ordinary direction:
`--capacity` exists to override every source, which is what its help text says, and an operator
typing it is somebody saying. The defect was never the override — it was the override happening when
nothing was overridden.

**Let the declared range decide alone and drop the flag.** It removes the one way to run the same
procedure at a different count from the outside, which is the dial `capacity` is
([ADR-0009](0009-capacity-is-a-dial-not-part-of-a-procedures-identity.md)).

## Consequences

- **The same shape had already been caught once and was not generalised.** `--size` silently
  deciding the canvas is this defect in the other subsystem, fixed in
  [ADR-0077](0077-the-canvas-belongs-to-the-session-and-the-window-gets-no-vote.md); a run that
  quietly used a default is indistinguishable from one that honoured what was typed. The same
  reasoning is why `number_for` refuses an unparsable value rather than defaulting: `--frames 24O`
  rendered 240 frames and said nothing.
- **A `.kir` may change its declared capacity under a flag that was never typed**, and that is the
  intended behaviour rather than a hazard — the procedure is where the count belongs.
- **`Set::build_many`'s signature is what keeps this from regressing.** Passing one capacity to
  several sources is no longer expressible without writing the same number down repeatedly, which is
  visible at the call site.
