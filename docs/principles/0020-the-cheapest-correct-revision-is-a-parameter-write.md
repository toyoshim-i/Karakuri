# The cheapest correct revision is a parameter write

"A bit slower" is answered by trying three things in order: a **uniform write** if an existing
param's range covers it, a **range change and a fork** if the structure is the same, and
**regeneration** only if it is not. The first lands inside the frame — no fork, no compile, no
priming.

**What it rules out.** Treating every instruction as a generation request, which is the obvious
implementation, is orders of magnitude slower, and throws away the parts that were already right.

**This is what mandatory ranges are for.** Told that `speed` is `[0.0, 2.0]` and currently `0.15`,
a model can turn "slower" into `0.08`; without a range it can judge nothing and everything falls
through to code generation. The specification lists three jobs for a range — the fader's extent, an
agent's search space, the normalisation basis for a binding. This is the fourth.

**Where it holds.** [ir-spec.md](../ir-spec.md), `param`. Decided in
[ADR-0022](../adr/0022-revision-goes-param-then-range-then-regeneration.md).
