use super::common::*;

// ---------------------------------------------------------------------------
// kind L5 — the frame effect
// ---------------------------------------------------------------------------

/// Baseline legal L5 procedure fixture.
const LEGAL_FRAME: &str = r#"
proc dim {
  kind L5

  param amount : float [0.0, 1.0] = 0.5

  frame {
    let s = texel(src);
    color = vec4(s.xyz * amount, s.w);
  }
}
"#;

#[test]
fn a_legal_frame_block_is_accepted() {
    let checked = check_ok(LEGAL_FRAME);
    assert_eq!(checked.kind, Kind::L5);
    assert!(checked.block(BlockKind::Frame).is_some());
    assert!(!checked.retains, "no `retains` was declared");
    // L5 post-processing procedures declare no topology.
    assert_eq!(checked.topology, None);
}

/// Fullscreen passes have no element seed ambient.
#[test]
fn an_l5_reading_seed_is_refused_with_the_fix_named() {
    let src = r#"
proc speckle {
  kind L5

  frame {
    let s = texel(src);
    color = vec4(s.xyz * hash1(seed), s.w);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`seed` is not available to an L5")),
        "the refusal has to name the value and the kind: {errs:?}"
    );
    let hint = errs
        .iter()
        .find_map(|e| e.hint.clone())
        .expect("the refusal carries a hint");
    assert!(
        hint.contains("a fullscreen pass has no element"),
        "the hint has to say why: {hint}"
    );
    assert!(
        hint.contains("point_coord") && hint.contains("param"),
        "the hint has to name what to reach for instead: {hint}"
    );

    // The negative control: the same shape without `seed` is accepted, so this
    // test cannot pass by the kind being refused outright.
    check_ok(LEGAL_FRAME);
}

/// Verifies that all ambients disallowed in L5 return specific, explanatory hints.
#[test]
fn every_ambient_an_l5_refuses_gets_its_own_sentence() {
    for (name, expr, because) in [
        ("copy", "float(copy)", "has no element"),
        ("source", "float(source)", "several decks"),
        ("point", "point.x", "field"),
        ("capacity", "float(capacity)", "how much material"),
        (
            "camera",
            "(camera * vec4(0.0, 0.0, 0.0, 1.0)).x",
            "several cameras",
        ),
        ("eye", "eye.x", "marcher"),
        ("ray", "ray.x", "marcher"),
    ] {
        let src = format!(
            r#"
proc probe {{
  kind L5

  frame {{
    let s = texel(src);
    color = vec4(s.xyz * {expr}, s.w);
  }}
}}
"#
        );
        let errs = check_err(&src);
        assert!(
            errs.iter()
                .any(|e| e.message == format!("`{name}` is not available to an L5")),
            "`{name}` must be refused with a sentence about the kind, got: {errs:?}"
        );
        let hint = errs
            .iter()
            .find(|e| e.message == format!("`{name}` is not available to an L5"))
            .and_then(|e| e.hint.clone())
            .unwrap_or_default();
        assert!(
            hint.contains(because),
            "`{name}`'s hint has to say why: {hint}"
        );
    }
}

/// Verifies that reading `held` requires a `retains` header declaration.
#[test]
fn held_outside_retains_is_refused_and_retains_makes_it_readable() {
    let without = r#"
proc echo {
  kind L5

  param amount : float [0.0, 1.0] = 0.5

  frame {
    let s = texel(src);
    let h = texel(held);
    color = vec4(s.xyz + amount * h.xyz, s.w);
  }
}
"#;
    let errs = check_err(without);
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`held` is available only under `retains`")),
        "the refusal has to name the declaration: {errs:?}"
    );
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("add a bare `retains`"),
        "the hint has to say what to add: {hint}"
    );
    // Tone map cuts are specified by the set rather than procedure files.
    assert!(
        hint.contains("mix") && hint.contains("exit"),
        "the hint has to say the cut is answered elsewhere: {hint}"
    );

    let with = without.replace("kind L5", "kind L5\n  retains");
    let checked = check_ok(&with);
    assert!(checked.retains, "a bare `retains` is carried through");
    // Retaining previous frame marks procedure as non-closed-form accumulation.
    assert!(
        !checked.closed_form,
        "a procedure that reads a retained frame cannot be evaluated at any `t`"
    );
}

/// Retains header declaration is refused on all non-L5 procedure kinds.
#[test]
fn retains_is_refused_off_an_l5() {
    let src = r#"
proc drift {
  kind L4
  retains
  blend additive

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`retains` is L5 only")),
        "{errs:?}"
    );
}

/// Verifies that Field slots are disallowed on L5 procedures while Texture slots are allowed.
#[test]
fn a_field_slot_on_an_l5_is_refused_and_a_texture_slot_is_not() {
    let field = r#"
proc carve {
  kind L5

  uses shape : Field

  frame {
    let s = texel(src);
    color = vec4(s.xyz * shape(vec3(0.0, 0.0, 0.0)), s.w);
  }
}
"#;
    let errs = check_err(field);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`uses … : Field` is not an L5")),
        "{errs:?}"
    );
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("frame coordinate"),
        "the hint has to say what an L5 has instead of a point: {hint}"
    );

    // The negative control, and the one slot type an L5 *may* declare: a
    // nested merge's fan-in, `uses` plus `edge` and no new mechanism.
    let texture = r#"
proc over {
  kind L5

  param mix_amount : float [0.0, 1.0] = 0.5

  uses under : Texture

  frame {
    let a = texel(src);
    let b = tap(under, point_coord);
    color = vec4(a.xyz + (b.xyz - a.xyz) * mix_amount, a.w);
  }
}
"#;
    let checked = check_ok(texture);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "under".into(),
            ty: SlotTy::Texture,
        }]
    );
    assert_eq!(checked.texture_slots(), vec!["under"]);
}

/// Texture slots are exclusive to L5 procedures.
#[test]
fn a_texture_slot_is_refused_on_every_other_kind() {
    for (kind, body) in [
        (
            "L1",
            "topology points\n  capacity [1, 1024] = 512\n\n  emit position\n\n  element {\n    position = vec3(0.0, 0.0, 0.0);\n  }",
        ),
        ("L2", "deform {\n    let a = 1.0;\n  }"),
        ("L3", "camera {\n    eye = vec3(0.0, 0.0, 6.0);\n    target = vec3(0.0, 0.0, 0.0);\n  }"),
        (
            "L4",
            "blend additive\n\n  fragment {\n    color = vec4(1.0, 1.0, 1.0, 1.0);\n  }",
        ),
        ("Field", "field {\n    distance = sd_sphere(point, 1.0);\n  }"),
    ] {
        let src = format!(
            r#"
proc probe {{
  kind {kind}

  uses back : Texture

  {body}
}}
"#
        );
        let errs = refusals(&src);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("`uses … : Texture` is L5 only")),
            "a {kind} must refuse a Texture slot: {errs:?}"
        );
    }
}

/// Textures cannot be used as bare values without sampling builtins.
#[test]
fn reading_a_texture_as_a_value_is_refused_with_the_two_builtins_named() {
    let src = r#"
proc grab {
  kind L5

  frame {
    let s = src;
    color = vec4(s, s, s, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`src` is a texture, not a value")),
        "{errs:?}"
    );
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("texel(src)") && hint.contains("tap(src, uv)"),
        "the hint has to name both fetches: {hint}"
    );
}

/// Frame builtins (`texel`, `tap`, `frame_step`) are exclusive to L5 procedures.
#[test]
fn the_frame_builtins_are_refused_outside_an_l5() {
    let src = r#"
proc smear {
  kind L4
  blend additive

  fragment {
    let d = frame_step(0.01);
    color = vec4(d.x, d.y, 0.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`frame_step` is an L5 builtin")),
        "{errs:?}"
    );

    // The negative control: the same call in a `frame` block is accepted.
    check_ok(
        r#"
proc smear {
  kind L5

  frame {
    let d = frame_step(0.01);
    color = tap(src, point_coord + d);
  }
}
"#,
    );
}

/// L5 procedures reject geometry declarations at header check.
#[test]
fn an_l5_declaring_geometry_is_refused() {
    let src = r#"
proc wrong {
  kind L5
  topology points
  capacity [1, 16] = 8
  blend additive
  amplify 2

  emit position
  consumes position

  frame {
    color = texel(src);
  }
}
"#;
    let errs = check_err(src);
    for what in [
        "`topology` is not an L5",
        "`capacity` is not an L5",
        "`blend` is not an L5",
        "`amplify` is not an L5",
    ] {
        assert!(
            errs.iter().any(|e| e.message.contains(what)),
            "expected {what}: {errs:?}"
        );
    }
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`emit` and `consumes` are about elements, and an L5 has none")),
        "{errs:?}"
    );
}

/// Vertex blocks are refused in L5 procedures.
#[test]
fn a_vertex_block_in_an_l5_is_refused_as_l4s() {
    let src = r#"
proc wrong {
  kind L5

  vertex {
    clip       = vec4(0.0, 0.0, 0.0, 1.0);
    point_rate = 0.01;
  }

  frame {
    color = texel(src);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("a `vertex` block is not valid in a `L5` procedure")),
        "{errs:?}"
    );
}

/// L5 procedures require exactly one frame block.
#[test]
fn an_l5_without_a_frame_block_is_refused() {
    let errs = check_err("proc empty {\n  kind L5\n}\n");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("L5 procedures require a `frame` block")),
        "{errs:?}"
    );
}

/// Verifies that output name `color` is reserved in L5 frame blocks against param declarations.
#[test]
fn color_is_reserved_in_an_l5() {
    let src = r#"
proc wrong {
  kind L5

  param color : float [0.0, 1.0] = 0.5

  frame {
    color = texel(src);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`color` shadows a stage output name")),
        "{errs:?}"
    );
}

/// Reserved frame block inputs (`src`, `held`) cannot name slots.
#[test]
fn a_texture_slot_may_not_be_called_src_or_held() {
    for name in ["src", "held"] {
        let src = format!(
            r#"
proc collide {{
  kind L5
  retains

  uses {name} : Texture

  frame {{
    color = texel(src);
  }}
}}
"#
        );
        let errs = check_err(&src);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("is a texture an L5 is already handed")),
            "`{name}` must be refused as a slot name: {errs:?}"
        );
    }
}

/// `texel` builtin fetches current coordinate without arguments.
#[test]
fn texel_with_a_coordinate_is_refused_with_the_reason() {
    let src = r#"
proc resample {
  kind L5

  frame {
    color = texel(src, point_coord);
  }
}
"#;
    let errs = check_err(src);
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("invitation to resample"),
        "the hint has to give the reason: {hint}"
    );
    assert!(
        hint.contains("tap(<texture>, uv)"),
        "the hint has to name the filtered read: {hint}"
    );
}
