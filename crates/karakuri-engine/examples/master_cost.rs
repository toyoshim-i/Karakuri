//! **What the master chain costs, per slot and all together** — the
//! measurement `docs/contributing.md` §1 asks of a change that touches the
//! frame path, for
//! [ADR-0340](../../../docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md)
//! and beside
//! [ADR-0317](../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md)'s
//! figures for the three hand-written passes these replace.
//!
//! `examples/frame_cost.rs`'s harness, narrowed to one question: the same deck,
//! the same frame, at the same 1280x720, with the chain empty and then with
//! each shipped procedure in it alone and then with all three. The interesting
//! number is the **difference** — what a slot adds to a frame that was already
//! being drawn — and the interesting claim is that the first row and a build
//! with no chain in it are the same frame, which is `tests/master.rs`'s job and
//! not this file's.
//!
//! **Bloom is the row ADR-0340 owes a number for.** The 0.80 ms on record was
//! taken on the two-pass separable form; the shipped procedure is one 9x9
//! kernel, 81 fetches per texel against the pair's 19, because a chain slot's
//! output replaces the frame and the pair's second half needs both. This is
//! what that costs.
//!
//! **Host clock throughout, and it says so** — `frame_cost.rs`'s reasoning
//! verbatim: GPU timestamps do not survive `Probe::new`'s calibration on this
//! crate's development adapter (P-0095, ADR-0169), so the GPU figure is
//! `Device::poll` to a drained queue, biased high by the poll's own round trip.
//! There is no swapchain and no panel here, so what is printed is what the
//! frame would cost a loop that never waits.
//!
//! `cargo run -p karakuri-engine --example master_cost --release`
//! Run from the repository root: the `.kir` paths are relative to it.

use std::time::{Duration, Instant};

use std::collections::BTreeMap;

use karakuri_engine::deck::Deck;
use karakuri_engine::master::{Chain, Cut, Slot};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::{Gpu, Present, Set};
use karakuri_ir::typed::Checked;

/// **The final output size**, which the mix is composited once at and which
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

/// **The chain alone, timed as its own submission.**
///
/// The deck is drawn *outside* the timed stretch, once, so that what is timed
/// is the chain's passes and the retentions and nothing else. That is the
/// number P-0091 asks for — what a pass costs per frame — and it is the one a
/// whole frame cannot give: four slots and a composite are 4.6 ms of GPU on
/// this machine and the chain is a fraction of a millisecond, so a difference
/// of differences is inside the drift.
///
/// **The chain-off row is not zero and is not meant to be**: it is an encoder,
/// a submit and a poll with nothing recorded between them, which is exactly the
/// floor every other row is over.
fn run(gpu: &Gpu, deck: &mut Deck, present: &mut Present, chain: Chain) -> f64 {
    present.set_chain(&gpu.device, &gpu.queue, chain);
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

    // **Each list is made fresh**, because a `Chain` owns its slots' pipelines
    // and installing one moves it: what is being timed is a chain running, not
    // a chain being built, and building is what `Present::set_chain` is
    // measured as costing by the resize figure beside it (ADR-0325).
    // A named type because a list is built fresh per row — see below.
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
