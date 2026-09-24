#![allow(unused_imports, dead_code)]

use super::naga_common::*;

// ---------------------------------------------------------------------------
// L3 — the camera
// ---------------------------------------------------------------------------

/// A sweep round the origin: `eye` from the clock and a param, `target` at the
/// centre, and the other four left to their defaults.
fn sweep() -> Checked {
    let angle = bin(
        BinOp::Mul,
        bin(
            BinOp::Mul,
            ambient(Ambient::T, Ty::Float),
            param("speed", Ty::Float),
            Ty::Float,
        ),
        lit_f(std::f32::consts::TAU),
        Ty::Float,
    );
    let on_circle = |f: Builtin| {
        bin(
            BinOp::Mul,
            call(f, vec![local("a", Ty::Float)], Ty::Float),
            param("radius", Ty::Float),
            Ty::Float,
        )
    };
    let camera = TBlock {
        kind: BlockKind::Camera,
        span: span(),
        stmts: vec![
            let_("a", angle),
            assign_output(
                Output::Eye,
                construct(
                    Ty::Vec3,
                    vec![on_circle(Builtin::Cos), lit_f(2.0), on_circle(Builtin::Sin)],
                ),
            ),
            assign_output(Output::Target, construct(Ty::Vec3, vec![lit_f(0.0)])),
        ],
    };

    Checked {
        name: "sweep".to_string(),
        kind: Kind::L3,
        topology: None,
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        retains: false,
        blend: None,
        params: vec![
            param_decl("radius", Ty::Float, 1.0, 40.0),
            param_decl("speed", Ty::Float, 0.0, 2.0),
        ],
        emit: vec![],
        consumes: vec![],
        blocks: vec![camera],
        cost: None,
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

#[test]
fn a_camera_shader_parses_and_validates() {
    validate(&karakuri_codegen::generate_l3(&sweep(), &[]).source);
}

/// Tests that unassigned camera outputs receive standard defaults in the emitted WGSL.
#[test]
fn every_camera_output_is_written_whether_or_not_the_block_assigned_it() {
    let src = karakuri_codegen::generate_l3(&sweep(), &[]).source;
    for field in ["eye", "look_at", "up", "fov_y", "near", "far"] {
        assert!(
            src.contains(&format!("cam.{field}")),
            "`{field}` never reaches the state buffer:\n{src}"
        );
    }
    // And the block's own assignment lands on the local, not on the buffer, so
    // a block that writes `eye` twice writes the buffer once.
    assert_eq!(
        src.matches("cam.eye").count(),
        1,
        "the eye is stored more than once:\n{src}"
    );
    assert!(
        src.contains("_eye ="),
        "the block's assignment did not reach a local:\n{src}"
    );

    // A third of pi, and 0.1 to 100 — see `karakuri_engine::camera::Orbit`'s
    // `Default`. `up` is `+y`, which is the only one of the four that a wrong
    // value would show as a picture rather than as no picture.
    assert!(
        src.contains("_up     = vec3<f32>(0.0, 1.0, 0.0)"),
        "up is not +y:\n{src}"
    );
    assert!(
        src.contains("_fov_y  = 1.0471976"),
        "the field of view is not a third of pi:\n{src}"
    );
    assert!(src.contains("_near   = 0.1"), "{src}");
    assert!(src.contains("_far    = 100.0"), "{src}");
}

/// **One invocation, and no index.** A camera block runs once a frame and
/// produces six numbers — there is nothing to be at index `i` of, so a bounds
/// check or an alive flag here would be a transcription of another layer's
/// entry point rather than this one's.
#[test]
fn a_camera_shader_dispatches_one_invocation_over_nothing() {
    let src = karakuri_codegen::generate_l3(&sweep(), &[]).source;
    assert!(src.contains("@workgroup_size(1)"), "{src}");
    assert!(
        !src.contains("global_invocation_id"),
        "a camera indexed something:\n{src}"
    );
    assert!(
        !src.contains("counts"),
        "a camera read a live range:\n{src}"
    );
}

/// The uniform carries the clock and the params, and nothing per element.
#[test]
fn a_camera_uniform_carries_the_clock_and_its_params() {
    let shader = karakuri_codegen::generate_l3(&sweep(), &[]);
    let names: Vec<&str> = shader
        .uniform_layout
        .fields
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["t", "beats", "dt", "seed_salt", "radius", "speed"]
    );
}

/// Tests that local variables with identical names across mask and deform blocks do not collide.
#[test]
fn a_mask_and_a_deform_may_declare_the_same_local() {
    let src = r#"
proc collides {
  kind L2
  consumes position, age
  mask   { let d = length(position); strength = d; }
  deform { let d = age * 2.0; position = position * d; }
}
"#;
    let parsed = karakuri_ir::parse(src).expect("parses");
    let checked = karakuri_ir::check::check(&parsed).expect("checks");
    let shader = karakuri_codegen::generate_l2(
        &checked,
        &[Attr::Position, Attr::Age],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
        None,
        &[],
    );
    validate(&shader.source);
}

// ---------------------------------------------------------------------------
// Amplification: an L2 whose output count differs from its input's.
// ---------------------------------------------------------------------------

/// The amplifying entry point is a different shape from the endomorphic one —
/// a loop over the copies, a second element index, and a liveness buffer of its
/// own — so it gets its own trip through naga.
#[test]
fn an_amplifying_l2_lowers_to_valid_wgsl() {
    let shader = compiled_l2(
        MIRROR,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    validate(&shader.source);
    assert_eq!(shader.amplify, Some(4));
    assert!(
        shader.synthetic.copy,
        "the node that amplified is where `copy` starts existing"
    );
    assert!(
        shader.element_layout.slots.iter().any(|s| s.name == "copy"),
        "the output carries the index: {:?}",
        shader.element_layout
    );
}

/// **An amplifier writes liveness and an endomorphism does not**, because an
/// endomorphism shares the very buffer its input came with and an amplifier
/// cannot — its own is `factor` times as long. The binding's presence is the
/// observable half of that.
#[test]
fn only_an_amplifying_l2_binds_a_liveness_buffer_to_write() {
    let amplifying = compiled_l2(
        MIRROR,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    assert!(
        amplifying.source.contains("dst_alive"),
        "{}",
        amplifying.source
    );

    let plain = compiled_l2(
        r#"
proc plain {
  kind L2
  consumes position
  deform { position = position * 2.0; }
}
"#,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    assert!(!plain.source.contains("dst_alive"), "{}", plain.source);
}

/// Tests that stacked amplification passes compose copy indices via mixed-radix multiplication.
#[test]
fn a_second_amplifier_composes_the_copy_index_rather_than_replacing_it() {
    // The first has nothing above it, so it starts the numbering from the
    // loop variable alone.
    let first = compiled_l2(
        MIRROR,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    assert!(
        first.source.contains("dst[i].copy = 0u * 4u + _c;"),
        "a first amplifier numbers from nothing: {}",
        first.source
    );

    // The second is handed the first's output, and reads the parent's index.
    let second = compiled_l2(
        r#"
proc again {
  kind    L2
  amplify 3
  consumes position
  deform { position = position * 2.0; }
}
"#,
        &[Attr::Position],
        first.synthetic,
    );
    validate(&second.source);
    assert!(
        second
            .source
            .contains("dst[i].copy = src[_e].copy * 3u + _c;"),
        "a second amplifier composes: {}",
        second.source
    );
}

/// A node below an amplifier that does not amplify itself carries the index
/// through untouched — it is somebody else's identity, and passing it on is the
/// same pass-through every other slot gets.
#[test]
fn a_plain_l2_below_an_amplifier_carries_the_copy_index_through() {
    let plain = compiled_l2(
        r#"
proc plain {
  kind L2
  consumes position
  deform { position = position + vec3(0.0, float(copy), 0.0); }
}
"#,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic { copy: true },
    );
    validate(&plain.source);
    assert!(
        plain.source.contains("dst[i].copy = src[i].copy;"),
        "{}",
        plain.source
    );
    assert!(
        !plain.source.contains("dst_alive"),
        "it did not amplify: {}",
        plain.source
    );
}

/// Where nothing upstream amplified, `copy` is `0u` rather than a read of a
/// slot that is not there — the answer a chain with no amplifier in it gives at
/// every position in it.
#[test]
fn copy_reads_zero_where_no_slot_exists() {
    let plain = compiled_l2(
        r#"
proc plain {
  kind L2
  consumes position
  deform { position = position + vec3(0.0, float(copy), 0.0); }
}
"#,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    validate(&plain.source);
    assert!(plain.source.contains("f32(0u)"), "{}", plain.source);
}

/// A mask over an amplifying node has to end up inside the copy loop, because
/// what it gates is one copy rather than one parent — and the whole thing still
/// has to be WGSL naga accepts, which is what a spliced block in a nested scope
/// is easiest to get wrong.
#[test]
fn an_amplifying_l2_with_a_mask_lowers_to_valid_wgsl() {
    let shader = compiled_l2(
        r#"
proc masked {
  kind    L2
  amplify 4
  param   weight : float [0.0, 1.0] = 1.0
  consumes position
  mask   { let d = length(position); strength = d; }
  deform { let d = float(copy); position = position + vec3(0.0, d, 0.0); }
}
"#,
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
    );
    validate(&shader.source);
}

/// **An L4 reads `copy` the way it reads `seed`** — off the element in the
/// vertex stage, carried to the fragment as a flat varying. It is the other
/// half of identity below an amplifier, and a renderer is downstream.
#[test]
fn an_l4_reading_copy_lowers_to_valid_wgsl() {
    let src = r#"
proc tinted {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    let h = hash1(seed + copy * 8191u);
    color   = vec4(h, h, h, 1.0);
  }
}
"#;
    let parsed = karakuri_ir::parse(src).expect("parses");
    let checked = karakuri_ir::check::check(&parsed).expect("checks");
    let amplified = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic { copy: true },
        &[],
    );
    let shader = karakuri_codegen::generate_l4(&checked, &amplified, &[]);
    validate(&shader.source);
    assert!(
        shader.source.contains("let copy = elements[elem].copy;"),
        "{}",
        shader.source
    );
    assert!(
        shader.source.contains("@interpolate(flat) copy: u32"),
        "{}",
        shader.source
    );
}

/// Tests that an L4 shader referencing copy compiles cleanly to 0u when geometry is unamplified.
#[test]
fn an_l4_reading_copy_over_unamplified_geometry_reads_zero() {
    let src = r#"
proc tinted {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    let h = hash1(copy);
    color   = vec4(h, h, h, 1.0);
  }
}
"#;
    let parsed = karakuri_ir::parse(src).expect("parses");
    let checked = karakuri_ir::check::check(&parsed).expect("checks");
    let plain = karakuri_ir::layout::generate_element_layout(
        &[Attr::Position],
        karakuri_ir::layout::Synthetic::NONE,
        &[],
    );
    let shader = karakuri_codegen::generate_l4(&checked, &plain, &[]);
    validate(&shader.source);
    assert!(
        shader.source.contains("let copy = 0u;"),
        "{}",
        shader.source
    );
    assert!(
        !shader.source.contains("elements[elem].copy"),
        "{}",
        shader.source
    );
}
