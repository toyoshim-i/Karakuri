# Simulation time comes from a record, never from a clock

`t` advances by `steps * dt`, where `steps` is read from a `tick` record. Running live, the
engine derives the step count from real time and **writes it into the record**; replaying,
it reads the same number back verbatim and derives nothing. A recorded session is therefore
frame-exact on a machine of a different speed.

A clock may be read to judge *cost* — the frame-interval watchdog in
[swap.rs](../../crates/karakuri-engine/src/swap.rs) and the measurements in
[probe.rs](../../crates/karakuri-engine/src/probe.rs). No value derived from one may reach
simulation state. That is the whole of the exception, and it is what the crate header means
when it says nothing reads a clock.

**What it rules out.** Recomputing the step count at replay from the replaying machine's
own timing, which makes a replay a re-run. Reading wall time anywhere a procedure can
observe it. Substepping that is a measurement rather than a record — the cap is 4, and past
it the simulation is allowed to fall behind rather than spiral.

**Where it holds.** [ir-spec.md](../ir-spec.md), "On `dt` and simulation time" and the
session stream format; [roadmap.md](../roadmap.md), "Settled decisions".
