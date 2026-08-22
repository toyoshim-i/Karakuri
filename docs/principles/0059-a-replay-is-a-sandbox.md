# A replay is a sandbox

Some records have an effect **outside** the stream. A replay does not perform those effects, and it
**says what it skipped**. Bringing outside state into a replay is the operator's responsibility.

**The test for whether an effect must be reproduced** is whether a later record depends on it. Saving a
Set does not, today — there is exactly one caller of the loader and it runs once at startup — and the
one thing that would change that is named: a live "load a Set into a slot" control. The escape is
already open, because the system is content-addressed: such a record naming a **hash** needs only the
kind of artifact a replay already needs for its head.

**What it rules out.** Both silent alternatives. Not recording a side-effecting act leaves the timeline
without its only landmark — a surface showing every fader move and no saves. Performing it on replay
gives `--replay` side effects: files appearing in someone else's store that nobody asked for, and a
function from stream to frames that is no longer one.

**It does not disturb the existing classification.** `is_set_state` asks *whose state is this*; this is
an **orthogonal second question** — *does this reach outside the stream*.

**Where it holds.** [ir-spec.md](../ir-spec.md), "Records with an effect outside the stream". Decided in
[ADR-0120](../adr/0120-a-record-may-reach-outside-the-stream.md).
