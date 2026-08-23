# Karakuri Contributing & Development Guide

This document outlines the engineering principles, workflow guidelines, build/test commands, and verification practices for **Karakuri**. Both human contributors and AI coding agents MUST adhere to these rules when submitting changes.

---

## 1. Project Invariants & Core Rules

Every change MUST respect the standing rules in [docs/principles/](principles/) — **one file each**,
so a rule is stated once and cannot drift between documents. `ls docs/principles/` is the index,
because each filename is the rule it states.

They were previously copied here and into [architecture.md](architecture.md), and the two copies had
already stopped agreeing on which four were foundational, which is what moved them.

Start with these, and read the rest before changing anything they touch:

- [Nothing allocates or compiles a shader on the render thread](principles/0001-nothing-allocates-or-compiles-a-shader-on-the-render-thread.md)
- [Simulation time comes from a record, never from a clock](principles/0002-simulation-time-comes-from-a-record-never-from-a-clock.md)
- [Compaction preserves order](principles/0003-compaction-preserves-order.md)
- [A live Set is never mutated in place](principles/0004-a-live-set-is-never-mutated-in-place.md)
- [Nothing checks clean and comes up short at runtime](principles/0024-nothing-checks-clean-and-comes-up-short-at-runtime.md)
- [A test meant to catch something is run against the defect](principles/0025-a-test-meant-to-catch-something-is-run-against-the-defect.md)
- [Only reviewed work enters history](principles/0017-only-reviewed-work-enters-history.md)
- [Run what the question needs, when it is asked](principles/0057-run-what-the-question-needs-when-it-is-asked.md)

**Why** each is the way it is, and what was rejected on the way, is in [docs/adr/](adr/). A rule that
stops being true is deleted and re-recorded under a new number rather than edited — see
[ADR-0000](adr/0000-record-decisions-here-and-standing-rules-in-principles.md).

---

## 2. Environment Setup & Development Tools

### Required Prerequisites
- **Rust**: Stable Rust (`1.84` or higher) with `cargo`.
- **GPU Driver & WebGPU Environment**: `wgpu` compatible Vulkan, Metal, or DX12 backend.

### Git Hooks
Karakuri provides pre-commit and pre-push hooks under `.githooks/`. Enable them in your local repository:

```sh
git config core.hooksPath .githooks
```

- **`pre-commit`**: Runs `cargo fmt --check` on the **staged content**, and nothing else. It
  reads what is being committed rather than the working tree, so a half-finished edit on
  disk neither blocks a good commit nor hides a bad one.
- **`pre-push`**: Runs `cargo fmt --check`, `cargo clippy` and the whole suite **on a tag
  push, and on nothing else**. A tag is the deploy; a branch push is part of working —
  backing up, moving between machines, opening something for review — and a gate there asks
  whether the work is good at a moment nobody was claiming it was.

Neither hook runs tests on an ordinary commit or branch push, and that is a decision rather
than an omission: this workspace has nearly a thousand tests, and while only about three
hundred of them want a GPU those three hundred are ~99% of the five minutes the suite costs,
so any fixed subset spends minutes answering a question nobody asked. What replaces it is
deliberate: whoever makes a change names the smallest suite that answers it and runs that
(§3 lists them per crate), and the whole workspace runs at a boundary — before a tag, after
a refactor, and before a change is called done (§5). The reasoning is written into the hooks
themselves.

---

## 3. Build, Lint, and Test Commands

**Name the smallest suite that answers your question and run that** — see §2 for why, and
[P-0057](principles/0057-run-what-the-question-needs-when-it-is-asked.md) for the rule. The whole
workspace belongs at a boundary, not at every step.

### Testing one crate
```sh
cargo test -p karakuri-ir          # IR: lexer, parser, checker, cost
cargo test -p karakuri-codegen     # WGSL generation and naga validation
cargo test -p karakuri-engine      # render graph, Set lifecycle, deck (needs a GPU)
cargo test -p karakuri-audio       # analysis, tempo tracking, beat lock
cargo test -p karakuri-signal      # oscillator, synthesized bus, noise
cargo test -p karakuri-store       # records, ndjson, content addressing
cargo test -p karakuri-midi        # wire parsing and the map
cargo test -p karakuri-cli         # flags, replay, MCP, live save (needs a GPU)
```

**Two crates take a device, not one.** Most of `karakuri-engine`'s integration suites do,
and so does part of `karakuri-cli`: eleven tests in the binary build a Set, and the five in
`tests/replay.rs` drive `karakuri-cli` as a subprocess, which takes a device of its own. The
other six crates are pure CPU.

### Running only the part that needs no GPU

**Every test that reaches a device lives under a module called `gpu`**, so the whole CPU-only
set is one filter away:

```sh
cargo test --workspace -- --skip gpu::          # everything that needs no device
cargo test -p karakuri-engine -- --skip gpu::   # the render graph's own arithmetic
cargo test -p karakuri-engine --lib -- --skip gpu::
cargo test -p karakuri-cli --bins -- --skip gpu::   # `karakuri-cli` has no library target
```

The GPU tests are most of the time the whole workspace costs and a minority of its tests —
288 of 1010 as of 2026-08-23, about 4 s against 61 s. So this is nearly all of the suite for
a fraction of the wall clock. (The first run after a clean rebuild is slower while the freshly
written binaries page in; it settles after that.) `--skip` is a substring match on the full
test path, which is why the module is named `gpu` and nothing else is.

**Counts and timings here carry the date they were true**, and are meant to be read as an
order of magnitude rather than as a current figure — the suite only grows. A number with no
date reads as current forever, which is how this section came to say 997 in one paragraph and
301 in another after a few days of ordinary work.

**Do not turn incremental compilation back on** without reading
[ADR-0144](adr/0144-the-test-suites-largest-cost-was-a-directory-listing.md). It leaves
~865 object files per rebuild in `target/debug/deps/`, and macOS makes every test binary
run from that directory pay a full listing of it before it can reach a GPU. That one fact
was worth more than every other change to this suite put together: the whole workspace went
from over 370 s to about 60 s (2026-08-23). If the suite ever starts creeping up again, count
the files there
first — `ls target/debug/deps | wc -l` — before looking at any test.

The convention and its enforcement are argued in
[ADR-0140](adr/0140-a-gpu-test-lives-under-mod-gpu-and-the-rule-is-enforced-both-ways.md),
and the no-skip rule below in
[ADR-0141](adr/0141-a-gpu-test-with-no-adapter-fails-rather-than-skipping.md), which also
records what would reverse it.

`cargo test -p <crate>` keeps its exact meaning — everything runs, and the pre-push hook is
untouched. The filter only ever subtracts. `#[ignore]` would have inverted that default, so
`cargo test` would have quietly stopped meaning "everything"; that is why this is a module
path and not an attribute.

The convention is enforced by
[`crates/karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`](../crates/karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs),
which reads the workspace's own source and fails both ways round: a GPU test outside `mod
gpu` would be run by the filtered command, and a CPU test inside one would silently stop
running whenever anyone filtered. It counts spawning a binary that reaches `Gpu::headless`
as reaching one, which is the only way `tests/replay.rs` can be seen at all.

A test that needs a device is only *untestable* to the extent that the line needing it is —
where one line needs the device and the rest is a decision, the decision comes out
([ADR-0130](adr/0130-a-wrapper-that-needs-a-gpu-does-not-excuse-the-decision-inside-it.md)).

**There is no in-test skip, in either direction.** A test that needs a device and cannot get
one panics; every one of them does. Eight used to print a message and return instead, and one
of those returned in silence; they were changed when this filter landed, because on a machine
with no adapter `--skip gpu::` is a better answer than a green run that measured nothing. If
you are on such a machine, the filtered command is the suite you have, and it says as much.

### At a boundary — before a tag, after a refactor, before calling a change done
```sh
cargo test --workspace
```

### When the work is split across several hands

Several agents or sessions working at once multiply whatever each of them runs: a suite that
costs a minute costs it *per worker*, mostly to re-answer a question somebody else already
answered. The rule in §2 does not change, but who applies it does.

- **The side directing the work runs the tests.** A worker says what it changed and which test
  would catch a regression in it; proving that by running the workspace suite is the expensive
  way to say the same thing.
- **After a fix, run the test that failed — first, and on its own.** `cargo test -p <crate>
  <name>` runs one. Broaden only once it passes; a green suite is a slower way to learn the
  same fact, and a red one tells you less.
- **Running a test against its injected defect
  ([P-0025](principles/0025-a-test-meant-to-catch-something-is-run-against-the-defect.md)) is
  one test's evidence.** Run that test with the defect in place, not the suite around it.
- **`cargo check -p <crate>` answers "does this compile"** without building or running a test,
  which is often the whole question.

### Running Workspace Linter (Clippy)
```sh
cargo clippy --workspace --all-targets -- -D warnings
```

### Checking Code Formatting
```sh
cargo fmt --check
```

---

## 4. Recording a Decision

**When a decision is made, write it down then — not later.** These records exist because they were
once reconstructed from four weeks of session transcripts, and reconstruction only works while the
transcripts still exist and someone remembers to look.

**Write an ADR when a plausible alternative lost.** Not for every change — for a choice someone could
reasonably re-propose in three months. The record's job is the *rejected* alternative and why it lost;
the conclusion alone does not stop anyone re-proposing it. Number it after the highest existing one,
add a row to [docs/adr/INDEX.md](adr/INDEX.md), and follow the shape in
[ADR-0000](adr/0000-record-decisions-here-and-standing-rules-in-principles.md).

**Add a principle only if a future proposal could violate it.** *An element is never identified by its
buffer slot* is a rule somebody will otherwise re-propose; *rekordbox never publishes its deck BPM* is
a fact about the world and belongs in the ADR alone. Every principle has an ADR; not every ADR yields
a principle.

**When a rule stops being true, delete its file and re-record it under a new number** — never edit it
into something else, and never reuse the retired number. Record the retirement in `INDEX.md` and
re-point the ADRs that cited it
([ADR-0059](adr/0059-a-records-pointer-into-the-principles-registry-is-metadata.md)).

**An ADR is not edited after it lands**, except to set `status` and `superseded_by`. If the reasoning
would have to change, that is a new record.

**Hook it from where the work is, or nobody will find it.** `INDEX.md` makes a record
*findable*; it does not make anyone *look*. A record that changes what is planned or what is
still owed gets a pointer from [roadmap.md](roadmap.md), beside the item it changes — including
the sentence naming what the decision leaves undone, because a consequence recorded only in
the ADR is a consequence the next person meets rather than reads. A record that changes how
the thing is used gets one from [manual.md](manual.md) or [ir-spec.md](ir-spec.md); a record
that settles a standing rule gets a principle, which is the working set people actually read.

This is not decoration. Nineteen records were written on 2026-08-22 and 2026-08-23 and
fourteen of them were reachable from nothing outside `docs/adr/` — the reasoning was all
there, and the plan did not know any of it had happened.

---

## 5. Verification Checklist for Code Changes

Before marking a task or pull request as complete, ensure the following checklist is satisfied:

- [ ] **A decision with a losing alternative has a record**: see §4. If nothing was decided, nothing is owed.
- [ ] **All workspace tests pass**: `cargo test --workspace` returns 0 exit code.
- [ ] **Naga validation passes**: Any modifications to `karakuri-codegen` MUST be verified against `naga_test.rs` to guarantee generated WGSL text parses and validates cleanly.
- [ ] **Documentation integrity**: Existing comments, docstrings (`//!` and `///`), and Markdown documentation are updated accordingly.
- [ ] **No unhandled errors or silent fallbacks**: Core logic should produce explicit error types (`IrError`, `SetError`, `GpuError`, etc.) rather than swallowing exceptions.
- [ ] **Relative links in documentation**: Markdown links to repository files MUST use relative paths (e.g. `../crates/karakuri-ir` or `architecture.md`).

---

## 6. Related Architecture & Specification Reference

- [architecture.md](architecture.md): Source code structure, multi-crate map, pipeline, and threading model
- [ir-spec.md](ir-spec.md): `.kir` DSL specification and language invariants
- [manual.md](manual.md): CLI arguments and VJ keyboard controls reference
- [plugins.md](plugins.md): Out-of-process helper plugin specification
- [roadmap.md](roadmap.md): Architectural roadmap and future milestones
- [adr/](adr/): Every decision, with the alternatives that lost — append-only
- [principles/](principles/): The rules in force, one per file — current only

