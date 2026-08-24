# A node claims only what its visible content can use

A layout node's stored size and its declared minimum are both **claims about a node whose content
is whole**. Neither may hold space the node's visible children cannot fill.

`karakuri-layout` measures, bottom-up and before anything is placed, how much each node can use:
a flexible leaf any amount, a fixed leaf its stored size, a split the sum of its visible children
and the dividers between them, zero if none is visible, capped by the node's own maximum, and any
amount when the split is laid out across its parent's axis. A `Fixed` child then claims
`min(stored, usable)` and a declared minimum becomes `min(declared, usable)`.

**The failure it prevents is silent and reads as a design choice.** A fixed bay whose only
flexible child is folded away used to keep its whole stored height and let whatever was left inside
swell to fill it — so a control that hid a region gave the space to its neighbour *inside* the bay
instead of back to the panel, and the document that said otherwise looked merely aspirational.

**The exception, which was tried the other way and reverted:** the branch that disposes of space no
child claimed is not capped. A cap says what a node would *ask for*; there, nobody is asking, and a
split whose children are all folded can use zero — capping it leaves a hole nothing can ever fill.

Measuring writes into the solved buffers and never into the arrangement, so
[P-0071](0071-solving-a-layout-never-mutates-it.md) is unaffected: fold and unfold restore the
original rectangles exactly.

See [ADR-0174](../adr/0174-a-node-claims-only-what-its-visible-content-can-use.md).
