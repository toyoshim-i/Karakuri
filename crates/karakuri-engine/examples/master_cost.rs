//! **What the master chain costs, per pass and all together** — the
//! measurement `docs/contributing.md` §1 asks of a change that touches the
//! frame path, for
//! [ADR-0317](../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md).
//!
//! `examples/frame_cost.rs`'s harness, narrowed to one question: the same deck,
//! the same frame, at the same 1280x720, with the chain off and then with each
//! pass on and then with all three. The interesting number is the **difference**
//! — what a pass adds to a frame that was already being drawn — and the
//! interesting claim is that the first row and a build with no chain in it are
//! the same frame, which is `tests/master.rs`'s job and not this file's.
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

use karakuri_engine::deck::Deck;
use karakuri_engine::master::{Chain, Cut};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::{Gpu, Present, Set};
use karakuri_ir::typed::Checked;

/// **The final output size**, which the mix is composited once at and which
/// every output is a resize of (ADR-0247) — and the size every figure in this
/// file is about, because a pass's cost is per output texel.
const OUTPUT: (u32, u32) = (1280, 720);

const L1: &str = "examples/coil_vortex.kir";
const L4: &str = "examples/star_flares.kir";
const CAPACITY: u32 = 10_240;
const SLOTS: u32 = 4;
const FRAMES: usize = 120;
const WARMUP: usize = 20;
const STEPS: u8 = 1;

fn compile(path: &str) -> Checked {
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let proc = karakuri_ir::parse(&src).unwrap_or_else(|e| panic!("{path}: parse: {e:?}"));
    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{path}: check: {e:?}"))
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
/// is the chain's passes and the copy and nothing else. That is the number
/// P-0091 asks for — what a pass costs per frame — and it is the one a whole
/// frame cannot give: four slots and a composite are 4.6 ms of GPU on this
/// machine and the chain is a fraction of a millisecond, so a difference of
/// differences is inside the drift.
///
/// **The chain-off row is not zero and is not meant to be**: it is an encoder,
/// a submit and a poll with nothing recorded between them, which is exactly the
/// floor every other row is over.
fn run(gpu: &Gpu, deck: &mut Deck, present: &mut Present, chain: Chain) -> f64 {
    present.set_chain(&gpu.queue, chain);
    // One real frame into whatever the mix writes now, so the chain's entry
    // holds material rather than whatever the last setting left — and so a
    // feedback pass has a history that is a picture.
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

    let off = Chain::default();
    let settings: [(&str, Chain); 6] = [
        ("chain off (no pass recorded)", off),
        (
            "feedback 0.5, mix cut",
            Chain {
                feedback: 0.5,
                cut: Cut::Mix,
                ..off
            },
        ),
        (
            "feedback 0.5, exit cut",
            Chain {
                feedback: 0.5,
                cut: Cut::Exit,
                ..off
            },
        ),
        ("bloom 0.6", Chain { bloom: 0.6, ..off }),
        (
            "rgb shift 0.4",
            Chain {
                rgb_shift: 0.4,
                ..off
            },
        ),
        (
            "all three",
            Chain {
                feedback: 0.5,
                cut: Cut::Exit,
                bloom: 0.6,
                rgb_shift: 0.4,
            },
        ),
    ];

    // A cold pass over every setting first, so that no row pays for another
    // row's pipeline or cache state — the same reason `WARMUP` exists.
    for (_, chain) in settings {
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
    for (name, chain) in settings {
        let took = run(&gpu, &mut deck, &mut present, chain);
        let base = *baseline.get_or_insert(took);
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
    println!(
        "  memory: four frame-sized Rgba16Float targets at {}x{} is {:.1} MB, allocated at \
         build and at resize whatever the chain is set to.",
        OUTPUT.0,
        OUTPUT.1,
        4.0 * f64::from(OUTPUT.0) * f64::from(OUTPUT.1) * 8.0 / 1_048_576.0
    );
}
