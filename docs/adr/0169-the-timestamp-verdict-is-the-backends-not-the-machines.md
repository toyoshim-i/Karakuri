---
id: 0169
title: The timestamp verdict is the backend's, not the machine's
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [performance]
---

# The timestamp verdict is the backend's, not the machine's

## Context

Every performance figure in this repository is a host-clock number, and
[`docs/contributing.md`](../contributing.md) §1 says why: GPU timestamp queries do not work.
`wgpu` advertises and enables `TIMESTAMP_QUERY`, a deliberately enormous workload resolves to zero
or to a negative delta, and `Probe` therefore calibrates against a known-heavy workload rather than
trusting the feature flag, falling back to a host measurement **that says so in its result**.

**Three machines wrote that down as a property of hardware.** Then one machine ran both of its
backends, with one binary and one driver:

- **DX12: calibration clears 23 of 25.**
- **Vulkan: 0 of 25.**

And on the machine this repository is developed on, Metal falls back **20 times out of 20**.

The Radeon run had already found the second half of it: when calibration does clear there, the
numbers are good — 34.05 ms against a host clock's 34.2–34.6 on the same workload, agreeing to
within a percent. So on that backend it is the calibration gate that fails and not the timestamps,
which is a different fault from Metal's, where the values themselves come back as zero.

## Decision

**The verdict is per backend, and §1 says so instead of naming a machine.** "Timestamps do not
work" was never true as stated; what is true is that they do not work on the two backends this
project is developed and run on.

**And we do not switch backends to get them.** DX12 is not free: same GPU, same driver, same
binary, one environment variable, and the frame goes from 0.562 ms to 1.943, with `submit` from
0.298 to 1.395. **A measurement taken on a backend the product does not use is a measurement of
something else** — it would buy a GPU clock by changing the thing being timed, which is the failure
every measurement rule here exists to prevent.

So `Probe`'s fallback stays, its label stays, and the host-clock rule stays — **as a fact about
Metal and Vulkan rather than about the world.** What changes is that it is now known to be
recoverable somewhere, which is worth having written down the day somebody needs a real GPU number
badly enough to pay 3.5× the frame for it.

**One old suspicion is retired on the way.** Metal's 1.001 ms `submit` had looked like Apple being
slow. DX12 pays 1.395 on hardware that does the same work in 0.298 under Vulkan — **Vulkan is the
outlier at the cheap end**, and Metal is unremarkable.

## Alternatives

**Leave §1 alone.** It reads as a hardware verdict and is false as one; three machines had already
copied it forward as a conclusion.

**Take measurements on DX12 from now on.** A working GPU clock is worth a great deal and this is
the only way anyone here has to get one. Rejected above: it measures a configuration nobody ships,
and the 3.5× spread means the numbers could not be put beside the existing ones — which is exactly
what §1's reference-workload rule exists to protect.

**Loosen `Probe`'s calibration**, since ten consecutive clears turns a per-bracket flake into a
near-certain overall failure. Tempting on Vulkan, where the numbers are good when they get through,
and wrong on Metal, where they are not. A gate loosened until the flaky backend passes is a gate
that lets the broken one through, and *flaky is worse than broken* is why the gate is strict.
