# Frame cost off this machine

**You are reading a protocol, not a document about the project.** It asks you to run one program
on hardware this project has not run on, write down what it says, and change nothing.

It has been run three times. The first machine is the one it was written on — Apple M4 Pro, Metal,
unified memory. The second is an AMD Radeon 780M under Windows: a different backend, and still
unified memory, because it is an APU. **The third is an NVIDIA RTX 2070 SUPER — discrete, with its
own memory across a PCIe bus — and it is the machine the file spent two runs asking for.**

**Question 1 is answered, and the answer is no.** Moving the panel's geometry to a discrete GPU
costs 0.022 ms a frame. That is twice what it costs on an APU that does not cross a bus at all,
and a seventh of what it costs on the M4 Pro, which also does not cross one. There is no bus
penalty to find. What that retires is in the next section, and it is the reason this file no longer
needs a fourth machine for the question it was written to ask.

Every number below carries the conditions it was taken under, because that is the only thing that
makes two of them comparable — `docs/principles/0012-a-measurement-carries-how-it-was-taken.md`.

**The Windows run changed the program in three places, and you are running the changed one.** The
readout records the upload split again, because without it the second machine could not answer the
question this file was written to ask. And two of the four test failures it turned up were defects
rather than platform gaps and were fixed. All three are described where they belong — the split
under question 1, the defects in the catalogue at the end. Everything the *protocol* asks you to
leave alone, you should still leave alone.

**The third run changed one word of the workspace, and it was this file's own instruction that was
broken.** `WGPU_BACKEND` never reached `Gpu::instance()`, so the backend comparison this file had
been recommending since the second machine had never once happened. Fixing it took
`new_without_display_handle_from_env()` in place of its one-word-shorter sibling, and **it turned
question 2 over**: GPU timestamps work on DX12 nine times in ten and have never worked on Vulkan.

Everything else it found, it wrote down rather than fixed. No new test failure. Two sentences the
readout prints that are true of one machine only, under *What the third machine found that is not a
measurement* — and one thing the fix leaves undone, which is that nothing here prints which backend
a run used.

---

## Where the questions stand

**1. How much of the frame is the cost of moving data to the GPU? — ANSWERED. It is 0.022 ms on a
discrete GPU, and the bus is not what it is made of.**

The design decision waiting on it was **whether to stop re-uploading parts of the panel that did
not change.** That is worth a lot where an upload crosses a bus and nearly nothing where it does
not — and now that a machine with a bus has run it, **it does not cross the bus expensively
either. Do not take the optimisation.** It is worth at most 0.022 ms of a 0.562 ms frame, which is
3.9% of the frame and 0.13% of a 60 Hz second, and it would buy that by adding a dirty-tracking
mechanism to a panel that currently has none.

**The readout prints the split now, and it did not when this file was written.** The `0.166 ms`
below came from temporary instrumentation in commit `62478fa`, which split `upload+pass` into its
parts, published the conclusion, and removed the instrumentation — so the second machine to run
this could not answer the question the file exists for. `Cost` in
[`crates/karakuri-console/examples/panel.rs`](../../crates/karakuri-console/examples/panel.rs)
carries the four parts again, and you get them by running the program.

| | memory | backend | texture uploads | **buffer uploads** | record the pass | submit |
|---|---|---|---|---|---|---|
| Apple M4 Pro | unified | Metal | 0.000 | **0.166 ms** | 0.014 | 1.001 |
| AMD Radeon 780M | unified | Vulkan | 0.000 | **0.010 ms** | 0.011 | 0.273 |
| **NVIDIA RTX 2070 SUPER** | **separate** | **Vulkan** | 0.000 | **0.022 ms** | 0.015 | 0.298 |
| **NVIDIA RTX 2070 SUPER** | **separate** | **DX12** | 0.000 | **0.064 ms** | 0.016 | 1.395 |

**The row that was supposed to be the large one is the second smallest.** The machine that has to
move bytes across a PCIe bus pays 0.022 ms to do it; the APU that moves them across nothing pays
0.010; the M4 Pro, which also moves them across nothing, pays 0.166 — **seven times the machine
with the bus.** Whatever the M4 Pro's 0.166 ms is, the ordering rules out the only thing it was
ever suspected of being.

Read down the other columns and the reason is plain. Recording the pass costs 0.014, 0.011 and
0.015 on three different backends: the work is the same everywhere and everyone agrees about it.
`submit` on the discrete GPU is 0.298 against the APU's 0.273 — a machine of an entirely different
class, within 10%. **These numbers are a property of `wgpu` and the CPU, not of the GPU or of what
lies between them**, which is the same thing *Traps* says in one sentence and the whole table now
says in numbers.

**The last two rows are the same GPU and the same bus, and the upload still doubles.** 0.022 ms
under Vulkan, 0.064 under DX12, on hardware that did not change between them. If the number were
the price of moving bytes it could not do that. It is the price of a backend's upload path, which
is what the first two rows suggested and what only one machine with two backends could show.

What that settles for the decision: not re-uploading the unchanged panel saves at most 0.010 ms of
a 0.514 ms frame on the 780M, 0.166 of 2.025 ms on the M4 Pro, **0.022 of 0.562 ms here under
Vulkan and 0.064 of 1.943 under DX12** — 2%, 8%, 3.9% and 3.3% of a frame respectively. It is a
small share of the frame on every backend and every machine anyone has run this on, and the
suspicion that a discrete GPU would be where it finally mattered is refuted by the discrete GPU.
There is no further row that would change this: a bus was the mechanism the optimisation was
supposed to pay for, and the bus costs 0.012 ms over not having one.

**2. Do GPU timestamp queries work? — YES, ON DX12. The failure is the Vulkan backend, and three
machines mistook it for the hardware.**

On one machine, one driver and one build, with nothing changing but the backend:

| | `Probe` trusted the timestamps |
|---|---|
| `WGPU_BACKEND=dx12` | **23 of 25** |
| `WGPU_BACKEND=vulkan` | **0 of 25** |

That is the whole finding, and everything below it was written before it. **It could not have been
found until now**, because `WGPU_BACKEND` did not reach this codebase until the run that produced
this table — see *`WGPU_BACKEND` reaches this codebase now* under *The machines*. Every earlier
verdict in this file, on every machine, was Vulkan or Metal.

What it trusted, on the same workload the Vulkan runs fall back on:

```text
measured via GpuTimestamp:   light 0.008 ms, heavy 17.807 ms, ratio 2173.7x
measured via HostWallClock:  light 0.093 ms, heavy 17.087 ms, ratio  130.9x
```

The heavy figures agree to within 4%. The light one is a factor of twelve apart, and that gap is
the host clock's submit-and-wait overhead sitting on top of a workload too small to hide it — which
is exactly the shape the 780M saw in the three runs of thirty-six that got through, and the reason
this file has always said the timestamps are good when the calibration lets them past.

**This bears on a standing rule.** [`docs/contributing.md`](../contributing.md) §1 says every
performance figure in this repository is a host-clock number biased high, and that any change
touching performance owes a GPU-timestamp measurement that *does not currently work*. On this
machine it works, on DX12, nine times in ten. Whether that is worth having is a real decision and
not an obvious one, because of what the readout table shows DX12 costs — the frame is 3.5 times
more expensive there. **Measuring on one backend and shipping on another is its own trap**, and
nobody should take this as licence to quote a DX12 timestamp beside a Vulkan frame cost.

The rest of this section is the earlier, hardware-shaped reading of the same evidence. It is left
as it was written because it is how the wrong conclusion was reached, and the wrong conclusion was
reasonable.

---

**The earlier reading: "not reliably here either".**

They do not work on the M4 Pro, and the project has a standing rule about it:
[`docs/contributing.md`](../contributing.md) §1 says every performance figure in this repository is
a host-clock number, biased high.

On the Radeon 780M the adapter advertises `TIMESTAMP_QUERY`, `TIMESTAMP_QUERY_INSIDE_ENCODERS` and
`TIMESTAMP_QUERY_INSIDE_PASSES`, and `Probe`'s calibration **fails about nine times out of ten**.
Measured, rather than observed once: 36 constructions of a `Probe`, of which 3 trusted the
timestamps and 33 fell back to the host clock.

When it does get through, the timestamps are good. Two of the three passing runs printed

```text
measured via GpuTimestamp: light 0.015 ms, heavy 34.050 ms, ratio 2228.4x
measured via GpuTimestamp: light 0.016 ms, heavy 34.113 ms, ratio 2153.6x
```

against a host clock that saw `light 0.117–0.147 ms, heavy 34.2–34.6 ms` on the same workload. The
heavy figures agree to within a percent, and the light one is where the host clock's submit-and-wait
overhead shows. **So it is the calibration that fails, not the timestamps.** `Probe::calibrate`
requires all ten of `CALIBRATION_ATTEMPTS` to clear, so one bad bracket in ten sinks the run; a 90%
failure rate for the whole is roughly a 20% failure rate per bracket.

That is a finding about the mechanism rather than about the hardware, and **a third machine should
report the same fraction rather than a single verdict.** One run gave the opposite answer here.

**The third machine reported a fraction, and it is zero.** On the RTX 2070 SUPER the adapter
advertises all three features, exactly as the 780M does, and `Probe`'s calibration cleared **0 of
31 constructions** under Vulkan. Every one of them printed the fallback:

```text
measured via HostWallClock: light 0.131 ms, heavy 17.087 ms, ratio 130.9x
measured via HostWallClock: light 0.093 ms, heavy 18.227 ms, ratio 197.0x
measured via HostWallClock: light 0.121 ms, heavy 17.158 ms, ratio 142.0x
```

**It was the backend, and this is where that was nearly missed.** A second batch of 25 was taken
with `WGPU_BACKEND=dx12` set and cleared 0 of 25 — a clean, plausible, completely wrong DX12
result, because the variable was not reaching `Gpu::instance()` and those were 25 more Vulkan
constructions. Read together the two batches are **0 of 56 on Vulkan**, and the 780M's 3 of 36 is
the only time any Vulkan run has ever got through.

Had that batch been believed, this file would now say that timestamps fail on both backends, on the
strength of a comparison that never happened. It took `0 of 25` looking *too much* like `0 of 31`
to prompt the check. **A no-op that returns the expected answer is the hardest kind to notice.**

**So the rule holds on three machines and the fallback is doing its job.** Nothing here should be
read as "the timestamps are wrong": the 780M showed they are good when the calibration lets them
through, and what has never once worked is the gate in front of them. If anyone revisits
`Probe::calibrate` — and the ten-consecutive-brackets requirement at `CALIBRATION_ATTEMPTS` is the
obvious place — **this file is where the three fractions are**, and a change that does not move
0/31 and 0/25 has not fixed anything.

---

## What this file still owes

Both questions it was written to ask are now answered, and
[`docs/contributing.md`](../contributing.md) §6 says what happens next: the numbers go into a
record and **this file is deleted.** That has not been done, because it is a maintainer's decision
and because three things are still open. In rough order of what they cost:

- **The record.** Question 1 retired a plausible alternative — *stop re-uploading the parts of the
  panel that did not change* — which is precisely what §4 says an ADR is for. Three rows and the
  reason the third one refutes the second are the whole argument, and they are above. **Until that
  ADR exists, this file is the only place the decision is written down**, which is the situation
  the experiments directory is meant to be temporary about.
- **The replay fraction.** One clean run is not a fraction. About a minute; see failure 4.
- **The DX12 column, and whether it is wanted.** It cannot be taken as things stand:
  `WGPU_BACKEND` does not reach `Gpu::instance()`, so no run in this file has ever been on DX12.
  Making it possible is a one-word change to a function every GPU path goes through, which is a
  decision and not a chore — it has its own section under *The machines*.

Nothing above needs a fourth machine. The two prose defects under *What the third machine found
that is not a measurement* are separate, and are also a decision rather than a chore.

---

## What the program is

`karakuri-console` is the panel of a real-time visual instrument: a window divided into regions —
a transport, three columns of bays, an outputs strip — drawn with `egui` over `wgpu`, with one of
those regions showing a frame the engine rendered. It is an example rather than an application:

```sh
cargo run -p karakuri-console --example panel
```

It opens a window, prints a legend of every region and the keys, runs for as long as you leave it,
and prints a cost readout once. `esc` quits. **The readout is the whole point of this exercise** —
you do not have to understand the panel, only run it and read what it says.

**The readout is printed once and never again.** It fires three seconds after the window was last
touched by anything, and `Costs::said` makes it one-shot. Anything you do to the window resets that
three seconds, so there is no race to win — but there is also no second reading.

---

## Before anything: does it build?

```sh
cargo build --workspace
cargo test --workspace --no-fail-fast
```

**Use `--no-fail-fast`.** Without it `cargo test` stops at the first failing target and you learn
about one platform problem instead of all of them.

It builds clean on Windows: no errors and no warnings, 2m 33s cold on the 780M and 3m 49s cold on
the 2070 SUPER. The suite takes about 80 s on the 780M and 2m 31s on the 2070 SUPER including
compiling the test binaries, against roughly a minute on the M4 Pro. `cargo fmt --all -- --check`
and `cargo clippy --workspace --all-targets -- -D warnings` pass on both Windows machines.

**Four test targets failed on Windows; two of them are now fixed and two are not.** All four are
catalogued at the end of this file, with what changed for the two that were defects. Read that
section before reporting failures of your own, so the ones that are already known do not crowd out
the ones that are not. What you should still see fail: the ten `sh` tests, always, and the replay
tests, sometimes.

**The third machine saw exactly the ten and nothing else**: 1124 passed, 10 failed, 4 ignored, and
every one of the ten was `tempo_source::runner_tests::*` saying `` `sh`: program not found ``. Both
fixes hold on a third driver — the numbers are under the catalogue entries they belong to.

### If you are on Windows and starting from nothing

- **Rust.** `winget install Rustlang.Rustup` gets `stable-x86_64-pc-windows-msvc`. It needs a
  linker: Visual Studio Build Tools 2022 with the C++ toolchain, plus a Windows SDK. If both are
  already installed, nothing else is needed and nothing has to be configured. **2022 is not
  actually the floor** — the third machine linked the whole workspace with what it already had,
  Visual Studio Community 2019's MSVC 14.29.30133 and Windows SDK 10.0.16299 from 2017, and needed
  no download beyond rustup itself. Check `vswhere` for
  `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` before installing several gigabytes on the
  strength of a version number.
- **There is no `sh`.** Ten tests spawn one, and that is the first failure below.
- **Put `~/.cargo/bin` on the PATH the git hooks see, and reopen the shell after installing
  rustup.** `.githooks/pre-commit` pipes the staged file through `rustfmt` and judges it by whether
  anything came out — which is the right call for `rustfmt --check` reading stdin, since it prints
  its diff and still exits 0. But it captures stderr too, so a `rustfmt` that is not on PATH reads
  as `not formatted:` against a file that is perfectly formatted. Ten minutes went into a
  formatting problem that did not exist. Nothing here is Windows-specific except how easy it is to
  have a shell older than the toolchain.
- **`cargo run --example panel` builds the example first.** If you want to drive the window from a
  script, build it once with `cargo build -p karakuri-console --example panel` and then launch
  `target\debug\examples\panel.exe` directly: same profile, same binary, and the process you start
  is the process that owns the window, which `cargo run` is not.
- **Watch the display's refresh rate.** `PresentMode::Fifo` means the vsync wait — and therefore
  the frame period and the frames-a-second figure — is the display's, not the program's. The M4 Pro
  numbers are at 60 Hz and the ones below are at 90 Hz. Say what yours is.
- **Watch the DPI scale.** The window asks for 1440x900 *logical*. At 125% or 150% that is a
  physical window half again as large, on a display that may not fit it. Both runs so far were at
  100%. Say what yours is.

---

## The run

There is a driver script beside this file — [`panel-run.ps1`](panel-run.ps1) — that does all three
runs without touching the window by hand. It is Windows-only and it is a convenience, not the
protocol: doing it by hand is equally good and the script exists because a pointer that drifts over
the window invalidates the reading.

### Run 1 — untouched

```sh
cargo run -p karakuri-console --example panel
```

Leave the window alone — **do not move the pointer over it, do not resize it** — for at least
thirty seconds, then quit with `esc`. Paste the whole readout.

### Run 2 — with the picture folded away

Move the pointer over the picture and press `f`.

**Aim it from the panel's own report rather than by eye.** Press `p` first: it prints every region's
rectangle in the 1440x900 logical viewport, and `program-view` is the picture. The centre of that
rectangle, converted to screen coordinates, is where `f` has to land. Guessing "the large upper
region of the middle column" put the pointer in a gap twice here, and the panel said `fold: nothing
under the pointer`.

**What this run was written to show does not happen, and that is not a platform difference.** The
instruction used to say the panel stops redrawing. It does not: with `program-view` folded, this
machine still drew **270 frames in 3 s, at 90.0 frames a second**, and the engine pass was
unchanged at 0.071 ms against 0.072 unfolded. Two things in
[`panel.rs`](../../crates/karakuri-console/examples/panel.rs) say why, and neither is
platform-specific — `costs.live` is set to `true` once when the engine is created and is never set
back, and the `request_redraw()` at the end of every frame is unconditional. So the reading takes
its "live picture" branch whatever is folded, and the `still panel` wording is unreachable in this
build while an engine exists.

Run it anyway — it is thirty seconds, and a second platform agreeing is worth having — but **report
the frame count, not the wording.** If your machine prints `0 frames drawn`, that is a genuine
difference and it is the most interesting thing in your report.

### Run 3 — while the machine is busy

Load the other cores and repeat run 1. **This one is not optional and the reason is under
*Traps*** — though the trap turned out to be smaller here than on the machine that found it.

---

## What the two machines said

### The committed readout, both machines

Medians, host clock, dev profile with dependencies at `opt-level = 3`, 1440x900 logical window,
1280x720 canvas, 262144 elements, `PresentMode::Fifo`, 240 sampled frames, window untouched.

The Apple column is the run in commit `62478fa`; its four `upload+pass` parts were taken with
instrumentation that was removed, and the same numbers are in the finer table below with the two
rows that are still not printed anywhere.

| what the readout calls it | Apple M4 Pro, Metal, 60 Hz | AMD Radeon 780M, Vulkan, 90 Hz | RTX 2070 SUPER, Vulkan, 60 Hz | RTX 2070 SUPER, **DX12**, 60 Hz |
|---|---|---|---|---|
| engine pass | 0.336 ms | 0.071 ms | 0.076 ms | **0.323 ms** |
| egui pass | 0.300 ms | 0.147 ms | 0.146 ms | **0.142 ms** |
| upload+pass | 1.204 ms | 0.295 ms | 0.340 ms | **1.479 ms** |
| — of which texture uploads | 0.000 ms | 0.000 ms | 0.000 ms | **0.000 ms** |
| — of which buffer uploads | 0.166 ms | 0.010 ms | 0.022 ms | **0.064 ms** |
| — of which record the pass | 0.014 ms | 0.011 ms | 0.015 ms | **0.016 ms** |
| — of which submit | 1.001 ms | 0.273 ms | 0.298 ms | **1.395 ms** |
| waiting for vsync | 14.713 ms | 10.309 ms | 15.899 ms | **14.327 ms** |
| the whole frame, CPU | 2.025 ms | 0.514 ms | 0.562 ms | **1.943 ms** |
| frames a second | 60 | 89.3 | 59.3 | **59.7** |
| share of the second spent drawing | 12% | 4.6% | 3.3% | **11.6%** |
| egui allocations a frame | 184 / 226.2 kB (ADR-0164) | 163 / 200.3 kB | 163 / 200.3 kB | **163 / 200.3 kB** |

The two 2070 SUPER columns are the same machine, the same build and the same run of the protocol
forty seconds apart, with `WGPU_BACKEND` the only difference. They sampled 182 and 181 frames
rather than 240; the readout prints after three untouched seconds and at 60 Hz that is what three
seconds holds.

**`vulkan-1.dll` is absent from the DX12 run's module list** — `d3d12.dll`, `D3D12Core.dll`,
`dxgi.dll`, `DXGIDebug.dll` and `opengl32.dll`, and no Vulkan — where the Vulkan run loaded
`vulkan-1.dll` and `nvoglv64.dll`. That is the evidence the override took effect, and it is only
available because the override is real now; the warning further down about both runtimes loading
during enumeration was written when every backend was being initialised.

**The shape survived and the magnitudes did not.** On all three machines the submission dominates
`upload+pass` and the vsync wait dominates everything; the Windows machines are about four times
cheaper per frame than the M4 Pro across the board, and `submit` — `wgpu`'s per-submission
bookkeeping — fell furthest, from 1.001 ms to 0.273.

**The two Vulkan columns on Windows agree in a way they had no business agreeing.** An integrated
Radeon and a discrete GeForce two GPU generations and a whole product class apart land within 10%
on `submit`, within 4% on the egui pass, and *exactly* on the egui allocation count — 163 and
200.3 kB on both. Read on their own they say the readout is not measuring a GPU at all.

**The fourth column says what it is measuring instead: the backend.** Same GPU, same driver, same
binary, one environment variable — and the frame goes from 0.562 ms to 1.943, with `submit` from
0.298 to 1.395. That is a larger spread than anything between the two *machines*. Whatever `submit`
is, it is a property of the backend's driver-side bookkeeping, not of the silicon and not of the
CPU alone.

Which sharpens something the file has been circling since the M4 Pro. **Metal's 1.001 ms `submit`
and its 2.025 ms frame stop looking like Apple being slow** — they sit beside DX12's 1.395 and
1.943 on hardware that does the same work in 0.562 under Vulkan. Three backends, and Vulkan is the
outlier at the cheap end rather than Metal at the expensive one. Two columns of a single row remain
backend-independent across all four: `record the pass` at 0.014 to 0.016, and the egui allocation
count. Those are the CPU-side work. Everything else moves with the backend.

**The vsync wait lives in `get_current_texture` on all four.** On Metal that is `nextDrawable`; on
Vulkan and on DX12 the wait is inside the swapchain acquire the same way, and it appears in none of
the other numbers. 10.309 ms of an 11.1 ms period at 90 Hz on the 780M; 15.899 and 14.327 ms of a
16.7 ms period at 60 Hz on the 2070 SUPER's two backends, which is the same statement three more
times. The DX12 figure is the lower one for the obvious reason: the frame took 1.4 ms longer to
draw, so there was less of the period left to wait out. The two sum to the period either way.

**One number the Apple run varied and no Windows run did.** `submit` is flat on Metal across
canvases from 320x180 to 5120x2880 — a 256-fold range of pixel work — so it scales with what was
*recorded* rather than with what the GPU does. The discrete GPU was the machine where those two
might finally have come apart, and **the sweep was still not re-run there**: this protocol asks for
one canvas size and changing it is one of the things it tells you not to do. It is now a better
experiment than it was, because a backend that pays 1.395 ms for a submission has more room to show
a slope than one that pays 0.298. Whoever wants that answer should write it as its own experiment
file, and both 2070 SUPER columns are what it would be compared against.

### Two rows that are still nobody's

The M4 Pro run measured `acquire` at 14.713 ms — `get_current_texture`, of which 99.1% was the
thread blocked — and `present` at 0.044 ms, in a 16.715 ms period. The readout prints the wait but
not `present`, and not the period. Neither was worth a field: `present` is the display's pace rather
than a cost, and the period is the refresh rate, which you are asked for anyway.

### Run 2, the picture folded — Radeon 780M

`fold: program-view is now folded`, then 270 frames in 3.0 s at 90.0 fps. engine 0.071, egui 0.148,
upload+pass 0.288, submit 0.265, vsync 10.321, whole frame 0.508 ms — taken before the upload
split landed. See the run 2 section above for why folding changes nothing.

### Run 2, the picture folded — RTX 2070 SUPER

`fold: program-view is now folded`, then **179 frames in 3.0 s at 59.7 fps**. engine 0.072 against
0.076 unfolded, egui 0.142, upload+pass 0.309, buffer uploads 0.021, submit 0.268, vsync 15.942,
whole frame 0.523 ms.

**Not zero frames, and the engine pass is unchanged.** That is the third machine agreeing, and it
settles the question the run 2 instructions leave open: `costs.live` and the unconditional
`request_redraw()` are the reason, exactly as [`panel.rs`](../../crates/karakuri-console/examples/panel.rs)
says they are, and no platform has ever produced the `still panel` wording. **Whoever fixes that
should delete this run rather than keep asking three machines to confirm a fact about one `bool`.**

### Run 3, the machine loaded — Radeon 780M

14 of 16 logical processors spinning, everything else identical to run 1. **Both columns predate
the upload split**, which is why they sit a hair above the table further up — the same run, a
different build, and the difference is within what two runs of either build differ by.

| | idle | loaded |
|---|---|---|
| engine pass | 0.072 | 0.095 |
| egui pass | 0.149 | 0.195 |
| upload+pass | 0.307 | 0.397 |
| — of which submit | 0.284 | 0.367 |
| waiting for vsync | 10.291 | 10.065 |
| the whole frame | 0.528 ms | 0.687 ms |
| frames a second | 89.3 | 88.3 |

**The confound the M4 Pro found is not present here.** On that machine, loading the other cores
made every CPU number fall to about a sixth, because a loop that sleeps most of every frame is
otherwise measured on a downclocked core. Here loading the machine makes the numbers about 1.3
times *worse* — ordinary contention — and the proportions are unchanged. That does not retire the
trap; it says the trap belongs to that machine's power management. **Check it on yours; do not
assume either result.**

### Run 3, the machine loaded — RTX 2070 SUPER

14 of 16 logical processors spinning, everything else identical to run 1.

| | idle | loaded | |
|---|---|---|---|
| engine pass | 0.076 | 0.116 | 1.53x worse |
| egui pass | 0.146 | 0.242 | 1.66x worse |
| upload+pass | 0.340 | 0.554 | 1.63x worse |
| — of which buffer uploads | 0.022 | 0.028 | 1.27x worse |
| — of which submit | 0.298 | 0.500 | 1.68x worse |
| waiting for vsync | 15.899 | 15.426 | unchanged — it is the display |
| the whole frame | 0.562 ms | 0.912 ms | 1.62x worse |
| frames a second | 59.3 | 60.0 | unchanged |

**Second denial, and it is enough to name the trap's owner.** Loading this machine costs about 1.6
times, in the direction contention costs things, with the proportions between the numbers intact.
Two machines have now failed to reproduce the sixfold *improvement* the M4 Pro showed, so it is a
property of Apple's power management and not of the measurement. The vsync wait not moving while
everything around it does is the tell: it is the display's period, and the display did not care
that fourteen cores were spinning.

### The machines

**Apple M4 Pro**, Metal, macOS. Unified memory. 60 Hz.

**AMD Ryzen 7 8745HS w/ Radeon 780M Graphics**, 8 cores / 16 threads, 3.8 GHz max. Windows 11 Home
10.0.26200. 1920x1080 at 90 Hz (120 Hz capable), DPI scale 100%. Integrated GPU — **unified memory,
which is why it does not answer question 1** — driver 32.0.31036.15. rustc 1.98.0,
`x86_64-pc-windows-msvc`.

**wgpu chose Vulkan.** Not obvious and worth stating, because the panel does not print it and both
backends are present. `request_adapter(HighPerformance)` returns:

```text
Vulkan | AMD Radeon 780M Graphics | IntegratedGpu | AMD proprietary driver 26.6.1 (LLPC)
```

with these also visible and not chosen:

```text
Dx12 | AMD Radeon 780M Graphics    | IntegratedGpu | driver 32.0.31036.15
Dx12 | Microsoft Basic Render Driver | Cpu         | driver 10.0.26100.8972
Gl   | AMD Radeon 780M Graphics    | Other         | 4.6.0 Compatibility Profile Context 26.6.1.260716
```

`max_buffer_size` is 2.0 GiB, which matters for one of the failures below.

**Intel Core i7 w/ NVIDIA GeForce RTX 2070 SUPER**, 16 logical processors. Windows 11 Home
10.0.26200. 1920x1080 at **60 Hz**, DPI scale 100%, single active display. **Discrete GPU —
separate memory across a PCIe bus, which is the whole reason this run answers question 1** — driver
32.0.15.9186 (NVIDIA 591.86). rustc 1.98.0, `x86_64-pc-windows-msvc`, the same compiler as the
780M run.

**wgpu chose Vulkan here too.** `request_adapter(HighPerformance)` returns:

```text
Vulkan | NVIDIA GeForce RTX 2070 SUPER | DiscreteGpu | driver NVIDIA 591.86
```

with these also visible and not chosen:

```text
Dx12 | NVIDIA GeForce RTX 2070 SUPER | DiscreteGpu | driver 32.0.15.9186
Dx12 | NVIDIA GeForce RTX 2070 SUPER | DiscreteGpu | driver 32.0.15.9186
Dx12 | Microsoft Basic Render Driver | Cpu         | driver 10.0.26100.8972
Gl   | NVIDIA GeForce RTX 2070 SUPER/PCIe/SSE2 | Other | 4.6.0 NVIDIA 591.86
```

`max_buffer_size` is **4.0 GiB**, twice the 780M's, and failure 2 below is the reason to say so.

**Say which backend yours chose.** Nothing in the repository prints it, and on Windows the process
loads both backends' runtimes during enumeration, so the loaded DLLs do not answer it either — this
run watched them, and the folded run loaded `d3d12.dll`, `D3D12Core.dll` and `vulkan-1.dll`
together while running on Vulkan, which is that warning demonstrated rather than repeated. The way
to find out is the throwaway crate below.

### `WGPU_BACKEND` reaches this codebase now

**The line this section used to carry — "setting `WGPU_BACKEND` to `vulkan` or `dx12` forces the
other one" — was false for two machines, and the third is where it was first tried.** It has since
been made true, and **that one-word change is the only thing in the workspace this run altered.**
Everything the protocol asks you to leave alone was left alone.

`Gpu::instance()` in
[`crates/karakuri-engine/src/gpu.rs`](../../crates/karakuri-engine/src/gpu.rs) builds its
descriptor with `InstanceDescriptor::new_without_display_handle()`. wgpu 30 has a second
constructor one word longer — `new_without_display_handle_from_env()` — and **that is the one that
reads the environment.** The two are otherwise the same call, which is why nothing looks wrong.

Measured rather than read off the signature, on the RTX 2070 SUPER, both constructors in one
process:

```text
WGPU_BACKEND unset      new_without_display_handle          -> Vulkan
                        new_without_display_handle_from_env -> Vulkan
WGPU_BACKEND=dx12       new_without_display_handle          -> Vulkan   <-- what karakuri does
                        new_without_display_handle_from_env -> Dx12
WGPU_BACKEND=vulkan     new_without_display_handle          -> Vulkan
                        new_without_display_handle_from_env -> Vulkan
```

**`Gpu::instance()` now uses `new_without_display_handle_from_env()`.** One word, in a function
every GPU-touching path in the workspace goes through. The reasoning is in the commit; the short
version is that the two failure modes are not symmetrical. Ignoring the variable fails *silently
and identically to success* — the operator sets it, gets a plausible number, and writes down a
backend the run was never on, which is exactly what happened here and was caught only because
`0 of 25` looked too much like `0 of 31`. Honouring it fails *visibly*, because the backend is a
property of the adapter and every existing way of asking reports it truthfully.

The cost is real and worth stating: **a stray `WGPU_BACKEND` in a shell now silently moves every
measurement in this repository to another backend**, and the readout table above shows that is
worth a factor of 3.5. Nothing in the workspace sets the variable, so the exposure is to an
operator's own environment.

**What it leaves undone: nothing in the repository prints which backend a run used.** This makes
the override real; it does not make it visible. A DX12 measurement can still be written into a
table as though it were Vulkan, and the only defence is an operator remembering to ask separately —
which is the actual root of the failure above, since a no-op is undetectable precisely because the
readout is silent. The place it would cost least to fix is
[`panel.rs`](../../crates/karakuri-console/examples/panel.rs), whose readout is the thing operators
are asked to paste into reports. **That was not done**, because this file says not to change what
is being measured and one exception was already more than it wanted.

Until it is, the throwaway crate under *How to find out* is how you ask which backend a machine
picks, and the process's loaded modules now answer it too — see the note under the readout table.

### How to find out, without touching this workspace

Make a throwaway crate **outside the repository** — `cargo new adapterinfo`, with `wgpu = "30"` and
`pollster = "1"` — and put this in its `main.rs`. It asks `wgpu` the same question `Gpu::from_instance`
asks, and prints the limit that failure 2 below runs into.

```rust
fn main() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

    println!("== every adapter wgpu can see ==");
    for a in pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all())) {
        let i = a.get_info();
        println!("  {:?} | {} | {:?} | driver {} {}",
                 i.backend, i.name, i.device_type, i.driver, i.driver_info);
    }

    println!("\n== what request_adapter(HighPerformance) picks, which is what karakuri does ==");
    let chosen = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))
    .expect("an adapter");
    let i = chosen.get_info();
    println!("  {:?} | {} | {:?} | driver {} {}",
             i.backend, i.name, i.device_type, i.driver, i.driver_info);

    let f = chosen.features();
    println!("\n== the features the probe depends on ==");
    for (name, bit) in [
        ("TIMESTAMP_QUERY", wgpu::Features::TIMESTAMP_QUERY),
        ("TIMESTAMP_QUERY_INSIDE_ENCODERS", wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
        ("TIMESTAMP_QUERY_INSIDE_PASSES", wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES),
    ] {
        println!("  {name:<32} {}", f.contains(bit));
    }

    let l = chosen.limits();
    println!("\n  max_buffer_size {} bytes ({:.1} GiB)",
             l.max_buffer_size, l.max_buffer_size as f64 / 1073741824.0);

    // Both constructors, side by side, so `WGPU_BACKEND` can be caught doing
    // nothing. The first is what `Gpu::instance()` uses; only the second reads
    // the environment. Run it with the variable unset and again set to `dx12`.
    println!("\n== WGPU_BACKEND = {} ==",
             std::env::var("WGPU_BACKEND").unwrap_or_else(|_| "(unset)".into()));
    for (label, instance) in [
        ("new_without_display_handle",
         wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle())),
        ("new_without_display_handle_from_env",
         wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env())),
    ] {
        let a = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .expect("an adapter");
        println!("  {label:<38} -> {:?}", a.get_info().backend);
    }
}
```

Outside the repository on purpose: this is a question about your machine, not a feature of the
product, and the workspace should not grow a crate to answer it.

---

## Traps

**The CPU frequency confound.** On the M4 Pro, switching to a non-blocking present mode made the
submission fall from 1.001 to 0.248 ms, which reads as the present mode costing something — but
*every* CPU number fell by the same factor, and loading the other cores reproduced the whole drop
with the present mode unchanged. The magnitudes there are a function of the machine's power state
to within a factor of six. **On neither Windows machine does this reproduce** (see the two run 3
sections): the 780M gets 1.3 times worse under load and the 2070 SUPER 1.6 times worse, both of
them ordinary contention with the proportions intact. Two denials is enough to call it a property
of that machine's power management rather than of the measurement, but it is also the trap most
likely to make a fourth machine's numbers meaningless, so **run run 3 anyway and find out which
kind yours is.**

**Everything measured is recording, not execution.** These are host clocks around code that records
GPU commands. What the GPU then does is invisible to them, which is exactly why question 2 matters.
Do not describe a number here as "how long the GPU took".

**Do not optimise anything.** Not the frame, not the ordering, not the panel. A number arrived at by
changing the thing being measured is not the number being asked for. If you see something wasteful,
write it down and leave it.

**Do not change what is drawn**, and do not change the canvas size, the window size or the present
mode except where this file asks you to.

**Do not report one run of the probe.** The first probe run on the Radeon 780M reported
`GpuTimestamp` and the next twenty-five reported it three times. Run it enough to give a fraction.

---

## What to report

1. Whether it built, and anything that failed **that is not in the catalogue below**.
2. The probe's verdict on timestamp queries **as a fraction over at least twenty runs**, and what it
   printed when it trusted them.
3. The three readouts, pasted whole, each with its refresh rate and DPI scale.
4. Your machine: CPU, GPU, **whether the GPU is discrete**, driver, OS build, and which `wgpu`
   backend the run chose — and, if it has both, a run on each.
5. `upload+pass` minus `submit`, against **0.203 ms on the M4 Pro, 0.023 ms on the 780M and
   0.042 ms on the 2070 SUPER**. This is the closest thing to question 1 that the committed program
   can produce.
6. Where the vsync wait lives on your backend, and whether it is inside `get_current_texture`.
7. Whether run 3 moves the numbers up, down, or not at all.
8. Anything that surprised you.

---

## What the third machine found that is not a measurement

**The readout states two of the first machine's results as if they were facts about the world**,
and a discrete GPU is where they come apart. Both are prose in
[`panel.rs`](../../crates/karakuri-console/examples/panel.rs), both were printed verbatim by a
machine they are false on, and **neither has been changed** — this file says not to touch what is
being measured, and a sentence in the readout is part of it.

- `of which buffer uploads … the tessellated geometry, and close to a memory copy **on unified
  memory**`. The 2070 SUPER has no unified memory. The number it printed beside that clause,
  0.022 ms, is the measurement that answers question 1, and the clause explaining it names a
  property the machine does not have.
- `the identical run with this machine's other cores loaded reports **about a sixth** of these
  numbers, with the proportions between them unchanged`. Run 3 above reports 1.6 times *worse*, and
  the 780M reported 1.3 times worse. The claim is true of exactly one machine out of three, and it
  is printed unconditionally by all of them.

The second one is the sharper problem, because it is printed *as part of a measurement* and reads
as a caveat that has been checked. A reader on a fourth machine has no way to tell it apart from
the numbers above it, which is the failure mode
[`P-0012`](../principles/0012-a-measurement-carries-how-it-was-taken.md) exists to prevent — a
figure has to carry how it was taken, and a machine-specific conclusion printed by every machine
carries the opposite.

**What to do about it is a decision, not a fix**, which is why it is written down here rather than
edited away. The narrow version is to condition both clauses on what the run actually saw. The
wider one is that a readout should print measurements and let a document draw conclusions, and that
would remove several other sentences too.

---

## What Windows found that is not a measurement

Four test targets failed. **None of them was a frame-cost result** — they are here so the next
operator can tell a known failure from a new one, and because
[`docs/contributing.md`](../contributing.md) says a test that fails only on your platform is worth
more to this project than the measurements are.

**Two are fixed and two are not.** The two that are fixed were defects rather than platform gaps:
in both cases the code was wrong everywhere and only Windows made it show. The fixes are described
below with what changed, because you are running a build that has them and the maintainer reviewing
this will want to know what to look at.

### 1. `karakuri-cli`, 10 tests — there is no `sh` on Windows. NOT FIXED.

```text
thread 'tempo_source::runner_tests::...' panicked at `crates/karakuri-cli/src/tempo_source.rs` line 755:
open: "`sh`: program not found"
```

A test fixture, not the product. `try_fake` writes a script to a temp directory and spawns
`sh <script>`; `Source::open` itself takes any program name and assumes nothing about it. A POSIX
`sh` does exist on this machine — Git for Windows ships one — but it is not on `PATH`.

Left alone deliberately. Every way out is a design choice somebody should make on purpose: a
`.cmd` beside the `.sh` is a platform branch in a fixture, a helper binary is a second target in
the package, and re-entering the test executable as its own fake source is a mode a test binary
does not otherwise have. **Expect these ten to fail on Windows and do not report them as new.**

**The 2070 SUPER failed the same ten and no others**, with Git for Windows' `sh` present at
`C:\Program Files\Git\usr\bin\sh.exe` and off `PATH` — the same shape as the 780M, on a machine
set up from nothing. Two Windows machines have now paid for this, which is worth knowing to
whoever eventually decides which of the three ways out to take.

### 2. `karakuri-engine --test generated`, 1 test — the process died where a message was promised. FIXED.

`gpu::a_validation_error_at_build_is_returned_rather_than_fatal` aborted the whole test binary:

```text
memory allocation of 137438953440 bytes failed
process didn't exit successfully: ... (exit code: 0xc0000409, STATUS_STACK_BUFFER_OVERRUN)
```

`137438953440` is `u32::MAX × 32`: the element stride times the capacity the test deliberately asks
for. The backtrace put it in `Set::build` → `Set::build_many` → `Simulation::initialize` →
`initial_state`, at a `vec![0u8; capacity as usize * stride]` — **a host allocation sized from the
requested capacity, taken before the device's refusal had become a value.** The adapter's
`max_buffer_size` is 2 GiB, so the device had already refused the buffers and put the error in
`build_many`'s scope; nothing had read it yet.

macOS survives this because a 128 GiB `alloc_zeroed` is a lazy reservation there. Windows commits
and the allocator aborts. So the net this test exists to prove — that a capacity past the device is
a diagnostic and not a dead process — did not hold, and had not held anywhere: macOS was reaching
the right answer by way of an allocation it should never have made.

**The fix** is in
[`crates/karakuri-engine/src/node/simulation.rs`](../../crates/karakuri-engine/src/node/simulation.rs):
`Simulation::build` now asks `device.limits().max_buffer_size` before it allocates, records the
answer on the node, and `Simulation::initialize` returns without building the host image when the
device has already refused the buffers. `Simulation::build` stays infallible, which its own doc
comment argues for and which is still right — a capacity is refused by `capacity_in_range` before
anything is built, and a second `Err` in that constructor would be a second home for one refusal.

**The fix holds against a different limit, which is the confirmation worth having.** The 2070 SUPER
reports `max_buffer_size` of 4.0 GiB where the 780M reports 2.0, so the same 128 GiB request is
refused at a different threshold — and `generated.rs` passed 13 of 13. The fix reads the device's
own limit rather than a number that happened to work on the machine it was written on.

### 3. `karakuri-engine --test deck`, 1 test — an infinite alpha reached the mix as NaN. FIXED.

```text
gpu::an_alpha_that_is_not_a_coverage_cannot_invert_the_mix_or_nan_it
93630 colour channels of the mix are NaN under the infinite card
```

The test's own doc comment predicted a backend difference here and one arrived — though not at the
spelling it expected. It failed on the **infinite** card rather than the NaN one.

The saturation in `composite.wgsl` was not where it went wrong: `covered = opacity * select(0.0,
min(src.a, 1.0), src.a > 0.0)` handles an infinite alpha correctly. The NaN was in the slot target
before the mix was reached. The L4 pass blends with `src_factor: SrcAlpha` (see
[`crates/karakuri-engine/src/node/renderer.rs`](../../crates/karakuri-engine/src/node/renderer.rs)),
which is what makes the colour arrive premultiplied, the card is black, and `inf × 0` is a NaN. The
mix then multiplied that colour by a finite weight and it survived. **Colour, not coverage — and the
saturation only ever guarded coverage.**

**The fix** is one expression in `composite.wgsl`'s `layer`, beside the coverage saturation and
argued the same way: the colour is held to the finite on the way in, with comparisons rather than
`clamp` or a `min`/`max` pair, because WGSL leaves `min`/`max` indeterminate on a NaN operand and a
comparison against a NaN is false on every backend. It changes no finite value, and the deck's
bit-exactness tests — a deck of one reproducing a bare Set, and the same seeds compositing
identically — still pass.

**And the card that had never been reached now is.** The loop tests four spellings in order and
aborted at the third, so the NaN card's own behaviour on this backend was unknown when this failure
was first written down. With the fix in place all four pass, which is the first time any machine has
run that case against a driver that propagates NaN through fixed-function blending.

**A second such driver now agrees.** `deck.rs` passed 34 of 34 on the 2070 SUPER — NVIDIA 591.86,
a different vendor's Vulkan implementation of the same fixed-function blend. This is the fix worth
re-checking on every new machine, because unlike failure 2 it depends on what the driver does with
a NaN rather than on a limit it reports, and a backend that quietly flushed the NaN would let a
broken `composite.wgsl` pass without saying so.

### 4. `karakuri-cli --test replay`, intermittently — concurrent replays die silently. NOT FIXED.

Zero to three of the five fail per run, a different set each time, and **all five pass with
`--test-threads=1`**. Each test spawns `karakuri-cli` as a subprocess and each subprocess takes a
device of its own.

Reproduced outside the test harness: six concurrent `--store … --replay … --render` runs, repeated;
one round in two has one or two of them exit `0xC0000409` — `STATUS_STACK_BUFFER_OVERRUN` — **with
no Rust panic message and no output after the line announcing the render.** A Rust `abort` prints
first, so this looks like a fault detected in native code rather than a panic. Six concurrent
`--render` runs at 1920x1080 *without* the store and replay path did not reproduce it in the
attempts made.

Not diagnosed and not fixed. Serialising the tests would make the suite green while hiding the
thing worth knowing. **Report whether it happens on your driver**, since a wrong answer here is
"the test suite is flaky" and the right one may be "concurrent device use is not safe on this
driver".

**It did not happen on the 2070 SUPER — in one run, which is not a fraction and should not be read
as one.** `tests/replay.rs` passed 5 of 5 under the ordinary `cargo test --workspace`, with the
default thread count and no `--test-threads=1`. Against a failure the 780M sees zero-to-three times
out of five *per run*, a single clean run is weak evidence: it is exactly what a 780M run looks like
one time in several. **This is the open question the third machine did not close**, and closing it
costs about a minute — `cargo test -p karakuri-cli --test replay` twenty times, tallied. Whoever
runs it should put the fraction here beside the 780M's.
