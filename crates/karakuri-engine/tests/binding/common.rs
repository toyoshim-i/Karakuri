#![allow(unused_imports, dead_code)]

pub(crate) use karakuri_engine::binding::{blend, Curve, Signals};
pub(crate) use karakuri_engine::deck::{Deck, DeckSlot, Residency};
pub(crate) use karakuri_engine::swap::HotSwap;
pub(crate) use karakuri_engine::{Binding, Gpu, Present, Set};
pub(crate) use karakuri_ir::typed::Checked;
pub(crate) use karakuri_ir::Kind;
pub(crate) use karakuri_signal::{AudioFrame, NoiseConfig, NoiseKind};

pub(crate) const WIDTH: u32 = 128;
pub(crate) const HEIGHT: u32 = 128;
pub(crate) const CAPACITY: u32 = 4096;
pub(crate) const SEED: u32 = 19_274;
pub(crate) const BPM: f32 = 120.0;

/// 120 bpm at `dt = 1/60` is exactly 30 frames to the beat.
pub(crate) const FRAMES_PER_BEAT: usize = 30;

/// A `spawn` block, so that `spawn_rate` is a real param with the accumulator
/// behind it — that second consumer is the whole reason `docs/ir-spec.md`
/// answers irregular spawning with a noise binding.
pub(crate) const L1: &str = r#"
proc sparks {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius     : float [0.1, 8.0]      = 2.5
  param spawn_rate : float [0.0, 40000.0]  = 1000.0

  emit position, age

  spawn {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius;
    age      = 0.0;
  }

  element {
    position = position;
    age      = age + dt;
    if age > 4.0 { kill(); }
  }
}
"#;

pub(crate) const L4: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param point_scale : float [0.0039, 0.3125] = 0.0625
  param hue         : float [0.0, 1.0]  = 0.6

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = point_scale;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    let a = max(0.0, 1.0 - d);
    let c = hsv_to_rgb(vec3(hue, 0.7, 1.0));
    color = vec4(c, a);
  }
}
"#;

/// The same L4, except that it declares a `radius` of its own — a name `L1`
/// already declares. Nothing in the IR forbids that: the two procedures are
/// written independently and neither can see the other's parameter names.
pub(crate) const L4_CLASHING: &str = r#"
proc soft_points {
  kind  L4
  blend additive

  consumes position

  param point_scale : float [0.0039, 0.3125] = 0.0625
  param radius      : float [0.1, 8.0]  = 7.5

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = point_scale * radius;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(vec3(max(0.0, 1.0 - d)), 1.0);
  }
}
"#;

pub(crate) use crate::engine_common::{compile, render};

pub(crate) fn build(gpu: &Gpu) -> Set {
    Set::build(
        &gpu.device,
        &gpu.queue,
        &compile(L1),
        &compile(L4),
        CAPACITY,
        SEED,
    )
    .expect("the pair is compatible and the capacity is in range")
}

pub(crate) fn deck_of(gpu: &Gpu, sets: Vec<Set>, seed: u64) -> (Deck, Present) {
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, WIDTH, HEIGHT);
    let swaps = sets.into_iter().map(HotSwap::fixed).collect();
    let mut deck = Deck::new(&gpu.device, swaps, WIDTH, HEIGHT);
    deck.set_signals(Signals::new(BPM, seed));
    (deck, present)
}

pub(crate) fn frame(gpu: &Gpu, deck: &mut Deck, present: &Present, steps: u8) {
    let mut f = deck.begin_frame(&gpu.device, &gpu.queue);
    f.render(present.hdr_view(), present.size(), steps);
    f.finish();
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
}

/// The mix, read back. `Rgba16Float` in and out with one slot at unity gain
/// has no rounding anywhere in it, so these comparisons are exact.
pub(crate) fn readback(gpu: &Gpu, texture: &wgpu::Texture) -> Vec<u16> {
    let (width, height) = (texture.width(), texture.height());
    let bytes_per_row = width * 8;
    assert_eq!(bytes_per_row % 256, 0, "row pitch must be 256-byte aligned");

    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("binding readback"),
        size: u64::from(bytes_per_row * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = slice.get_mapped_range().expect("map");
    let out = data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    drop(data);
    buffer.unmap();
    out
}

pub(crate) fn value_of(deck: &Deck, slot: usize, key: &str) -> f32 {
    deck.slot(DeckSlot(slot as u8))
        .set()
        .bound()
        .find(|(name, _)| *name == key)
        .unwrap_or_else(|| panic!("`{key}` is not bound on slot {slot}"))
        .1
}
