A Set owns its element buffers, and the drawing pipeline is closed within it. **Geometry is
never shared across Sets.** A second deck slot that wants the same geometry holds its own
copy and simulates it itself.

What this rules out is the optimisation that keeps suggesting itself: several deck slots
holding alternatives that differ only in their renderer, backed by one simulation. It is
real savings and it loses anyway, because a Set that reads element buffers it does not own
stops being the unit that compaction order, the parity flip, `rewind`, `seek` and the
per-slot probe measurement are all defined against. Every one of those is written as "this
Set's sources", and a shared source makes each of them a question about two Sets whose
clocks need not agree.

Sharing *within* one Set is the supported answer and costs nothing: `Set::step` walks
sources and `Set::draw` walks renderers, so several renderers over one geometry is one
simulation and one draw pass each.

The price is stated rather than hidden: two slots showing the same cloud are two
simulations, and a pool of alternatives across deck slots costs one simulation per
alternative even when they differ only at L4. See
[ADR-0147](../adr/0147-geometry-is-not-shared-across-sets.md).
