#![allow(unused_imports, dead_code)]

use super::naga_common::*;

#[test]
fn drift_shell_l1_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l1(&drift_shell(), &[], &[]);
    validate(&shader.source);
}

#[test]
fn soft_points_l4_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l4(&soft_points(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// `param array : float ...` parses, checks, and costs — WGSL reserves
/// `array` but `.kir` does not — and before the fix this generated a struct
/// field literally named `array`, which naga rejects as a reserved keyword.
/// Every param name in this fixture is a real WGSL reserved word.
#[test]
fn params_named_after_wgsl_reserved_words_compile_and_validate() {
    let shader = karakuri_codegen::generate_l1(&reserved_word_params_l1(), &[], &[]);
    validate(&shader.source);
}

/// Tests that UniformLayout field names and WGSL uniform struct member names match.
#[test]
fn uniform_layout_and_emitted_struct_text_agree() {
    let l1 = karakuri_codegen::generate_l1(&drift_shell(), &[], &[]);
    assert_layout_matches_text(&l1.source, &l1.uniform_layout);

    let l4 = karakuri_codegen::generate_l4(&soft_points(), &layout_for(&drift_shell()), &[]);
    assert_layout_matches_text(&l4.source, &l4.uniform_layout);

    let reserved = karakuri_codegen::generate_l1(&reserved_word_params_l1(), &[], &[]);
    assert_layout_matches_text(&reserved.source, &reserved.uniform_layout);

    // And specifically: the layout's semantic `name` must stay the
    // undecorated param name (what a Set record and the engine's packer
    // both address it by), even though the WGSL text spells it mangled.
    let array_field = reserved
        .uniform_layout
        .fields
        .iter()
        .find(|f| f.name == "array")
        .expect("the `array` param must be in the layout under its declared name");
    assert_eq!(array_field.wgsl_name, "param_array");
}

/// Tests that user-declared locals cannot shadow internal generator identifiers in L1.
#[test]
fn l1_locals_cannot_shadow_generated_identifiers() {
    let shader = karakuri_codegen::generate_l1(&shadowing_locals_l1(), &[], &[]);
    validate(&shader.source);
}

#[test]
fn l4_locals_cannot_shadow_generated_identifiers() {
    let shader = karakuri_codegen::generate_l4(
        &shadowing_locals_l4(),
        &layout_for(&shadowing_locals_l1()),
        &[],
    );
    validate(&shader.source);
}

/// Returns an L4 fixture that consumes a reordered subset of attributes.
fn reordered_subset_l4() -> Checked {
    let clip = bin(
        BinOp::Mul,
        ambient(Ambient::Camera, Ty::Mat4),
        construct(Ty::Vec4, vec![attr(Attr::Position), lit_f(1.0)]),
        Ty::Vec4,
    );
    let vertex = TBlock {
        kind: BlockKind::Vertex,
        span: span(),
        stmts: vec![
            assign_output(Output::Clip, clip),
            assign_output(
                Output::PointRate,
                bin(BinOp::Add, attr(Attr::Age), lit_f(1.0), Ty::Float),
            ),
        ],
    };
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![assign_output(
            Output::Color,
            construct(Ty::Vec4, vec![lit_f(1.0)]),
        )],
    };

    Checked {
        name: "reordered_subset_l4".to_string(),
        kind: Kind::L4,
        // Not `None`: an L4's topology is inferred by the check pass, and
        // `generate_l4` reads it to choose the quad expansion. These are
        // hand-built stand-ins for checked trees, so they carry what `check`
        // would have put here.
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        retains: false,
        blend: Some(Blend::Additive),
        params: vec![],
        emit: vec![],
        consumes: vec![Attr::Age, Attr::Position],
        blocks: vec![vertex, fragment],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// Returns an L4 fixture that consumes zero attributes.
fn consumes_nothing_l4() -> Checked {
    let vertex = TBlock {
        kind: BlockKind::Vertex,
        span: span(),
        stmts: vec![
            assign_output(
                Output::Clip,
                construct(
                    Ty::Vec4,
                    vec![lit_f(0.0), lit_f(0.0), lit_f(0.0), lit_f(1.0)],
                ),
            ),
            assign_output(Output::PointRate, lit_f(4.0)),
        ],
    };
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![assign_output(
            Output::Color,
            construct(
                Ty::Vec4,
                vec![call(
                    Builtin::Hash1,
                    vec![ambient(Ambient::Seed, Ty::Uint)],
                    Ty::Float,
                )],
            ),
        )],
    };

    Checked {
        name: "consumes_nothing_l4".to_string(),
        kind: Kind::L4,
        // Not `None`: an L4's topology is inferred by the check pass, and
        // `generate_l4` reads it to choose the quad expansion. These are
        // hand-built stand-ins for checked trees, so they carry what `check`
        // would have put here.
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        retains: false,
        blend: Some(Blend::Additive),
        params: vec![],
        emit: vec![],
        consumes: vec![],
        blocks: vec![vertex, fragment],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// `consumes` names a strict subset of `emit`, out of declaration order.
/// This is the case the module doc on `generate_l4` calls out by name:
/// byte identity must hold regardless of what a paired L4 happens to read.
#[test]
fn l4_consuming_a_reordered_subset_compiles_and_validates() {
    let shader =
        karakuri_codegen::generate_l4(&reordered_subset_l4(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// `consumes` is empty. Legal per the ir-spec; the mirror image of
/// `soft_points`, which emits nothing.
#[test]
fn l4_consuming_nothing_compiles_and_validates() {
    let shader =
        karakuri_codegen::generate_l4(&consumes_nothing_l4(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// Returns an L1 fixture emitting all declarable attributes.
fn emits_every_attribute_l1() -> Checked {
    let element = TBlock {
        kind: BlockKind::Element,
        span: span(),
        stmts: karakuri_ir::Attr::ALL
            .into_iter()
            .map(|a| assign_attr(a, zero_expr(a.ty())))
            .collect(),
    };

    Checked {
        name: "emits_every_attribute".to_string(),
        kind: Kind::L1,
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        retains: false,
        blend: None,
        params: vec![],
        emit: karakuri_ir::Attr::ALL.to_vec(),
        consumes: vec![],
        blocks: vec![element],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    }
}

/// A zero value of `ty`, using the same broadcast-scalar vector constructor
/// the ir-spec documents (`vec3(0.0)`), since `Lit` itself only carries
/// scalars.
fn zero_expr(ty: Ty) -> TExpr {
    match ty {
        Ty::Float => lit_f(0.0),
        Ty::Vec2 | Ty::Vec3 => construct(ty, vec![lit_f(0.0)]),
        other => panic!("zero_expr does not cover {other:?}; no attribute needs it"),
    }
}

/// Tests that emitting all declarable attributes generates valid WGSL with correct slot counts.
#[test]
fn l1_emitting_every_attribute_compiles_and_validates() {
    let checked = emits_every_attribute_l1();
    let shader = karakuri_codegen::generate_l1(&checked, &[], &[]);
    assert_eq!(
        shader.element_layout.slots.len(),
        2 + karakuri_ir::Attr::ALL.len()
    );
    // Maximum attribute element stride matches WGSL struct layout alignment.
    assert_eq!(shader.element_layout.stride, 96);
    // And it is smaller than the layout it replaced, which is the whole point.
    assert!(shader.element_layout.stride < (2 + karakuri_ir::Attr::ALL.len() as u32) * 16);
    validate(&shader.source);
}

/// The L4 side of the same claim, paired against the all-attributes L1:
/// consuming every attribute still produces a byte-identical `Element`
/// struct and valid WGSL.
#[test]
fn l4_consuming_every_attribute_compiles_and_validates() {
    let l1 = emits_every_attribute_l1();
    let clip = construct(Ty::Vec4, vec![attr(Attr::Position), lit_f(1.0)]);
    let vertex = TBlock {
        kind: BlockKind::Vertex,
        span: span(),
        stmts: vec![
            assign_output(Output::Clip, clip),
            assign_output(Output::PointRate, lit_f(1.0)),
        ],
    };
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![assign_output(
            Output::Color,
            construct(Ty::Vec4, vec![lit_f(1.0)]),
        )],
    };
    let l4 = Checked {
        name: "consumes_every_attribute".to_string(),
        kind: Kind::L4,
        // Not `None`: an L4's topology is inferred by the check pass, and
        // `generate_l4` reads it to choose the quad expansion. These are
        // hand-built stand-ins for checked trees, so they carry what `check`
        // would have put here.
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        retains: false,
        blend: Some(Blend::Additive),
        params: vec![],
        emit: vec![],
        consumes: karakuri_ir::Attr::ALL.to_vec(),
        blocks: vec![vertex, fragment],
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: span(),
    };
    let shader = karakuri_codegen::generate_l4(&l4, &layout_for(&l1), &[]);
    validate(&shader.source);
}

/// Tests that L1 and paired L4 modules generate byte-identical Element struct definitions.
#[test]
fn l1_and_paired_l4_declare_byte_identical_element_structs() {
    fn element_struct_text(source: &str) -> &str {
        let start = source
            .find("struct Element {")
            .expect("no Element struct in source");
        let end = source[start..]
            .find("};")
            .expect("unterminated Element struct")
            + start
            + 2;
        &source[start..end]
    }

    let l1 = karakuri_codegen::generate_l1(&drift_shell(), &[], &[]);
    let l4 = karakuri_codegen::generate_l4(&soft_points(), &layout_for(&drift_shell()), &[]);
    assert_eq!(
        element_struct_text(&l1.source),
        element_struct_text(&l4.source),
        "L1:\n{}\n\nL4:\n{}",
        l1.source,
        l4.source
    );
}

/// Tests that Naga rejects deliberately invalid shader code.
#[test]
fn naga_rejects_a_deliberately_broken_shader() {
    let broken = "\
@fragment
fn fs() -> @location(0) f32 {
    return vec3<f32>(1.0, 2.0, 3.0);
}
";
    let module =
        naga::front::wgsl::parse_str(broken).expect("this fixture is syntactically valid WGSL");
    let result = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module);
    assert!(
        result.is_err(),
        "expected a return-type mismatch to fail validation"
    );
}

/// Returns an L4 line-segment fixture based on soft_points.
fn soft_streaks() -> Checked {
    let mut checked = soft_points();
    checked.name = "soft_streaks".to_string();
    checked.topology = Some(Topology::Lines);
    let tail = bin(
        BinOp::Mul,
        ambient(Ambient::Camera, Ty::Mat4),
        construct(
            Ty::Vec4,
            vec![
                bin(
                    BinOp::Sub,
                    attr(Attr::Position),
                    attr(Attr::Velocity),
                    Ty::Vec3,
                ),
                lit_f(1.0),
            ],
        ),
        Ty::Vec4,
    );
    let vertex = checked
        .blocks
        .iter_mut()
        .find(|b| b.kind == BlockKind::Vertex)
        .expect("the points fixture has a vertex block");
    vertex.stmts.push(assign_output(Output::ClipB, tail));
    checked
}

/// The segment expansion has to be WGSL a front end accepts, not merely text
/// that reads correctly. It uses `mix` on vectors, a swizzle-built
/// perpendicular, and a division by an interpolated `w` — three things that
/// are easy to write and easy to get past a reviewer while naga refuses them.
#[test]
fn a_lines_l4_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l4(&soft_streaks(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// Tests that Points and Lines topologies emit their respective quad expansions exclusively.
#[test]
fn each_topology_emits_its_own_placement_and_not_the_others() {
    let layout = layout_for(&drift_shell());
    let points = karakuri_codegen::generate_l4(&soft_points(), &layout, &[]).source;
    let lines = karakuri_codegen::generate_l4(&soft_streaks(), &layout, &[]).source;

    assert!(
        points.contains("ndc_offset"),
        "the points shader lost the sprite offset"
    );
    assert!(
        !points.contains("half_vp"),
        "the points shader is expanding a segment"
    );
    assert!(
        lines.contains("half_vp"),
        "the lines shader is not expanding a segment"
    );
    assert!(
        !lines.contains("ndc_offset"),
        "the lines shader is still offsetting around a point"
    );

    // The fragment stage is topology-blind: same entry point, same varyings.
    let fs = |s: &str| s[s.find("@fragment").expect("a fragment stage")..].to_string();
    assert_eq!(
        fs(&points),
        fs(&lines),
        "the topology reached the fragment stage"
    );
}

/// `soft_points` under the other blend mode. A *modification* of the points
/// fixture for the same reason `soft_streaks` is: what differs between the two
/// generated shaders is then the blend mode and nothing else.
fn soft_glass() -> Checked {
    let mut checked = soft_points();
    checked.name = "soft_glass".to_string();
    checked.blend = Some(Blend::Weighted);
    checked
}

/// The weighted fragment stage is two targets, a clamp, a `pow` and a divide
/// that has not happened yet — none of which a reviewer can confirm is WGSL a
/// front end accepts. A struct-returning `@fragment` with a scalar at
/// `@location(1)` is the part most likely to be almost right.
#[test]
fn a_weighted_l4_compiles_and_validates() {
    let shader = karakuri_codegen::generate_l4(&soft_glass(), &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// And the same for segments, because the depth a weighted stroke is weighted
/// by comes out of the segment expansion rather than straight off `clip`.
#[test]
fn a_weighted_lines_l4_compiles_and_validates() {
    let mut checked = soft_streaks();
    checked.blend = Some(Blend::Weighted);
    let shader = karakuri_codegen::generate_l4(&checked, &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
}

/// Tests that weighted blended fullscreen L4 shaders compile and validate cleanly.
#[test]
fn a_weighted_fullscreen_l4_compiles_and_validates() {
    let fragment = TBlock {
        kind: BlockKind::Fragment,
        span: span(),
        stmts: vec![assign_output(
            Output::Color,
            construct(Ty::Vec4, vec![ambient(Ambient::Ray, Ty::Vec3), lit_f(0.5)]),
        )],
    };
    let l4 = Checked {
        name: "weighted_march".to_string(),
        kind: Kind::L4,
        topology: Some(Topology::Fullscreen),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        retains: false,
        blend: Some(Blend::Weighted),
        params: vec![],
        emit: vec![],
        consumes: vec![],
        blocks: vec![fragment],
        cost: None,
        closed_form: false,
        reads_beats: false,
        span: span(),
    };
    let shader = karakuri_codegen::generate_l4(&l4, &layout_for(&drift_shell()), &[]);
    validate(&shader.source);
    assert!(
        !shader
            .uniform_layout
            .fields
            .iter()
            .any(|f| f.name == "depth_range"),
        "a frame is not at a depth, so there is nothing for a range to normalise it against"
    );
}

/// Tests that Additive and Weighted blend modes emit their respective fragment epilogues exclusively.
#[test]
fn each_blend_mode_emits_its_own_fragment_epilogue_and_not_the_others() {
    let layout = layout_for(&drift_shell());
    let additive = karakuri_codegen::generate_l4(&soft_points(), &layout, &[]);
    let weighted = karakuri_codegen::generate_l4(&soft_glass(), &layout, &[]);

    assert!(
        weighted.source.contains("@location(1) reveal"),
        "no revealage target"
    );
    assert!(
        weighted.source.contains("view_depth"),
        "no depth to weigh by"
    );
    assert!(
        weighted.source.contains("cam.depth_range"),
        "no camera planes to measure the depth against"
    );
    assert!(
        weighted.camera_group.is_some(),
        "nothing to read those planes from"
    );

    // Not `@location(1)`: an additive shader has one of those already, for its
    // first varying. What it must not have is a fragment output struct.
    assert!(
        !additive.source.contains("FsOut"),
        "the additive shader grew a second target"
    );
    assert!(
        !additive.source.contains("reveal"),
        "the additive shader accumulates revealage"
    );
    assert!(
        !additive.source.contains("view_depth"),
        "the additive shader carries a depth nothing reads"
    );
    assert!(
        !additive.source.contains("cam.depth_range"),
        "the additive shader weighs a fragment by its depth"
    );

    // The vertex stage's *placement* is blend-blind: the same six corners land
    // in the same six places whichever way the fragments are combined.
    let placement = |s: &str| {
        let vs = &s[s.find("@vertex").expect("a vertex stage")..];
        vs[..vs.find("@fragment").expect("a fragment stage")]
            .lines()
            .filter(|l| !l.contains("view_depth"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        placement(&additive.source),
        placement(&weighted.source),
        "the blend mode moved the geometry"
    );
}
