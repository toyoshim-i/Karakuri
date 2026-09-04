# Looking never writes back

A read produces an answer and leaves what it read alone. Solving a layout is a function from an
arrangement and a viewport to rectangles; auditioning a slot adds a draw and never a step. Only an
explicit operation — a divider dragged, a pane folded, a frame advanced — changes stored state.

**What it rules out.** Clamping a stored width during the solve because the viewport is narrow: one
line, indistinguishable from the right thing at the moment it runs, and a slow leak, because every
excursion into a small window permanently rewrites what the operator arranged. **It fails only across
time**, so nothing at the moment of the change looks wrong. Solved as a function instead, everything
scales down in proportion, nothing goes negative, and dragging the window back reproduces the
arrangement exactly rather than approximately — the same for a resolution change, a disconnected
projector and a session restored on a smaller screen. A preview that steps the slot it previews,
which fails in the least visible way there is: you watch a moving picture, decide you like it, put it
on air, and it is not where you were looking. And a second render pass built for the purpose, when
running the ordinary mix with the target at unity and the rest skipped leaves the tone mapper, the
present pass and the readback untouched — faders ignored while auditioning, which is the difference
between looking and auditioning.

**Where it holds.** [`karakuri-layout`](../../crates/karakuri-layout/), whose solve destructures the
layout and takes the arrangement by shared reference, so a line storing a solved size into a node
does not compile; and
[deck.rs](../../crates/karakuri-engine/src/deck.rs), where the Allocated branch draws and never
steps and `t` moves only through `Set::prepare`. Decided in
[ADR-0072](../adr/0072-auditioning-adds-a-draw-and-never-a-step.md),
[ADR-0156](../adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md) and
[ADR-0250](../adr/0250-below-the-minima-the-arrangement-scales-rather-than-being-rewritten.md).
