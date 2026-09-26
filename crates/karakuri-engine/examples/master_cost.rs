//! Measures frame cost of master chain slots individually and combined (ADR-0340).
//!
//! Usage: `cargo run -p karakuri-engine --example master_cost --release`
//! (Run from repository root; `.kir` paths are relative to root).

use std::time::{Duration, Instant};

use std::collections::BTreeMap;

use karakuri_engine::deck::Deck;
use karakuri_engine::master::{Chain, Cut, Slot};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::{Gpu, Present, Set};
use karakuri_ir::typed::Checked;

/// Target composite resolution for master output pass (ADR-0247).
/// every output is a resize of (ADR-0247) — and the size every figure in this
/// file is about, because a pass's cost is per output texel.
const OUTPUT: (u32, u32) = (1280, 720);

const L1: &str = "examples/coil_vortex.kir";
const L4: &str = "examples/star_flares.kir";

/// The three shipped procedures, compiled in so that a figure does not depend
/// on a working directory the way the deck's two do.
const FEEDBACK: &str = include_str!("../../../examples/feedback.kir");
const BLOOM: &str = include_str!("../../../examples/bloom.kir");
const RGB_SHIFT: &str = include_str!("../../../examples/rgb_shift.kir");
const CAPACITY: u32 = 10_240;
const SLOTS: u32 = 4;
const FRAMES: usize = 120;
const WARMUP: usize = 20;
const STEPS: u8 = 1;

fn compile(path: &str) -> Checked {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    check(&src)
}

fn check(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"))
}

fn slot(gpu: &Gpu, present: &Present, source: &str, cut: Option<Cut>, amount: f32) -> Slot {
    let params: BTreeMap<String, f32> = [("amount".to_string(), amount)].into_iter().collect();
    Slot::build(
        &gpu.device,
        present.chain_layout(),
        format!("example:{}", source.len()),
        &check(source),
        cut,
        params,
    )
    .expect("a shipped procedure is a legal chain slot")
}

fn deck_of(gpu: &Gpu, l1: &Checked, l4: &Checked) -> Deck {
    let swaps = (0..SLOTS)
        .map(|slot| {
            let set = Set::build(&gpu.device, &gpu.queue, l1, l4, CAPACITY, 19_274 + slot)
                .expect("the pair is compatible and the capacity is in range");
            HotSwap::fixed(set)
        })
        .collect();
    Deck::new(&gpu.device, swaps, OUTPUT.0, OUTPUT.1)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn median(xs: &mut [f64]) -> f64 {
    xs.sort_by(f64::total_cmp);
    xs[xs.len() / 2]
}

/// Measures execution time of master chain passes in an isolated submission.
fn run(gpu: &Gpu, deck: &mut Deck, present: &mut Present, chain: Chain) -> f64 {
    drop(present.set_chain(&gpu.device, &gpu.queue, chain));
    // One real frame into whatever the mix writes now, so the chain's entry
    // holds material rather than whatever the last setting left — and so a
    // retaining slot has a history that is a picture.
    for _ in 0..2 {
        let mut frame = deck.begin_frame(&gpu.device, &gpu.queue);
        frame.render(present.mix_target(), present.size(), STEPS);
        present.draw_chain(frame.encoder());
        frame.finish();
    }
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");

    let mut took: Vec<f64> = Vec::with_capacity(FRAMES);
    for i in 0..FRAMES {
        let started = Instant::now();
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        present.draw_chain(&mut encoder);
        gpu.queue.submit([encoder.finish()]);
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        if i >= WARMUP {
            took.push(ms(started.elapsed()));
        }
    }
    median(&mut took)
}

fn main() {
    let gpu = Gpu::headless().expect("no GPU available");
    let info = gpu.adapter.get_info();
    println!(
        "taken on {:?} — {} ({:?})",
        info.backend, info.name, info.device_type
    );

    let l1 = compile(L1);
    let l4 = compile(L4);
    let mut deck = deck_of(&gpu, &l1, &l4);
    let mut present = Present::new(&gpu.device, Present::HDR_FORMAT, OUTPUT.0, OUTPUT.1);

    // Fresh chains instantiated per configuration to isolate steady-state runtime cost.
    type Make = fn(&Gpu, &Present) -> Chain;
    let lists: [(&str, Make); 6] = [
        ("empty chain (no pass recorded)", |_, _| Chain::default()),
        ("feedback 0.5, mix cut", |gpu, present| {
            Chain::new(vec![slot(gpu, present, FEEDBACK, Some(Cut::Mix), 0.5)])
        }),
        ("feedback 0.5, exit cut", |gpu, present| {
            Chain::new(vec![slot(gpu, present, FEEDBACK, Some(Cut::Exit), 0.5)])
        }),
        ("bloom 0.6 (one 9x9 pass)", |gpu, present| {
            Chain::new(vec![slot(gpu, present, BLOOM, None, 0.6)])
        }),
        ("rgb shift 0.4", |gpu, present| {
            Chain::new(vec![slot(gpu, present, RGB_SHIFT, None, 0.4)])
        }),
        ("all three, exit cut", |gpu, present| {
            Chain::new(vec![
                slot(gpu, present, FEEDBACK, Some(Cut::Exit), 0.5),
                slot(gpu, present, BLOOM, None, 0.6),
                slot(gpu, present, RGB_SHIFT, None, 0.4),
            ])
        }),
    ];

    // A cold pass over every list first, so that no row pays for another row's
    // pipeline or cache state — the same reason `WARMUP` exists.
    for (_, make) in lists {
        let chain = make(&gpu, &present);
        run(&gpu, &mut deck, &mut present, chain);
    }

    println!();
    println!(
        "what the master chain's own passes cost at {}x{}, over {} submissions each \
         ({WARMUP} discarded cold):",
        OUTPUT.0,
        OUTPUT.1,
        FRAMES - WARMUP
    );
    let mut baseline = None;
    let mut ops = Vec::new();
    let mut targets = Vec::new();
    for (name, make) in lists {
        let chain = make(&gpu, &present);
        let price = chain.ops_per_fragment();
        let took = run(&gpu, &mut deck, &mut present, chain);
        let base = *baseline.get_or_insert(took);
        ops.push((name, price));
        targets.push((name, present.chain_targets()));
        println!(
            "  {name:<30} {took:6.3} ms   ({:+.3} ms over an empty submission, \
             {:.1}% of a 60 Hz frame)",
            took - base,
            100.0 * (took - base) / 16.667
        );
    }
    println!();
    println!(
        "  every figure is a median of an encoder, a submit and `Device::poll` to a \
         drained queue, host clock — so each carries one submission's overhead, which \
         is what the first row is. The deck is drawn outside the timed stretch: a whole \
         frame of four slots is an order of magnitude more than this and the difference \
         would be inside its drift."
    );
    println!();
    println!("  the estimate, before anything was built — ops per fragment:");
    for (name, price) in ops {
        println!("    {name:<30} {price:>6}");
    }
    println!();
    println!(
        "  memory: one frame-sized Rgba16Float target at {}x{} is {:.2} MB, and a chain \
         holds the entry, at most two to ping-pong between, and one per retained cut — \
         allocated when the list is installed and when the frame is resized, never when \
         a parameter moves.",
        OUTPUT.0,
        OUTPUT.1,
        f64::from(OUTPUT.0) * f64::from(OUTPUT.1) * 8.0 / 1_048_576.0
    );
    for (name, held) in targets {
        println!(
            "    {name:<30} {held} target{} — {:.2} MB",
            if held == 1 { "" } else { "s" },
            held as f64 * f64::from(OUTPUT.0) * f64::from(OUTPUT.1) * 8.0 / 1_048_576.0
        );
    }
}
