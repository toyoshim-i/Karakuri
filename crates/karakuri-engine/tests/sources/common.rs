use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

pub(crate) const W: u32 = 64;
pub(crate) const H: u32 = 64;
pub(crate) const SIDE: u32 = 8;

/// A lattice laid out by `seed`, tinted by `hash1(seed)`.
pub(crate) fn lattice(name: &str, z: f32) -> String {
    format!(
        r#"
proc {name} {{
  kind     L1
  topology points
  capacity [64, 64] = 64

  emit position, tint

  element {{
    let i = seed % {SIDE}u;
    let j = seed / {SIDE}u;
    position = vec3(float(i) * 0.5 - 1.75, float(j) * 0.5 - 1.75, {z:?});
    tint     = vec3(hash1(seed), hash1(seed + 1u), hash1(seed + 2u));
  }}
}}
"#
    )
}

/// A lattice offset along the screen's horizontal, so that a morph moves.
pub(crate) fn lattice_at(name: &str, z: f32) -> String {
    lattice(name, 0.0).replace("0.0);\n    tint", &format!("{z:?});\n    tint"))
}

pub(crate) const DOTS: &str = r#"
proc dots {
  kind  L4
  blend additive

  consumes position, tint

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.03125;
  }

  fragment {
    color = vec4(tint, 1.0);
  }
}
"#;

pub(crate) const MORPH: &str = r#"
proc morph {
  kind L2
  uses far : Geometry

  param k : float [0.0, 1.0] = 0.0

  consumes position

  deform {
    position = mix(position, far.position, vec3(k, k, k));
  }
}
"#;

pub(crate) const DISSOLVE: &str = r#"
proc dissolve {
  kind L2

  uses only : Source

  consumes position, tint

  mask {
    strength = 0.0;
    if source == only { strength = 1.0; }
  }

  deform {
    tint = vec3(0.0, 0.0, 0.0);
  }
}
"#;

pub(crate) fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    let checked =
        karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

pub(crate) fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn build(gpu: &Gpu, l1s: &[&str]) -> Set {
    build_with(gpu, l1s, &[DOTS], Layering::Overdraw)
}

/// A Set whose L2 declares a geometry slot, with the slot bound to the last
/// source named.
pub(crate) fn build_paired(
    gpu: &Gpu,
    l1s: &[&str],
    l2: &str,
) -> Result<Set, karakuri_engine::set::SetError> {
    let far = compile(l1s[l1s.len() - 1]).name;
    build_wired(gpu, l1s, l2, &[edge("morph", "far", &far)])
}

pub(crate) fn edge(node: &str, slot: &str, to: &str) -> karakuri_engine::set::Edge {
    karakuri_engine::set::Edge {
        node: node.to_string(),
        slot: slot.into(),
        to: to.to_string(),
    }
}

/// The same, with the wiring spelled out.
pub(crate) fn build_wired(
    gpu: &Gpu,
    l1s: &[&str],
    l2: &str,
    edges: &[karakuri_engine::set::Edge],
) -> Result<Set, karakuri_engine::set::SetError> {
    let compiled: Vec<Checked> = l1s.iter().map(|s| compile(s)).collect();
    let sources: Vec<(&Checked, u32)> = compiled.iter().map(|c| (c, 64)).collect();
    let l2 = compile(l2);
    let l4 = compile(DOTS);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &sources,
        &[&l2],
        &[],
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring {
            edges,
            ..Default::default()
        },
    )?;
    set.resize(&gpu.device, W, H);
    set.aim_camera(karakuri_engine::camera::Orbit {
        radius: 6.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    });
    Ok(set)
}

/// The same Set, with a salt assigned to each geometry.
pub(crate) fn build_salted(gpu: &Gpu, l1s: &[&str], salts: &[Option<u32>]) -> Set {
    build_all(gpu, l1s, &[DOTS], Layering::Overdraw, salts)
}

pub(crate) fn build_with(gpu: &Gpu, l1s: &[&str], l4s: &[&str], layering: Layering) -> Set {
    build_all(gpu, l1s, l4s, layering, &[])
}

/// Multi-source Set builder.
pub(crate) fn build_all(
    gpu: &Gpu,
    l1s: &[&str],
    l4s: &[&str],
    layering: Layering,
    salts: &[Option<u32>],
) -> Set {
    let compiled: Vec<Checked> = l1s.iter().map(|s| compile(s)).collect();
    let sources: Vec<(&Checked, u32)> = compiled.iter().map(|c| (c, 64)).collect();
    let draw: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
    let draw_refs: Vec<&Checked> = draw.iter().collect();
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &sources,
        &[],
        &[],
        &[],
        &draw_refs,
        layering,
        7,
        salts,
        karakuri_engine::set::Wiring::default(),
    )
    .expect("several sources and some renderers");
    set.resize(&gpu.device, W, H);
    // Head-on and still, so the lattice lands on the frame as a lattice.
    set.aim_camera(karakuri_engine::camera::Orbit {
        radius: 6.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    });
    set
}

pub(crate) fn frame(gpu: &Gpu, set: &mut Set) -> Vec<f32> {
    let present = Present::new(&gpu.device, wgpu::TextureFormat::Rgba16Float, W, H);
    set.prepare(&gpu.queue, 1, &Signals::default());
    let bytes_per_row = W * 8;
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(bytes_per_row * H),
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
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
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

pub(crate) fn f16(bits: u16) -> f32 {
    let sign = f32::from_bits(u32::from(bits & 0x8000) << 16);
    let exp = (bits >> 10) & 0x1f;
    let mant = u32::from(bits & 0x3ff);
    let v = match exp {
        0 => f32::from_bits(mant << 13) * 2.0f32.powi(-112),
        0x1f => f32::from_bits(0x7f80_0000 | (mant << 13)),
        _ => f32::from_bits(((u32::from(exp) + 112) << 23) | (mant << 13)),
    };
    f32::from_bits(v.to_bits() | sign.to_bits())
}

/// Total light in the frame.
pub(crate) fn total(gpu: &Gpu, set: &mut Set) -> f64 {
    frame(gpu, set)
        .chunks_exact(4)
        .map(|t| f64::from(t[0] + t[1] + t[2]))
        .sum()
}
