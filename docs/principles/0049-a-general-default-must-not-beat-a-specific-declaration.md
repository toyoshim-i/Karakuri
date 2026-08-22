# A general default must not beat a specific declaration

A flag overrides only when it is actually given. `.kir` declares `capacity [min, max] = default`, and
the range was honoured while **the declared default always lost to `--capacity`'s own default of
262144** — so a procedure written for 131072 ran at 262144 unless someone who knew said otherwise. The
same shape as `--size` silently deciding the canvas.

**What it rules out.** A default that outranks a statement someone made on purpose. A general option's
fallback is what to do when nobody said; a declaration *is* somebody saying.

**A limit is a declaration too, and it is calibrated for a shape.** The 512-op fragment ceiling stood
in for *how many fragments will there be* — capacity times area times overdraw. A fullscreen renderer
is exactly one canvas, once, so there was no unknown to stand in for, and the proxy did not constrain
ray marching, it **prohibited** it. **A ceiling calibrated for one shape rejects the next**, so give
each shape its own.

**Where it holds.** `capacity_for` in [karakuri-cli](../../crates/karakuri-cli); the fragment ceilings
in `cost.rs`. Decided in
[ADR-0090](../adr/0090-a-ceiling-calibrated-for-one-shape-rejects-the-next.md).
