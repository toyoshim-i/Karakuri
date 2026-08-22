# A check the compiler may assume away is not a check

A finiteness test written as a float comparison is not a test. **WGSL explicitly permits an
implementation to assume NaN does not occur**, and Metal's default fast-math folds such a comparison to
`true` — so the value the check exists to exclude rejoins the sum on a shipping backend, silently, with
the suite green. Write it as a **bitcast of the exponent**: integer work, which float optimisation
cannot touch.

**What it rules out.** Trusting a guard whose semantics the toolchain is allowed to discard, and
trusting a measurement of the guard taken on one backend. It also means a behaviour that happens to
hold here is labelled as such: NaN containment through `clamp` is **Metal's answer**, not WGSL's — WGSL
defines `clamp` by `min`/`max`, which propagate — so the code says it is an argument rather than a
measurement.

**Where it holds.** `meter.wgsl` and `composite.wgsl` in
[karakuri-engine](../../crates/karakuri-engine). Decided in
[ADR-0071](../adr/0071-there-is-no-panic-key.md).
