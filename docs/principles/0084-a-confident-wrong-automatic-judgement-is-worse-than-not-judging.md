# A confident wrong automatic judgement is worse than not judging

Where a system must choose between interpretations, prefer a computation that cannot be subtly wrong
and hand the residue to the operator. A value arrives with how well it is known — what is written is
`lerp(the manual value, the mapped signal, confidence)`, and that blend is the only thing anyone
branches on. Sampling cannot fail and returns no `Option`; nothing anywhere asks whether a provider
exists, and absence and a measured zero never collapse into one fact.

**What it rules out.** A tempo heuristic, which settled at half tempo above about 160 bpm — drum and
bass, footwork, hardcore, the top fifth of the search range — and whose octave check was **disabled
by exactly the condition it exists to catch**: its phase came from a single DFT bin at `1/period`, so
at twice the real pulse spacing consecutive pulses land half a turn apart and cancel, the magnitude
goes to nearly zero, and the guard `on > 0.0` is false. It does not look like a bug, because a wrong
octave collapses confidence and reads as *there is no beat here*. The arithmetic that replaced it —
`fold(bpm, centre) = bpm · 2^-round(log2(bpm/centre))` over `[centre/√2, centre·√2]`, one octave
because the octaves of a one-octave window tile the tempo axis with no overlap and no gap — plus
`×2` and `÷2` keys for the residue, and what it deliberately does not fix written down where a reader
looks: a 3:2 error lands inside the window and looks correct. A threshold on `bad_texels`: non-finite
texels are commonplace and usually harmless, which is exactly what makes them a poor proxy, so the
count is shown and is deliberately not a fault indicator. A panic key returning to a known-good Set,
which adds no capability — `space` takes a slot down, `\` returns gain, `` ` `` returns exposure — and
could not do its stated job anyway, because `HotSwap::previous` exists only inside the trial window
and a permanent rollback target costs a Set's worth of VRAM per slot for ever; and one learned to be
sometimes wrong becomes a control the operator hesitates over, the single property a panic key must
not have. Treating absence as zero, which makes an unplugged interface look like a silent room and
freezes the picture mid-set instead of letting the oscillator free-run: an `energy` of 0.0 at high
confidence is silence, and a frame with no audio record is a frame with no provider. And ignoring
confidence to make a demo livelier — a synthesized `energy` behaving convincingly with nothing
plugged in is a lie you would have to write out again later.

**Where it holds.** [binding.rs](../../crates/karakuri-engine/src/binding.rs), whose fourth step is
the blend and whose `Signals::sample` returns a `Sample` rather than an `Option`, so there is no
branch on a provider to write; [tempo.rs](../../crates/karakuri-audio/src/tempo.rs), where `fold` and
`tracking_window` are the arithmetic and the intended failure — a window that starts an octave off
locks an octave off — is asserted by a test so nobody repairs it back into a heuristic; and
[meter.rs](../../crates/karakuri-engine/src/meter.rs), which counts non-finite texels, keeps them in
the denominator and attaches no threshold to the count. [ir-spec.md](../ir-spec.md), *What a binding
does* and *Measurement in the stream*, states the blend and the record shape. Decided in
[ADR-0047](../adr/0047-a-binding-blends-on-confidence.md),
[ADR-0055](../adr/0055-a-measurement-enters-the-record-stream-raw-audio-does-not.md),
[ADR-0057](../adr/0057-the-transport-is-driven-by-position-not-by-tempo-and-phase.md),
[ADR-0060](../adr/0060-the-tempo-octave-is-folded-not-judged.md) and
[ADR-0071](../adr/0071-there-is-no-panic-key.md).
