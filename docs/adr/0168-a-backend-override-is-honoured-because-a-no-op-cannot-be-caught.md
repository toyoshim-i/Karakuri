---
id: 0168
title: A backend override is honoured, because a no-op cannot be caught
status: accepted
date: 2026-08-25
supersedes: []
superseded_by: []
principles: []
tags: [performance, process]
---

# A backend override is honoured, because a no-op cannot be caught

## Context

`Gpu::instance()` built its descriptor with `InstanceDescriptor::new_without_display_handle()`.
`wgpu` 30 has a sibling one word longer, `new_without_display_handle_from_env()`, and **only that
one reads `WGPU_BACKEND`**. Same call, same arguments, same type, at the call site.

So the override did nothing, and had done nothing for as long as a measurement protocol in this
repository had been telling operators to use it. **The third machine to run that protocol set
`WGPU_BACKEND=dx12`, took twenty-five `Probe` constructions, and wrote them down as a DX12
result. They were twenty-five more Vulkan constructions.** Nothing looked wrong: the variable was
set, the program ran, plausible numbers came out, and they were of the wrong backend. It was caught
only because *0 of 25* looked too much like the *0 of 31* taken beside it.

The instruction was written by somebody who never checked that the program honoured it, which is
the actual mistake and is not fixed by either answer below.

## Decision

**Honour it.** `Gpu::instance()` reads the environment.

**The alternative was to leave the code alone and delete the instruction**, on the grounds that a
shell variable is the worst possible way to change a measurement:
[P-0095](../principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md) says a measurement carries
how it was taken, and an environment variable does not travel with a number in a table. That cost
is real and is now real — a stray `WGPU_BACKEND` moves every figure in this repository to another
backend, which is worth a factor of 3.5.

**It loses on which failure can be found afterwards.** Ignoring the variable fails *silently and
identically to success*. Honouring it fails *visibly*, because the backend is a property of the
adapter and every existing way of asking reports it truthfully — `adapter.get_info().backend`, and
on Windows a loaded-module list that shows `d3d12.dll` with no `vulkan-1.dll` when the override
takes. A wrong answer produced this way can be found; one produced by a no-op cannot.

Nothing in the workspace sets the variable, so the exposure is to an operator's own shell, and an
operator who exports it has said what they want.

**What this leaves undone, and it is the root rather than a detail: nothing in the repository
prints which backend a run used.** This makes the override real, not visible. A DX12 measurement
can still be written into a table as though it were Vulkan. The no-op was undetectable *because the
readout is silent*, and the cheapest place to end that is the readout operators are asked to paste
into reports.

## Alternatives

**Delete the instruction and leave the code.** Above.

**Refuse to start when the variable is set.** Honest, and it throws away the one thing that made the
timestamp verdict in
[ADR-0169](0169-the-timestamp-verdict-is-the-backends-not-the-machines.md) findable — two backends
on one machine, one binary, one driver.
