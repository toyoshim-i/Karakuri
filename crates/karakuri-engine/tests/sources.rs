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

fn build(gpu: &Gpu, l1s: &[&str]) -> Set {
    build_with(gpu, l1s, &[DOTS], Layering::Overdraw)
}

fn build_paired(gpu: &Gpu, l1s: &[&str], l2: &str) -> Result<Set, karakuri_engine::set::SetError> {
    let compiled: Vec<Checked> = l1s.iter().map(|s| compile(s)).collect();
    let sources: Vec<(&Checked, u32)> = compiled.iter().map(|c| (c, 64)).collect();
    let l2 = compile(l2);
    let l4 = compile(DOTS);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &sources,
        &[&l2],
        None,
        None,
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::NodeNames::default(),
    )?;
    set.resize(&gpu.device, W, H);
    set.camera = karakuri_engine::camera::Orbit {
        radius: 6.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    };
    Ok(set)
}

/// The same Set, with a salt assigned to each geometry — `None`, or an entry
/// past the end, is a source nobody salted, which is what a bare `--set` hands
/// over.
fn build_salted(gpu: &Gpu, l1s: &[&str], salts: &[Option<u32>]) -> Set {
    build_all(gpu, l1s, &[DOTS], Layering::Overdraw, salts)
}

fn build_with(gpu: &Gpu, l1s: &[&str], l4s: &[&str], layering: Layering) -> Set {
    build_all(gpu, l1s, l4s, layering, &[])
}

/// Every dial the two above turn, in one place — a salt list is one more of
/// them, and a second copy of this function with one argument changed is how
/// two builds in one file end up disagreeing about the camera.
fn build_all(
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
        None,
        None,
        &draw_refs,
        layering,
        7,
        salts,
        karakuri_engine::set::NodeNames::default(),
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
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
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
    frame(gpu, set)
        .chunks_exact(4)
        .map(|t| f64::from(t[0] + t[1] + t[2]))
        .sum()
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

/// How many pixels two frames disagree about, past what a float target rounds.
///
/// Zero is the same picture and a large number is a different one; there is
/// nothing interesting in between here, because a salt that moved moves every
/// element it touches.
fn disagreements(a: &[f32], b: &[f32]) -> usize {
    assert_eq!(a.len(), b.len(), "two frames of one canvas");
    a.chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(x, y)| (0..3).any(|c| (x[c] - y[c]).abs() > 1e-3))
        .count()
}

/// **A salt that was recorded travels with its geometry rather than with its
/// position in the list**, which is the whole of what assigning one buys over
/// deriving one — `docs/ir-spec.md`, "A `source` value is assigned and
/// recorded, never derived".
///
/// The story a Set file tells, in four builds: a bare `--set` is salted by
/// ordinal, `--save-set` writes down what it was salted with, `--load-set`
/// hands those numbers back, and a Set whose records come back in the other
/// order is still the Set that was saved. The last build is the control, and it
/// is the behaviour this replaces.
///
/// **Two lattices at different depths**, so that which one is which colour is
/// visible in the frame at all: at one depth they are the same points twice,
/// and two sources trading colours there is a picture nothing can tell apart.
#[test]
fn a_recorded_salt_survives_the_list_being_reordered() {
    let gpu = Gpu::headless().expect("a GPU");
    let near = lattice("near", 0.0);
    let far = lattice("far", 1.4);

    // `--set near.kir,far.kir`: nothing assigned a salt, so each source is
    // salted from the Set's seed and its ordinal.
    let mut run = build_salted(&gpu, &[&near, &far], &[]);
    let saved = frame(&gpu, &mut run);

    // What `--save-set` writes into the `seed` records — asked of the Set that
    // is running rather than worked out a second time.
    let recorded: Vec<Option<u32>> = run.source_salts().iter().copied().map(Some).collect();
    assert_eq!(recorded.len(), 2, "a salt per geometry, and there are two");

    // `--load-set`: the same two geometries, each handed the salt it was
    // running at.
    let mut reloaded = build_salted(&gpu, &[&near, &far], &recorded);
    assert_eq!(
        disagreements(&saved, &frame(&gpu, &mut reloaded)),
        0,
        "a reloaded Set drew something other than the Set that was saved"
    );

    // The same Set with its two geometries read in the other order. A record is
    // addressed, so nothing about the Set has changed but the order it arrives
    // in — and the picture must not know.
    let mut swapped = build_salted(&gpu, &[&far, &near], &[recorded[1], recorded[0]]);
    assert_eq!(
        disagreements(&saved, &frame(&gpu, &mut swapped)),
        0,
        "reordering a recorded Set repainted it, so the salt is still following the \
         position rather than the geometry"
    );

    // **The control.** The same reorder with nothing recorded hands each source
    // the other one's salt, and the two lattices trade colours. Without this,
    // every assertion above would pass against an engine that ignored the
    // salts it was given.
    let mut derived = build_salted(&gpu, &[&far, &near], &[]);
    assert!(
        disagreements(&saved, &frame(&gpu, &mut derived)) > 16,
        "reordering an unrecorded Set drew the same picture, so this test cannot tell a \
         salt that follows the geometry from one that follows the list"
    );
}

/// **[`Set::source_salts`] answers for every geometry**, assigned or not, which
/// is what makes it something a writer can record: a file that carried salts
/// for the sources somebody happened to name would come back as a Set whose
/// other sources are salted by whatever the ordinal was at the time.
#[test]
fn source_salts_answers_for_the_derived_ones_too() {
    let gpu = Gpu::headless().expect("a GPU");
    let one = lattice("one", 0.0);
    let two = lattice("two", 0.0);

    let both = build_salted(&gpu, &[&one, &two], &[Some(0xfeed), Some(0xbeef)]);
    assert_eq!(both.source_salts(), [0xfeed, 0xbeef]);

    // A file that named the first geometry's salt and not the second's — which
    // is every Set file written before a salt was recorded per source.
    let half = build_salted(&gpu, &[&one, &two], &[Some(0xfeed)]);
    assert_eq!(
        half.source_salts(),
        [0xfeed, karakuri_engine::set::derived_salt(7, 1)],
        "an unassigned source is salted from the Set's seed and its ordinal, and says so"
    );
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

    let mut set = build_with(
        &gpu,
        &[&one_src, &two_src],
        &[LIT, HALO],
        Layering::Composite,
    );
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
    assert!(
        one < both * 0.9,
        "silencing one renderer takes light out: {one} of {both}"
    );
    assert!(
        one > both * 0.05,
        "and leaves the other lit: {one} of {both}"
    );

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

/// A lattice offset along the screen's horizontal, so that a morph *moves*.
fn lattice_at(name: &str, z: f32) -> String {
    lattice(name, 0.0).replace("0.0);\n    tint", &format!("{z:?});\n    tint"))
}

const MORPH: &str = r#"
proc morph {
  kind  L2
  pairs

  param k : float [0.0, 1.0] = 0.0

  consumes position

  deform {
    position = mix(position, other.position, vec3(k, k, k));
  }
}
"#;

/// **A pairing L2 blends two geometries element by element.**
///
/// The two lattices sit at different depths along the screen's horizontal, so
/// `k` slides the material from one to the other — and at the ends it has to
/// land *on* each of them, which is what makes this a morph rather than a
/// wobble. Measured as the horizontal centre of the lit texels.
#[test]
fn a_pairing_l2_morphs_between_two_sources() {
    let gpu = Gpu::headless().expect("a GPU");
    let near = lattice_at("near", -1.2);
    let far = lattice_at("far", 1.2);

    let centre_x = |set: &mut Set| -> f32 {
        let px = frame(&gpu, set);
        let (mut sum, mut weight) = (0.0f64, 0.0f64);
        for (i, t) in px.chunks_exact(4).enumerate() {
            let v = f64::from(t[0] + t[1] + t[2]);
            if v > 0.02 {
                sum += v * f64::from(i as u32 % W);
                weight += v;
            }
        }
        assert!(weight > 0.0, "nothing was drawn");
        (sum / weight) as f32
    };

    let mut set = build_paired(&gpu, &[&near, &far], MORPH).expect("two static sources");
    let at_zero = centre_x(&mut set);

    assert!(set.set_param_at(karakuri_ir::Kind::L2, 0, "k", 1.0));
    let at_one = centre_x(&mut set);

    // **The two ends are the two sources.** Built alone, each lands where the
    // morph's corresponding end does — which is the assertion that makes this a
    // blend of *those* geometries rather than of something else.
    let mut just_near = build(&gpu, &[&near]);
    let mut just_far = build(&gpu, &[&far]);
    let (n, f) = (centre_x(&mut just_near), centre_x(&mut just_far));

    assert!(
        (at_zero - n).abs() < 1.5,
        "k=0 is the near source: {at_zero} against {n}"
    );
    assert!(
        (at_one - f).abs() < 1.5,
        "k=1 is the paired source: {at_one} against {f}"
    );
    assert!(
        (n - f).abs() > 8.0,
        "the two sources are far enough apart to tell apart"
    );

    // And the middle is between them, so `k` is a dial rather than a switch.
    assert!(set.set_param_at(karakuri_ir::Kind::L2, 0, "k", 0.5));
    let half = centre_x(&mut set);
    assert!(
        (half - (n + f) / 2.0).abs() < 2.0,
        "k=0.5 sits between the two: {half} against {}",
        (n + f) / 2.0
    );
}

/// **The paired geometry is not drawn**, which is the question a Set-level
/// pairing answers by construction: a pairing Set has one source with two
/// simulations, one chain and one set of renderers.
#[test]
fn the_paired_geometry_is_never_drawn_on_its_own() {
    let gpu = Gpu::headless().expect("a GPU");
    let near = lattice_at("near", -1.2);
    let far = lattice_at("far", 1.2);

    let mut paired = build_paired(&gpu, &[&near, &far], MORPH).expect("two static sources");
    let mut alone = build(&gpu, &[&near]);

    let (a, b) = (total(&gpu, &mut paired), total(&gpu, &mut alone));
    assert!(
        (a - b).abs() < b * 0.05,
        "a morph at k=0 holds one lattice's worth of light, not two: {a} against {b}"
    );
    assert_eq!(
        paired.capacity(),
        alone.capacity(),
        "and allocates one lattice to be drawn"
    );
}

/// **Pairing is by slot index, so a source that compacts cannot be paired.**
/// After a compaction element 5 of one source is not element 5 of the other,
/// and the pairing would match each element with a stranger without changing a
/// line of the `.kir`.
#[test]
fn a_source_that_compacts_cannot_be_paired() {
    let gpu = Gpu::headless().expect("a GPU");
    let near = lattice_at("near", -1.2);
    let culled = lattice_at("culled", 1.2).replace(
        "    tint     =",
        "    if seed == 3u { kill(); }\n    tint     =",
    );

    let err = build_paired(&gpu, &[&near, &culled], MORPH)
        .err()
        .expect("a killing source moves its elements between slots");
    let text = err.to_string();
    assert!(
        text.contains("culled") && text.contains("slot"),
        "the refusal names the source and why: {text}"
    );
}

/// A pairing L2 needs exactly two sources, and it has to be first in the chain
/// because its second input is a *source* rather than whatever reached it.
#[test]
fn a_pairing_l2_states_what_it_needs() {
    let gpu = Gpu::headless().expect("a GPU");
    let near = lattice_at("near", -1.2);

    let err = build_paired(&gpu, &[&near], MORPH)
        .err()
        .expect("one source is not two");
    assert!(err.to_string().contains("this Set has 1"), "{err}");
}

/// **The paired geometry's own `param`s reach it.**
///
/// It is an L1 procedure like any other and `--param L1:1:…` addresses it — but
/// every L1 used to resolve against the *first* source's map, so a name the two
/// did not share came back as a miss and reached the shader as zero. The shape
/// that hides: the picture still has a shape in it, drawn from a value nobody
/// set. Here the far lattice collapses to the origin, which reads as a morph
/// that ends somewhere it should not.
#[test]
fn a_paired_sources_own_params_reach_it() {
    let gpu = Gpu::headless().expect("a GPU");
    let near = lattice_at("near", -1.2);
    // The far side's offset is a param of its own, and the near side has no
    // such name — which is exactly the case a shared map answers wrongly.
    let far = lattice_at("far", 1.2).replace(
        "  emit position, tint",
        "  param push : float [0.0, 2.0] = 0.0\n\n  emit position, tint",
    );
    let far = far.replace("* 0.5 - 1.75, 1.2)", "* 0.5 - 1.75, 1.2 + push)");

    let centre_x = |set: &mut Set| -> f32 {
        let px = frame(&gpu, set);
        let (mut sum, mut weight) = (0.0f64, 0.0f64);
        for (i, t) in px.chunks_exact(4).enumerate() {
            let v = f64::from(t[0] + t[1] + t[2]);
            if v > 0.02 {
                sum += v * f64::from(i as u32 % W);
                weight += v;
            }
        }
        assert!(weight > 0.0, "nothing was drawn");
        (sum / weight) as f32
    };

    let mut set = build_paired(&gpu, &[&near, &far], MORPH).expect("two static sources");
    assert!(set.set_param_at(karakuri_ir::Kind::L2, 0, "k", 1.0));
    let before = centre_x(&mut set);

    // `push` belongs to the *second* L1 procedure, which is the paired one.
    assert!(
        set.set_param_at(karakuri_ir::Kind::L1, 1, "push", 2.0),
        "the paired geometry is an L1 procedure and `L1:1` addresses it"
    );
    let after = centre_x(&mut set);
    assert!(
        (after - before).abs() > 3.0,
        "the paired source's own param has to reach its own shader: {before} against {after}"
    );
}

/// **Both sides of a pairing carry the same element struct**, including the
/// slots the *chain* decided on rather than either procedure.
///
/// `age` is derived here — the renderer consumes it and neither source emits it
/// — so every element grows a `birth_t` slot. Build the far side without that
/// decision and its stride is sixteen bytes short of the struct the pairing
/// node addresses it with, so `other[i].position` reads from the middle of the
/// element before it. The picture still has a lattice in it; it is simply not
/// the far one.
#[test]
fn both_sides_of_a_pairing_share_the_element_struct() {
    let gpu = Gpu::headless().expect("a GPU");
    let near = lattice_at("near", -1.2);
    let far = lattice_at("far", 1.2);

    // Consumes an attribute nothing emits, so the chain derives it and every
    // element gains a slot for what the rule reads.
    let aged = DOTS
        .replace("consumes position, tint", "consumes position, tint, age")
        .replace(
            "    color = vec4(tint, 1.0);",
            "    color = vec4(tint, 1.0) * (1.0 + age * 0.0);",
        );

    let compiled: Vec<Checked> = [&near, &far].iter().map(|s| compile(s)).collect();
    let sources: Vec<(&Checked, u32)> = compiled.iter().map(|c| (c, 64)).collect();
    let l2 = compile(MORPH);
    let l4 = compile(&aged);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &sources,
        &[&l2],
        None,
        None,
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::NodeNames::default(),
    )
    .expect("two static sources and a derived attribute");
    set.resize(&gpu.device, W, H);
    set.camera = karakuri_engine::camera::Orbit {
        radius: 6.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    };

    let centre_x = |set: &mut Set| -> f32 {
        let px = frame(&gpu, set);
        let (mut sum, mut weight) = (0.0f64, 0.0f64);
        for (i, t) in px.chunks_exact(4).enumerate() {
            let v = f64::from(t[0] + t[1] + t[2]);
            if v > 0.02 {
                sum += v * f64::from(i as u32 % W);
                weight += v;
            }
        }
        assert!(weight > 0.0, "nothing was drawn");
        (sum / weight) as f32
    };

    assert!(set.set_param_at(karakuri_ir::Kind::L2, 0, "k", 1.0));
    let at_one = centre_x(&mut set);

    // The far lattice, built alone with the same renderer, is where k=1 has to
    // land — and it is a *position* rather than a brightness, because reading
    // the wrong bytes gives a picture that is lit and in the wrong place.
    let mut just_far = build_with(&gpu, &[&far], &[&aged], Layering::Overdraw);
    let f = centre_x(&mut just_far);
    assert!(
        (at_one - f).abs() < 1.5,
        "k=1 has to land on the far lattice: {at_one} against {f}"
    );
}

/// **Every node of a Set has a name, and a name resolves to the node the rest
/// of this system addresses by position.** A name is an alias over
/// `(layer, index)` — the same shape `Published` gives a parameter — so it can
/// be chosen, recorded and changed without anything underneath it moving.
#[test]
fn a_name_resolves_to_the_node_it_addresses() {
    let gpu = Gpu::headless().expect("a GPU");
    let near = compile(&lattice("near_grid", 0.0));
    let far = compile(&lattice("far_grid", 0.5));
    let draw = compile(DOTS);

    let set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&near, 64), (&far, 64)],
        &[],
        None,
        None,
        &[&draw],
        Layering::Overdraw,
        0,
        &[],
        karakuri_engine::set::NodeNames {
            l1s: &[Some("near".to_string()), None],
            ..Default::default()
        },
    )
    .expect("builds");

    // What was written is what it is called; what was not is called after its
    // procedure.
    assert_eq!(
        set.node_names(),
        ["near", "far_grid", "dots"].map(String::from)
    );
    assert_eq!(set.node_named("near"), Some((karakuri_ir::Kind::L1, 0)));
    assert_eq!(set.node_named("far_grid"), Some((karakuri_ir::Kind::L1, 1)));
    assert_eq!(set.node_named("dots"), Some((karakuri_ir::Kind::L4, 0)));
    assert_eq!(set.node_named("nothing_is_called_this"), None);
}

/// **Two uses of one procedure are two nodes**, so the derived name — which is
/// the procedure's, and therefore a *type* name — collides and the second is
/// told apart. Derived in the engine and nowhere else: a caller that derived
/// too would be the second place the fact lives.
#[test]
fn a_procedure_used_twice_gives_its_second_node_a_different_name() {
    let gpu = Gpu::headless().expect("a GPU");
    let grid = compile(&lattice("grid", 0.0));
    let draw = compile(DOTS);

    let set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&grid, 64), (&grid, 64)],
        &[],
        None,
        None,
        &[&draw, &draw],
        Layering::Overdraw,
        0,
        &[],
        karakuri_engine::set::NodeNames::default(),
    )
    .expect("builds");

    assert_eq!(
        set.node_names(),
        ["grid", "grid-2", "dots", "dots-2"].map(String::from)
    );
    assert_eq!(set.node_named("grid-2"), Some((karakuri_ir::Kind::L1, 1)));
    assert_eq!(set.node_named("dots-2"), Some((karakuri_ir::Kind::L4, 1)));
}

/// **Two *written* names that collide are refused**, where two derived ones are
/// told apart. A written name is an address somebody chose, so picking one of
/// the two for them would leave whatever was pointed at it following a
/// tie-break.
#[test]
fn two_written_names_that_collide_are_refused() {
    let gpu = Gpu::headless().expect("a GPU");
    let grid = compile(&lattice("grid", 0.0));
    let draw = compile(DOTS);

    let err = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&grid, 64)],
        &[],
        None,
        None,
        &[&draw],
        Layering::Overdraw,
        0,
        &[],
        karakuri_engine::set::NodeNames {
            l1s: &[Some("shape".to_string())],
            l4s: &[Some("shape".to_string())],
            ..Default::default()
        },
    );
    let err = match err {
        Err(e) => e,
        Ok(_) => panic!("two nodes cannot share a name"),
    };
    assert!(format!("{err}").contains("both called `shape`"), "{err}");
}

/// **A written name is not stolen by a derived one that reaches it first.**
/// `dots` is written on the *second* renderer, so the first — which would
/// otherwise derive `dots` from its procedure — has to take `dots-2`.
#[test]
fn a_derived_name_never_takes_one_that_was_written() {
    let gpu = Gpu::headless().expect("a GPU");
    let grid = compile(&lattice("grid", 0.0));
    let draw = compile(DOTS);

    let set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&grid, 64)],
        &[],
        None,
        None,
        &[&draw, &draw],
        Layering::Overdraw,
        0,
        &[],
        karakuri_engine::set::NodeNames {
            l4s: &[None, Some("dots".to_string())],
            ..Default::default()
        },
    )
    .expect("builds");

    assert_eq!(set.node_named("dots"), Some((karakuri_ir::Kind::L4, 1)));
    // Node 0 is the geometry; node 1 is the renderer that had to give way.
    assert_eq!(set.node_names()[1], "dots-2");
}
