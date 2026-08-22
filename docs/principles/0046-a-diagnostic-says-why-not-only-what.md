# A diagnostic says why, not only what

Every hint states the constraint behind the rule, not merely the correction. *Loop bounds cannot
reference a param or an ambient — **cost estimation needs them fixed at parse time**.*

**What it rules out.** A hint that only repairs syntax. Given one, a model fixes its `for` statement
and produces **a procedure that compiles and cannot be afforded**. Given the reason, it concluded that
integrating a streamline per sample is incompatible with the cost model itself and switched method
entirely — to analytic curves with `seed` split into a strand id and a position along it.

**A diagnostic written for a human turned out to be a specification for a model.** All fifty-eight
hints are held to it.

**And the estimator's output should be a way forward, not only a refusal.** Naming `rot_y` as 18% of
an estimate is what turned a rejection into an optimisation.

**Where it holds.** `check.rs` and `cost.rs` in [karakuri-ir](../../crates/karakuri-ir). Decided in
[ADR-0086](../adr/0086-a-hint-says-why.md); see also
[P-0010](0010-a-rejection-carries-what-the-next-attempt-needs.md).
