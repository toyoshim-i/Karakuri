#![allow(unused_imports, dead_code)]

use super::naga_common::*;

// End-to-end compilation tests for shipped L5 procedures parsed directly from `.kir` files.

/// Parse, check, cost and lower one of the shipped L5 procedures.
fn shipped_l5(name: &str) -> karakuri_codegen::L5Shader {
    let path = format!("../../examples/{name}.kir");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let proc = karakuri_ir::parse::parse(&src).unwrap_or_else(|errs| {
        panic!(
            "{name} failed to parse:\n{}",
            errs.iter()
                .map(|e| e.render(&src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|errs| {
        panic!(
            "{name} failed to check:\n{}",
            errs.iter()
                .map(|e| e.render(&src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    // Cost estimation validation ensures shipped procedures remain within budget.
    karakuri_ir::cost::estimate(&checked)
        .unwrap_or_else(|errs| panic!("{name} is over the ceiling: {errs:?}"));
    karakuri_codegen::generate_l5(&checked)
}

/// Validates every `kind L5` in `examples/` against Naga WGSL frontend.
#[test]
fn every_shipped_l5_procedure_compiles_and_validates() {
    let mut names: Vec<String> = std::fs::read_dir("../../examples")
        .expect("read examples/")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "kir"))
        .filter(|p| {
            let src = std::fs::read_to_string(p).expect("read an example");
            karakuri_ir::parse::parse(&src).is_ok_and(|proc| proc.kind == Kind::L5)
        })
        .map(|p| {
            p.file_stem()
                .expect("a stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    for name in ["feedback", "bloom", "rgb_shift"] {
        assert!(
            names.iter().any(|n| n == name),
            "{name}.kir is not found as a `kind L5` in examples/"
        );
    }
    for name in &names {
        let shader = shipped_l5(name);
        validate(&shader.source);
    }
}

/// Bind group layout matches master post-processing pipeline layout.
#[test]
fn an_l5_lays_its_bind_group_out_the_way_the_chain_does() {
    let feedback = shipped_l5("feedback");
    for want in [
        "@group(0) @binding(0) var<uniform> u: Uniforms;",
        "@group(0) @binding(1) var src: texture_2d<f32>;",
        "@group(0) @binding(2) var held: texture_2d<f32>;",
        "@group(0) @binding(3) var samp: sampler;",
    ] {
        assert!(
            feedback.source.contains(want),
            "expected `{want}`:\n{}",
            feedback.source
        );
    }
    assert!(feedback.retains, "`feedback` declares `retains`");

    // Binding index 2 remains unallocated when history texture is not retained.
    let shift = shipped_l5("rgb_shift");
    assert!(!shift.retains);
    assert!(
        !shift.source.contains("@binding(2)"),
        "nothing is bound at 2 without `retains`:\n{}",
        shift.source
    );
    assert!(
        shift
            .source
            .contains("@group(0) @binding(3) var samp: sampler;"),
        "and the sampler keeps its number:\n{}",
        shift.source
    );
}

/// Tests that feedback.kir lowers to exact textureLoad calls and alpha pass-through.
#[test]
fn feedbacks_body_is_the_hand_written_pass_term_for_term() {
    let src = shipped_l5("feedback").source;
    let body = fragment_body(&src);

    assert!(
        body.contains("let usr_s = textureLoad(src, _at, 0);"),
        "the frame is loaded, not sampled:\n{body}"
    );
    assert!(
        body.contains("let usr_h = textureLoad(held, _at, 0);"),
        "and so is the retained cut:\n{body}"
    );
    assert!(
        body.contains("_color = vec4<f32>((usr_s.xyz + (u.param_amount * usr_h.xyz)), usr_s.w);"),
        "`s.rgb + amount * h`, with the source's alpha through unchanged:\n{body}"
    );
    assert!(
        !body.contains("textureSampleLevel"),
        "nothing in `feedback` is filtered:\n{body}"
    );
    assert!(
        !src.contains("keepable") && !src.contains("select("),
        "the retained frame is sanitised at the copy, not at the read:\n{src}"
    );
}

/// Tests that rgb_shift.kir lowers to appropriate texture sample offsets and center tap.
#[test]
fn rgb_shifts_body_is_the_hand_written_pass_term_for_term() {
    let src = shipped_l5("rgb_shift").source;
    let body = fragment_body(&src);

    // `SHIFT_MAX` is 0.02, and `frame_step` is `step_uv` with one fewer thing
    // exposed.
    assert!(
        body.contains("let usr_d = frame_step((0.02 * u.param_amount));"),
        "the displacement is 2% of the frame's height at full amount:\n{body}"
    );
    assert!(
        body.contains("let usr_centre = textureLoad(src, _at, 0);"),
        "the centre tap is unfiltered:\n{body}"
    );
    assert!(
        body.contains(
            "let usr_r = textureSampleLevel(src, samp, (point_coord + vec2<f32>(usr_d.x, 0.0)), 0.0);"
        ),
        "red is sampled forward along x:\n{body}"
    );
    assert!(
        body.contains(
            "let usr_b = textureSampleLevel(src, samp, (point_coord - vec2<f32>(usr_d.x, 0.0)), 0.0);"
        ),
        "blue is sampled back along x:\n{body}"
    );
    assert!(
        body.contains("_color = vec4<f32>(usr_r.x, usr_centre.y, usr_b.z, usr_centre.w);"),
        "green stays where it was, and so does alpha:\n{body}"
    );
}

/// Tests that frame_step emits viewport-based aspect-correct displacement helpers when called.
#[test]
fn frame_step_lowers_to_the_viewport_conversion_and_only_when_it_is_called() {
    let shift = shipped_l5("rgb_shift").source;
    assert!(
        shift.contains("fn frame_step(r: f32) -> vec2<f32> {"),
        "the helper is emitted:\n{shift}"
    );
    assert!(
        shift.contains("return vec2<f32>(r * u.viewport.y / u.viewport.x, r);"),
        "`step_uv`'s arithmetic, term for term:\n{shift}"
    );
    assert!(
        shift.contains("viewport: vec2<f32>,"),
        "and the uniform carries the size it reads:\n{shift}"
    );

    // Helper emission is demand-driven based on shader body usage.
    let feedback = shipped_l5("feedback").source;
    assert!(
        !feedback.contains("fn frame_step("),
        "`feedback` calls no `frame_step`:\n{feedback}"
    );
}

/// Single-pass 9x9 kernel (81 taps) for post-processing bloom shader.
#[test]
fn blooms_lowering_is_one_pass_of_eighty_one_taps() {
    let src = shipped_l5("bloom").source;
    let body = fragment_body(&src);
    assert_eq!(
        body.matches("for (var usr_").count(),
        2,
        "two literal-bounded loops, so the estimate multiplies them out:\n{body}"
    );
    assert!(
        body.contains("usr_i: i32 = -4; usr_i < 5") && body.contains("usr_j: i32 = -4; usr_j < 5"),
        "nine by nine:\n{body}"
    );
    assert_eq!(
        body.matches("textureSampleLevel").count(),
        1,
        "one tap in the body, replayed 81 times by the loops:\n{body}"
    );
    assert!(
        body.contains("textureLoad(src, _at, 0)"),
        "and the frame it is added back to is loaded, not sampled:\n{body}"
    );
}

/// Nested merge fan-in binds declared slots in declaration order.
#[test]
fn texture_slots_are_bound_after_the_sampler_in_header_order() {
    let src = r#"
proc fold {
  kind L5

  param amount : float [0.0, 1.0] = 0.5

  uses under : Texture
  uses over  : Texture

  frame {
    let a = tap(under, point_coord);
    let b = tap(over, point_coord);
    let c = texel(src);
    color = vec4(c.xyz + a.xyz * amount + b.xyz, c.w);
  }
}
"#;
    let proc = karakuri_ir::parse::parse(src).expect("parses");
    let checked = karakuri_ir::check::check(&proc)
        .unwrap_or_else(|errs| panic!("expected this to check clean: {errs:?}"));
    let shader = karakuri_codegen::generate_l5(&checked);
    assert_eq!(shader.texture_slots, vec!["under", "over"]);
    assert_eq!(shader.slot_binding_number("under"), Some(4));
    assert_eq!(shader.slot_binding_number("over"), Some(5));
    assert!(
        shader
            .source
            .contains("@group(0) @binding(4) var tex_under: texture_2d<f32>;"),
        "{}",
        shader.source
    );
    assert!(
        shader
            .source
            .contains("@group(0) @binding(5) var tex_over: texture_2d<f32>;"),
        "{}",
        shader.source
    );
    validate(&shader.source);
}

/// The `fs` entry point's body, so a term-for-term assertion is not satisfied
/// by a matching line in the prelude or the vertex stage.
fn fragment_body(source: &str) -> &str {
    let at = source
        .find("fn fs(in: VsOut)")
        .unwrap_or_else(|| panic!("no fragment entry point:\n{source}"));
    &source[at..]
}
