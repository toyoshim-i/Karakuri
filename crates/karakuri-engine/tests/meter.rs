//! Per-slot level metering.
//!
//! Two halves, deliberately. The first drives [`Meters`] over textures written
//! from the host, texel by texel, so that what a reading *is* can be asserted
//! exactly rather than approximately — the Rec.709 weights, a mean over the
//! whole frame including its black, a peak that is one texel's, and a black
//! frame reading exactly zero. None of those can be pinned down against a Set:
//! a rendered frame's true mean is only knowable by computing the thing under
//! test.
//!
//! The second half drives a real `Deck` of real Sets, where what matters is
//! the properties an operator relies on: a brighter Set reads higher than a
//! dimmer one, an off-air slot reads nothing at all, and the whole thing works
//! without a `poll(Wait)` anywhere in the frame path.

use std::sync::mpsc;
use std::time::Instant;

use karakuri_engine::deck::{Deck, Residency};
use karakuri_engine::meter::{Level, Meters};
use karakuri_engine::swap::{Event, HotSwap, Request};
use karakuri_engine::{Gpu, Present, Set};
use karakuri_ir::typed::Checked;

const WIDTH: u32 = 256;
const HEIGHT: u32 = 256;
const CAPACITY: u32 = 4096;
const SEED: u32 = 19274;

/// Frames to wait for a reading before calling it lost. Generous on purpose:
/// the lag is one frame under this harness's pacing and tens of frames without
/// it, and none of these tests is about how few it can be.
const PATIENCE: usize = 240;

/// The Rec.709 luminance weights, restated here so that the test knows them
/// independently of the shader. If these two ever disagree, that is the bug
/// this file exists to catch.
const R: f32 = 0.2126;
const G: f32 = 0.7152;
const B: f32 = 0.0722;

// ---------------------------------------------------------------------------
// Half one: a meter over a texture of known contents.
// ---------------------------------------------------------------------------

/// `f32` to `f16` bits, for values that are exactly representable — which
/// every value in this file is, on purpose. A test that wrote 0.1 and expected
/// 0.1 back would be measuring the conversion rather than the meter, so the
/// inputs are all halves and powers of two and this refuses anything else.
fn f16(x: f32) -> u16 {
    if x == 0.0 {
        return 0;
    }
    let bits = x.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127;
    let mantissa = bits & 0x007f_ffff;
    assert!(
        (-14..=15).contains(&exponent) && mantissa & 0x1fff == 0,
        "{x} is not exactly representable as an f16, so this test would be measuring \
         a rounding rather than a reduction"
    );
    sign | (((exponent + 15) as u16) << 10) | (mantissa >> 13) as u16
}

/// A linear HDR texture of `width` x `height`, filled by `texel` — the same
/// format and the same sample type a deck slot's target has, so what the meter
/// is pointed at here is the thing it is pointed at in use.
fn image(
    gpu: &Gpu,
    width: u32,
    height: u32,
    texel: impl Fn(u32, u32) -> [f32; 3],
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("meter test image"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: Present::HDR_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let mut data: Vec<u8> = Vec::with_capacity((width * height * 8) as usize);
    for y in 0..height {
        for x in 0..width {
            let rgb = texel(x, y);
            for c in rgb {
                data.extend_from_slice(&f16(c).to_le_bytes());
            }
            // Alpha, which the meter does not read: luminance is a property of
            // the colour, and the mix multiplies alpha through like any other
            // channel. Set to something other than the colour so that a meter
            // that accidentally included it would show up.
            data.extend_from_slice(&f16(1.0).to_le_bytes());
        }
    }
    gpu.queue.write_texture(
        texture.as_image_copy(),
        &data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 8),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&Default::default());
    (texture, view)
}

/// Measure one image, once, and wait for the answer.
///
/// The `poll(Wait)` here is the test harness's, not the meter's: this half is
/// about what the reduction computes, and waiting is how a test gets a
/// deterministic answer out of an asynchronous thing. That the frame path
/// itself never waits is the *other* half's claim, and it is asserted there
/// with no `Wait` anywhere in it.
fn measure(gpu: &Gpu, view: &wgpu::TextureView) -> Level {
    let mut meters = Meters::new(&gpu.device, &[view]);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    meters.record(0, &mut encoder);
    gpu.queue.submit([encoder.finish()]);
    meters.arm();
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    meters.collect(&gpu.device);
    meters
        .level(0)
        .expect("the measurement was submitted and waited for")
}

/// **A black frame measures zero, and a brighter frame measures higher.**
///
/// Exactly zero, both numbers: a mean that came back as a small positive value
/// would mean the reduction is summing something the image does not contain,
/// and a peak floored anywhere above zero would hide a dark Set entirely.
///
/// The three levels are a constant wash each, so the expected mean and peak
/// are the same number and both are known in closed form — the only assertion
/// available that does not compute the answer with the code under test.
#[test]
fn a_black_frame_measures_zero_and_a_brighter_frame_measures_higher() {
    let gpu = Gpu::headless().expect("no GPU available");

    let (_black, black_view) = image(&gpu, 64, 64, |_, _| [0.0, 0.0, 0.0]);
    let (_dim, dim_view) = image(&gpu, 64, 64, |_, _| [0.25, 0.25, 0.25]);
    let (_bright, bright_view) = image(&gpu, 64, 64, |_, _| [2.0, 2.0, 2.0]);

    let black = measure(&gpu, &black_view);
    let dim = measure(&gpu, &dim_view);
    let bright = measure(&gpu, &bright_view);

    assert_eq!(black.mean, 0.0, "a black frame measured light");
    assert_eq!(black.peak, 0.0, "a black frame has a peak");

    // A grey of `v` has luminance `v` exactly: the weights sum to 1.0.
    let close = |x: f32, y: f32| (x - y).abs() <= 1e-4 * (1.0 + y.abs());
    assert!(close(dim.mean, 0.25), "0.25 grey measured {}", dim.mean);
    assert!(close(dim.peak, 0.25), "0.25 grey peaked at {}", dim.peak);
    assert!(close(bright.mean, 2.0), "2.0 grey measured {}", bright.mean);
    // Above 1.0, which is the half of the range the pipeline is HDR for.
    assert!(bright.peak > 1.0, "the bright frame did not exceed 1.0");
    assert!(bright.mean > dim.mean && dim.mean > black.mean);
}

/// **Peak and mean move independently.**
///
/// A small very bright core and a large dim wash, constructed to have the
/// *same* mean — one 64th of the frame at 64 times the luminance — so that the
/// only thing separating them is the peak, which differs by that same factor
/// of 64. A meter reporting one number could not tell these apart, and the one
/// it would miss is the one that matters on stage: the bright core dominates
/// the mix wherever it lands, at a mean the fader says is matched.
#[test]
fn peak_and_mean_move_independently() {
    let gpu = Gpu::headless().expect("no GPU available");

    // 64x64 = 4096 texels; the core is 8x8 = 64 of them, one 64th.
    let (_wash, wash_view) = image(&gpu, 64, 64, |_, _| [0.5, 0.5, 0.5]);
    let (_core, core_view) = image(&gpu, 64, 64, |x, y| {
        if x < 8 && y < 8 {
            [32.0, 32.0, 32.0]
        } else {
            [0.0, 0.0, 0.0]
        }
    });

    let wash = measure(&gpu, &wash_view);
    let core = measure(&gpu, &core_view);

    let close = |x: f32, y: f32| (x - y).abs() <= 1e-3 * (1.0 + y.abs());
    assert!(close(wash.mean, 0.5), "the wash measured {}", wash.mean);
    assert!(
        close(core.mean, 0.5),
        "the core frame measured {} where the wash measured {} — they were built to \
         agree, so the reduction is not a mean over the whole frame",
        core.mean,
        wash.mean
    );
    assert!(close(wash.peak, 0.5), "the wash peaked at {}", wash.peak);
    assert!(
        close(core.peak, 32.0),
        "the core peaked at {} rather than 32.0",
        core.peak
    );
    assert!(
        core.peak > 8.0 * wash.peak,
        "two frames of the same mean reported peaks within 8x of each other, so the \
         pair carries no more information than the mean alone"
    );
}

/// **The luminance weights are the ones claimed: linear Rec.709.**
///
/// Pure green and pure blue at the same RGB magnitude. Under the weights this
/// pipeline's primaries actually imply, green carries very nearly ten times the
/// luminance blue does; under the average-the-channels shortcut they would
/// measure identically. An operator matching faders on the second number would
/// be matching the wrong thing, and this is the assertion that says which one
/// is being reported.
#[test]
fn the_luminance_weights_are_rec_709() {
    let gpu = Gpu::headless().expect("no GPU available");

    let (_r, r_view) = image(&gpu, 32, 32, |_, _| [1.0, 0.0, 0.0]);
    let (_g, g_view) = image(&gpu, 32, 32, |_, _| [0.0, 1.0, 0.0]);
    let (_b, b_view) = image(&gpu, 32, 32, |_, _| [0.0, 0.0, 1.0]);

    let red = measure(&gpu, &r_view);
    let green = measure(&gpu, &g_view);
    let blue = measure(&gpu, &b_view);

    let close = |x: f32, y: f32| (x - y).abs() <= 1e-4;
    assert!(close(red.mean, R), "pure red measured {}", red.mean);
    assert!(close(green.mean, G), "pure green measured {}", green.mean);
    assert!(close(blue.mean, B), "pure blue measured {}", blue.mean);
    assert_ne!(
        green.mean, blue.mean,
        "green and blue of the same magnitude measured the same, which is what \
         averaging the channels would do"
    );
    assert!(
        green.mean > 9.0 * blue.mean,
        "green measured {} against blue's {}, a ratio of {:.2} rather than Rec.709's \
         {:.2}",
        green.mean,
        blue.mean,
        green.mean / blue.mean,
        G / B
    );
}

/// **The reduction covers the whole image, including the part the workgroups
/// do not divide evenly.**
///
/// The first pass dispatches a fixed 64 workgroups of 64 threads and walks the
/// image in strides of 4096, so every image whose texel count is not a multiple
/// of 4096 has a tail that only some threads reach and some workgroups miss
/// entirely. Three ways that can go wrong and all three are checked here: a
/// mean divided by the padded thread count rather than the texel count, a
/// partial that covered no texels dragging a mean down, and a peak that never
/// looks at the tail — which is why the brightest texel is deliberately the
/// very last one in every case.
///
/// Every other test in this file uses a power-of-two square, and 32x32, 64x64
/// and 256x256 are all exactly divided. This is the one that is not.
#[test]
fn the_reduction_covers_the_tail_the_workgroups_do_not_divide() {
    let gpu = Gpu::headless().expect("no GPU available");

    for &(w, h) in &[
        (1u32, 1u32), // one texel: 4095 threads with nothing to do
        (63, 65),     // 4095: one short of a full pass
        (65, 65),     // 4225: a tail of 129
        (101, 97),    // 9797: two passes and a ragged tail
        (257, 129),   // 33153
        (4097, 1),    // one very wide row
    ] {
        // 0.25 everywhere but the last texel, which is 8.0 — so a peak that
        // stops at the last whole stride reports 0.25.
        let texel = |x: u32, y: u32| {
            let v = if (x, y) == (w - 1, h - 1) { 8.0 } else { 0.25 };
            [v, v, v]
        };
        let (_image, view) = image(&gpu, w, h, texel);
        let measured = measure(&gpu, &view);

        // Closed form, from the same numbers the image was built from: a grey
        // of `v` has luminance `v`, since the weights sum to 1.0.
        let texels = f64::from(w) * f64::from(h);
        let expected_mean = ((0.25 * (texels - 1.0) + 8.0) / texels) as f32;
        assert!(
            (measured.mean - expected_mean).abs() <= 1e-4 * (1.0 + expected_mean),
            "{w}x{h} ({} texels) measured a mean of {} rather than {expected_mean}",
            w * h,
            measured.mean
        );
        assert!(
            (measured.peak - 8.0).abs() <= 1e-3,
            "{w}x{h}: the brightest texel is the last one and the peak came back as \
             {}, so the tail the workgroups do not evenly cover was not looked at",
            measured.peak
        );
    }
}

/// **A frame that is negative everywhere reports a negative peak.**
///
/// Nothing in the pipeline rejects a negative colour: a generated L4 that
/// subtracts, or takes a `1.0 - x` of something larger than one, compiles and
/// runs and writes it. `shaders/meter.wgsl` starts its peak at the lowest
/// finite `f32` rather than at zero for exactly this, and the difference is
/// invisible on every other frame in this file. A peak floored at zero would
/// report this frame as having a peak it does not have — the one number an
/// operator reads as "this Set is not clipping" would be the one number that
/// was invented.
#[test]
fn a_negative_frame_reports_a_negative_peak() {
    let gpu = Gpu::headless().expect("no GPU available");
    let (_image, view) = image(&gpu, 64, 64, |_, _| [-0.5, -0.5, -0.5]);
    let measured = measure(&gpu, &view);

    let close = |x: f32, y: f32| (x - y).abs() <= 1e-4 * (1.0 + y.abs());
    assert!(close(measured.mean, -0.5), "a -0.5 frame measured {}", measured.mean);
    assert!(
        close(measured.peak, -0.5),
        "a frame that is -0.5 everywhere reported a peak of {}, so the peak is \
         floored at zero and a negative Set reads as one that merely touches black",
        measured.peak
    );
}

// ---------------------------------------------------------------------------
// Half two: a meter on a real deck.
// ---------------------------------------------------------------------------

const L1: &str = r#"
proc static_shell {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5

  emit position, age

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = rot_y(sphere_point(u, v) * radius, t * 0.3);
    age      = age + dt;
  }
}
"#;

/// The deck tests' L4, with its exposure left open so that two slots can hold
/// the same material at different levels — which is exactly the situation the
/// meter exists for.
const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = {{EXPOSURE}}

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    let checked =
        karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build(gpu: &Gpu, exposure: f32) -> Set {
    let l4 = L4.replace("{{EXPOSURE}}", &format!("{exposure:.3}"));
    let mut set = Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(L1),
        &compile(&l4),
        CAPACITY,
        SEED,
    )
    .expect("the pair is compatible and the capacity is in range");
    set.resize(WIDTH, HEIGHT);
    set
}

/// A metered deck of fixed Sets at the given exposures — no worker, so nothing
/// can swap underneath a measurement.
fn metered_deck(gpu: &Gpu, exposures: &[f32]) -> Deck {
    let swaps = exposures
        .iter()
        .map(|&e| HotSwap::fixed(build(gpu, e)))
        .collect();
    let mut deck = Deck::new(&gpu.device, swaps, WIDTH, HEIGHT);
    deck.enable_meters(&gpu.device);
    deck
}

/// Skipped measurements on slot 0, reached through the deck's read-only view of
/// its meters.
fn skipped(deck: &Deck) -> u64 {
    deck.meters().expect("this deck is metered").skipped(0)
}

/// One frame, with the harness's `poll(Wait)` standing in for vsync exactly as
/// `tests/deck.rs` does. It is outside the frame, not in it.
fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present) {
    frame_without_waiting(gpu, deck, present);
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
}

/// The same frame with nothing after it. Used by the test that has to be able
/// to say there was no wait anywhere.
fn frame_without_waiting(gpu: &Gpu, deck: &mut Deck, present: &Present) {
    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), 1);
    f.finish();
}

/// Run frames until a slot reports a level, and return it with how many frames
/// that took.
fn wait_for_level(gpu: &Gpu, deck: &mut Deck, present: &Present, slot: usize) -> (Level, usize) {
    for n in 1..=PATIENCE {
        frame(gpu, deck, present);
        if let Some(level) = deck.level(slot) {
            return (level, n);
        }
    }
    panic!("no level arrived for slot {slot} in {PATIENCE} frames");
}

/// **A brighter Set measures higher than a dimmer one, and a Set drawing
/// nothing measures zero.**
///
/// Three slots of identical geometry at three exposures, so the only thing
/// separating their targets is how much light each puts out — which is the
/// question a fader is set to answer, and the one an operator has until now
/// been answering by eye.
///
/// The dark slot is not a black texture: it is a Set that runs, draws every
/// element, and multiplies its colour by zero. Its target is written every
/// frame and reads exactly zero, which is a stronger statement than a cleared
/// texture reading zero.
#[test]
fn a_brighter_set_measures_higher_than_a_dimmer_one() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = metered_deck(&gpu, &[1.0, 0.25, 0.0]);

    let (bright, _) = wait_for_level(&gpu, &mut deck, &present, 0);
    let dim = deck.level(1).expect("every slot is metered from the same frame");
    let dark = deck.level(2).expect("every slot is metered from the same frame");

    assert!(
        bright.mean > 0.0,
        "the bright slot measured {}, so this test is comparing nothing",
        bright.mean
    );
    assert!(
        dim.mean > 0.0 && bright.mean > 2.0 * dim.mean,
        "a Set at exposure 1.0 measured {} against 0.25's {}, which is not the four \
         times more light it is putting out",
        bright.mean,
        dim.mean
    );
    assert!(
        bright.peak > dim.peak,
        "the brighter Set's peak ({}) did not exceed the dimmer one's ({})",
        bright.peak,
        dim.peak
    );
    assert_eq!(dark.mean, 0.0, "a Set drawing black measured light");
    assert_eq!(dark.peak, 0.0, "a Set drawing black has a peak");
}

/// **An Allocated slot reads nothing — `None`, not its last frame.**
///
/// Its target still holds whatever it last drew, so there is a number
/// available; reporting it would be presenting a stale reading as a live one,
/// which is the failure this codebase keeps finding. Going off air retires the
/// meter, including the measurements still in flight, so a result recorded
/// while the slot was Live cannot arrive two frames later and resurrect a
/// level for a slot that is not producing one.
///
/// The second half is what stops the first from passing on a meter that had
/// simply stopped working: back on air, a level must return.
#[test]
fn an_allocated_slot_reports_no_level() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = metered_deck(&gpu, &[1.0, 1.0]);

    let (live, _) = wait_for_level(&gpu, &mut deck, &present, 1);
    assert!(live.mean > 0.0, "the slot measured nothing while Live");

    deck.set_residency(1, Residency::Allocated);
    assert_eq!(
        deck.level(1),
        None,
        "a slot taken off air kept the level it had while it was on"
    );
    // Long enough that every measurement outstanding at the moment it went off
    // air has had time to come back.
    for _ in 0..30 {
        frame(&gpu, &mut deck, &present);
        assert_eq!(
            deck.level(1),
            None,
            "a measurement recorded while the slot was Live arrived after it went off \
             air and became a level for a slot that is rendering nothing"
        );
    }
    // The other slot is unaffected: retiring is per slot.
    assert!(deck.level(0).is_some(), "retiring one slot retired another");

    deck.set_residency(1, Residency::Live);
    let (again, frames) = wait_for_level(&gpu, &mut deck, &present, 1);
    assert!(
        again.mean > 0.0,
        "the returning slot measured nothing, so the first half of this test would \
         pass on a meter that had stopped working"
    );
    assert!(
        frames > 1,
        "a level was available on the very first frame back on air, so it was not a \
         fresh measurement"
    );
}

/// **A build landing on a slot retires its meter.**
///
/// A swap replaces the material outright: the incoming Set is cold, `t` back at
/// zero, nothing primed. Whatever is in flight for that slot measured the Set
/// that was there, and installing it afterwards reports the outgoing Set's
/// level as the incoming one's — which is the same stale-number-as-a-live-one
/// failure that going off air and a resize are retired for, arriving through
/// the one door in this engine that opens by itself. `--watch` opens it on
/// every save, and a rollback opens it again from the other side.
///
/// Constructed so the wrong answer is unmistakable rather than a shade off: a
/// Set at exposure 1.0 is swapped for one at exposure 0.0, which draws every
/// element and multiplies its colour by zero. A meter that kept the old reading
/// reports a mean around 1.0 for a target that is exactly black.
///
/// The second half is what stops the first from passing on a meter that simply
/// stopped: the fresh reading has to arrive, and it has to be the black Set's.
#[test]
fn a_build_landing_on_a_slot_retires_its_meter() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);

    let (requests, source) = mpsc::channel::<Request>();
    // A budget no frame in this harness will come near: what is under test is
    // the swap landing, not the watchdog's opinion of it.
    let swap = HotSwap::new(&gpu.device, &gpu.queue, build(&gpu, 1.0), 10_000.0, Box::new(source));
    let mut deck = Deck::new(&gpu.device, vec![swap], WIDTH, HEIGHT);
    deck.enable_meters(&gpu.device);

    let (bright, _) = wait_for_level(&gpu, &mut deck, &present, 0);
    assert!(bright.mean > 0.0, "the Set measured nothing before the swap");

    requests
        .send(Request {
            l1: compile(L1),
            l4: compile(&L4.replace("{{EXPOSURE}}", "0.000")),
            capacity: CAPACITY,
            seed_salt: SEED,
            params: Vec::new(),
            bindings: Vec::new(),
            label: "black".to_string(),
        })
        .expect("the worker is alive");

    let mut landed = false;
    for _ in 0..PATIENCE {
        frame(&gpu, &mut deck, &present);
        if deck.events(0).any(|e| matches!(e, Event::Swapped { .. })) {
            landed = true;
            break;
        }
    }
    assert!(landed, "the build never landed in {PATIENCE} frames");

    // The frame that just ran was drawn entirely by the black Set. Anything
    // here is the previous Set's light, reported as this one's.
    assert_eq!(
        deck.level(0),
        None,
        "a build landed and the slot kept the outgoing Set's reading — the Set on air \
         draws black and the meter is reporting the one before it, which is a stale \
         number presented as a live one"
    );

    // The frame the swap landed on was itself drawn by the black Set, so the
    // next reading is that frame's and is due immediately. What makes it
    // demonstrably fresh is not when it arrives but what it says.
    let (fresh, _) = wait_for_level(&gpu, &mut deck, &present, 0);
    assert_eq!(
        fresh.mean, 0.0,
        "the reading that arrived after the swap measured {} for a Set that multiplies \
         its colour by zero, so it is still the outgoing Set's",
        fresh.mean
    );
}

/// **The measurement does not block the frame path, and a result arrives
/// anyway.**
///
/// There is no `poll(Wait)` in this test at all — not in the frame, not after
/// it, nowhere — so anything that waited for the GPU would have to be inside
/// `Deck::begin_frame` or `Frame::render`. The frames complete and the levels
/// come back regardless, which is what "reduce, copy, `map_async`, never wait"
/// buys.
///
/// [`Level::frames_behind`] is the sharper half of the claim: it can only be
/// zero if a reading was taken of the frame being recorded, which is impossible
/// without a stall. Every reading here must be at least one frame old.
///
/// **That alone does not catch a stall, and the assertion at the bottom is what
/// does.** A `collect` that waited for the GPU would pace this loop itself, and
/// a paced loop still produces readings one frame old — it would satisfy every
/// check above while being exactly the thing this test is named for. What
/// separates the two is how far ahead the loop gets: with nothing waiting, the
/// CPU encodes frames as fast as it can and runs dozens ahead of the GPU, so
/// readings come back tens of frames old and most frames find the ring still
/// busy and skip. With a wait anywhere in the frame path, neither can happen:
/// the ring is drained every frame, so the lag is pinned at one and the skip
/// count at zero. So the run must show *one* of a reading older than one frame
/// or a skipped measurement, and this is the same shape as `README.md`'s "three
/// to five frames were rendered between the request going out and the swap
/// landing, which is what does not block means operationally".
///
/// What this loop is *not* is a measurement of the lag: running dozens of
/// frames ahead of the GPU is what an unpaced headless loop does, not what a
/// display-paced one does. The ring correctly declines to grow to cover it. The
/// lag is measured in [`the_lag_and_the_ring_are_measured_under_pacing`], which
/// has the pacing; the numbers printed here are the module doc's second row.
#[test]
fn the_meter_never_blocks_the_frame_path() {
    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = metered_deck(&gpu, &[1.0]);

    let mut first: Option<usize> = None;
    let mut lags: Vec<u32> = Vec::new();
    for n in 1..=PATIENCE {
        frame_without_waiting(&gpu, &mut deck, &present);
        if let Some(level) = deck.level(0) {
            first.get_or_insert(n);
            lags.push(level.frames_behind);
            assert!(
                level.frames_behind >= 1,
                "a reading claimed to be of the frame being recorded, which cannot \
                 happen without waiting for the GPU"
            );
            assert!(
                level.mean > 0.0,
                "the reading arrived but measured nothing, so an empty result would \
                 satisfy this test"
            );
        }
    }

    assert!(
        first.is_some(),
        "no level ever arrived in {PATIENCE} frames without a `poll(Wait)` to force \
         one, so nothing came back on its own"
    );

    let mut sorted = lags.clone();
    sorted.sort_unstable();
    let skipped = skipped(&deck);
    eprintln!(
        "\nmeter over {PATIENCE} unpaced frames at {WIDTH}x{HEIGHT}:\n  \
         frames behind: min {} median {} max {}\n  \
         a level on {} of {PATIENCE} frames, {skipped} measurements skipped\n",
        sorted[0],
        sorted[sorted.len() / 2],
        sorted[sorted.len() - 1],
        lags.len(),
    );

    assert!(
        sorted[sorted.len() - 1] > 1 || skipped > 0,
        "over {PATIENCE} frames with nothing pacing them, every reading came back \
         exactly one frame old and not one measurement was skipped — which is what a \
         loop paced by a wait looks like, and there is no wait in this test, so the \
         wait is inside the frame path"
    );
}

/// **The lag, measured: how many frames behind a reading is, and whether the
/// ring is deep enough to keep producing one every frame.**
///
/// Paced by the harness's `poll(Wait)` per frame, which is what
/// `tests/deck.rs` and `tests/hot_swap.rs` use in place of the vsync a headless
/// run does not get. That pacing is the point rather than an inconvenience: the
/// lag is a function of how far ahead of the GPU the frame loop is allowed to
/// run, so a number taken from an unpaced loop would be a number about the test
/// harness. The wait is outside the frame, exactly as it is there.
///
/// Printed as well as asserted — run with `--nocapture`. The assertions are
/// deliberately loose, because the tight number belongs in the module doc where
/// it can be read, not in a bound that fails on someone else's driver.
#[test]
fn the_lag_and_the_ring_are_measured_under_pacing() {
    const FRAMES: usize = 120;

    let gpu = Gpu::headless().expect("no GPU available");
    let present = Present::new(&gpu.device, Present::HDR_FORMAT, WIDTH, HEIGHT);
    let mut deck = metered_deck(&gpu, &[1.0]);

    let mut first: Option<usize> = None;
    let mut lags: Vec<u32> = Vec::new();
    let started = Instant::now();
    for n in 1..=FRAMES {
        frame(&gpu, &mut deck, &present);
        if let Some(level) = deck.level(0) {
            first.get_or_insert(n);
            lags.push(level.frames_behind);
        }
    }
    let elapsed = started.elapsed();

    let first = first.expect("no level arrived at all");
    let mut sorted = lags.clone();
    sorted.sort_unstable();
    eprintln!(
        "\nmeter lag over {FRAMES} display-paced frames at {WIDTH}x{HEIGHT}:\n  \
         first level after {first} frames\n  \
         frames behind: min {} median {} max {}\n  \
         a level on {} of {FRAMES} frames, {} measurements skipped (ring of {})\n  \
         {FRAMES} frames in {:.1} ms\n",
        sorted[0],
        sorted[sorted.len() / 2],
        sorted[sorted.len() - 1],
        lags.len(),
        skipped(&deck),
        karakuri_engine::meter::RING,
        elapsed.as_secs_f32() * 1000.0,
    );

    // The ring is sized so that a paced frame never has to give up its
    // measurement. If this starts failing, the ring is too small for whatever
    // queue depth the driver is running at — a number to raise deliberately,
    // with this test's printed output as the evidence.
    assert_eq!(
        skipped(&deck),
        0,
        "the ring of {} staging buffers ran dry under pacing",
        karakuri_engine::meter::RING
    );
    // Every frame after the first reading has one, so a meter reads as a meter
    // rather than as something that updates now and then.
    assert_eq!(
        lags.len(),
        FRAMES - first + 1,
        "{} of the {} frames after the first reading had no level",
        FRAMES - first + 1 - lags.len(),
        FRAMES - first + 1
    );
    // Loose, and one-sided on purpose: the claim is "a few frames", and a
    // reading that was somehow instant would mean something waited.
    assert!(
        (1..=16).contains(&sorted[sorted.len() / 2]),
        "the median reading was {} frames behind, which is neither a lag of a few \
         frames nor a stall",
        sorted[sorted.len() / 2]
    );
}
