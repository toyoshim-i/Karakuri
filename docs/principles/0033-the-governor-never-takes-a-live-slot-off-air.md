# The governor never takes a Live slot off air

It budgets from the cost the probe measured when the Set was built, it moves slots between residency
levels below Live, and it warns. **Deciding to load four heavy Sets is the operator's.**

**What it rules out.** Automatic demotion of something on screen. A governor pulling a slot down
during a performance is the worst behaviour it could have — there is no recovery from it and no way
to explain it to an audience. It also rules out steering on the deck's frame interval, which cannot
separate one slot's cost from its neighbours' — the defect a review had already found in the swap
watchdog, which using the same quantity here would have reproduced deliberately — and on the host
clock, which reacts to submission overhead as though it were the Set's cost.

**One probe, reused.** Calibration is expensive, and two probes can disagree about whether this
adapter's timestamps work at all, which would make their numbers incomparable — the one property a
governor summing them must not lack.

**Where it holds.** [governor.rs](../../crates/karakuri-engine/src/governor.rs). Decided in
[ADR-0054](../adr/0054-the-governor-budgets-from-the-probe-and-never-touches-a-live-slot.md).
