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

/// The layout this crate publishes and the WGSL text it emits must never
/// disagree about a field's name — `karakuri-engine`'s uniform packer
/// writes bytes at the offset `UniformLayout` gives it, on the assumption
/// that the struct in the WGSL text has a field at that same offset under
/// the name it looked up. A mismatch here would not fail loudly in this
/// crate; it would fail as a panic in `UniformPacker`, or worse, silently
/// pack a value into the wrong param's bytes.
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

/// The regression this whole file exists for: a local named `u` (or `seed`,
/// or `hash1`, or any of this crate's other fixed identifiers) must not be
/// capturable. Before the fix, this failed exactly the way the bug report
/// describes — naga rejecting a `u.<param>` access because a same-named
/// local shadowed the uniform binding.
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

/// An L4 that consumes a strict subset of `drift_shell`'s `emit` and names
/// them in a different order than `drift_shell` declares (`emit position,
/// velocity, age`; this consumes `age` then `position`, skipping
/// `velocity`). Attribute reads are resolved by field name
/// (`elements[elem].<name>`), never by struct position, so `generate_l4`
/// must not care about `consumes`' order or completeness relative to
/// `emit` — only about which names it happens to read.
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

/// An L4 that consumes nothing at all — legal per the ir-spec ("the
/// `soft_points` example ... consumes three attributes and emits nothing,
/// which is not an error but the normal shape of an L4 file"; the reverse,
/// consuming nothing, is equally legal and untested elsewhere in this
/// file). It still declares the full `Element` struct L1 wrote and still
/// reads `seed`, just no named attribute.
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

/// An L1 procedure that emits every declarable attribute (all seven of
/// `Attr::ALL`), the shape this whole change exists for: before it, this
/// would have bound 2 synthetic + 7 declared = 9 attributes, times prev and
/// next, an 18-buffer compute stage. The `Element` struct is 9 `vec4`
/// slots (`seed`, `birth_frac`, plus the 7 declared) regardless, and the
/// bind group stays 2 storage buffers per direction.
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

/// The buffer-count claim this whole change exists for: the `Element`
/// struct's field count tracks `emit.len()` exactly (`seed`, `birth_frac`,
/// and every declared attribute), and it still validates as real WGSL at
/// the largest shape the language allows: all seven declarable attributes,
/// not just the two or three every other fixture in this file emits.
#[test]
fn l1_emitting_every_attribute_compiles_and_validates() {
    let checked = emits_every_attribute_l1();
    let shader = karakuri_codegen::generate_l1(&checked, &[], &[]);
    assert_eq!(
        shader.element_layout.slots.len(),
        2 + karakuri_ir::Attr::ALL.len()
    );
    // **Every attribute at once, which is the widest element the language can
    // ask for**, and the number is asserted rather than derived so that a
    // layout change has to come here and say what it did. The padded layout
    // made this `(2 + ALL) * 16`; WGSL's own placement makes it this.
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

/// The core claim the L4-takes-a-layout signature exists for: L1 and its
/// paired L4 must declare byte-identical `Element` structs, since L4 reads
/// the same physical buffer L1 wrote. Extracts the `struct Element { ... };`
/// block from each generated source and compares the text directly, rather
/// than trusting that "compiles" implies "same layout" — two structs with a
/// different field order would each compile fine on their own.
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

/// The negative half of "a generator whose output is never fed to a
/// compiler emits plausible nonsense": confirm this test harness actually
/// catches a broken shader, not just that well-formed ones pass. The type
/// mismatch below is caught by validation, not by parsing — WGSL's grammar
/// alone does not know a function's declared return type disagrees with
/// what it returns, so this has to go through both stages, exactly like the
/// two real fixtures above do.
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

/// `soft_points` with a second endpoint: the same procedure, made a line
/// renderer by the one assignment that decides it.
///
/// Deliberately a *modification* of the points fixture rather than a fresh
/// tree, so the difference between the two generated shaders is the
/// difference between the two topologies and nothing else.
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

/// Each topology emits **its own** placement, and neither emits the other's.
///
/// The first version of this asserted only that the two sources differed,
/// which they do for a reason that has nothing to do with the expansion: a
/// lines procedure declares `_clip_b` and assigns it in the body whatever the
/// generator then does with it. A generator that ignored the topology and ran
/// the point path twice passed. So the assertion names the two placements
/// instead — `ndc_offset`, the offset around a point, and `half_vp`, the pixel
/// conversion only the segment expansion needs — and requires each shader to
/// have exactly one of them.
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

/// A weighted L4 with no `vertex` block. **`Set::build` refuses this pairing**
/// — one layer per texel makes the resolve the identity — so nothing in the
/// engine's tests can reach the code that lowers it, and without this the
/// generator's fullscreen-plus-weighted arm would be the one path in the crate
/// no front end had ever read.
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

/// **Each blend mode emits its own fragment epilogue, and neither emits the
/// other's** — the same shape of assertion as
/// `each_topology_emits_its_own_placement_and_not_the_others`, and written after
/// that one caught a generator ignoring the topology it was handed.
///
/// The three things named are the three that have to move together: the second
/// target, the varying the weight is computed from, and the camera planes that
/// varying is measured against. A shader with any one of them missing would
/// still compile.
///
/// **The planes are a read and not a declaration.** They live in the camera's
/// bind group, which an additive shader binds too — it projects with the same
/// camera — so what separates the two modes is `cam.depth_range` appearing in
/// the body, not `depth_range` appearing anywhere. Asserting on the name alone
/// would pass for both, since both emit the struct that declares it.
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
