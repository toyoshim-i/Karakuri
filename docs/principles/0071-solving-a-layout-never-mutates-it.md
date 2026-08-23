# Solving a layout never mutates it

Laying the console out is a function from an arrangement and a viewport to rectangles. It reads the
arrangement and it does not write to it. **Only an explicit operation — a divider dragged, a pane
folded, a pane restored — changes a stored size.**

**What it buys is that nothing is lost by looking at it.** A window dragged narrower than the sum
of the panes' minimum widths still has to produce rectangles, and it does: everything scales down
in proportion and nothing goes negative. But the sizes the operator set are untouched, so dragging
the window back reproduces the arrangement exactly rather than approximately. The same holds for a
display that changes resolution, a projector that disconnects, and a window restored from a saved
session on a smaller screen.

**The violation is easy to write and hard to see.** Clamping the stored width during the solve —
"the pane cannot be 240 here, so it is 180 now" — is one line, it looks like the same thing, and it
is a slow leak: every excursion into a small viewport permanently rewrites what the operator
arranged. It fails only across time, so nothing at the moment of the change looks wrong.

**Where it holds.** [`karakuri-layout`](../../crates/karakuri-layout/), whose solve writes into a
rectangle buffer and takes the arrangement by shared reference, so the compiler enforces the rule
this states.

**Recorded in** [ADR-0156](../adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md).
