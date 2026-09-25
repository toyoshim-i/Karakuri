use karakuri_engine::camera::Orbit;
use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

use super::common::{compile, f16};

/// Single-element procedural fixture positioned off the view axis for camera transform tracking.
pub const MARK: &str = r#"
proc mark {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = vec3(0.0, 1.0, -1.5);
  }
}
"#;

pub const DOT: &str = r#"
proc dot {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

/// A Set of one element and one renderer, at `w` by `h`, seen from `camera`.
pub fn build(gpu: &Gpu, w: u32, h: u32, camera: Orbit) -> Set {
    let l4 = compile(DOT);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(MARK), 1)],
        &[],
        &[],
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring::default(),
    )
    .expect("one L1 and one L4");
    set.resize(&gpu.device, w, h);
    set.aim_camera(camera);
    set
}

/// The camera these tests measure against: **still**, so a frame is a frame and
/// not a moment in a sweep.
pub fn pinned() -> Orbit {
    Orbit {
        radius: 5.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    }
}

/// Brightness-weighted mean column and row of the lit texels, in texels.
pub fn centroid(gpu: &Gpu, set: &mut Set, w: u32, h: u32) -> (f32, f32) {
    let px = frame(gpu, set, w, h);
    let (mut sx, mut sy, mut weight) = (0.0f64, 0.0f64, 0.0f64);
    for (i, t) in px.chunks_exact(4).enumerate() {
        if t[0] > 0.01 {
            let (x, y) = ((i as u32 % w) as f64, (i as u32 / w) as f64);
            sx += f64::from(t[0]) * x;
            sy += f64::from(t[0]) * y;
            weight += f64::from(t[0]);
        }
    }
    assert!(
        weight > 0.0,
        "nothing was drawn, so there is nowhere to measure"
    );
    ((sx / weight) as f32, (sy / weight) as f32)
}

/// RGBA f32 per texel, after one frame.
pub fn frame(gpu: &Gpu, set: &mut Set, w: u32, h: u32) -> Vec<f32> {
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, w, h);
    set.prepare(&gpu.queue, 1, &Signals::default());

    let bytes_per_row = w * 8;
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(bytes_per_row * h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    set.render(&mut encoder, present.hdr_view(), 1);
    encoder.copy_texture_to_buffer(
        present.hdr_texture().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    set.commit();

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = slice.get_mapped_range().expect("map");
    let out: Vec<f32> = data
        .chunks_exact(2)
        .map(|b| f16(u16::from_le_bytes([b[0], b[1]])))
        .collect();
    drop(data);
    readback.unmap();
    out
}

/// A camera on the clock alone, parameterised so a test can move it. Writes two
/// of the six outputs and leaves the other four to their defaults, which is what
/// the simplest camera anyone writes looks like.
pub fn sweep(dist: f32) -> String {
    format!(
        r#"
proc sweep {{
  kind L3
  param dist : float [1.0, 40.0] = {dist:?}
  camera {{
    eye    = vec3(dist, 0.0, 0.0);
    target = vec3(0.0, 0.0, 0.0);
  }}
}}
"#
    )
}

/// **Sized by its own param**, so a picture that changes when the param does
/// proves three things at once: the L3's pass ran, its uniform reached it, and
/// the state it wrote was what the derivation read.
pub const GAIN_DOT: &str = r#"
proc gain_dot {
  kind  L4
  blend additive

  param gain : float [0.0, 4.0] = 1.0

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(gain, gain, gain, 1.0);
  }
}
"#;

pub fn with_camera(gpu: &Gpu, l3: Option<&str>, l4: &str, w: u32, h: u32) -> Set {
    let l3s: Vec<Checked> = l3.map(compile).into_iter().collect();
    let l4 = compile(l4);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(MARK), 1)],
        &[],
        &l3s.iter().collect::<Vec<_>>(),
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring::default(),
    )
    .expect("one L1, an optional camera, and one L4");
    set.resize(&gpu.device, w, h);
    set.aim_camera(pinned());
    set
}

/// A camera on the `+x` axis or the `-x` axis, looking at the origin. **The two
/// are mirror images**, so material off the view axis lands on opposite sides
/// of the frame — which is a reading that cannot be produced by a Set that drew
/// both renderers from one camera, whichever one it picked.
pub fn from_x(name: &str, x: f32) -> String {
    format!(
        r#"
proc {name} {{
  kind L3
  camera {{
    eye    = vec3({x:?}, 0.0, 0.0);
    target = vec3(0.0, 0.0, 0.0);
  }}
}}
"#
    )
}

/// A renderer that says which camera it draws from, in one colour channel so
/// that two of them in one frame can be measured apart.
pub fn through(name: &str, colour: [f32; 3]) -> String {
    let (r, g, b) = (colour[0], colour[1], colour[2]);
    format!(
        r#"
proc {name} {{
  kind  L4
  blend additive

  uses view : Camera

  consumes position

  vertex {{
    clip       = view.clip * vec4(position, 1.0);
    point_rate = 0.03125;
  }}

  fragment {{
    color = vec4({r:?}, {g:?}, {b:?}, 1.0);
  }}
}}
"#
    )
}

/// A renderer that names no camera and reads the Set's, which is what every
/// renderer written before the slot existed does.
pub const PLAIN: &str = r#"
proc plain {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(0.0, 0.0, 1.0, 1.0);
  }
}
"#;

/// A Set of the one mark, `l3s` cameras and `l4s` renderers, wired by `edges`.
pub fn wired(
    gpu: &Gpu,
    l3s: &[String],
    l4s: &[&str],
    edges: &[(&str, &str, &str)],
    w: u32,
    h: u32,
) -> Result<Set, karakuri_engine::set::SetError> {
    let l3s: Vec<Checked> = l3s.iter().map(|s| compile(s)).collect();
    let l4s: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
    let edges: Vec<karakuri_engine::set::Edge> = edges
        .iter()
        .map(|(node, slot, to)| karakuri_engine::set::Edge {
            node: node.to_string(),
            slot: (*slot).into(),
            to: to.to_string(),
        })
        .collect();
    let set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(MARK), 1)],
        &[],
        &l3s.iter().collect::<Vec<_>>(),
        &[],
        &l4s.iter().collect::<Vec<_>>(),
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring {
            edges: &edges,
            ..Default::default()
        },
    );
    set.map(|mut set| {
        set.resize(&gpu.device, w, h);
        set.aim_camera(pinned());
        set
    })
}

/// Brightness-weighted mean column of one colour channel's lit texels.
pub fn column(px: &[f32], w: u32, channel: usize) -> f32 {
    let (mut sx, mut weight) = (0.0f64, 0.0f64);
    for (i, t) in px.chunks_exact(4).enumerate() {
        if t[channel] > 0.01 {
            sx += f64::from(t[channel]) * f64::from(i as u32 % w);
            weight += f64::from(t[channel]);
        }
    }
    assert!(weight > 0.0, "channel {channel} drew nothing to measure");
    (sx / weight) as f32
}
