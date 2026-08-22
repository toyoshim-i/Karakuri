# A guarantee is structural, or it is a convention that says so

A frame cannot be built from two generations of Sets because the frame guard owns both `&mut Deck`
and the command encoder, so a second `begin_frame` is a borrow error. Where a property is only a
convention — `HotSwap::begin_frame` driven on its own still depends on its caller — it says so, and
points at the place that enforces it.

**What it rules out.** Claiming the compiler enforces something it does not. The hot-swap brief
asserted the borrow checker made a mid-frame swap impossible; it did not, because the encoder belongs
to the caller and borrows nothing, and a review **demonstrated** it by recording two Sets of different
capacity into one submit. It also rules out handing out a mutable reference that reopens the hole —
`Deck::slot` returns a shared reference for that reason.

**Write down what it does not cover.** Other passes may join the frame's encoder, an early return
ends a frame rather than cancelling it, and nothing is said about two separate decks.

**And take it while it is cheap.** This was tens of lines with one Set and every call site once the
deck composited four. It was written into the roadmap at the moment it was found, to be paid before
the second Set existed.

**Where it holds.** [deck.rs](../../crates/karakuri-engine/src/deck.rs). Decided in
[ADR-0034](../adr/0034-the-frame-guard-owns-the-encoder.md).
