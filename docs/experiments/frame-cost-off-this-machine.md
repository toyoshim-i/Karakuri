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

**The Windows run changed the program in three places, and you are running the changed one.** The
readout records the upload split again, because without it the second machine could not answer the
question this file was written to ask. And two of the four test failures it turned up were defects
rather than platform gaps and were fixed. All three are described where they belong — the split
under question 1, the defects in the catalogue at the end. Everything the *protocol* asks you to
leave alone, you should still leave alone.

---

## Where the questions stand

**1. How much of the frame is the cost of moving data to the GPU? — MEASURED ON TWO MACHINES,
and both of them have unified memory, so the question it was asked for is still open.**

The design decision waiting on it is **whether to stop re-uploading parts of the panel that did
not change.** That is worth a lot where an upload crosses a bus and nearly nothing where it does
not.

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
| **a discrete GPU** | **separate** | **Vulkan or DX12** | | **the row this is for** | | |

**The two rows that exist differ only in the backend, and together they say the upload is not what
it looked like.** Recording the pass costs the same on both — 0.014 against 0.011, near enough
identical — so the two backends agree about the work. The buffer upload differs by a factor of
seventeen between two machines that *both* share memory between CPU and GPU, which means **the M4
Pro's 0.166 ms cannot be the price of moving bytes across anything.** There is no bus in either
row for it to be the price of. It is what that backend's upload path costs.

What that leaves for the decision: on the Radeon 780M, not re-uploading the unchanged panel could
save at most 0.010 ms out of a 0.514 ms frame — 2% of the frame, which is itself 4.6% of a 90 Hz
second. On the M4 Pro the same change is worth 0.166 of 2.025 ms, or 8% of the frame. **A discrete
GPU is the row that says whether the number can be large for the reason originally suspected**, and
it is the strongest reason left to run this at all.

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
takes about 80 s, against roughly a minute on the M4 Pro. `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings` both pass.

**Four test targets failed on Windows; two of them are now fixed and two are not.** All four are
catalogued at the end of this file, with what changed for the two that were defects. Read that
section before reporting failures of your own, so the ones that are already known do not crowd out
the ones that are not. What you should still see fail: the ten `sh` tests, always, and the replay
tests, sometimes.

### If you are on Windows and starting from nothing

- **Rust.** `winget install Rustlang.Rustup` gets `stable-x86_64-pc-windows-msvc`. It needs a
  linker: Visual Studio Build Tools 2022 with the C++ toolchain, plus a Windows SDK. If both are
  already installed, nothing else is needed and nothing has to be configured.
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

| what the readout calls it | Apple M4 Pro, Metal, 60 Hz | AMD Radeon 780M, Vulkan, 90 Hz |
|---|---|---|
| engine pass | 0.336 ms | **0.071 ms** |
| egui pass | 0.300 ms | **0.147 ms** |
| upload+pass | 1.204 ms | **0.295 ms** |
| — of which texture uploads | 0.000 ms | **0.000 ms** |
| — of which buffer uploads | 0.166 ms | **0.010 ms** |
| — of which record the pass | 0.014 ms | **0.011 ms** |
| — of which submit | 1.001 ms | **0.273 ms** |
| waiting for vsync | 14.713 ms | **10.309 ms** |
| the whole frame, CPU | 2.025 ms | **0.514 ms** |
| frames a second | 60 | 89.3 |
| share of the second spent drawing | 12% | 4.6% |
| egui allocations a frame | 184 / 226.2 kB (ADR-0164) | 163 / 200.3 kB |

**The shape survived and the magnitudes did not.** On both machines the submission dominates
`upload+pass` and the vsync wait dominates everything; the Windows machine is about four times
cheaper per frame across the board, and `submit` — `wgpu`'s per-submission bookkeeping — fell
furthest, from 1.001 ms to 0.273.

**The vsync wait lives in `get_current_texture` on both.** On Metal that is `nextDrawable`; on
Vulkan the wait is inside the swapchain acquire the same way, and it appears in none of the other
numbers. 10.309 ms of an 11.1 ms period at 90 Hz.

**Two numbers the Windows run did not vary and the Apple one did.** `submit` is flat on Metal
across canvases from 320x180 to 5120x2880 — a 256-fold range of pixel work — so it scales with what
was *recorded* rather than with what the GPU does. Nothing here re-ran that sweep. It is worth doing
on a discrete GPU, where the two might finally come apart.

### Two rows that are still nobody's

The M4 Pro run measured `acquire` at 14.713 ms — `get_current_texture`, of which 99.1% was the
thread blocked — and `present` at 0.044 ms, in a 16.715 ms period. The readout prints the wait but
not `present`, and not the period. Neither was worth a field: `present` is the display's pace rather
than a cost, and the period is the refresh rate, which you are asked for anyway.

### Run 2, the picture folded — Radeon 780M

`fold: program-view is now folded`, then 270 frames in 3.0 s at 90.0 fps. engine 0.071, egui 0.148,
upload+pass 0.288, submit 0.265, vsync 10.321, whole frame 0.508 ms — taken before the upload
split landed. See the run 2 section above for why folding changes nothing.

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
