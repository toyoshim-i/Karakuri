//! Several geometry sources in one Set.
//!
//! `docs/ir-spec.md`, "Multiple L1 sources". The claim is that two geometries
//! can share a Set without colliding: **each counts its own `seed` from zero**,
//! so a structured layout works identically in both, and **each has its own
//! hash salt**, so two identical grids differ in colour by default rather than
//! by being arranged to.
//!
//! Both halves are measured on the picture, because both are about what the
//! elements *are* rather than about how many of them there happen to be.

use karakuri_engine::set::Layering;
use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const W: u32 = 64;
const H: u32 = 64;
const SIDE: u32 = 8;

/// A lattice laid out by `seed`, tinted by `hash1(seed)`.
///
/// **Both halves matter and they are different claims.** The position is
/// structure — `seed % side` — and has to be *identical* between two sources;
/// the tint is randomness and has to *differ*. One procedure gives both, which
/// is what makes the pair of assertions below a pair rather than two fixtures.
fn lattice(name: &str, z: f32) -> String {
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

const DOTS: &str = r#"
proc dots {
  kind  L4
  blend additive

  consumes position, tint

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 2.0;
  }

  fragment {
    color = vec4(tint, 1.0);
  }
}
"#;

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
}

fn build(gpu: &Gpu, l1s: &[&str]) -> Set {
    build_with(gpu, l1s, &[DOTS], Layering::Overdraw)
}

fn build_with(gpu: &Gpu, l1s: &[&str], l4s: &[&str], layering: Layering) -> Set {
    let compiled: Vec<Checked> = l1s.iter().map(|s| compile(s)).collect();
    let sources: Vec<(&Checked, u32)> = compiled.iter().map(|c| (c, 64)).collect();
    let draw: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
    let draw_refs: Vec<&Checked> = draw.iter().collect();
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &sources,
        &[],
        None,
        None,
        &draw_refs,
        layering,
        7,
    )
    .expect("several sources and some renderers");
    set.resize(&gpu.device, W, H);
    // Head-on and still, so the lattice lands on the frame as a lattice.
    set.camera = karakuri_engine::camera::Orbit {
        radius: 6.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    };
    set
}

fn frame(gpu: &Gpu, set: &mut Set) -> Vec<f32> {
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
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device.poll(wgpu::PollType::Wait).expect("poll");
    let data = slice.get_mapped_range();
    let out: Vec<f32> = data
        .chunks_exact(2)
        .map(|b| f16(u16::from_le_bytes([b[0], b[1]])))
        .collect();
    drop(data);
    readback.unmap();
    out
}

fn f16(bits: u16) -> f32 {
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

/// Total light in the frame — proportional to how many elements drew, under
/// `blend additive`, whether or not they overlap.
fn total(gpu: &Gpu, set: &mut Set) -> f64 {
    frame(gpu, set).chunks_exact(4).map(|t| f64::from(t[0] + t[1] + t[2])).sum()
}

// ---------------------------------------------------------------------------

/// **Both sources simulate and both are drawn.** One renderer, two geometries,
/// and the renderer was written knowing about neither.
#[test]
fn two_sources_are_both_simulated_and_both_drawn() {
    let gpu = Gpu::headless().expect("a GPU");
    let one = lattice("one", 0.0);
    let two = lattice("two", 0.0);

    let mut single = build(&gpu, &[&one]);
    let mut pair = build(&gpu, &[&one, &two]);

    let (a, b) = (total(&gpu, &mut single), total(&gpu, &mut pair));
    assert!(a > 0.0, "the single source drew something");
    assert!(
        b > a * 1.5,
        "two sources at the same place should be brighter than one: {b} against {a}"
    );

    assert_eq!(
        pair.capacity(),
        single.capacity() * 2,
        "and a Set of two allocates both"
    );
}

/// **Each source counts its own `seed` from zero**, so a structured layout
/// works identically in both.
///
/// Two lattices at the same place, laid out by `seed % side`: if the second
/// source's counter continued from the first's, its elements would land on
/// different lattice points and the figure would be twice as wide. The lit
/// *area* is the measurement, which is what makes it about position rather
/// than about count — the light test above cannot see this.
#[test]
fn each_source_counts_its_own_seed_from_zero() {
    let gpu = Gpu::headless().expect("a GPU");
    let one = lattice("one", 0.0);
    let two = lattice("two", 0.0);

    let lit = |set: &mut Set| -> usize {
        frame(&gpu, set)
            .chunks_exact(4)
            .filter(|t| t[0] + t[1] + t[2] > 0.02)
            .count()
    };

    let mut single = build(&gpu, &[&one]);
    let mut pair = build(&gpu, &[&one, &two]);
    let (a, b) = (lit(&mut single), lit(&mut pair));
    assert!(a > 0, "the single source covered something");
    assert_eq!(
        a, b,
        "two lattices laid out by `seed` land on the same points, so the pair covers exactly \
         what one does — a shared counter would put the second somewhere else"
    );
}

/// **Each source has its own hash salt**, so two identical geometries differ in
/// colour by default rather than by being arranged to.
///
/// Same lattice twice, at the same place, tinted from `hash1(seed)`. With one
/// salt the two would agree colour for colour and the frame would be exactly
/// twice as bright as one of them; with two, the colours differ and the sum
/// does not land on that number.
#[test]
fn two_sources_of_one_procedure_differ_in_colour() {
    let gpu = Gpu::headless().expect("a GPU");
    let one = lattice("one", 0.0);
    let two = lattice("two", 0.0);

    let mut single = build(&gpu, &[&one]);
    let mut pair = build(&gpu, &[&one, &two]);

    // Per channel, because a salt that differed only in total brightness would
    // be a salt that had not really changed the colours.
    let sum = |set: &mut Set| -> [f64; 3] {
        let px = frame(&gpu, set);
        let mut out = [0.0; 3];
        for t in px.chunks_exact(4) {
            for c in 0..3 {
                out[c] += f64::from(t[c]);
            }
        }
        out
    };
    let a = sum(&mut single);
    let b = sum(&mut pair);

    let identical = (0..3).all(|c| (b[c] - a[c] * 2.0).abs() < a[c] * 0.02);
    assert!(
        !identical,
        "the second source drew the same colours as the first, so the salt did not move: \
         {a:?} against {b:?}"
    );
    // And it is still the same *material*: every channel grew, so the second
    // source drew a lattice rather than nothing.
    assert!((0..3).all(|c| b[c] > a[c] * 1.2), "{a:?} against {b:?}");
}

/// A second renderer, so that "two pipelines" is two of each.
const HALO: &str = r#"
proc halo {
  kind  L4
  blend additive

  consumes position, tint

  // **Dark by default**, so that any light in the frame came from the
  // published control. With a non-zero default, a control reaching one renderer
  // and not the other is indistinguishable from one reaching both.
  param exposure : float [0.0, 2.0] = 0.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 5.0;
  }

  fragment {
    let d = length(point_coord * 2.0 - 1.0);
    color = vec4(tint * exposure, max(0.0, 1.0 - d) * 0.4);
  }
}
"#;

const LIT: &str = r#"
proc lit {
  kind  L4
  blend additive

  consumes position, tint

  // **Dark by default**, so that any light in the frame came from the
  // published control. With a non-zero default, a control reaching one renderer
  // and not the other is indistinguishable from one reaching both.
  param exposure : float [0.0, 2.0] = 0.0

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 2.0;
  }

  fragment {
    color = vec4(tint * exposure, 1.0);
  }
}
"#;

/// **Two pipelines, merged by a nested L5, published as one control.**
///
/// The case the L5 node was built for and could not be shown doing: what a
/// merge composited was several renderers over *one* geometry, because a Set
/// held one source. Two sources and two renderers is four instances, two
/// targets, and — the point — **one knob**.
///
/// The control is a wildcard, so it reaches every procedure declaring the name
/// rather than one addressed node; both renderers move together, which is what
/// "published as one control" means and is the half a picture alone cannot
/// show.
#[test]
fn two_pipelines_merge_and_publish_as_one_control() {
    let gpu = Gpu::headless().expect("a GPU");
    let one_src = lattice("one", 0.0);
    let two_src = lattice("two", 0.0);

    let mut set = build_with(&gpu, &[&one_src, &two_src], &[LIT, HALO], Layering::Composite);
    set.publish(karakuri_engine::set::Published {
        name: "level".to_string(),
        at: None,
        key: "exposure".to_string(),
        range: [0.0, 2.0],
    })
    .expect("both renderers declare `exposure`, so a wildcard reaches them");

    assert_eq!(set.published().len(), 1, "one control over two procedures");

    // **The control has to reach every renderer, and the way to see that is to
    // turn each of them off by hand afterwards.** With the published maximum in
    // place, silencing renderer 0 must take light out and leave some, and
    // silencing renderer 1 as well must take the rest — which is only true if
    // the control put a value into both.
    assert!(set.set_published("level", 2.0));
    let both = total(&gpu, &mut set);

    assert!(set.set_param_at(karakuri_ir::Kind::L4, 0, "exposure", 0.0));
    let one = total(&gpu, &mut set);
    assert!(one < both * 0.9, "silencing one renderer takes light out: {one} of {both}");
    assert!(one > both * 0.05, "and leaves the other lit: {one} of {both}");

    assert!(set.set_param_at(karakuri_ir::Kind::L4, 1, "exposure", 0.0));
    let none = total(&gpu, &mut set);
    assert!(
        none < both * 0.02,
        "silencing both takes all of it, so the control had reached both: {none} of {both}"
    );

    // **And the merge folds both sources into each target**, which is what
    // "first onto this attachment" has to mean when several sources draw into
    // one: the composited frame carries twice a single source's light, not one
    // source's because the second cleared it.
    assert!(set.set_published("level", 2.0));
    let two_sources = total(&gpu, &mut set);
    let mut alone = build_with(&gpu, &[&one_src], &[LIT, HALO], Layering::Composite);
    alone
        .publish(karakuri_engine::set::Published {
            name: "level".to_string(),
            at: None,
            key: "exposure".to_string(),
            range: [0.0, 2.0],
        })
        .expect("the same interface");
    assert!(alone.set_published("level", 2.0));
    let one_source = total(&gpu, &mut alone);
    assert!(
        two_sources > one_source * 1.5,
        "two sources composited hold more light than one: {two_sources} against {one_source}"
    );
}
