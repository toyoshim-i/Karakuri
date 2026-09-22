use super::common::{compile, f32_to_f16 as f16};
use karakuri_engine::deck::{Deck, DeckSlot};
use karakuri_engine::meter::{Level, Meters};
use karakuri_engine::swap::HotSwap;
use karakuri_engine::{Gpu, Present, Set};

pub const WIDTH: u32 = 256;
pub const HEIGHT: u32 = 256;
pub const CAPACITY: u32 = 4096;
pub const SEED: u32 = 19274;

/// Frames to wait for a reading before calling it lost. Generous on purpose:
/// the lag is one frame under this harness's pacing and tens of frames without
/// it, and none of these tests is about how few it can be.
pub const PATIENCE: usize = 240;

/// The Rec.709 luminance weights, restated here so that the test knows them
/// independently of the shader. If these two ever disagree, that is the bug
/// this file exists to catch.
pub const R: f32 = 0.2126;
pub const G: f32 = 0.7152;
pub const B: f32 = 0.0722;

/// A linear HDR texture of `width` x `height`, filled by `texel` — the same
/// format and the same sample type a deck slot's target has, so what the meter
/// is pointed at here is the thing it is pointed at in use.
pub fn image(
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
            // the colour, and alpha in a slot target is coverage rather than a
            // fourth colour channel. Set to something other than the colour so
            // that a meter that accidentally included it would show up.
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
pub fn measure(gpu: &Gpu, view: &wgpu::TextureView) -> Level {
    let mut meters = Meters::new(&gpu.device, &[view]);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    meters.record(0, &mut encoder);
    gpu.queue.submit([encoder.finish()]);
    meters.arm();
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    meters.collect(&gpu.device);
    meters
        .level(0)
        .expect("the measurement was submitted and waited for")
}

pub const L1: &str = r#"
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
pub const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = {{EXPOSURE}}

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(1.0, 1.0, 1.0) * exposure, max(0.0, 1.0 - d));
  }
}
"#;

/// The same shape, rendering a NaN — a `sqrt` of a negative, which parses,
/// type-checks, costs, compiles and runs. A generated L4 reaches this by
/// dividing by a parameter that got to zero as easily as by writing it, and
/// that is the point: this is what an ordinary shader accident looks like
/// arriving through a real fragment block and a real additive blend, rather
/// than a NaN a test wrote into a texture by hand.
pub const L4_NAN: &str = r#"
proc nan_points {
  kind  L4
  blend additive

  consumes position

  param exposure : float [0.0, 8.0] = 1.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.015625;
  }

  fragment {
    let d   = length(point_coord * 2.0 - 1.0);
    let bad = sqrt(0.0 - exposure);
    color = vec4(vec3(bad, bad, bad), max(0.0, 1.0 - d));
  }
}
"#;

pub fn build(gpu: &Gpu, exposure: f32) -> Set {
    build_l4(gpu, &L4.replace("{{EXPOSURE}}", &format!("{exposure:.3}")))
}

pub fn build_l4(gpu: &Gpu, l4: &str) -> Set {
    let mut set = Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(L1),
        &compile(l4),
        CAPACITY,
        SEED,
    )
    .expect("the pair is compatible and the capacity is in range");
    set.resize(&gpu.device, WIDTH, HEIGHT);
    set
}

/// A metered deck of fixed Sets at the given exposures — no worker, so nothing
/// can swap underneath a measurement.
pub fn metered_deck(gpu: &Gpu, exposures: &[f32]) -> Deck {
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
pub fn skipped(deck: &Deck) -> u64 {
    deck.meters().expect("this deck is metered").skipped(0)
}

/// One frame, with the harness's `poll(Wait)` standing in for vsync exactly as
/// `tests/deck.rs` does. It is outside the frame, not in it.
pub fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present) {
    frame_without_waiting(gpu, deck, present);
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
}

/// The same frame with nothing after it. Used by the test that has to be able
/// to say there was no wait anywhere.
pub fn frame_without_waiting(gpu: &Gpu, deck: &mut Deck, present: &Present) {
    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), 1);
    f.finish();
}

/// Run frames until a slot reports a level, and return it with how many frames
/// that took.
pub fn wait_for_level(
    gpu: &Gpu,
    deck: &mut Deck,
    present: &Present,
    slot: usize,
) -> (Level, usize) {
    for n in 1..=PATIENCE {
        frame(gpu, deck, present);
        if let Some(level) = deck.level(DeckSlot(slot as u8)) {
            return (level, n);
        }
    }
    panic!("no level arrived for slot {slot} in {PATIENCE} frames");
}
