# A quiet room is not a missing microphone

An `energy` of 0.0 at high confidence is a **measurement** — silence. A frame with no audio record is
a frame with **no provider**, and every name answers what it answered before audio existed. Nothing
anywhere asks whether a device is attached.

**What it rules out.** Treating absence as zero, which is the reading that makes an unplugged
interface look like a silent room and freezes the picture mid-set instead of letting it free-run. It
is the same distinction that decides what happens during a scratch: with acoustic analysis only,
confidence collapses and the oscillator free-runs; with a deck attached, confidence stays high and the
position moves.

**And what is recorded is the analyser's output, never its input.** Raw audio replayed through the
analyser freezes the analyser; recording the measurement — in fact the *correction* rather than the
estimate — lets it improve without changing how an old session replays.

**Where it holds.** [ir-spec.md](../ir-spec.md), "Measurement in the stream"; `karakuri-audio`.
Decided in [ADR-0055](../adr/0055-a-measurement-enters-the-record-stream-raw-audio-does-not.md).
