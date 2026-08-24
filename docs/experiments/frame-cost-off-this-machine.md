# Frame cost off this machine

**You are reading a protocol, not a document about the project.** It asks you to run one program
on hardware this project has not run on, write down what it says, and change nothing.

It has been run twice now. The first machine is the one it was written on — Apple M4 Pro, Metal,
unified memory. The second is an AMD Radeon 780M under Windows: a different backend, and still
unified memory, because it is an APU. **The machine this now wants is a discrete GPU**, and the
reason is in the next section: two of the three variables have been separated and the third has
not.

Every number below carries the conditions it was taken under, because that is the only thing that
makes two of them comparable — `docs/principles/0012-a-measurement-carries-how-it-was-taken.md`.

---

## Where the questions stand

**1. How much of the frame is the cost of moving data to the GPU? — STILL OPEN, and it is now
blocked on something other than hardware.**

The design decision waiting on it is **whether to stop re-uploading parts of the panel that did
not change.** That is worth a lot where an upload crosses a bus and nearly nothing where it does
not.

The trouble is that the committed readout does not print an upload figure. The `0.166 ms` quoted
below came from temporary instrumentation in commit `62478fa`, which split `upload+pass` into its
parts, published the conclusion, and removed the instrumentation. `Cost` in
[`crates/karakuri-console/examples/panel.rs`](../../crates/karakuri-console/examples/panel.rs)
carries five numbers today — `engine`, `ui`, `paint`, `submit`, `wait` — and none of them is the
buffer upload. **So this question cannot be answered by running the panel as it stands, on any
machine.** Somebody has to decide whether to record that split again first. Until then, what the
next machine can contribute to question 1 is the `upload+pass` total minus `submit`, which bounds
it from above.

What separating the two variables has bought so far:

| | memory | backend | `upload+pass` − `submit` |
|---|---|---|---|
| Apple M4 Pro | unified | Metal | 0.203 ms (of which 0.166 was the buffer upload) |
| AMD Radeon 780M | unified | Vulkan | 0.023 ms |
| **a discrete GPU** | **separate** | **Vulkan or DX12** | **the number this is for** |

The first two rows differ only in the backend, and the whole non-submit part of `upload+pass` fell
by a factor of nine. The third row is what says whether that is about the bus or about the driver.

**2. Do GPU timestamp queries work? — ANSWERED, and the answer is "not reliably here either".**

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

It builds clean on Windows: no errors and no warnings, 2m 33s cold on the machine below. The suite
takes about 80 s, against roughly a minute on the M4 Pro.

**Four test targets fail on Windows.** They are catalogued at the end of this file. Read that
section before reporting failures of your own, so the ones that are already known do not crowd out
the ones that are not.

### If you are on Windows and starting from nothing

- **Rust.** `winget install Rustlang.Rustup` gets `stable-x86_64-pc-windows-msvc`. It needs a
  linker: Visual Studio Build Tools 2022 with the C++ toolchain, plus a Windows SDK. If both are
  already installed, nothing else is needed and nothing has to be configured.
- **There is no `sh`.** Ten tests spawn one, and that is the first failure below.
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

| what the readout calls it | Apple M4 Pro, Metal, 60 Hz | AMD Radeon 780M, Vulkan, 90 Hz |
|---|---|---|
| engine pass | 0.336 ms | **0.072 ms** |
| egui pass | 0.300 ms | **0.149 ms** |
| upload+pass | 1.204 ms | **0.307 ms** |
| — of which submit | 1.001 ms | **0.284 ms** |
| waiting for vsync | 14.713 ms | **10.291 ms** |
| the whole frame, CPU | 2.025 ms | **0.528 ms** |
| frames a second | 60 | 89.3 |
| share of the second spent drawing | 12% | 4.7% |
| egui allocations a frame | 184 / 226.2 kB (ADR-0164) | 163 / 200.3 kB |

**The shape survived and the magnitudes did not.** On both machines the submission dominates
`upload+pass` and the vsync wait dominates everything; the Windows machine is about four times
cheaper per frame across the board, and `submit` — `wgpu`'s per-submission bookkeeping — fell
furthest, from 1.001 ms to 0.284.

**The vsync wait lives in `get_current_texture` on both.** On Metal that is `nextDrawable`; on
Vulkan the wait is inside the swapchain acquire the same way, and it appears in none of the other
numbers. 10.291 ms of an 11.1 ms period at 90 Hz.

### The finer split, M4 Pro only

Taken once with temporary instrumentation, at 1440x900 window, 183 sampled frames, and **removed
before it was committed**. It is here because it is the only place question 1 has ever been
measured, not because you can reproduce it.

| part | ms | what it is |
|---|---|---|
| acquire | 14.713 | `get_current_texture`, of which 99.1% is the thread blocked |
| engine pass | 0.336 | recording the deck's passes |
| egui pass | 0.300 | building and tessellating the panel's shapes |
| upload+pass | 1.204 | everything from the uploads to the submission |
| — texture uploads | 0.000 | the font atlas is built once |
| — **buffer uploads** | **0.166** | **question 1** |
| — record the pass | 0.014 | |
| — submit | 1.001 | `encoder.finish` + `queue.submit`, and 99% of it is CPU |
| present | 0.044 | |
| period | 16.715 | 60 Hz |

Established on the M4 Pro and worth checking rather than assuming:

- **`submit` is work, not waiting.** On Metal it is flat across canvases from 320x180 to 5120x2880
  — a 256-fold range of pixel work — so it scales with what was *recorded* rather than with what
  the GPU does. The Windows run is consistent with that (it did not vary the canvas), and it is
  four times cheaper.

### Run 2, the picture folded — Radeon 780M

`fold: program-view is now folded`, then 270 frames in 3.0 s at 90.0 fps. engine 0.071, egui 0.148,
upload+pass 0.288, submit 0.265, vsync 10.321, whole frame 0.508 ms. See the run 2 section above
for why folding changes nothing.

### Run 3, the machine loaded — Radeon 780M

14 of 16 logical processors spinning, everything else identical to run 1.

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

**Say which backend yours chose.** Nothing in the repository prints it, and on Windows the process
loads both backends' runtimes during enumeration, so the loaded DLLs do not answer it either.
Setting `WGPU_BACKEND` to `vulkan` or `dx12` forces the other one, which is worth a run each on a
machine that has both — on a discrete GPU that comparison is a second free result.

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
to within a factor of six. **On the Radeon 780M this does not reproduce** (see run 3), so it is a
property of a machine rather than of the measurement. Run run 3 and find out which kind yours is.

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
5. `upload+pass` minus `submit`, against **0.203 ms on the M4 Pro and 0.023 ms on the 780M**. This
   is the closest thing to question 1 that the committed program can produce.
6. Where the vsync wait lives on your backend, and whether it is inside `get_current_texture`.
7. Whether run 3 moves the numbers up, down, or not at all.
8. Anything that surprised you.

---

## What Windows found that is not a measurement

Four test targets fail. **None of them is a frame-cost result** — they are here so the next
operator can tell a known failure from a new one, and because
[`docs/contributing.md`](../contributing.md) says a test that fails only on your platform is worth
more to this project than the measurements are.

Nothing was fixed. All four were reproduced and characterised and left alone.

### 1. `karakuri-cli`, 10 tests — there is no `sh` on Windows

```text
thread 'tempo_source::runner_tests::...' panicked at crates\karakuri-cli\src\tempo_source.rs:755:17:
open: "`sh`: program not found"
```

A test fixture, not the product. `try_fake` writes a script to a temp directory and spawns
`sh <script>`; `Source::open` itself takes any program name and assumes nothing. A POSIX `sh` does
exist on this machine — Git for Windows ships one — but it is not on `PATH`.

### 2. `karakuri-engine --test generated`, 1 test — the process dies where a message was promised

`gpu::a_validation_error_at_build_is_returned_rather_than_fatal` aborts the whole test binary:

```text
memory allocation of 137438953440 bytes failed
process didn't exit successfully: ... (exit code: 0xc0000409, STATUS_STACK_BUFFER_OVERRUN)
```

`137438953440` is `u32::MAX × 32`: the element stride times the capacity the test deliberately asks
for. The backtrace puts it in `Set::build` → `Set::build_many` → `Simulation::initialize` →
`initial_state`, at the `vec![0u8; capacity as usize * stride]` in
[`crates/karakuri-engine/src/node/simulation.rs`](../../crates/karakuri-engine/src/node/simulation.rs)
— **a host allocation sized from the requested capacity, taken before the device's refusal has
become a value.** The adapter's `max_buffer_size` is 2 GiB, so the device would certainly refuse.

macOS survives it because a 128 GiB `alloc_zeroed` is a lazy virtual reservation there; Windows
commits and the allocator aborts. **So the net this test exists to prove — a capacity past the
device is a diagnostic and not a dead process — does not hold on Windows.** That is the test doing
its job.

### 3. `karakuri-engine --test deck`, 1 test — an infinite alpha reaches the mix as NaN

```text
gpu::an_alpha_that_is_not_a_coverage_cannot_invert_the_mix_or_nan_it
93630 colour channels of the mix are NaN under the infinite card
```

The test's own doc comment predicted a backend difference here, and one arrived — though not at the
spelling it expected. It failed on the **infinite** card, not the NaN one, and the loop aborts
there, so **the NaN card was never reached on this machine and its behaviour is still unknown.**

The saturation in `composite.wgsl` is not where it goes wrong. `covered = opacity * select(0.0,
min(src.a, 1.0), src.a > 0.0)` handles an infinite alpha correctly. The NaN is already in the slot
target: the L4 pass blends with `src_factor: SrcAlpha` (see
[`crates/karakuri-engine/src/node/renderer.rs`](../../crates/karakuri-engine/src/node/renderer.rs)),
the card is black, and `inf × 0` is NaN. The mix then multiplies that colour by a finite weight and
the NaN survives. Metal's blend hardware evidently does not produce it; AMD's Vulkan driver does,
which is IEEE-correct.

Colour, not coverage — and the saturation only ever guarded coverage. `renderer.rs` says clamping in
L4 was rejected because it would change the colour too; that argument is still sound, and this is a
case it did not cover.

**Worth asking your machine:** does the infinite card NaN the mix on your driver, and what does the
NaN card do once the infinite one stops aborting the loop?

### 4. `karakuri-cli --test replay`, intermittently — concurrent replays die silently

Zero to three of the five fail per run, a different set each time, and **all five pass with
`--test-threads=1`**. Each test spawns `karakuri-cli` as a subprocess and each subprocess takes a
device of its own.

Reproduced outside the test harness: six concurrent `--store … --replay … --render` runs, repeated;
one round in two has one or two of them exit `0xC0000409` — `STATUS_STACK_BUFFER_OVERRUN` — **with
no Rust panic message and no output after the line announcing the render.** A Rust `abort` prints
first, so this looks like a fault detected in native code rather than a panic. Six concurrent
`--render` runs at 1920x1080 *without* the store and replay path did not reproduce it in the
attempts made.

Not diagnosed further. **Report whether it happens on your driver**, since a wrong answer here is
"the test suite is flaky" and the right one may be "concurrent device use is not safe on this
driver".
