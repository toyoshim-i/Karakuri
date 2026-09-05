# An instrument that cannot measure says so rather than reporting a number

**A measurement carries how it was taken**, so nothing downstream receives a number whose meaning it
cannot check. **What cannot be measured is offered as an offset rather than as a number** — buffer
length, window length and queue depth are known, and the display's own latency is a dial the
operator tunes by ear. An instrument establishes for itself that its clock works, against a load
whose answer it already knows, rather than against what the platform says it supports; and once
caught lying it stays on the fallback for the rest of its life, because two scales cannot be summed
in one budget.

**What it rules out.** Trusting a feature flag: this adapter advertises `TIMESTAMP_QUERY`, and a
load that cannot be zero — a million invocations by two thousand trigonometric iterations —
returned literal zeros through both query paths, then plausible values, zeros and negatives run to
run. **Falling back silently is the failure this exists to prevent**, because `0.0 ms` reads as a
very fast shader and gets promoted. Treating one verdict as a lifetime guarantee: calibration passed
and a later measurement lied, so a caught probe is pinned to the fallback with its earlier numbers
redone, because a governor summing two scales cannot say which is which. A check the toolchain is
permitted to discard: WGSL explicitly allows an implementation to assume NaN does not occur, and
Metal's default fast-math folds such a comparison to `true`, so the value the check excludes rejoins
the sum on a shipping backend with the suite green here — written instead as a bitcast of the
exponent, integer work float optimisation cannot touch, and noted that NaN containment through
`clamp` is Metal's answer, not WGSL's. And handing an inferred number out as a measured one.

**Where it holds.** [probe.rs](../../crates/karakuri-engine/src/probe.rs) is most of it.
`MeasurementMethod` rides on every `Measurement`, so a consumer can see which clock answered;
`Probe::plausible` is the second measurement rather than a constant, and its floor is what keeps the
test off the side where a wrong answer is harmless; `run` takes `&mut self` because a probe caught
lying sets `self.method = MeasurementMethod::HostWallClock` and stays there, the two scales not
being comparable in a budget that sums them; and the reading that got through, 0.095 ms against
60 ms, is in the test beside it. The toolchain half is in the shaders:
[meter.wgsl](../../crates/karakuri-engine/src/shaders/meter.wgsl)'s `is_light` is
`(bitcast<u32>(x) & 0x7f800000u) != 0x7f800000u` rather than a pair of float comparisons Metal's
fast-math may fold to `true`, and
[composite.wgsl](../../crates/karakuri-engine/src/shaders/composite.wgsl) bounds coverage and colour
with comparisons rather than `clamp`, because WGSL leaves `min`/`max` indeterminate on a NaN — with
the note beside it that this is an argument rather than a measurement on this machine, since Metal's
suppress. [meter.rs](../../crates/karakuri-engine/src/meter.rs) states its lag as inferred from the
queue depth rather than measured, because measuring it needs a surface.
[swap.rs](../../crates/karakuri-engine/src/swap.rs)'s module documentation is where a governor
steering on an unlabelled number is named as the failure this prevents. Decided in
[ADR-0015](../adr/0015-a-measurement-carries-how-it-was-taken.md),
[ADR-0056](../adr/0056-beat-lock-is-feed-forward-and-the-unmeasurable-part-is-an-offset.md),
[ADR-0068](../adr/0068-a-timestamp-is-checked-against-a-second-measurement-not-a-constant.md),
[ADR-0070](../adr/0070-a-channel-nobody-reads-is-a-free-variable.md),
[ADR-0071](../adr/0071-there-is-no-panic-key.md) and
[ADR-0169](../adr/0169-the-timestamp-verdict-is-the-backends-not-the-machines.md), which narrows the
verdict from a machine to a backend: the same binary and driver clear calibration 23 times in 25 on
DX12 and 0 in 25 on Vulkan, so the probe's own answer is the only one that settles it.

**How to read a number this instrument produced** — check it against a second measurement whose bias
direction you know rather than against a constant, never relax a threshold until the reports stop,
and read this machine as evidence about this machine — is a discipline for whoever is working rather
than a property of the engine, and is `docs/contributing.md` §1, *Working style*. The reference
workload every host-clock figure in this repository is taken at is there too.
