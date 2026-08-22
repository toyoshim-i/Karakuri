# Arithmetic plus an operator key beats a rule that is usually right

Where a system must choose between interpretations, prefer a computation that cannot be subtly wrong
and hand the residue to the operator. The tempo octave is **folded** — double or halve the peak until
it lands in a one-octave window centred on the current estimate — rather than judged by a heuristic,
and the ×2 / ÷2 keys are the escape.

**What it rules out.** A rule that is right most of the time. **A confident wrong automatic judgement
is worse than not judging**: the heuristic it replaced settled at half tempo above about 160 bpm, and
because a wrong octave collapses confidence it read as *there is no beat here* rather than as a bug.
It also rules out repairing such a rule when the repairs are themselves fragile — the two fixes made
hours earlier were deleted with it.

**State the trade, and assert the intended failure.** A window that starts an octave off locks an
octave off and does not recover; there is a test saying so, written specifically so that nobody
repairs it back into a heuristic in three months. And what is *not* addressed is written where a
reader will look: folding by powers of two says nothing about a 3:2 error, which lands inside the
window and looks correct.

**Where it holds.** `tempo.rs` in [karakuri-audio](../../crates/karakuri-audio). Decided in
[ADR-0060](../adr/0060-the-tempo-octave-is-folded-not-judged.md).
