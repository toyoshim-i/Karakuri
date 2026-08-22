# A consumer branches only on confidence

Sampling a signal cannot fail and does not return an `Option` — an unknown name yields a synthesized
value. What a binding actually writes is `lerp(the param's own value, the mapped signal, confidence)`.
That blend **is** the rule; nothing anywhere asks whether a device exists.

**What it rules out.** Ignoring confidence to make a demo livelier. Bound to `energy` with no
microphone attached, a parameter barely moves, and the brief said so in advance: *ignoring confidence
would make this lively, and it is a lie you would have to write out again later.* A synthesized
`energy` behaving convincingly with nothing plugged in is far worse than a dull demo. `beat` and
`bar` work completely, because the local oscillator is the single truth about phase.

**The manual value is the base of the blend and is never overwritten**, so a manual write and a
binding are not competing writers — order-independent by construction rather than last-writer-wins.

**Where it holds.** [binding.rs](../../crates/karakuri-engine/src/binding.rs);
[ir-spec.md](../ir-spec.md), "What a binding does". Decided in
[ADR-0047](../adr/0047-a-binding-blends-on-confidence.md).
