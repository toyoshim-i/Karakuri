---
id: 0156
title: The console's arrangement is a tree this repository owns, not the toolkit's panels
status: accepted
date: 2026-08-23
supersedes: []
superseded_by: []
principles: [0082]
tags: [ui]
---

# The console's arrangement is a tree this repository owns, not the toolkit's panels

## Context

The console is a column holding a transport, a row of three, and an outputs strip, where each of
the three holds its own stack of bays. Every boundary in it drags, every one of the three has a
width it will not go below and one it will not go above, and any of them can be folded away to
give its space to the rest.

*(Two words in that sentence are corrected. It read "a status line", which is not what the manual
calls that row — it heads it `Outputs` — and it called all three columns panes, which
[ADR-0159](0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md) later
settled as `left pane`, `centre` and `right pane`. Both were descriptions of the console that were
wrong about it, so both are corrected rather than left standing; nothing this record decided has
changed.)* `docs/manual/console.html` states the reason for the first of those: *a preview's size is a
machine's answer rather than a layout's*, so the operator sets it and the layout does not.

[ADR-0155](0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md) chose `egui` to draw it, and
**`egui` can express that arrangement.** This is worth stating plainly, because the alternative
here is a real one rather than a straw one. Its `Panel` type anchors to an edge — `left`, `right`,
`top`, `bottom` — with a `CentralPanel` taking what is left, which is exactly the console's row of
three. Panels nest through `show_inside`, so a pane's stack is panels within a panel. And
`resizable`, `min_size`, `max_size`, `size_range` and `show_collapsible` cover dragging,
constraints and folding without anything being written here.

So the question is not whether the toolkit can do it. It is **who owns the arrangement**.

## Decision

**The arrangement is a constraint tree in `karakuri-layout`, a crate with no toolkit in it, and
what `egui` receives is a rectangle.**

Two things decide it, and both are about the interface milestone rather than about `egui`.

**An arrangement is made of operations, and operations are named once.** Widening the library,
folding the sequencer away, soloing the program — these are operations in exactly the sense the
console's vocabulary means, and the rule is that each is named once and every surface routes into
that name: the panel, the keyboard alone, a mapped MIDI control, and MCP. If the arrangement lives
in `egui::Memory` under an `Id`, then the model of record belongs to the toolkit, and every surface
that is not a pointer has to reach into the toolkit's memory to be heard. Owning the tree makes
the panel the same kind of client as MCP is: it reads rectangles and it emits operations, and it
holds no authority the other surfaces lack.

**The arrangement has to answer without a GPU or a window.** Testing that folding the inspector
gives the program its height back should be arithmetic, not a frame. With the toolkit owning it,
that test runs a `Context`, feeds it synthetic input and reads panel rectangles back out of
`Id`-keyed memory — which works, and which welds every one of these tests to the choice
ADR-0155 made. A tree that knows nothing about drawing is tested at full speed on a machine with
no adapter, and survives the toolkit being replaced.

**What this does not claim.** It is not that `egui`'s panels are poor, and it is not that they
would have failed. It is that the arrangement is application state that four surfaces write to,
and application state does not live in the renderer.

**What this leaves owed, and it is not small.** The argument above calls arranging the panel a set
of operations, and the manual's operations page does not yet carry a single one of them — no row
for folding a pane, none for moving a divider, and none for `solo`, which the console page already
draws as a control with a tooltip and therefore already promises. The count on that page is 41
operations, and this decision says it is short. The sharp end is the first rule rather than the
bookkeeping: **every operation is reachable from the keyboard alone**, and a divider that can only
be dragged is reachable from a pointer and nothing else. Naming these operations is what settles
how a pane is resized without a mouse, so the rows are owed before the panel is drawn, not after.

**Solving never mutates.** A viewport too small for the minima produces small rectangles and
changes nothing stored, so a window dragged narrow and back comes back to exactly what it left.
That is [P-0082](../principles/0082-looking-never-writes-back.md), and it is the rule most
of this crate's tests are about.

## Alternatives

**`egui`'s panels, with the arrangement in the toolkit's memory.** Nothing to write, everything to
maintain elsewhere. Rejected on the two points above: it puts the model of record where only one
of four surfaces can reach it comfortably, and it makes the arrangement's tests a function of the
toolkit.

**A tree here, but solved by `egui` — our nodes, its dragging.** A halfway house that was
considered and is worse than either end: two things would hold a width, and they would drift.

**Fixed regions with no dragging at all, sized by the window.** Rejected by the manual, which
states the opposite as a design reason rather than a feature: a preview's size is a property of
the machine it is running on, and no layout can compute it.
