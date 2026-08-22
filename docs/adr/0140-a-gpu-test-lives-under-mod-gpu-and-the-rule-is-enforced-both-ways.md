---
id: 0140
title: A GPU test lives under `mod gpu`, and the rule is enforced both ways
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [testing, workflow]
---

# A GPU test lives under `mod gpu`, and the rule is enforced both ways

## Context

997 tests, 301 of which acquire a device. Measured: the full suite is ~306 seconds and the
301 are ~99% of it. There was no way to ask for the rest — `cargo test -p karakuri-engine
--lib` was offered as one and was wrong, running six GPU tests, one of which hard-fails
without an adapter.

The per-test cost is **pipeline construction, not device creation**: one test alone takes
3.57s, thirteen serially take 48.7s, and a shared device made the serial run *slower*
(53.3s) while breaking a test in parallel. So there was no fixing this from the inside; the
answer had to be a way to not run them.

## Decision

Every test that reaches a device lives under `mod gpu { … }` in its target.
`cargo test -p <crate>` keeps its exact present meaning — everything runs — and
`-- --skip gpu::` is the subtraction. **696 tests in 9.3 seconds against 306.**

**A source-scanning test enforces it**, in the shape `no_clock_access.rs` already
established here: it blanks comments and string literals, walks brace depth for module and
function paths, and judges each `#[test]` by its body plus every same-file function it
calls, transitively. Three separate anti-vacuity floors, because a wrong root, a broken
blanker and a renamed `Gpu::headless` are three different ways to scan nothing.

**Enforced in both directions.** The obvious rule is "a GPU test must be marked". The
converse matters as much and is easier to miss: `--skip` is a substring match, so a CPU test
inside any module named `gpu` stops running the moment anyone filters — and
`karakuri_engine::gpu` is a real module whose first test would be swept up silently.

**A subprocess spawn counts as a device reach.** `karakuri-cli/tests/replay.rs` spawns
`CARGO_BIN_EXE_karakuri-cli`, which acquires a device inside its own process, invisible to
any in-process rule. The scanner treats the whole `env!("CARGO_BIN_EXE_…")` expression as a
reach — deliberately coarse, since no static rule can read the flags that decide whether
that run builds a Set. Coarse in this direction only ever keeps a CPU test out of the fast
set; it can never let a GPU test in. A second test re-checks that the named binary still
reaches `Gpu::headless`, so the entry cannot go stale and reopen the hole quietly.

## Alternatives

**`#[ignore]` plus `--include-ignored`.** The most native option — the repo already uses
`#[ignore]` for a test needing a microphone. Rejected because it **inverts the default**:
`cargo test -p karakuri-engine` would silently become the CPU run, and `pre-push`'s
`cargo test --workspace` would stop exercising 301 tests at the one moment the project has
decided everything must run. It would also collide with the two `#[ignore]`s already there
for a different reason.

**A cargo feature.** Rejected on the fingerprint: toggling one compiles a second copy of the
workspace, so "run the fast tests" would begin with a multi-minute rebuild — inverting the
point. No crate here defines `[features]` at all.

**Splitting the 25 all-GPU targets into separate files.** Would have avoided ~17k lines of
reindentation and preserved blame. Rejected because the four mixed targets need an in-file
module regardless, and one convention beats two.

**A shared device.** Measured and rejected; see Context.

## Consequences

`examples/` is outside the convention: two `Gpu::headless` sites live there and hold no
tests, so the scanner does not walk them.
