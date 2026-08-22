# Closed-form material needs no warming

A procedure that never reads an attribute it emits is **closed-form**: its picture at any `t` is a
pure function of `t`, so it can be brought up instantly, cued and scrubbed. One that reads its own
output **accumulates** and has to be run to its attractor before it is fit to show. The compiler
decides which — the check pass already tracks attribute reads and writes — and records it in the
artifact's metadata.

**What it rules out.** Treating every Set as needing to be warmed, which reserves a deck slot for
material that does not need one, and makes beat-resolution variant switching look unaffordable when
in fact it is a **closed-form** feature: an accumulating candidate must be paid for to stay
resident, a closed-form one costs nearly nothing.

**A pulled-down Set keeps its state.** Allocated means the buffers are held and stepping has
stopped, so returning to Priming resumes rather than restarts — and since `t` is simulation time,
minutes parked advance it by nothing.

**Where it holds.** [ir-spec.md](../ir-spec.md); the residency states in
[deck.rs](../../crates/karakuri-engine/src/deck.rs). Decided in
[ADR-0028](../adr/0028-closed-form-and-accumulating-is-a-static-classification.md).
