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

/// **`amplify` is L2's, and each refusal names what the layer does instead.**
/// An L1's count is `capacity`, which a Set turns; an L3 makes a viewpoint; an
/// L4 draws what reaches it. Amplification is a multiplier on an input, which
/// is a thing only a stage with an input can be.
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

/// **Below two, and the two cases are refused for different reasons.** Zero
/// would make the layer decide liveness, which belongs entirely to the L1's
/// compaction; one would allocate a second buffer to hold a copy of the first.
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

/// **The mirror of the rule above, and it had the same failure mode.**
/// `seed` is per-element identity, and a fullscreen L4 has no element: no
/// element buffer is bound and the vertex stage is the engine's, not the
/// procedure's. Read there it lowered to `in.seed` against a `VsOut` with no
/// such field — valid `.kir`, invalid WGSL, and wgpu's uncaptured error handler
/// took the process down before a frame was drawn.
///
/// Refused here rather than in `Ambient::available_in` for the reason `eye` and
/// `ray` are: what makes an L4 a marcher is the *absence* of a `vertex` block,
/// and a block does not know its siblings.
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

/// A modulator that wobbles what reaches it and tints it. Reads `position` from
/// upstream, writes it back, and **widens the element** by emitting a `color`
/// nothing before it produced.
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

/// **A `deform` is the whole of what an L2 is**, and the resolved shape says
/// which layer it belongs to without any of the L1 or L4 header state.
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

/// **Vacuously closed form, and that is the decision the layer rests on.**
///
/// An L2 is stateless by rule, so it can never be the reason a Set has to be
/// run forward to reach an instant — which keeps `closed_form` and priming
/// questions the L1 alone answers, however long a chain gets. A `deform` that
/// reported `false` here would drag every Set it appeared in into needing a
/// warm-up.
#[test]
fn an_l2_is_closed_form_whatever_it_writes() {
    assert!(check_ok(WOBBLE).closed_form);
}

/// **A `deform` may read what it emits**, which is not the same permission an
/// `element` block has even though it looks like it. There the two lists are one
/// buffer; here `consumes` is the input edge and `emit` is the output one, and
/// an L2 that adds an attribute has to be able to read the field it is writing.
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

/// **`kill()` is refused, and the reason is structural rather than a
/// restriction.** Compaction runs once, after L1, and nothing downstream of a
/// deformation reconsiders liveness — so an L2 removing an element would remove
/// it from a range already decided. The hint has to say that rather than
/// offering the `element` block an L2 does not have.
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

/// **`consumes` is not checked against `emit` on an L2**, unlike an L1 where
/// consuming something unemitted is a contradiction inside one file. What an L2
/// may read depends on its position in a chain, which no single procedure can
/// know — that is the Set's check, against the whole chain.
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

// ---------------------------------------------------------------------------
// L3 — the camera
// ---------------------------------------------------------------------------

/// The simplest camera anyone would write: a sweep round the origin, on the
/// clock. Two outputs, and the other four take their defaults.
const SWEEP: &str = r#"
proc sweep {
  kind L3

  param radius : float [1.0, 40.0] = 8.0
  param speed  : float [0.0, 2.0]  = 0.15

  camera {
    let a = t * speed * 6.2831853;
    eye    = vec3(cos(a) * radius, 2.0, sin(a) * radius);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;

#[test]
fn an_l3_checks_clean_and_carries_no_geometry() {
    let checked = check_ok(SWEEP);
    assert_eq!(checked.kind, Kind::L3);
    assert_eq!(checked.blocks.len(), 1);
    assert_eq!(checked.blocks[0].kind, BlockKind::Camera);
    // A camera is a viewpoint, not geometry: nothing here says what is drawn,
    // how much of it there is, or how it is combined.
    assert_eq!(checked.topology, None);
    assert_eq!(checked.blend, None);
    assert!(checked.capacity.is_none());
    assert!(checked.emit.is_empty());
    assert!(checked.consumes.is_empty());
}

/// **`eye` and `target` are required and the other four are not.** Where the
/// camera is and what it looks at are the whole of what makes one camera
/// different from another; `up`, the field of view and the two planes have
/// answers that are right far more often than not, and the lowering writes them
/// before the block runs.
#[test]
fn a_camera_must_say_where_it_is_and_what_it_looks_at() {
    for missing in ["eye", "target"] {
        let src = format!(
            r#"
proc half {{
  kind L3
  camera {{
    {} = vec3(0.0, 0.0, 5.0);
  }}
}}
"#,
            if missing == "eye" { "target" } else { "eye" }
        );
        let errs = check_err(&src);
        let rendered = errs
            .iter()
            .map(|e| e.render(&src))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains(missing),
            "`{missing}` was not required: {rendered}"
        );
    }
}

#[test]
fn the_other_four_camera_outputs_are_optional_and_writable() {
    let src = r#"
proc full {
  kind L3
  camera {
    eye    = vec3(0.0, 0.0, 5.0);
    target = vec3(0.0, 0.0, 0.0);
    up     = vec3(0.0, 0.0, 1.0);
    fov_y  = 0.6;
    near   = 0.05;
    far    = 250.0;
  }
}
"#;
    check_ok(src);
}

/// **A camera reads the clock and its params, and no element.** An L3 runs once
/// a frame over nothing, so there is no element for an attribute to belong to —
/// and the hint has to send an author to the spec rather than to a `consumes`
/// list that would not help.
#[test]
fn a_camera_block_cannot_read_an_attribute() {
    let src = r#"
proc follow {
  kind L3
  camera {
    eye    = position + vec3(0.0, 0.0, 5.0);
    target = position;
  }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("position"), "{rendered}");
    assert!(
        rendered.contains("reduction") || rendered.contains("element zero"),
        "the hint does not say what pointing a camera at geometry will mean: {rendered}"
    );
}

/// And `consumes` is refused at the header for the same reason, rather than
/// checked clean and silently ignored by a lowering that reads no geometry.
#[test]
fn an_l3_cannot_declare_consumes_yet_and_is_told_why() {
    let src = r#"
proc follow {
  kind L3
  consumes position
  camera {
    eye    = vec3(0.0, 0.0, 5.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("consume"), "{rendered}");
    assert!(
        rendered.contains("ir-spec"),
        "the hint does not point at where this is decided: {rendered}"
    );
}

/// **`seed` is not available**, which is the one ambient an L3 loses relative to
/// every other layer: it is a per-element value, and there is no element.
#[test]
fn a_camera_block_has_no_seed() {
    let src = r#"
proc noisy {
  kind L3
  camera {
    eye    = vec3(hash1(seed), 0.0, 5.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("seed"), "{rendered}");
}

/// **`dt` is**, and an L4 does not get it. That asymmetry is the one an L3 being
/// allowed to hold state buys: a camera's craft is mostly smoothing, and
/// smoothing is written against a step.
#[test]
fn a_camera_block_gets_dt_where_a_renderer_does_not() {
    let src = r#"
proc stepper {
  kind L3
  camera {
    eye    = vec3(0.0, 0.0, 5.0 + dt);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    check_ok(src);
}

/// The three header fields that belong to the other layers, each refused with
/// the reason it belongs there.
#[test]
fn an_l3_refuses_capacity_topology_blend_and_emit() {
    for (field, decl) in [
        ("capacity", "capacity [1, 8] = 4"),
        ("topology", "topology points"),
        ("blend", "blend additive"),
        ("emit", "emit position"),
    ] {
        let src = format!(
            r#"
proc bad {{
  kind L3
  {decl}
  camera {{
    eye    = vec3(0.0, 0.0, 5.0);
    target = vec3(0.0, 0.0, 0.0);
  }}
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

#[test]
fn an_l3_without_a_camera_block_is_refused() {
    let src = r#"
proc empty {
  kind L3
  param radius : float [1.0, 40.0] = 8.0
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("camera"), "{rendered}");
}

/// **A `camera` block belongs to an L3 and nowhere else**, which is what giving
/// it its own `BlockKind` buys — the same argument `deform` was named for.
#[test]
fn a_camera_block_is_refused_in_an_l4() {
    let src = r#"
proc confused {
  kind  L4
  blend additive
  camera {
    eye    = vec3(0.0, 0.0, 5.0);
    target = vec3(0.0, 0.0, 0.0);
  }
  fragment { color = vec4(1.0, 1.0, 1.0, 1.0); }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("camera"), "{rendered}");
    assert!(
        rendered.contains("L3"),
        "the diagnostic does not say where it belongs: {rendered}"
    );
}

/// And a camera output cannot be written from a stage that has no camera to
/// write to. `eye` is readable in a marching fragment stage — one concept, read
/// there and written in a `camera` block — so this is the assignment that has to
/// be caught rather than the read.
#[test]
fn a_marcher_cannot_assign_the_eye_it_reads() {
    let src = r#"
proc march {
  kind  L4
  blend additive
  fragment {
    eye   = vec3(0.0, 0.0, 5.0);
    color = vec4(ray, 1.0);
  }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("eye"), "{rendered}");
}

/// **A stage output is reserved in the layer that owns it, and nowhere else.**
///
/// `Output` went from four names to ten when the camera arrived, and a global
/// reservation would have taken `up`, `target`, `near`, `far` and `fov_y` out of
/// every layer's vocabulary at once — `let near = length(position)` in a
/// marcher and `let up = vec3(0.0, 1.0, 0.0)` in an L1 were both legal and
/// neither shadows anything reachable there. The diagnostic was worse than the
/// refusal: it named a `camera` block the procedure does not have.
#[test]
fn a_camera_output_is_not_reserved_in_the_layers_that_have_no_camera() {
    check_ok(
        r#"
proc grounded {
  kind     L1
  topology points
  capacity [1, 1] = 1
  param far : float [1.0, 90.0] = 40.0
  emit position
  element {
    let up   = vec3(0.0, 1.0, 0.0);
    let near = 0.5;
    position = up * near * far;
  }
}
"#,
    );
    check_ok(
        r#"
proc marcher {
  kind  L4
  blend additive
  param target : float [0.0, 4.0] = 1.0
  fragment {
    let near = length(ray);
    color    = vec4(near, target, 0.0, 1.0);
  }
}
"#,
    );
}

/// And it **is** reserved where it exists, which is the other half: a `camera`
/// block's `far` is a stage output, so a local of that name would shadow the
/// thing the block is there to write.
#[test]
fn a_camera_output_is_reserved_inside_a_camera_block() {
    for decl in ["param far : float [1.0, 90.0] = 40.0", ""] {
        let src = format!(
            r#"
proc shadowed {{
  kind L3
  {decl}
  camera {{
    let far = 40.0;
    eye     = vec3(0.0, 0.0, far);
    target  = vec3(0.0, 0.0, 0.0);
  }}
}}
"#
        );
        let errs = check_err(&src);
        let rendered = errs
            .iter()
            .map(|e| e.render(&src))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("far"), "{rendered}");
        assert!(rendered.contains("shadows"), "{rendered}");
    }
}

/// **`eye` stays refused everywhere**, and as an *ambient* rather than an
/// output: a marching fragment reads it, so it genuinely is in scope in an L4
/// and a local of that name would shadow a value the procedure can use.
#[test]
fn the_eye_is_reserved_in_every_layer_because_a_marcher_reads_it() {
    let src = r#"
proc grounded {
  kind     L1
  topology points
  capacity [1, 1] = 1
  emit position
  element {
    let eye  = vec3(0.0, 0.0, 5.0);
    position = eye;
  }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("ambient"), "{rendered}");
}

/// **`eye` and `ray` belong to a marcher and to nothing else**, and reading
/// either in a per-element L4 used to check clean and then panic.
///
/// The lowering defines both in the ray prologue a fullscreen fragment stage
/// opens with. A procedure with a `vertex` block gets no prologue, so `ray`
/// lowered to a bare identifier nothing declared: `generate_l4` produced WGSL
/// naga refuses, and wgpu's uncaptured-error handler took down whichever thread
/// built it — a `SetError::Panicked` on the swap worker, and the process at
/// startup.
///
/// The diagnostic has to name the `vertex` block rather than the fragment one,
/// because what is wrong is that the procedure has a vertex stage at all.
#[test]
fn a_per_element_renderer_cannot_read_a_marchers_ray() {
    for name in ["ray", "eye"] {
        let src = format!(
            r#"
proc confused {{
  kind  L4
  blend additive
  consumes position
  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }}
  fragment {{
    color = vec4({name}, 1.0);
  }}
}}
"#
        );
        let errs = check_err(&src);
        let rendered = errs
            .iter()
            .map(|e| e.render(&src))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains(name), "{rendered}");
        assert!(
            rendered.contains("vertex"),
            "the diagnostic does not say what makes this procedure per-element: {rendered}"
        );
    }
}

/// And a marcher still reads both, which is the half that must not regress.
#[test]
fn a_marcher_reads_the_eye_and_the_ray() {
    check_ok(
        r#"
proc march {
  kind  L4
  blend additive
  fragment {
    let d = length(eye) + length(ray);
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
}

// ---------------------------------------------------------------------------
// L2 — `weight` and `mask`
// ---------------------------------------------------------------------------

/// A modulator that applies partially, both ways at once.
const PARTIAL: &str = r#"
proc partial {
  kind L2

  param weight : float [0.0, 1.0] = 0.6

  consumes position, age

  mask {
    strength = smoothstep(0.2, 0.6, age);
  }

  deform {
    position = position * 1.5;
  }
}
"#;

#[test]
fn an_l2_may_carry_a_mask_beside_its_deform() {
    let checked = check_ok(PARTIAL);
    assert_eq!(checked.kind, Kind::L2);
    let kinds: Vec<BlockKind> = checked.blocks.iter().map(|b| b.kind).collect();
    assert_eq!(kinds, vec![BlockKind::Mask, BlockKind::Deform]);
}

/// **`strength` is the whole of a mask**, so a block that never assigns it is
/// one whose author meant to say where the deformation applies and did not.
#[test]
fn a_mask_must_assign_a_strength() {
    let src = r#"
proc silent {
  kind L2
  consumes age
  mask {
    let s = age * 2.0;
  }
  deform { }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("strength"), "{rendered}");
}

/// **A mask reads `consumes` and not `emit`**, unlike the `deform` beside it.
/// It decides where the deformation applies, which is a question about what
/// *reaches* this node — and an emitted attribute has not been written when the
/// mask runs, so reading one would read the zero the pass-through left. Refusing
/// it beats a rule an author has to remember.
#[test]
fn a_mask_cannot_read_what_the_deformation_is_about_to_emit() {
    let src = r#"
proc widen {
  kind L2
  consumes position
  emit tint
  mask {
    strength = tint.x;
  }
  deform {
    tint = vec3(1.0, 0.0, 0.0);
  }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("tint"), "{rendered}");
    assert!(
        rendered.contains("consumes"),
        "the hint does not say what a mask may read: {rendered}"
    );
}

/// And it writes nothing but `strength`: rewriting an attribute is what the
/// `deform` is for, and a mask that could do it would be a second deformation
/// running before the first.
#[test]
fn a_mask_cannot_rewrite_an_attribute() {
    let src = r#"
proc sneaky {
  kind L2
  consumes position
  mask {
    position = position * 2.0;
    strength = 1.0;
  }
  deform { }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("position"), "{rendered}");
    assert!(
        rendered.contains("deform"),
        "the hint does not offer the block that can: {rendered}"
    );
}

/// **`weight` is a name the layer gives a meaning to**, on the same terms
/// `spawn_rate` is one — so declaring it as something else is refused rather
/// than silently scaling the modulation by a vector's first component or by
/// nothing at all.
#[test]
fn an_l2s_weight_must_be_a_float() {
    let src = r#"
proc odd {
  kind L2
  param weight : vec3 [0.0, 1.0] = vec3(1.0, 1.0, 1.0)
  consumes position
  deform { position = position * 2.0; }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("weight"), "{rendered}");
    assert!(rendered.contains("float"), "{rendered}");
}

/// `strength` is reserved in L2 and nowhere else, on the terms every other
/// stage output is — see `a_camera_output_is_not_reserved_in_the_layers_that_have_no_camera`.
#[test]
fn strength_is_reserved_in_an_l2_and_free_elsewhere() {
    check_ok(
        r#"
proc grounded {
  kind     L1
  topology points
  capacity [1, 1] = 1
  emit position
  element {
    let strength = 2.0;
    position = vec3(strength, 0.0, 0.0);
  }
}
"#,
    );
    let src = r#"
proc shadowed {
  kind L2
  consumes position
  mask {
    let strength = 1.0;
    strength = 1.0;
  }
  deform { }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("strength"), "{rendered}");
}

/// A `mask` block belongs to an L2, which is what giving it its own `BlockKind`
/// buys — the same argument `deform` and `camera` were named for.
#[test]
fn a_mask_block_is_refused_in_an_l1() {
    let src = r#"
proc confused {
  kind     L1
  topology points
  capacity [1, 1] = 1
  emit position
  mask { strength = 1.0; }
  element { position = vec3(0.0, 0.0, 0.0); }
}
"#;
    let errs = check_err(src);
    let rendered = errs
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("mask"), "{rendered}");
    assert!(
        rendered.contains("L2"),
        "the diagnostic does not say where it belongs: {rendered}"
    );
}

/// **A derived attribute is carried state, and `closed_form` has to see it.**
///
/// `is_closed_form` decides "accumulating" by whether a block reads an
/// attribute the procedure carries, and the paragraph above it used to argue
/// that inside an L1 `consumes ⊆ emit` held, so the membership test was
/// belt-and-braces. Derivation removed that: an L1 may consume `velocity`
/// without emitting it, and `velocity` is a stored difference — reading it is
/// reading where the element has been.
///
/// A permissive answer here is silent and expensive. `Set::is_closed_form`
/// decides whether beat sync is allowed, whether the governor primes the Set
/// before putting it on air, and whether `Set::seek` may jump to an instant
/// instead of stepping to it. All three would have taken accumulating material
/// for closed form.
#[test]
fn reading_a_derived_attribute_makes_a_procedure_accumulating() {
    let bare = check_ok(
        r#"
proc bare {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#,
    );
    assert!(
        bare.closed_form,
        "nothing is read back, so `t` alone decides the state"
    );

    for attr in ["velocity", "age"] {
        let src = format!(
            r#"
proc reads_derived {{
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit     position
  consumes {attr}

  element {{
    position = vec3(0.0, float({attr}) * 0.0, 0.0);
  }}
}}
"#
        );
        // `float(...)` takes a scalar, so the vector case reads a component.
        let src = src.replace("float(velocity)", "velocity.y");
        let src = src.replace("float(age)", "age");
        let checked = check_ok(&src);
        assert!(
            !checked.closed_form,
            "`{attr}` is per-element state carried across frames, so reading it accumulates"
        );
    }
}

/// **A derivation is refused in a `spawn` block**, and refused rather than
/// substituted because there is nothing to substitute: every rule reads state
/// an element being allocated does not have yet.
///
/// It was neither, and that is why this is here. The read fell through to the
/// ordinary path and lowered to a field the `Element` struct does not have, so
/// a `.kir` that checked clean produced WGSL naga rejects — a wgpu validation
/// panic at build, which at startup takes the process down and on the swap
/// worker kills the thread instead of producing a rejection.
#[test]
fn a_derived_attribute_is_refused_in_a_spawn_block() {
    for attr in ["age", "velocity"] {
        let src = format!(
            r#"
proc spawn_reads {{
  kind     L1
  topology points
  capacity [8, 8] = 8

  param spawn_rate : float [0.0, 100.0] = 10.0

  emit     position
  consumes {attr}

  spawn {{
    position = vec3(0.0, 0.0, 0.0) + vec3(0.0, {read}, 0.0);
  }}

  element {{
    position = position + vec3(0.0, 0.25, 0.0) * dt;
  }}
}}
"#,
            read = if attr == "age" { "age" } else { "velocity.y" }
        );
        let errs = check_err(&src);
        assert!(
            errs.iter().any(|e| e.message.contains(attr)
                && e.hint.as_deref().unwrap_or_default().contains("derived")),
            "expected `{attr}` to be refused in `spawn` with the rule named, got: {errs:?}"
        );
    }

    // The same read in `element` is exactly what the rule is for.
    check_ok(
        r#"
proc element_reads {
  kind     L1
  topology points
  capacity [8, 8] = 8

  param spawn_rate : float [0.0, 100.0] = 10.0

  emit     position
  consumes age

  spawn {
    position = vec3(0.0, 0.0, 0.0);
  }

  element {
    position = vec3(0.0, age, 0.0);
  }
}
"#,
    );
}
