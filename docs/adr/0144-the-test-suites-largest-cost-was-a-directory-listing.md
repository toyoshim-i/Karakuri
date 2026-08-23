---
id: 0144
title: The test suite's largest cost was a directory listing
status: accepted
date: 2026-08-23
supersedes: [0143]
superseded_by: []
principles: []
tags: [testing, workflow, macos]
---

# The test suite's largest cost was a directory listing

## Context

`cargo test -p karakuri-engine` took 373 s. Every explanation offered for it was wrong, and
each wrong one was expensive:

- **A shared device.** One device per target instead of one per test made the serial run
  *slower* (48.7 s → 53.3 s) and broke a test in parallel.
- **cargo-nextest.** Parallelising across binaries made a crate's run 53% slower
  ([ADR-0143](0143-cargo-nextest-is-slower-here-and-was-measured-not-argued.md), whose
  *conclusion* was right and whose *reason* was wrong — this record supersedes it).
- **Pipeline construction, per test.** Stated as measured in a brief; it was inferred from
  the shared-device result and never measured.

What settled it was measuring the phases rather than reasoning about them. Every one of
them was fast: instance 0.000 s, adapter+device 0.026 s, compile 0.001 s, `Set::build`
0.011 s, a frame 0.003 s. The whole of a heavy test's 32-frame loop, replicated outside a
test harness, ran in 0.574 s against the 6.9 s the test took.

## The cause

`Gpu::headless()` → `request_adapter` → `MTLCreateSystemDefaultDevice` →
`MTLDeviceArrayInitialize` → `IOSurfaceClientCopyGPUPolicies` → `+[NSBundle mainBundle]` →
`_CFBundleGetBundleVersionForURL` → `_CFIterateDirectory` → `readdir`.

For an executable that is not in a bundle, CoreFoundation decides which bundle layout it
sits in by **iterating the executable's whole directory**, looking for `Contents`,
`Resources` or `Support Files`. Its published source never early-exits: the block returns
`true` for every entry.

`cargo test` runs every test binary out of `target/debug/deps/`. Incremental compilation
writes ~865 `*.rcgu.o` files there per rebuild under fresh hashes and removes none, so the
directory grows without bound — 622,832 object files after a month.

Measured with a 15-line Objective-C program, one Metal call, no Rust: **0.036 s** beside 1
file, 0.059 s beside 10k, 1.178 s beside 100k, **10.611 s** beside 626,948. The same binary,
the same arguments; only the neighbouring files differ. `cwd` is irrelevant — the
executable's own directory decides it.

Thirty GPU-touching test binaries each paid this once. It was larger than every test in the
suite put together.

## Decision

`incremental = false` in `[profile.dev]`. A rebuild costs about 8% more (18.8 s → 20.2 s for
`karakuri-engine --tests`) and the directory stops growing: 865 new object files per
rebuild becomes 0 in the steady state.

Reported to Apple with the reproducer.

## Alternatives

**Clean periodically.** Treats the symptom on a treadmill: at 865 files a rebuild, a hundred
rebuilds puts it back over 100k. Still required *once*, to drain what had accumulated.

**Run test binaries from elsewhere** — `cargo test --no-run`, copy, execute. Permanent
rather than a treadmill, and it works (0.03 s), but it replaces `cargo test` with a script
and every contributor has to know.

## What this record is really for

Four hypotheses about this cost were stated with confidence and every one was refuted by a
measurement that took minutes. Two of them were acted on first. The instinct that a fixed
cost must be device creation, or that serial binaries must be the queueing, was wrong every
time — and the last of them, "dyld is excluded because the same libraries load", was not
even the right *kind* of argument: identical outcomes are not evidence of identical work.

Measure the phases. It cost less than any of the wrong answers did.
