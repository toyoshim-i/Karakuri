# A swap happens on a frame boundary, and an over-budget Set rolls back on its own

Pipelines are double-buffered and exchanged only between frames, so no frame is built from
two generations. A newly installed Set that exceeds the frame budget is rolled back by the
governor without anyone asking, and the Set it replaced is parked unstepped so that it
returns where it left off.

**What it rules out.** Swapping when the new pipeline becomes ready, which is the moment
that is easiest to detect and is inside a frame. It also rules out leaving a bad Set on air
until an operator notices: this runs in front of an audience, so recovery is automatic and
the report is what reaches the operator.

**Where it holds.** [swap.rs](../../crates/karakuri-engine/src/swap.rs),
[governor.rs](../../crates/karakuri-engine/src/governor.rs), and
[deck.rs](../../crates/karakuri-engine/src/deck.rs), whose frame guard owns the encoder.
