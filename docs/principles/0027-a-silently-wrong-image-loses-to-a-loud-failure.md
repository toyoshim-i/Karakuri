# A silently wrong image loses to a loud failure

Where an API turns a programming mistake into *plausible output*, the mistake is converted back into
a failure at the place that caused it. The composite reads with `textureLoad`, and an out-of-range
load **returns zero by specification** rather than faulting — so resizing `Present` and forgetting
the deck gives a black frame every frame with nothing logged. `Frame::render` checks its sizes and
panics at the call site.

**What it rules out.** Tolerating a plausible wrong picture because nothing crashed. It also rules
out sanitising bad values in the composite: a slot at gain zero must be **skipped**, not multiplied,
because `0.0 * NaN` is `NaN` and one NaN spreads over the whole additive mix — and pulling a fader
to zero is a VJ's last resort for escaping broken material, so it has to work on exactly the
material that is broken.

**And what is dropped is announced.** Loading a Set file, the format is per layer and the engine per
Set, and they differ in three places — the layer of `seed` / `capacity` / `param`, vector `param`s,
and a `camera` record carrying two of `Orbit`'s six fields. Each one prints on load, because
half-applied and silent is the same failure wearing different clothes.

**Where a hole cannot be closed, name it.** `mem::forget` on a frame guard leaves `t`, parity and
the element buffers permanently out of step and is not closable without redesigning `Set`, so it is
listed in the module's "what this does not enforce" section along with the cases that were checked
and found harmless.

**Where it holds.** [deck.rs](../../crates/karakuri-engine/src/deck.rs). Decided in
[ADR-0040](../adr/0040-a-gain-of-zero-means-no-contribution-so-the-slot-is-skipped.md) and
[ADR-0042](../adr/0042-a-silently-wrong-image-loses-to-a-loud-failure.md), extended by
[ADR-0066](../adr/0066-a-flag-becomes-a-record-writer.md).
