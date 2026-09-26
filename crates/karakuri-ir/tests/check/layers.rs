use super::common::*;

// ---------------------------------------------------------------------------
// `amplify`: the one declaration that changes an element count.
// ---------------------------------------------------------------------------

/// The declaration lands on `Checked`, which is where the lowering and the
/// engine both read it from.
#[test]
fn an_amplifying_l2_checks_clean_and_carries_its_factor() {
    let checked = check_ok(
        r#"
proc mirror {
  kind    L2
  amplify 8

  consumes position

  deform {
    position = position * (1.0 + float(copy) * 0.1);
  }
}
"#,
    );
    assert_eq!(checked.amplify, Some(8));
    assert_eq!(checked.kind, Kind::L2);
}

/// An L2 with no declaration is the endomorphism it always was, and says so by
/// carrying `None` rather than `Some(1)`.
#[test]
fn an_l2_with_no_amplify_declaration_carries_none() {
    let checked = check_ok(
        r#"
proc plain {
  kind L2

  consumes position

  deform {
    position = position * 2.0;
  }
}
"#,
    );
    assert_eq!(checked.amplify, None);
}

/// `amplify` declaration is exclusive to L2 deformation stages.
#[test]
fn amplify_is_refused_outside_an_l2() {
    let l1 = r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 8] = 4
  amplify  8

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let l3 = r#"
proc cam {
  kind    L3
  amplify 8

  camera {
    eye    = vec3(0.0, 0.0, 5.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let l4 = r#"
proc dots {
  kind    L4
  blend   additive
  amplify 8

  consumes position

  vertex {
    clip       = vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    for src in [l1, l3, l4] {
        let errs = check_err(src);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("`amplify` is L2 only")),
            "expected `amplify` to be refused, got: {errs:?}"
        );
    }
}

/// Amplification factor must be at least two.
#[test]
fn an_amplify_factor_below_two_is_refused() {
    for (factor, expected) in [(0u32, "discards every element"), (1, "endomorphism")] {
        let src = format!(
            r#"
proc mirror {{
  kind    L2
  amplify {factor}

  consumes position

  deform {{
    position = position * 2.0;
  }}
}}
"#
        );
        let errs = check_err(&src);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("at least 2") && e.message.contains(expected)),
            "expected `amplify {factor}` to be refused as {expected}, got: {errs:?}"
        );
    }
}

/// A ceiling on the single factor, so that a chain's *product* cannot walk a
/// `u32` off its end before anything is allocated. The diagnostic for an
/// overflowed buffer size is a failed allocation, which says nothing about the
/// file that asked for it.
#[test]
fn an_amplify_factor_above_the_ceiling_is_refused() {
    let src = r#"
proc mirror {
  kind    L2
  amplify 4096

  consumes position

  deform {
    position = position * 2.0;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("above the ceiling")),
        "expected a ceiling refusal, got: {errs:?}"
    );
    // And the value just under it is accepted, which is what makes the line
    // above a ceiling rather than a refusal of large factors in general.
    check_ok(&src.replace("4096", "1024"));
}

/// `copy` is readable where an element has one and nowhere else. An L1 is
/// making the elements, so nothing has amplified above it; an L3 has no
/// element at all.
#[test]
fn copy_is_readable_in_an_l2_and_refused_in_an_l1() {
    check_ok(
        r#"
proc mirror {
  kind    L2
  amplify 4

  consumes position

  mask {
    strength = 1.0;
    if copy == 0u {
      strength = 0.0;
    }
  }

  deform {
    position = position + vec3(0.0, float(copy), 0.0);
  }
}
"#,
    );
    let errs = check_err(
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 8] = 4

  emit position

  element {
    position = vec3(float(copy), 0.0, 0.0);
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("copy")),
        "expected `copy` to be refused in an L1, got: {errs:?}"
    );
}

/// Verifies that element identity ambients (`seed`, `copy`) are disallowed in fullscreen L4 procedures.
#[test]
fn per_element_identity_is_refused_in_a_fullscreen_l4() {
    for name in ["seed", "copy"] {
        let src = format!(
            r#"
proc marcher {{
  kind  L4
  blend additive

  fragment {{
    let h = hash1({name});
    color   = vec4(h, h, h, 1.0);
  }}
}}
"#
        );
        let errs = check_err(&src);
        assert!(
            errs.iter().any(|e| e.message.contains(name)
                && e.message.contains("whole frame")
                && e.hint.as_deref().unwrap_or_default().contains("vertex")),
            "expected `{name}` to be refused as per-element, got: {errs:?}"
        );
    }
}

/// The same two values in a *per-element* L4 are exactly what they have always
/// been, and this is the half of the pair that keeps the refusal above from
/// being a refusal of `seed` outright.
#[test]
fn per_element_identity_is_readable_in_an_l4_with_a_vertex_block() {
    for name in ["seed", "copy"] {
        let src = format!(
            r#"
proc sprite {{
  kind  L4
  blend additive

  consumes position

  vertex {{
    clip       = vec4(position, 1.0);
    point_rate = 0.016;
  }}

  fragment {{
    let h = hash1({name});
    color   = vec4(h, h, h, 1.0);
  }}
}}
"#
        );
        check_ok(&src);
    }
}

/// `eye` and `ray` are fragment-only, and an L4 with a vertex block is
/// per-element, where a ray through a fragment is not a thing a vertex has.
#[test]
fn the_ray_ambients_are_refused_in_a_vertex_block() {
    let src = r#"
proc misplaced {
  kind  L4
  blend additive

  vertex {
    clip       = vec4(ray, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("ray")),
        "expected a diagnostic naming `ray`, got: {errs:?}"
    );
}

/// `fullscreen` is a renderer, not geometry. The value exists on the shared
/// field because an L4's inferred answer and an L1's declaration live there
/// together; an L1 declaring it is refused where it is written.
#[test]
fn an_l1_declaring_fullscreen_is_rejected() {
    let src = r#"
proc bad {
  kind     L1
  topology fullscreen
  capacity [1, 64] = 8

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("not geometry")
            && e.hint.as_deref().unwrap_or_default().contains("vertex")),
        "expected the diagnostic to say how an L4 does say it, got: {errs:?}"
    );
}

/// The control: an L4 *with* a vertex block is still per-element, so removing
/// the block is what changes the answer rather than the answer being fixed.
#[test]
fn an_l4_with_a_vertex_block_is_not_fullscreen() {
    let src = r#"
proc sprites {
  kind  L4
  blend additive

  vertex {
    clip       = vec4(0.0, 0.0, 0.0, 1.0);
    point_rate = 0.008;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    assert_eq!(check_ok(src).topology, Some(Topology::Points));
}

// ---------------------------------------------------------------------------
// L2 — geometry modulation
// ---------------------------------------------------------------------------

/// Modulator fixture: deforms `position` and widens element layout by emitting `color`.
const WOBBLE: &str = r#"
proc wobble {
  kind L2

  param amount : float [0.0, 4.0] = 0.6
  param rate   : float [0.1, 8.0] = 1.5

  consumes position
  emit tint

  deform {
    let phase = t * rate + hash1(seed) * 6.2831853;
    position = position + vec3(sin(phase), cos(phase), 0.0) * amount;
    tint     = vec3(hash1(seed + 3u), 0.4, 0.9);
  }
}
"#;

/// L2 procedures consist entirely of a deform stage without topology or blend state.
#[test]
fn an_l2_checks_clean_and_carries_neither_topology_nor_blend() {
    let checked = check_ok(WOBBLE);
    assert_eq!(checked.kind, Kind::L2);
    assert_eq!(checked.blocks.len(), 1);
    assert_eq!(checked.blocks[0].kind, BlockKind::Deform);
    // A deformation moves elements about and does not turn a cloud into
    // strands, so what the geometry reads as stays the L1's declaration.
    assert_eq!(checked.topology, None);
    assert_eq!(checked.blend, None);
    assert!(checked.capacity.is_none());
    assert_eq!(checked.emit, vec![Attr::Tint]);
    assert_eq!(checked.consumes, vec![Attr::Position]);
}

/// Verifies that L2 procedures are unconditionally classified as closed form.
#[test]
fn an_l2_is_closed_form_whatever_it_writes() {
    assert!(check_ok(WOBBLE).closed_form);
}

/// L2 deform stages may read attributes that they emit into the output buffer.
#[test]
fn a_deform_reads_both_what_it_consumes_and_what_it_emits() {
    let src = r#"
proc widen {
  kind L2
  consumes position
  emit tint
  deform {
    tint     = vec3(0.5, 0.5, 0.5);
    tint     = tint * 2.0;
    position = position * 1.5;
  }
}
"#;
    check_ok(src);
}

/// And nothing else: an attribute in neither list is not in scope, and the hint
/// names both lists because either could be the one that was meant.
#[test]
fn a_deform_cannot_read_an_attribute_it_neither_consumes_nor_emits() {
    let src = r#"
proc peek {
  kind L2
  consumes position
  deform {
    position = position + velocity * 0.1;
  }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("velocity"), "{rendered}");
    assert!(
        rendered.contains("consumes"),
        "the hint does not offer `consumes`: {rendered}"
    );
}

/// Verifies that `kill()` is disallowed in L2 deform blocks.
#[test]
fn a_deform_cannot_kill_and_is_told_why_in_its_own_terms() {
    let src = r#"
proc cull {
  kind L2
  consumes position, age
  deform {
    if age > 1.0 { kill(); }
    position = position;
  }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("kill()"), "{rendered}");
    assert!(
        rendered.contains("compaction") || rendered.contains("Fade it out"),
        "the hint sends an L2 author to the `element` block it does not have: {rendered}"
    );
}

/// The three header fields that belong to the other layers are each refused
/// with the reason they belong there, rather than as one "unexpected field".
#[test]
fn an_l2_refuses_capacity_topology_and_blend() {
    for (field, decl) in [
        ("capacity", "capacity [1, 8] = 4"),
        ("topology", "topology points"),
        ("blend", "blend additive"),
    ] {
        let src = format!(
            r#"
proc bad {{
  kind L2
  {decl}
  consumes position
  deform {{ position = position; }}
}}
"#
        );
        let errs = check_err(&src);
        let rendered = errs
            .iter()
            .map(|e| e.render(&src))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains(field),
            "`{field}` was not named: {rendered}"
        );
    }
}

/// An L2 with no `deform` has nothing to do, and the diagnostic says so at the
/// procedure rather than leaving an author to infer it from a later stage.
#[test]
fn an_l2_without_a_deform_block_is_refused() {
    let src = r#"
proc empty {
  kind L2
  consumes position
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("deform"), "{rendered}");
}

/// L2 consumes requirements are validated against upstream chain outputs by Set check.
#[test]
fn an_l2_may_consume_what_it_does_not_emit() {
    let src = r#"
proc pass {
  kind L2
  consumes position, velocity
  deform {
    position = position + velocity * 0.01;
  }
}
"#;
    let checked = check_ok(src);
    assert!(checked.emit.is_empty());
    assert_eq!(checked.consumes, vec![Attr::Position, Attr::Velocity]);
}
