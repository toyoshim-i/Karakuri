# Nothing allocates or compiles a shader on the render thread

The frame path allocates no heap memory and creates no pipeline. Buffers are sized and
pipelines compiled ahead of the frame that uses them — at initialisation, or on a
background thread whose result is installed at a frame boundary.

**What it rules out.** Creating a pipeline lazily on first use, which is the ordinary way
to write this and moves an unbounded stall to whichever frame happens to be first. It also
rules out sizing a buffer from what a record asks for at the moment it asks: the record
schedules the work, and the work lands later.

**Freeing counts, and so does anything queued.** Dropping a `Set` releases GPU resources, so a
retired Set is handed to a worker through a `try_lock` — never a `lock`, which would be the stall
this exists to prevent. And work moved off the render thread is not off it until its side effects
are: a 12 MB upload left staged on the queue is flushed by whoever submits next, which is the render
thread, landing the hitch on the very frame a swap was meant to be invisible to.

**Where it holds.** [karakuri-engine/src/lib.rs](../../crates/karakuri-engine/src/lib.rs)
states it for the crate; [swap.rs](../../crates/karakuri-engine/src/swap.rs) is where the
double buffering that makes it possible lives, and
[deck.rs](../../crates/karakuri-engine/src/deck.rs) owns the frame guard. Sharpened in
[ADR-0033](../adr/0033-freeing-on-the-render-thread-is-the-same-invariant-as-allocating.md).
