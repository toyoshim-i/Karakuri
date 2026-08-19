//! A `kind Field` procedure, spliced into whoever evaluates it.
//!
//! A field has no node: no buffer, no pass, no position in the chain. So there
//! is nothing to read back and nothing to address, and every claim here is
//! about the picture a *caller* draws — which is the right place for them,
//! since a field that produced the correct distances and reached nobody would
//! be a feature that does not exist.

use karakuri_engine::set::{Layering, SetError};
use karakuri_engine::{Gpu, Present, Set, Signals, VideoSource};
use karakuri_ir::typed::Checked;

const W: u32 = 64;
const H: u32 = 64;

/// One element, so the paired geometry costs nothing. A fullscreen renderer
/// reads no element, but a Set always has an L1.
const STILL: &str = r#"
proc still {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

/// A sphere of a declared radius, and nothing else.
const BALL: &str = r#"
proc ball {
  kind Field

  param radius : float [0.1, 3.0] = 0.8

  field {
    distance = sd_sphere(point, radius);
  }
}
"#;

/// Marches whatever the Set gives it. **It contains no shape at all**, which is
/// the whole claim: the same renderer draws any field.
const LENS: &str = r#"
proc lens {
  kind  L4
  blend additive

  fragment {
    var p = eye;
    var hit = 0.0;
    for i in 0..40 {
      let d = field(p);
      if d < 0.005 {
        hit = 1.0;
      }
      p = p + ray * max(d, 0.005);
    }
    color = vec4(hit, hit, hit, 1.0);
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

fn build(gpu: &Gpu, field: Option<&str>, l4: &str) -> Result<Set, SetError> {
    let field = field.map(compile);
    let mut set = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(STILL), 1)],
        &[],
        None,
        field.as_ref(),
        &[&compile(l4)],
        Layering::Overdraw,
        7,
    )?;
    set.resize(&gpu.device, W, H);
    set.camera = karakuri_engine::camera::Orbit {
        radius: 5.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    };
    Ok(set)
}

/// How many texels the marcher hit, which is the figure's area on screen.
fn covered(gpu: &Gpu, set: &mut Set) -> usize {
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
    let out = data
        .chunks_exact(8)
        .filter(|t| f16(u16::from_le_bytes([t[0], t[1]])) > 0.01)
        .count();
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

// ---------------------------------------------------------------------------

/// **A renderer with no shape in it draws a shape**, because the Set gave it
/// one. That is the whole feature.
#[test]
fn a_marcher_draws_a_field_it_does_not_contain() {
    let gpu = Gpu::headless().expect("a GPU");
    let mut set = build(&gpu, Some(BALL), LENS).expect("a Set with a field");
    let hit = covered(&gpu, &mut set);
    assert!(
        hit > 100,
        "the marcher should have found the sphere, and covered {hit} texels"
    );
    assert!(
        hit < (W * H) as usize,
        "and should not have filled the frame"
    );
}

/// **The field's own `param` is an operator's to ride**, addressed by its kind
/// even though it addresses no node — every caller writes the same value into
/// its own uniform.
#[test]
fn a_fields_param_reaches_every_caller() {
    let gpu = Gpu::headless().expect("a GPU");
    let mut small = build(&gpu, Some(BALL), LENS).expect("a Set with a field");
    let mut large = build(&gpu, Some(BALL), LENS).expect("a Set with a field");
    assert!(
        large.set_param_at(karakuri_ir::Kind::Field, 0, "radius", 2.0),
        "`Field:0:radius` has to reach the field's map"
    );

    let (a, b) = (covered(&gpu, &mut small), covered(&gpu, &mut large));
    assert!(
        b > a * 2,
        "a radius of 2.0 should cover much more than 0.8 does: {b} against {a}"
    );
}

/// **A procedure that evaluates a field and a Set that has none is refused by
/// name.** The call lowers to a function name, so a module without it is WGSL
/// naga refuses — a panic on the thread that built it, from a `.kir` the
/// checker accepted, which is the one shape a composition check exists to
/// prevent.
#[test]
fn a_caller_with_no_field_is_refused_rather_than_fatal() {
    let gpu = Gpu::headless().expect("a GPU");
    let err = build(&gpu, None, LENS)
        .err()
        .expect("nothing provides `field(p)`");
    let text = err.to_string();
    assert!(text.contains("lens") && text.contains("field"), "{text}");
}

/// **Neither file is over budget and the pair is**, which is what a Set-level
/// check exists for: a `field(p)` weighs nothing where a single procedure is
/// estimated, so the ceiling each of them passed was applied to a figure
/// missing the other.
#[test]
fn a_caller_and_its_field_are_costed_together() {
    let gpu = Gpu::headless().expect("a GPU");
    // Expensive on its own terms and nowhere near any ceiling: a field has no
    // ceiling of its own, precisely because what it costs is the caller's.
    let heavy = r#"
proc heavy {
  kind Field

  field {
    var d = 0.0;
    for i in 0..64 {
      d = d + fbm(point, 4) * 0.01;
    }
    distance = length(point) - 1.0 + d;
  }
}
"#;
    let cost = karakuri_ir::cost::estimate(&compile(heavy)).expect("a field has no ceiling");
    assert!(
        cost.ops_per_evaluation > 0,
        "the field costs something per evaluation"
    );

    let err = build(&gpu, Some(heavy), LENS)
        .err()
        .expect("forty evaluations of an expensive field is over the fragment ceiling");
    let text = err.to_string();
    assert!(
        text.contains("heavy") && text.contains("lens") && text.contains("evaluations"),
        "the refusal names both and says how many evaluations: {text}"
    );
}

/// **Two renderers in one Set read the same field value.** A field has no node,
/// so nothing owns the value: every caller writes it into its own uniform, and
/// "the same answer everywhere" is a property of the Set rather than of any one
/// of them.
///
/// Both renderers draw the same figure, so they agree exactly or not at all —
/// and the assertion is against a *third* Set at a different radius, so two
/// renderers agreeing on the wrong number cannot pass.
#[test]
fn two_renderers_in_one_set_agree_on_the_fields_value() {
    let gpu = Gpu::headless().expect("a GPU");
    let field = compile(BALL);
    let l4 = compile(LENS);

    let mut both = Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(STILL), 1)],
        &[],
        None,
        Some(&field),
        &[&l4, &l4],
        Layering::Overdraw,
        7,
    )
    .expect("two renderers over one field");
    both.resize(&gpu.device, W, H);
    both.camera = karakuri_engine::camera::Orbit {
        radius: 5.0,
        speed: 0.0,
        height: 0.0,
        ..Default::default()
    };
    assert!(both.set_param_at(karakuri_ir::Kind::Field, 0, "radius", 2.0));

    let mut one = build(&gpu, Some(BALL), LENS).expect("one renderer");
    assert!(one.set_param_at(karakuri_ir::Kind::Field, 0, "radius", 2.0));

    // Additive, so two renderers drawing the same figure cover the same texels
    // — the count is the figure's area, not its brightness.
    let (a, b) = (covered(&gpu, &mut both), covered(&gpu, &mut one));
    assert_eq!(a, b, "both renderers found the same sphere");

    let mut small = build(&gpu, Some(BALL), LENS).expect("one renderer");
    assert!(
        covered(&gpu, &mut small) < b / 2,
        "and the default radius covers much less, so the agreement above is not agreement on \
         a value nothing set"
    );
}

/// **A field's `param` is an operator's on every surface, not just `--param`.**
/// It has no node, and five separate loops over the layers decided which
/// surfaces reach it — `Field` was in none of them, so a value could be
/// overridden and then not bound, published, read back or saved.
#[test]
fn a_fields_param_reaches_every_operator_surface() {
    let gpu = Gpu::headless().expect("a GPU");
    let mut set = build(&gpu, Some(BALL), LENS).expect("a Set with a field");

    assert!(
        set.bind(karakuri_engine::Binding::new(
            karakuri_ir::Kind::Field,
            "radius",
            "energy",
            karakuri_engine::Curve::Lin,
            [0.3, 2.0],
        ))
        .attached(),
        "a signal has to be attachable to a field's param"
    );

    set.publish(karakuri_engine::set::Published {
        name: "size".to_string(),
        at: Some((karakuri_ir::Kind::Field, 0)),
        key: "radius".to_string(),
        range: [0.3, 2.0],
    })
    .expect("a field's param has to be publishable");

    assert!(
        set.published().iter().any(|p| p.key == "radius"),
        "and the published interface has to show it: {:?}",
        set.published()
    );
}
