---
id: 0243
title: The Program picture is an output and the four cells are monitors
status: accepted
date: 2026-09-02
supersedes: []
superseded_by: []
principles: [0080]
tags: [ui, operations]
---

# The Program picture is an output and the four cells are monitors

## Context

[ADR-0240](0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) retired the
operation that swapped the Program picture from the mix to one deck. The reason it gave was
redundancy with the four cells. The reason it should have given is that **once material is on the
main output it is the broadcast of the final result** and never a preview of anything — which is
recorded in
P-0080
and annotated onto ADR-0240.

That sentence says something about the picture as well as about the retired control, and the Program
bay holds two things that this puts on opposite sides of a line. Both are already modelled that way
in places, and nowhere as a rule:

- [`docs/manual/console.html`](../manual/console.html) lists the picture in the Outputs row as
  *program view*, the first of four sinks, and says *"The picture in the Program bay is a sink like
  any other and is the first row."*
- The same page said of the cells: *"The deck previews are auditions rather than outputs, so with
  every sink off you can still see the decks and work on them."* It reads *monitors rather than
  outputs* now, which is this record.
- `crates/karakuri-console/src/view.rs` draws for `Kind::Outputs` the word OUTPUTS and **one** sink,
  named by `PROGRAM_VIEW = "program view"` — the picture. `Kind::Previews` draws nothing, and no
  control anywhere in that file routes a cell.

## Decision

**The picture is one of the outputs.** It is the same kind of thing as the projector window, Syphon
and NDI: a place a composited frame goes. What routes it is *Choose where the frame goes*
(`Operation::RouteFrame`), and it belongs to the Outputs row's model rather than to the Program
bay's.

**The four cells are monitors and are not outputs at all.** Nothing routes them, nothing chooses
what goes to them, and each one is fixed to the deck slot it is lettered for. No frame is sent to a
cell; a cell draws its own slot's material, which is what makes P-0080's first clause true. A control
that names an output can never name a cell.

**This does not settle what names an output.** `Operation::RouteFrame` carries `output: Undecided`
because no output has an identity anywhere in this workspace, and that question is M5.6's. What this
record adds to it is one fact it now has to account for: **the picture is in the set that needs
naming, and a cell is not.** A naming scheme that covers windows and plugin sinks and cannot say
*the picture in the Program bay* is short by one.

## What this leaves standing that a reader will notice

The console's control over the picture is not a route today. The dot in the Outputs row reads
`Layout::visible` on the picture's node and a press on it asks for `Op::Fold` or `Op::Unfold` by
name, so the one thing an operator does to the picture is fold it away — the manual's *"it is on
screen exactly when that sink is on"*. That is a sink drawn as a layout node, which is the right
control and the wrong model, and it is named here rather than changed: it is the shape whoever
answers M5.6 has to reconcile.

## Alternatives rejected

**Call both the picture and the cells outputs.** It is what *Program bay* suggests, since they sit in
one bay and both show frames. It fails on the operation: something that can be routed and something
that is welded to one slot cannot share a model without the model saying nothing. It also gives the
retired control a way back — an output that can be pointed at a deck is *Choose what the output
shows* under a new name.

**Call the picture a preview, since it is on the operator's screen rather than at the projector.**
This is the mistake the retired operation was built on. Where the operator happens to be sitting is
not what makes something a preview; a sink carrying the composited master mix is an output wherever
its window is.

**Wait for M5.6 and decide both together.** The naming is genuinely open and this is not. Leaving
membership unstated until then is what lets the next reader put a cell in the routing model, which is
cheaper to refuse now than to remove later.

## Consequences

- **Two bays have a stated scope.** The Outputs row owns the picture; the Program bay's cells are the
  mixer's monitors drawn beside it and answer to P-0080 rather than to anything in Outputs.
- **`Operation::RouteFrame { output: Undecided }` has one more constraint on it**, recorded rather
  than resolved: the picture is nameable and the cells are not.
- **The console page says both**, in the Outputs note and beside the cells.
- **Turning every sink off still leaves the cells running**, which the manual already says and which
  is now a consequence of what a cell is rather than a behaviour of its own.
