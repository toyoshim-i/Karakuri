---
id: 0324
title: An output is a named destination with a size and an on/off
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0086, 0090, 0094]
tags: [ui, operations, engine, outputs, m5]
---

# An output is a named destination with a size and an on/off

## Context

`Operation::RouteFrame` carried `output: Undecided`, and the marker's own definition says what that
means: *this operation exists and what it acts on is an open question*. It was the oldest of the
nine, and three records had already put constraints on the answer without taking it.

- **The picture is in the set that needs naming and a preview cell is not**
  ([ADR-0243](0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md)). That record
  also named the shape whoever answered would have to reconcile: the picture's on and off is
  `Layout::visible` on a layout node, *"a sink drawn as a layout node, which is the right control and
  the wrong model."*
- **An output holds the size it is rendered at**, set by the operator or by the destination window
  ([ADR-0246](0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)).
- **One render, scaled into each output**, at the largest enabled one
  ([ADR-0247](0247-one-frame-is-rendered-and-scaled-into-each-output.md)).

The fan-out was built and had nothing to put in it. `frame::compose` takes `&mut [&mut dyn Sink]`
and ADR-0171 closes with *"what is still owed is a second sink to put in the slice."* The console's
Outputs row drew one chip of the mock's four and hit-tested the whole capsule. The vocabulary could
not name a destination, so the only route into the row was spelled as a fold's target.

## Decision

**An output is a destination, a size, and whether it is on.** Each of the three is answered
somewhere different, and that is the decision rather than a shortcoming.

### 1. The destination is a word from a closed list

`karakuri_operation::Output` is `Program`, `Projector(u8)` and `Plugin(u8)`, and
`Operation::RouteFrame { output: Output, on: bool }` names one of them.

**Not a string.** The console's projector chip carries the display it is on — `projector · DELL
U2720Q` in the mock — and that label changes when the cable does, when the display is renamed, and
when the window is dragged to the other screen. A name a MIDI map or an MCP call held would then
name nothing, and there would be nothing to refuse it with: a string parses whatever it is given, so
*there is no such output* would be a sentence somebody had to remember to write at every route in,
where a variant is a compile error at the one that mistyped it. It is also
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md)'s standing practice —
`karakuri-operation` *"owns the enumerations a destination is drawn from rather than passing
strings"* — and this crate has no `[dependencies]`, so a `String` would be the one allocating payload
in it.

**`on` and not a toggle**, for the same principle stated one clause along: a surface that can only
switch has no way to arrive, and two surfaces switching one sink disagree about where they are. The
chip's toggle is the console's affordance over the two states.

**`Plugin(u8)` exists and nothing constructs one.** A plugin sink is named by its place in the
manifest that loaded it; `docs/plugins.md` specifies the process and the handshake and nothing
implements them, so the console draws Syphon and NDI as `no plugin`. The variant is in the type so
that the day a manifest is read the *list* grows rather than the vocabulary changing.

### 2. The on/off is read where it is kept, and the operation names it anyway

**The picture's on and off is still `Layout::visible` on the picture's node**, and
`RouteFrame { output: Program, on }` is what a press *asks for*. The console performs it by folding
that node. There is one stored answer to *is the picture on* and the operation names it rather than
duplicating it — [ADR-0161](0161-solo-remembers-which-region-because-it-cannot-be-derived.md)'s rule
read on a sink.

**This is ADR-0243's reconciliation, and it is not what that record predicted.** It said naming an
output means *"a sink and a layout node stop being one thing read two ways."* They do not stop; what
stops is the *vocabulary* being one of the two ways. A press asked for `Op::Fold` by name, so the one
thing an operator did to the picture was spelled as an act on the arrangement. It is spelled as an
act on an output now, and the fold is how this surface carries it out — which is the same relation
every fader on this panel already has to the record it writes.

**A projector's on/off is the window existing.** `RouteFrame { Projector(0), on: true }` opens a
window and `on: false` drops it, which drops the surface and the last reference to the window on one
statement.

### 3. The size is the destination's own

The program view's size is the rectangle the Program bay gives the picture; a projector's is the
window the operating system gave it. Neither is stored as a setting anywhere: both are read off the
thing that decides them, once a frame. What the frame is then composited at is
[ADR-0325](0325-the-frame-follows-the-largest-enabled-output-and-a-resize-costs-0-145-ms.md).

### 4. What it writes is nothing, and that is an argument

`karakuri-operation-record` answers `Silent(Published)` — a new variant, because none of the four
that existed said this. **Publishing is not the performance.** `docs/plugins.md` says a sink writes
no record; ADR-0171 says an operator who turns every output off has stopped the publishing and not
the instrument, so two runs differing only in which sinks were on took the same steps and drew the
same frames; and [P-0086](../principles/0086-a-procedure-knows-only-what-it-declares.md) says the
render size is not part of the picture, so a replay on a machine with different outputs reproduces
the same picture at its own sizes. That answers the third question ADR-0246 left open — *what a
replay reproduces when the outputs it was performed to are gone* — with **the canvas**, which is
what `karakuri-cli --replay` already draws at.

## Alternatives rejected

**A string name, or a `String` label carried on the operation.** Argued above. It loses on the label
moving, on there being nothing to refuse a wrong one with, and on the vocabulary's own standing
practice.

**File `RouteFrame` under `Silent::Surface` rather than adding a variant.** One line, and the arm
already means *writes no record*. It loses on two things. An output is not a surface's own state — a
projector window is not the console — and that arm's documentation says every member of it is `gap`
in the MCP column *because a model has no window*
([ADR-0315](0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)), where a
model may ask for an output once the *inputs and outputs* class is open. Filing it there would have
made a written sentence false and taken a route away from a caller that has one.

**Store an `on: bool` per output, including the picture's.** It makes the four chips one uniform
thing and it is what a reader expects. It loses to the second copy: the picture is on screen exactly
when its node is visible, the keyboard's `f` folds that node, and a stored bool would go stale the
first time anybody folded the picture any other way. ADR-0161 stored `soloed` because it could not be
derived; this can.

**Wait for the plugin manifest before naming anything.** The two chips that need it are the two that
are drawn `no plugin`, and the two that do not are built. Deferring the whole identity for the half
that is absent is what left `Undecided` on this operation for four weeks.

**Draw only what is built and leave the plugin chips out of the row.** It is the scaffolding rule
this console keeps — *a control that looks finished does not get replaced*. It loses because the
chips are not controls: `.absent` is the mock's own state for *a control explaining that the thing it
would switch is not installed*, and a row that simply did not mention Syphon would leave an operator
wondering whether this program has heard of it.

## Consequences

- **`Operation::RouteFrame { output: Output, on: bool }`**, and `Undecided` is down to eight.
  `gate.rs`'s fixture names `Output::Program`; the class it is in — `InputsAndOutputs`, closed until
  the operator opens it — is unchanged.
- **`karakuri_operation_record::Silent` has a fifth variant**, `Published`, with its reason at the
  arm and its `why()` sentence beside the other four. `RouteFrame` left the `Owed(Undecided)` group,
  which that group's own comment predicted: it is the arm for payloads that are open, and this one
  is not open any more.
- **The console's Outputs row draws four chips**: `program view`, `projector`, `Syphon · no plugin`
  and `NDI · no plugin`. `Outputs` keeps `sink`/`dot`/`id`/`on` for the program view and carries the
  other three as `more: [SinkChip; 3]`. **The asymmetry is the model**: the program view's state is
  a layout node and no other output has one, so a uniform array would have needed an
  `Option<NodeId>` on every chip.
- **`input::on_sink` claims the whole row's chips** rather than the first, through
  `Outputs::chip_at`. The two plugin chips answer `false` — a control that switches nothing does not
  take a press away from the row it sits in.
- **A projector window is the second `Sink` in the slice**, which is what ADR-0171 said was owed.
  `crates/karakuri`'s `Projector` holds the window, a `WindowSink` and the size; `compose` is handed
  one array or two.
- **The panel badge of *Choose where the frame goes* is `has`.** `karakuri-console/src` constructs
  `Operation::RouteFrame` in `view::Outputs::route` and `view::SinkChip::route`, and an operator
  reaches it by pressing the projector chip. MIDI stays `gap` — no chip on the console page carries a
  target — and MCP stays `plan`.
- **The mock changed in two places and both are the page moving first**
  (`docs/contributing.md` §5 step 3): `projector · DELL U2720Q` is `projector`, because naming the
  display wants a list of monitors this program does not read; and `Syphon` is `Syphon · no plugin`
  and `.absent`, because with no plugin system it is the same state NDI's chip already drew.
- **The window that opens is not made fullscreen and not put on a chosen display.** It is an ordinary
  window, which is the console page's *fullscreen is just what an application does*. That is one line
  of `winit` and it is **not** taken here, because choosing *which* display is the part that is
  missing and a fullscreen with no way to say where is worse than none.
- **A projector surface must offer `PICTURE_FORMAT`.** `Present`'s pipeline names one target format
  at construction; a surface in another would want a second present pipeline, which is what ADR-0247
  says nothing needs. The window refuses to open and the refusal names both formats.
- **Closing the projector window turns the output off and does not quit.** `WindowEvent::
  CloseRequested` is routed by `WindowId`; the panel's own close is untouched.
- **`live()` counts the projector.** With the picture folded away and a projector on, the loop would
  otherwise stop asking for frames while a window on another display went on showing whatever was
  last presented into it.
- **Nothing verified this by running the panel.** A `winit` window cannot be created without an
  `&ActiveEventLoop`, which exists only inside a `winit` callback, so no test in this workspace opens
  the projector — see *What is not tested* below.

## What is not tested, and why

- **The projector window opening and closing.** `ActiveEventLoop::create_window` needs a live event
  loop, `crates/karakuri`'s `mod gpu` has none, and on macOS the loop must own the main thread. What
  *is* tested is everything on either side of it: the operation the chip constructs
  (`karakuri-console`), the size derivation over two outputs (`crates/karakuri`, CPU), and a second
  sink receiving the frame while a wedged one is skipped — which is `frame.rs`'s
  `a_sink_that_refuses_costs_the_others_nothing` and `two_sinks_of_different_sizes_both_get_the_canvas`,
  written for ADR-0171 against a third `Sink` implementation for exactly this reason.
- **The refusal when a surface does not offer `PICTURE_FORMAT`.** Reachable only from a window, and
  not reachable on this machine at all: Metal offers it.
