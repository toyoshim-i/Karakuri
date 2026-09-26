//! Benchmarks CPU recording duration and GPU queue execution cost per frame.
//!
//! Run with: `cargo run -p karakuri-engine --example frame_cost --release`

use std::time::{Duration, Instant};

use karakuri_engine::deck::{Deck, DeckSlot, Residency};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::{Gpu, Present, Set};
use karakuri_ir::typed::Checked;

/// Final composited output resolution.
const OUTPUT: (u32, u32) = (1280, 720);

/// Target resolution used for preview cell probe benchmarking.
const CELL: (u32, u32) = (252, 142);

/// The pair a bare `cargo run -p karakuri` opens on (ADR-0271), which is what
/// the panel's own reading is taken over — and not the workspace's
/// reference workload, which is `examples/drift_cloud.kset` at 1280x720
/// (`docs/contributing.md` §1, ADR-0270).
const L1: &str = "examples/coil_vortex.kir";
const L4: &str = "examples/star_flares.kir";

/// `coil_vortex.kir`'s declared default capacity, read across rather than
/// transcribed would be better; it is a constant here because this harness
/// takes no command line.
const CAPACITY: u32 = 10_240;

/// Slots in the deck, which is what `karakuri` opens with.
const SLOTS: u32 = 4;

/// Frames timed. The first [`WARMUP`] are discarded: the first frame of a
/// process pays for pipeline and cache state every later one gets free.
const FRAMES: usize = 120;
const WARMUP: usize = 20;

/// The step count a frame is committed with. One is what a 60 Hz display
/// yields (ADR-0297).
const STEPS: u8 = 1;

fn compile(path: &str) -> Checked {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let proc = karakuri_ir::parse(&src).unwrap_or_else(|e| panic!("{path}: parse: {e:?}"));
    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{path}: check: {e:?}"))
}

fn deck_of(gpu: &Gpu, l1: &Checked, l4: &Checked, at: (u32, u32)) -> Deck {
    let swaps = (0..SLOTS)
        .map(|slot| {
            let set = Set::build(
                &gpu.device,
                &gpu.queue,
                l1,
                l4,
                CAPACITY,
                // A seed per slot, so four slots are four different clouds
                // rather than the same one drawn four times.
                19_274 + slot,
            )
            .expect("the pair is compatible and the capacity is in range");
            HotSwap::fixed(set)
        })
        .collect();
    Deck::new(&gpu.device, swaps, at.0, at.1)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn median(xs: &mut [f64]) -> f64 {
    xs.sort_by(f64::total_cmp);
    xs[xs.len() / 2]
}

fn main() {
    let gpu = Gpu::headless().expect("no GPU available");
    let info = gpu.adapter.get_info();
    println!(
        "taken on {:?} — {} ({:?})",
        info.backend, info.name, info.device_type
    );
    println!(
        "  the adapter advertises TIMESTAMP_QUERY: {}",
        gpu.timestamps
    );

    let l1 = compile(L1);
    let l4 = compile(L4);

    // Measure slot execution cost across output and preview resolutions.
    for at in [OUTPUT, CELL] {
        let mut deck = deck_of(&gpu, &l1, &l4, OUTPUT);
        // The deck's output size is always `OUTPUT`; only the size the
        // measurement is taken at moves, which is the change ADR-0303 made
        // possible and the reason the two numbers below are comparable at all.
        deck.set_measure_size(at);
        let started = Instant::now();
        let measured = deck.measure_slots(&gpu.device, &gpu.queue);
        let took = started.elapsed();
        println!();
        println!(
            "measuring {measured} slots at {}x{} took {:.1} ms, calibration included",
            at.0,
            at.1,
            ms(took)
        );
        println!("  the clock it earned: {:?}", deck.clock());
        for slot in 0..SLOTS as usize {
            match deck.slot(DeckSlot(slot as u8)).measured_cost() {
                Some(cost) => println!(
                    "  slot {slot}: {:.3} ms at {}x{}, {:?}",
                    cost.ms, cost.resolution.0, cost.resolution.1, cost.method
                ),
                None => println!("  slot {slot}: not measured"),
            }
        }

        // Compare budget measurements against governor estimates (ADR-0296).
        let estimated = deck.estimate_slots(&gpu.device, &gpu.queue);
        deck.set_residency(karakuri_engine::DeckSlot(1), Residency::Priming);
        if let (Some(committed), Some(warming)) = (
            deck.slot(karakuri_engine::DeckSlot(0)).measured_cost(),
            deck.slot(karakuri_engine::DeckSlot(1)).measured_cost(),
        ) {
            deck.set_compute_budget_ms(committed.ms + warming.ms / 2.0);
        }
        println!("  {estimated} estimated at the deck's own size, and then:");
        println!("  {}", deck.govern());
    }

    // -- what a whole frame costs ------------------------------------------
    let mut deck = deck_of(&gpu, &l1, &l4, OUTPUT);
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, OUTPUT.0, OUTPUT.1);

    let mut cpu: Vec<f64> = Vec::with_capacity(FRAMES);
    let mut gpu_ms: Vec<f64> = Vec::with_capacity(FRAMES);
    let mut period: Vec<f64> = Vec::with_capacity(FRAMES);
    let mut last: Option<Instant> = None;

    for i in 0..FRAMES {
        let began = Instant::now();
        if let Some(previous) = last {
            period.push(ms(began - previous));
        }
        last = Some(began);

        // CPU recording cost for deck slots and transport.
        // composite, and the submission that carries them. This is the
        // quantity `Cost::engine` is on the panel.
        let started = Instant::now();
        {
            let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
            frame.render(present.hdr_view(), present.size(), STEPS);
            frame.finish();
        }
        let recorded = started.elapsed();

        // Measure GPU execution time by polling until device queue drains.
        let owed = Instant::now();
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let drained = owed.elapsed();

        if i >= WARMUP {
            cpu.push(ms(recorded));
            gpu_ms.push(ms(drained));
        }
    }

    let mut period: Vec<f64> = period.split_off(WARMUP);
    println!();
    println!(
        "what one frame of a four-slot deck costs at {}x{}, over {} frames \
         ({} discarded cold):",
        OUTPUT.0,
        OUTPUT.1,
        cpu.len(),
        WARMUP
    );
    let cpu_median = median(&mut cpu);
    let gpu_median = median(&mut gpu_ms);
    let period_median = median(&mut period);
    println!("  CPU, recording and submitting  median {cpu_median:.3} ms");
    println!("  GPU, submit to a drained queue median {gpu_median:.3} ms");
    println!("  the frame's period             median {period_median:.3} ms");
    println!(
        "  so the GPU is {:.1}x the CPU, and a reading that stopped at the submission \
         would report {:.1}% of what the frame cost.",
        gpu_median / cpu_median,
        100.0 * cpu_median / period_median
    );
    println!(
        "  host clock throughout: `Instant` around the recording, and `Instant` around \
         `Device::poll` to a drained queue. Biased high by the poll's round trip, and \
         with no swapchain in it — see this file's module doc for what that leaves out."
    );
}
