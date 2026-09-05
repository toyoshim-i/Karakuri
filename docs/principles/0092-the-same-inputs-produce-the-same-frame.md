# The same inputs produce the same frame

Bit-exact, because the only path observing element order is blend composition and float addition is
not associative. **Time comes from a record**: live, the engine derives the step count from real time
and writes it in; replaying, it reads the number back and derives nothing. A clock may be read to
judge *cost* and no value derived from one may reach simulation state — that is the whole of the
exception. **A Set is closed**: compaction order, the parity flip, `rewind`, `seek` and the probe
measurement are every one written as this Set's sources. **The boundary of this process is drawn
where the deterministic path ends** — not by platform, not by toolchain: what consumes the
composited frame and writes no record may live outside, what produces the records a key press
produces may live outside, and what reaches the pixels a replay must reproduce may not.

**What it rules out.** Reading a clock in the simulation, and recomputing the step count at replay
from the replaying machine's timing, which makes a replay a re-run and lets the faster machine see a
different performance. A free list with atomic allocation — cheaper, and GPU atomics complete in a
non-deterministic order, so the same session composes differently on a second run, additive blending
included — and behaviour-exact determinism, which was what would have permitted it: a replay that is
only approximately the performance is not a replay. Identifying an element by its buffer slot, whose
failure is silent and beautiful — a particle whose colour came from its slot changes colour while it
is alive. One file for a Set and its timeline, since a tick per frame written into a definition turns
it into a timeline hundreds of thousands of lines long and makes saving a Set save a performance.
Mutating a live Set in place, which opens a hole in the record stream and loses replay, undo, A/B
comparison and session recording together — the fork **is** the edit mechanism, not a fallback for
expensive edits. Two deck slots backed by one simulation: real savings, and the roadmap's implied
direction, bought by making the Set stop being a boundary. Performing a save on replay, which gives
`--replay` side effects and a function from stream to frames that is no longer one. Recording the raw
audio and re-running the analyser on replay, which freezes the analyser at today's version. And a
beat-clock event that computes the material it is switching to rather than choosing among material
already built.

**Where it holds.** [compaction.rs](../../crates/karakuri-engine/src/compaction.rs) is the
order-preserving prefix-sum scan, and **its primary argument is not this rule**: indirect dispatch
over a live count needs the live elements contiguous, a free list does not escape the scan it would
need to rebuild that list, and bit-exactness is what order preservation then gets for free
([ADR-0002](../adr/0002-compaction-preserves-order-and-determinism-is-bit-exact.md), which says in
as many words that a specification resting on its second-best reason is easier to argue out of
later). [set.rs](../../crates/karakuri-engine/src/set.rs) is the clock: `t` is `steps_taken * dt`
from an integer count rather than an accumulating `f32`, it advances **per substep** rather than per
frame, and `MAX_STEPS` is 4 — past it the simulation falls behind rather than spiralling.
[swap.rs](../../crates/karakuri-engine/src/swap.rs)'s frame-interval watchdog and
[probe.rs](../../crates/karakuri-engine/src/probe.rs)'s measurements are the whole of the clock
exception, and [karakuri-engine/src/lib.rs](../../crates/karakuri-engine/src/lib.rs) says so for the
crate. [ir-spec.md](../ir-spec.md) is the authority for the rest — *On `dt` and simulation time*,
*Element identity* (`seed` is an ordinal carried by the element and moved by compaction, and `id`
stays a reserved word so the compiler can name the replacement), the *Set file format* against the
*Session stream format*, and *Records with an effect outside the stream*.
[karakuri-store](../../crates/karakuri-store/) is what keeps those two apart: `Record::is_set_state`
is asserted on every path into a Set, including the projection.
[`transition.rs`](../../crates/karakuri-engine/src/transition.rs) is where the middle clock is a
function of `beats` and nothing else, which is what makes the same records give the same fade on a
machine running at a different rate. [plugins.md](../plugins.md) draws the process boundary by the
same test. Decided in
[ADR-0001](../adr/0001-identity-is-seed-an-ordinal-not-a-slot-index.md),
[ADR-0002](../adr/0002-compaction-preserves-order-and-determinism-is-bit-exact.md),
[ADR-0006](../adr/0006-the-step-count-is-a-record-not-a-measurement.md),
[ADR-0007](../adr/0007-a-set-file-is-a-projection-and-a-session-stream-is-the-timeline.md),
[ADR-0027](../adr/0027-a-set-value-is-immutable-and-its-compiled-instance-is-not.md),
[ADR-0030](../adr/0030-simulation-time-comes-from-an-integer-step-count.md),
[ADR-0055](../adr/0055-a-measurement-enters-the-record-stream-raw-audio-does-not.md),
[ADR-0074](../adr/0074-a-plugin-boundary-is-drawn-by-the-deterministic-path.md),
[ADR-0080](../adr/0080-the-gpl-boundary-is-a-process-and-the-protocol-is-generic.md),
[ADR-0120](../adr/0120-a-record-may-reach-outside-the-stream.md),
[ADR-0147](../adr/0147-geometry-is-not-shared-across-sets.md) and
[ADR-0255](../adr/0255-three-clocks-run-at-once-and-a-slower-ones-work-never-lands-on-a-faster-one.md).
