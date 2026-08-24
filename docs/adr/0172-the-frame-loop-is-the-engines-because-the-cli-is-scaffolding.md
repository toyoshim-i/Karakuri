---
id: 0172
title: The frame loop is the engine's, because the CLI is scaffolding
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [engine, architecture]
---

# The frame loop is the engine's, because the CLI is scaffolding

## Context

`frame::compose` was written in `karakuri-cli`, and its own module documentation says why it
exists: *"there were two frame loops and the seam between them had become five, and every replay
defect this project has found was one of the four extras"* — the look applied once instead of per
frame, the governor running on one path only, the present pass skipped on unkept frames. Making the
seam one line is what stopped them drifting.

`README.md` says the other half: **the CLI is scaffolding rather than the destination**, and the end
state is a GUI application. That application is `karakuri-console`, and `karakuri-cli` has **no
library target** (`docs/contributing.md` §3), so the console could not call `compose` at all. Its
`examples/panel.rs` hand-rolled a second frame loop instead — `deck.begin_frame`, a render, a
present pass, an `egui` pass — which is precisely the failure `frame.rs` was written to end,
reappearing one crate over and out of reach of the thing that ended it.

## Decision

**`Sink`, `Skip`, `Committed`, `Outcome`, `compose`, `Look` and `WindowSink` move to
`karakuri-engine`.** `PngSink` stays in `karakuri-cli`, and so do the clock, the recorder, the
watcher and the MCP server: those belong to a *program*, and a frame is a deck, a `Present` and
somewhere to put the result — all three of which are the engine's.

`Look` moves with them because its three fields are exactly `Present::set_tonemap`'s arguments. It
is an engine concept that happened to live in the CLI, and a look is what a frame is drawn under,
with every sink drawn under the same one.

**Nothing gains a dependency**, which is what makes this a move rather than a redesign:
`karakuri-engine` already depended on `wgpu` and `winit` and already opened surfaces in `gpu.rs`,
and `karakuri-console`'s example already depended on `karakuri-engine`. The workspace test count did
not change either.

**Rejected: keep the loop in `karakuri-cli` and give that crate a library target.** It works, and it
loses on what it says: the application would then depend on the scaffolding, and every later
question about where something belongs would be answered by whichever crate happened to have it
first. The README's sentence is a design statement, not a caveat.

**Rejected: leave it and let the console keep its own loop.** That is the status quo, and it is the
exact shape of the defect this module exists to prevent — two loops, a seam nobody is measuring,
and a divergence that is invisible until someone reads them side by side. It is worse here than it
was inside the CLI, because a crate boundary makes reading them side by side less likely rather than
more.

**The objection worth recording: does the engine now know too much?** A frame loop that owns a
surface is closer to an application than a renderer usually gets. The answer is that it already did
— `Gpu` opens surfaces and picks a surface-compatible adapter, and `winit` has been a dependency
since before this move. What the engine still does not know is what a *program* is: no clock, no
records, no files, no arguments.

## Consequences

- A consumer writes `use karakuri_engine::frame;` and calls `frame::compose`, or takes the root
  re-export. There is no `mod frame` in `karakuri-cli`: one name for one thing.
- `Look::name()` was deleted rather than moved. It wrapped `op_name`, which is the CLI's tone-map
  spelling table and stayed there, and all five call sites were inside `main.rs`.
- **`#[cfg_attr(test, derive(Debug))]` does not survive a crate boundary** and `Look` carried it:
  `cfg(test)` is set when the *engine's* tests compile, not when a consumer's do, so the CLI's
  `assert_eq!` on a decoded `Look` would have stopped compiling. It is an unconditional `Debug`
  now, as every other public engine type has. Two more types in `karakuri-cli` carry the same idiom
  and will hit the same wall the day either moves.
- **Three `Clock` tests came out.** They were filed under `frame` because a refused frame did not
  read the clock; [ADR-0171](0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md)
  deleted that rule and the tests stayed put. They are beside `Clock` now.
- Two counts in the documentation were wrong before this and are corrected rather than carried:
  `docs/contributing.md` said eleven tests in the CLI binary build a Set when it was fifteen, and
  `docs/architecture.md` said the workspace has **8** crates when it has **10** — it did not know
  `karakuri-console` or `karakuri-layout` existed at all, which is a poor state for the document
  that has to say where the frame loop lives.
