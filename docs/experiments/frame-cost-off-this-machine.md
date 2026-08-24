# Frame cost off this machine

**You are reading a protocol, not a document about the project.** It asks you to run one program on
hardware this project has never run on, write down what it says, and change nothing. It exists
because two of the questions behind a design decision **cannot be answered on the machine this was
written on**, and both of them are about your machine rather than about the code.

When the numbers come back they go into the records this file names, and **this file is deleted**.
It is not a document that accumulates.

---

## What the program is

`karakuri-console` is the panel of a real-time visual instrument: a window divided into regions —
a transport, three columns of bays, an outputs strip — drawn with `egui` over `wgpu`, with one of
those regions showing a frame the engine rendered. It is an example rather than an application:

```sh
cargo run -p karakuri-console --example panel
```

It opens a window, prints a legend of every region and the keys, runs for as long as you leave it,
and prints a cost readout as it goes. `esc` quits. **The readout is the whole point of this
exercise** — you do not have to understand the panel, only run it and read what it says.

## The two questions this machine cannot answer

**1. How much of the frame is the cost of moving data to the GPU?**

This was measured on an Apple M4 Pro, which has **unified memory**: the CPU and GPU share one pool,
so uploading vertex data is close to a memory copy. On a machine with a discrete GPU the same bytes
cross a bus. Every number below is therefore biased in one direction, and **the design decision
waiting on it is whether to stop re-uploading parts of the panel that did not change** — which is
worth much more where an upload is expensive and is nearly free to skip where it is not.

So: `buffer uploads` in the split below is **0.166 ms here**. What is it on yours?

**2. Do GPU timestamp queries work?**

They do not work here, and the project has a standing rule about it: `docs/contributing.md` §1 says
every performance figure in this repository is a host-clock number, biased high, because
`wgpu::Features::TIMESTAMP_QUERY` is advertised and enabled on this machine and returns zero, a
negative delta, or occasionally something plausible. `crates/karakuri-engine/src/probe.rs`
calibrates against a known-heavy workload rather than trusting the feature flag, and falls back to
a host measurement **that says in its result that it did so**.

If they work on your machine, that is the single most valuable thing you can report, because it
means a whole class of measurement this project currently cannot take is available somewhere.

```sh
cargo test -p karakuri-engine probe -- --nocapture
```

Report whether the probe's calibration trusted the timestamps or fell back, and what it printed.

---

## Before anything: does it build?

**This has never been built on Windows.** If it does not, that is a result and not a failure of
yours — write down exactly what failed and stop. Do not fix it broadly, do not add platform
branches, do not change dependency versions.

```sh
cargo build --workspace
cargo test --workspace
```

The suite takes about a minute here and roughly a quarter of it needs a GPU. If tests fail, say
which and what they said. A test that fails only on your platform is worth more to this project
than the measurements are.

---

## The run

```sh
cargo run -p karakuri-console --example panel
```

Leave the window alone — **do not move the pointer over it, do not resize it** — for at least
thirty seconds, then quit with `esc`. Paste the whole readout.

Then do it twice more:

- **With the picture folded away.** Move the pointer over the large upper region of the middle
  column and press `f`. The engine's picture disappears and the panel stops redrawing. Leave it
  thirty seconds and read the "still panel" line.
- **While the machine is busy.** Run something CPU-heavy on other cores and repeat the first run.
  This one is not optional and the reason is under *Traps*.

---

## What this machine said, for comparison

Apple M4 Pro, Metal, 1440x900 window, 1280x720 canvas, 262144 elements, `PresentMode::Fifo`,
183 sampled frames, host clock, dev profile with dependencies at `opt-level = 3`. Medians:

| part | ms | what it is |
|---|---|---|
| acquire | **14.713** | `get_current_texture`, of which 99.1% is the thread blocked |
| engine pass | 0.336 | recording the deck's passes |
| egui pass | 0.300 | building and tessellating the panel's shapes |
| upload+pass | 1.204 | everything from the uploads to the submission |
| — texture uploads | 0.000 | the font atlas is built once |
| — **buffer uploads** | **0.166** | **question 1** |
| — record the pass | 0.014 | |
| — submit | **1.001** | `encoder.finish` + `queue.submit`, and 99% of it is CPU |
| present | 0.044 | |
| period | 16.715 | 60 Hz |

**The whole frame is 2.025 ms of CPU inside 16.596 ms of wall: 12% work, 88% waiting for vsync.**

Two things that were established here and are worth checking against yours rather than assuming:

- **`submit` is work, not waiting.** On this backend it is flat across canvases from 320x180 to
  5120x2880 — a 256-fold range of pixel work — so it scales with what was *recorded* rather than
  with what the GPU does. It is `wgpu`'s per-submission bookkeeping. Whether that holds on DX12 or
  Vulkan is a real question.
- **The vsync wait lives entirely in `get_current_texture`** and appears in none of the other
  numbers. On Metal this is `nextDrawable`, which blocks when no drawable is free rather than
  issuing an asynchronous request. Where it lives on your backend is worth saying.

---

## Traps

**The CPU frequency confound, and this one nearly produced a false result here.** Switching to a
non-blocking present mode made the submission fall from 1.001 to 0.248 ms, which reads as the
present mode costing something — but *every* CPU number fell by the same factor. Loading the other
cores reproduced the whole drop with the present mode unchanged. **A loop that sleeps 88% of every
frame is measured on a downclocked core.** The magnitudes here are a function of the machine's power
state to within a factor of six; only the ratios survived every condition. That is why the busy-
machine run above is not optional, and why you should report the ratios as prominently as the
numbers.

**Everything measured is recording, not execution.** These are host clocks around code that records
GPU commands. What the GPU then does is invisible to them, which is exactly why question 2 matters.
Do not describe a number here as "how long the GPU took".

**Do not optimise anything.** Not the frame, not the ordering, not the panel. A number arrived at by
changing the thing being measured is not the number being asked for. If you see something wasteful,
write it down and leave it.

**Do not change what is drawn**, and do not change the canvas size, the window size or the present
mode except where this file asks you to.

---

## What to report

1. Whether it built, and anything that failed.
2. The probe's verdict on timestamp queries, verbatim.
3. The three readouts, pasted whole.
4. Your machine: CPU, GPU, whether the GPU is discrete, driver, OS build, and which `wgpu` backend
   the run chose.
5. **Buffer uploads against this machine's 0.166 ms**, and whether the split's proportions look like
   the table above or different.
6. Where the vsync wait lives on your backend.
7. Anything that surprised you.

Numbers without their conditions are worth very little to this project — `docs/principles/0012-a-measurement-carries-how-it-was-taken.md`
is a standing rule here. Say how each was taken, and say plainly what you could not measure.
