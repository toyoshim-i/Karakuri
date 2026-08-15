//! Runs a beat-locked slot forward, scrubs it back, and shows that the rewound
//! frame **is** the earlier frame rather than something like it.
//!
//! Not a test — `tests/transport.rs` asserts the same property. What this adds
//! is that the claim becomes something a person can check: it writes the frame
//! it rewound to, the frame it had drawn at that step on the way past, and the
//! frame it was at before scrubbing, then says whether the first two are
//! identical. Three files, and the answer is in the pixels rather than in an
//! assertion nobody watched run.
//!
//! The transport has no CLI flag — it is on the `y` and `u`/`i` keys, which a
//! headless run cannot press — so this is the only way to see it without a
//! window and a keyboard.
//!
//! `cargo run -p karakuri-engine --example transport_scrub` writes PNGs under
//! `target/transport_scrub/`. Run from the repository root: the `.kir` paths
//! are relative to it.

use std::path::{Path, PathBuf};

use karakuri_engine::deck::Deck;
use karakuri_engine::swap::HotSwap;
use karakuri_engine::transport::Sync;
use karakuri_engine::{Gpu, Present, Set, Signals, TonemapOp};
use karakuri_ir::typed::Checked;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 640;
const CAPACITY: u32 = 262_144;
const SEED: u32 = 19_274;
const BPM: f32 = 120.0;

/// Frames to run forward before scrubbing. At 120 bpm and `dt = 1/60` a beat is
/// thirty steps, so this is four beats — long enough that the material has
/// visibly moved and short enough to watch.
const FORWARD: usize = 120;

/// How far back to scrub, in beats. Negative: this is the control that reverses.
const SCRUB: f64 = -2.0;

fn compile(path: &Path) -> Checked {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e} — run from the repository root", path.display()));
    let proc = karakuri_ir::parse(&src)
        .unwrap_or_else(|errs| panic!("{}", render(&errs, &src)));
    let checked = karakuri_ir::check::check(&proc)
        .unwrap_or_else(|errs| panic!("{}", render(&errs, &src)));
    karakuri_ir::cost::estimate(&checked)
        .unwrap_or_else(|errs| panic!("{}", render(&errs, &src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
}

/// One frame: step the deck, tone map, read back sRGB bytes.
fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present, view: &wgpu::TextureView, target: &wgpu::Texture) -> Vec<u8> {
    let bytes_per_row = WIDTH * 4;
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("transport scrub readback"),
        size: u64::from(bytes_per_row * HEIGHT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), 1);
    present.draw(f.encoder(), view, (WIDTH, HEIGHT));
    f.encoder().copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
    );
    f.finish();

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let pixels = data.to_vec();
    drop(data);
    readback.unmap();
    pixels
}

fn write_png(path: &Path, pixels: &[u8]) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    }
    let file = std::fs::File::create(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut png = png::Encoder::new(std::io::BufWriter::new(file), WIDTH, HEIGHT);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .and_then(|mut w| w.write_image_data(pixels))
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

fn main() {
    let gpu = Gpu::headless().expect("no GPU available");
    let out = PathBuf::from("target/transport_scrub");

    // `beat_shell` is closed form and reads `beats`, which is the combination
    // that takes beat sync and refuses tempo sync — see `karakuri-engine`'s
    // `transport` module.
    let l1 = compile(Path::new("examples/beat_shell.kir"));
    let l4 = compile(Path::new("examples/soft_points.kir"));
    let mut set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, CAPACITY, SEED)
        .expect("the pair is compatible");
    set.resize(&gpu.device, WIDTH, HEIGHT);

    let mut deck = Deck::new(&gpu.device, vec![HotSwap::fixed(set)], WIDTH, HEIGHT);
    deck.set_signals(Signals::new(BPM, u64::from(SEED)));
    deck.set_transport(0, Sync::Beat, BPM, 0.0)
        .expect("`beat_shell` is closed form");

    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba8UnormSrgb, WIDTH, HEIGHT);
    present.set_tonemap(&gpu.queue, TonemapOp::Aces, 1.0, 4.0);
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("transport scrub target"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());

    // Forward, keeping every frame against the step it was drawn at, so the
    // comparison below can name the step it expects rather than a frame number
    // that happens to line up.
    let mut seen: Vec<(u64, Vec<u8>)> = Vec::with_capacity(FORWARD);
    for _ in 0..FORWARD {
        let pixels = frame(&gpu, &mut deck, &present, &view, &target);
        let step = (deck.slot(0).set().time() * 60.0).round() as u64;
        seen.push((step, pixels));
    }
    let (last_step, last) = seen.last().cloned().expect("frames were rendered");
    write_png(&out.join("a_forward.png"), &last);
    println!("forward: {FORWARD} frames, slot at step {last_step}");

    // Scrub back. The session advances one more step on the frame below, so
    // the slot lands at `last_step + 1` minus whatever two beats are worth.
    deck.set_transport(0, Sync::Beat, BPM, SCRUB)
        .expect("`beat_shell` is closed form");
    let rewound = frame(&gpu, &mut deck, &present, &view, &target);
    let landed = (deck.slot(0).set().time() * 60.0).round() as u64;
    write_png(&out.join("b_rewound.png"), &rewound);
    println!("scrubbed {SCRUB} beats: slot at step {landed}");

    let Some((_, expected)) = seen.iter().find(|(step, _)| *step == landed) else {
        println!("step {landed} was not one of the frames drawn on the way past");
        return;
    };
    write_png(&out.join("c_expected.png"), expected);

    let identical = &rewound == expected;
    println!(
        "\nrewound frame vs the frame drawn at step {landed} on the way past: {}",
        if identical {
            "IDENTICAL, pixel for pixel"
        } else {
            "DIFFERENT — the rewind did not reproduce it"
        }
    );
    // And that this is a rewind at all rather than a frame that happened to
    // match: the material has to have moved between the two positions.
    println!(
        "rewound frame vs the frame it was at before scrubbing:            {}",
        if rewound == last { "identical — nothing moved, so this proves nothing" } else { "different" }
    );
    println!("\nwritten to {}/", out.display());
}
