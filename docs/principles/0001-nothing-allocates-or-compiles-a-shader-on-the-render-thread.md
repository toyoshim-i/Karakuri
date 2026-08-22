# Nothing allocates or compiles a shader on the render thread

The frame path allocates no heap memory and creates no pipeline. Buffers are sized and
pipelines compiled ahead of the frame that uses them — at initialisation, or on a
background thread whose result is installed at a frame boundary.

**What it rules out.** Creating a pipeline lazily on first use, which is the ordinary way
to write this and moves an unbounded stall to whichever frame happens to be first. It also
rules out sizing a buffer from what a record asks for at the moment it asks: the record
schedules the work, and the work lands later.

**Where it holds.** [karakuri-engine/src/lib.rs](../../crates/karakuri-engine/src/lib.rs)
states it for the crate; [swap.rs](../../crates/karakuri-engine/src/swap.rs) is where the
double buffering that makes it possible lives, and
[deck.rs](../../crates/karakuri-engine/src/deck.rs) owns the frame guard.
