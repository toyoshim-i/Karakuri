#![allow(unused_imports, dead_code)]

use super::naga_common::*;

// ---------------------------------------------------------------------------
// L5 — the three shipped procedures, compiled from source
// ---------------------------------------------------------------------------
//
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
    // **Costed here too**, because a shipped part that lowers and is refused at
    // stage 4 is a part nobody can load.
    karakuri_ir::cost::estimate(&checked)
        .unwrap_or_else(|errs| panic!("{name} is over the ceiling: {errs:?}"));
    karakuri_codegen::generate_l5(&checked)
}

/// **All three, through a real WGSL front end** — the checklist item that is
/// not optional for this crate (`docs/contributing.md` §7).
#[test]
fn the_three_shipped_l5_procedures_compile_and_validate() {
    for name in ["feedback", "bloom", "rgb_shift"] {
        let shader = shipped_l5(name);
        validate(&shader.source);
    }
}

/// **The bind group is `master.wgsl`'s, entry for entry**, so pass 2 binds a
/// written slot with the layout the engine already has.
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

    // **Binding 2 is left empty rather than renumbered** where nothing is
    // retained: the sampler's number must not depend on whether a pass reads
    // its own history.
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

/// **`feedback` is `master.wgsl`'s `fs_feedback` term for term.**
///
/// The hand-written body is `s.rgb + chain.feedback * h`, where `s` and `h` are
/// both `textureLoad`s at this fragment's own texel and the source's alpha goes
/// through unchanged. Both loads rather than samples: the retained frame is the
/// same size as this one and lines up texel for texel, so there is nothing to
/// interpolate and a filtered read would only cost precision.
///
/// **What is deliberately absent is `keepable`.** The retained frame is
/// sanitised where it is *written* — at the copy, one step earlier than
/// `master.wgsl` does it — so a `.kir` neither needs the guard nor can leave it
/// out. A `keepable` here would be the language carrying an engine invariant.
///
/// **What this holds and what it does not.** It is an assertion about the
/// expression tree, not about pixels: the same operands, the same operator, the
/// same alpha. Two shaders computing the same expression on the same inputs
/// produce the same texels, but *that* is a GPU test and it belongs with the
/// chain that runs both — pass 2, where there is a frame to compare.
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

/// **`rgb_shift` is `master.wgsl`'s `fs_rgb_shift` term for term.**
///
/// Red and blue sampled apart along x by `SHIFT_MAX * amount` — 2% of the
/// frame's height at full — and green where it was. The centre tap is a load
/// for the reason `texel` takes no coordinate: at an amount just above zero the
/// two outer taps land back on it, and the pass should differ from one that did
/// not run by what the shift is rather than by what a filter did.
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

/// **`frame_step` performs the conversion without handing the number over.**
///
/// It is `master.wgsl`'s `step_uv` with the same arithmetic: `.y` is the
/// fraction of the frame's height itself and `.x` is that fraction scaled by
/// the aspect ratio, so the displacement is isotropic *in texels*. The size
/// comes from the uniform's `viewport` and is reachable no other way — no
/// ambient carries the render size, so a `.kir` has no way to write a radius in
/// texels.
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

    // **Demand-driven, like every other helper.** `feedback` never displaces
    // anything, so it must not carry the conversion — and the `viewport` field
    // stays in the uniform either way, because the layout is the engine's
    // contract rather than a function of what a body happened to call.
    let feedback = shipped_l5("feedback").source;
    assert!(
        !feedback.contains("fn frame_step("),
        "`feedback` calls no `frame_step`:\n{feedback}"
    );
}

/// **The 9x9 kernel is one pass and 81 taps**, which is the whole of what
/// `bloom` gives up by being a chain slot rather than a pair of them: the
/// second half of the separable blur needs both the bright buffer *and* the
/// frame it was taken from, and a slot's output replaces the frame.
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

/// **A nested merge's fan-in is `uses` plus `edge` and no new mechanism**, and
/// the bindings are numbered by what the file declared rather than by what an
/// edge bound first.
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
