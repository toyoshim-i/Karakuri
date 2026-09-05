---
id: 0217
title: The counting allocator ships, because a written number nothing checks goes stale
status: accepted
date: 2026-08-29
supersedes: []
superseded_by: []
principles: [0088, 0093]
tags: [performance, architecture, ui]
---

# The counting allocator ships, because a written number nothing checks goes stale

## Context

[ADR-0214](0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md)'s move turned
`karakuri-console/examples/panel.rs` into `crates/karakuri`, the program a player launches. The
example had five stated purposes and four of them are answered by the binary merely existing. **The
fifth came across whole and is a measuring instrument inside the instrument**: to print what a
window costs, this binary installs a counting `#[global_allocator]` over the entire process, so
every allocation a player's run makes pays a thread-local increment.

The move deliberately did not decide whether that belongs in a program a player launches. It wrote
the question into the module header instead. This is the answer.

## Decision

**It ships, and the question is closed rather than left open.**

### The counter is not diagnostics — it is what holds a declared number honest

`crates/karakuri/src/main.rs` declares `WRITTEN_ALLOCS = 525`, `WRITTEN_KB = 694.3` and
`WRITTEN_ON = "2026-08-26"`, and the reading a run prints is held against them within a factor of
two. **Its own documentation records what happened the last time nothing was checking:**

> [ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s 184 allocations and
> 226.2 kB … stopped being true when the mixer bay landed — 456 there, and 525 once deck B was
> parked … **Nobody re-checked it for two commits, because nothing was checking it.**

That is the whole argument. A build with the counter compiled out is a build in which the same
sentence goes stale the same way, and the numbers it holds are not decoration:
`karakuri-console`'s `budget.rs` names this file five times as what keeps `PANEL_PASS`'s 1.26 ms
true, and `crates/karakuri-console/tests/schedulable.rs` asserts both of
[ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md)'s
schedulability conditions against that constant.

`docs/contributing.md` §4 is the
rule: *a guarantee is structural, or it is a convention that says so.* **ADR-0164 chose to budget
the panel rather than forbid it to allocate, and a budget is a guarantee only while something
counts.** Compile the counter out of the shipped build and the budget quietly becomes a convention,
in a document that does not say so.

### Half of what it collects is not a harness at all

`Costs` holds the per-frame timing as well as the allocation count, and `crates/karakuri/src/main.rs`
builds the transport row's `fps` and `frame_ms` out of it. **That half is a readout the panel draws**,
so the machinery is present in a player's run whatever is decided about the counter; what was in
question was only the `GlobalAlloc` wrapper over it.

## Alternatives rejected

**A cargo feature, default off.** The obvious engineering answer, and it loses on
[P-0088](../principles/0088-no-number-is-trusted-further-than-its-instrument-has-been-checked.md) — *a measurement carries how
it was taken*. A number taken in a configuration nobody ships is a number about a different program,
and this repository already keeps one such distinction painfully: every host-clock figure in it
says so beside itself because a reader who subtracts two figures taken differently gets a result
that looks like a finding. Worse, it puts the guard exactly where the recorded failure was — the
number would be checked only by whoever remembered to pass the flag, which is
`docs/contributing.md` §4's *invariants enforced by
review*.

**Move the harness to a test target.** A test can open a window, so this is not impossible. It loses
on what it could measure: the declared numbers are per-frame medians of a **running instrument with
a deck under load and a parked slot**, and the reading prints what the panel had in it while they
were taken, because that is what makes them comparable. A test would measure a fixture and the
sentence would then be about the fixture.

**Delete it and stop declaring the numbers.** Honest, and it gives up ADR-0164's position: the panel
would be back to being forbidden to allocate, or to nothing being said at all. Neither is what that
record decided.

## Consequences

- **A player's run pays a thread-local increment per allocation, and that cost is not measured
  here.** It is stated rather than estimated, which is
  `docs/contributing.md` §4's clause: the claim that
  it is negligible is not made. What can be said is that the instrument it sits inside is the only
  thing able to measure it, which is an argument for keeping it rather than a defence of the cost.
- **The counter is now load-bearing rather than incidental**, so removing it later means retiring
  `WRITTEN_ALLOCS`, `WRITTEN_KB` and the reading that quotes them together, and saying what replaces
  the guard on `PANEL_PASS`.
- **The module header stops asking the question**, and says this instead.
