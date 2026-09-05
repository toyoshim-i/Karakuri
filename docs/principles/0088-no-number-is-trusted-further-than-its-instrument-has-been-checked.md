# No number is trusted further than its instrument has been checked

To tell whether an instrument is lying, measure the same work a second way **whose bias direction you
know**: the host clock includes submit and synchronisation, so it is an upper bound on GPU time.
**A measurement carries how it was taken**, so nothing receives a number whose meaning it cannot
check, and **what cannot be measured is offered as an offset rather than as a number** — buffer
length, window length and queue depth are known, and the display's own latency is a dial the operator
tunes by ear. **This machine is evidence about this machine.**

**What it rules out.** A threshold: low lets garbage through, high rejects a genuinely fast machine,
and the lying measurement this replaced reported 0.095 ms for work taking tens of milliseconds — two
million points at size 40 measuring lighter than sixty-four points — missing a constant floor of
0.1 ms by **five microseconds**, on a floor already raised once from `> 0` for the same reason.
`gpu_ns >= host_ns / 4` is the same test on a fast machine and a slow one. Relaxing that threshold
until the reports stop. Treating one verdict as a lifetime guarantee: calibration passed and a later
measurement lied, so a caught probe is pinned to the fallback for its life with earlier numbers
redone, because a governor summing two scales cannot say which is which. Trusting a feature flag:
this adapter advertises `TIMESTAMP_QUERY`, and a load that cannot be zero — a million invocations by
two thousand trigonometric iterations — returned literal zeros through both query paths, then
plausible values, zeros and negatives run to run; falling back silently is the failure this exists to
prevent, because `0.0 ms` reads as a very fast shader and gets promoted. A check the toolchain is
permitted to discard: WGSL explicitly allows an implementation to assume NaN does not occur, and
Metal's default fast-math folds such a comparison to `true`, so the value the check excludes rejoins
the sum on a shipping backend with the suite green here — written instead as a bitcast of the
exponent, integer work float optimisation cannot touch, and noted that NaN containment through
`clamp` is Metal's answer, not WGSL's. And using the development machine as evidence, which has been
done twice: once concluding GPU timestamps work, once concluding a wasteful element layout could wait
because four resident slots fit here — where under `std430` only scalars are recoverable, 80 B to
64 B for `drift_shell`, a fifth and not a half, and one `amplify 64` stage adds 1.25 GiB by
multiplying exactly the stride the layout shrinks.

**Where it holds.** [probe.rs](../../crates/karakuri-engine/src/probe.rs) is most of it.
`MeasurementMethod` rides on every `Measurement`, so a consumer can see which clock answered;
`Probe::plausible` is `host_ns < PLAUSIBILITY_FLOOR_NS || gpu_ns * PLAUSIBILITY_RATIO >= host_ns` —
the second measurement rather than a constant, and the floor is what keeps it off the side where a
wrong answer is harmless; `run` takes `&mut self` because a probe caught lying sets
`self.method = MeasurementMethod::HostWallClock` and stays there, the two scales not being
comparable in a budget that sums them; and the reading that got through, 0.095 ms against 60 ms, is
in the test beside it. The toolchain half is in the shaders:
[meter.wgsl](../../crates/karakuri-engine/src/shaders/meter.wgsl)'s `is_light` is
`(bitcast<u32>(x) & 0x7f800000u) != 0x7f800000u` rather than a pair of float comparisons Metal's
fast-math may fold to `true`, and
[composite.wgsl](../../crates/karakuri-engine/src/shaders/composite.wgsl) bounds coverage and colour
with comparisons rather than `clamp`, because WGSL leaves `min`/`max` indeterminate on a NaN — with
the note beside it that this is an argument rather than a measurement on this machine, since Metal's
suppress. [meter.rs](../../crates/karakuri-engine/src/meter.rs) states its lag as inferred from the
queue depth rather than measured, because measuring it needs a surface.
[roadmap.md](../roadmap.md)'s *Settled decisions* carries 4 GiB as the reference a design is checked
against rather than a ceiling, and [contributing.md](../contributing.md) §1 says every performance
figure in this repository is host-side and biased high, and names the one workload they are all
taken at. Decided in [ADR-0015](../adr/0015-a-measurement-carries-how-it-was-taken.md),
[ADR-0056](../adr/0056-beat-lock-is-feed-forward-and-the-unmeasurable-part-is-an-offset.md),
[ADR-0068](../adr/0068-a-timestamp-is-checked-against-a-second-measurement-not-a-constant.md),
[ADR-0070](../adr/0070-a-channel-nobody-reads-is-a-free-variable.md),
[ADR-0071](../adr/0071-there-is-no-panic-key.md),
[ADR-0110](../adr/0110-this-machine-is-not-the-reference.md) and
[ADR-0169](../adr/0169-the-timestamp-verdict-is-the-backends-not-the-machines.md), which narrows the
last of those from a machine to a backend: the same binary and driver clear calibration 23 times in
25 on DX12 and 0 in 25 on Vulkan, and DX12 costs 3.5× the frame, so the clock is not bought by
changing what is being timed.
