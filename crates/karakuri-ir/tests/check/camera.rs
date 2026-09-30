use super::common::*;

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

/// Verifies that `eye` and `target` assignments are required in a camera block.
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

/// Camera blocks execute once per frame without element attribute access.
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
        rendered.contains("subject[0u].position"),
        "the hint does not say how a camera reads geometry: {rendered}"
    );
}

/// `consumes` on an L3 names what it reads through its geometry slot, so
/// without one it is refused rather than checked clean and ignored.
#[test]
fn an_l3_consumes_only_through_a_geometry_slot() {
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
        rendered.contains("uses subject : Geometry"),
        "the hint does not say what `consumes` needs on an L3: {rendered}"
    );
}

/// Per-element seed ambient is unavailable in camera blocks.
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

/// Delta time ambient is available in camera blocks for smoothing calculations.
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

/// Camera blocks are exclusive to L3 procedures.
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

/// Verifies camera stage outputs are not reserved words outside of camera blocks.
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

/// Stage output variable `far` cannot be shadowed by a local variable.
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

/// Ambient `eye` is protected from local shadowing.
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

/// Verifies that marching ambients `eye` and `ray` cannot be read in per-element renderers.
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
// L3 — a camera that reads geometry
// ---------------------------------------------------------------------------

/// Wraps a camera block in an L3 that reads `subject` and consumes `consumes`.
fn follower(consumes: &str, body: &str) -> String {
    format!(
        "proc follow {{\n  kind L3\n  uses subject : Geometry\n  consumes {consumes}\n  \
         camera {{\n{body}\n  }}\n}}\n"
    )
}

fn rendered(src: &str) -> String {
    check_err(src)
        .iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_camera_reads_elements_by_age() {
    let checked = check_ok(&follower(
        "position",
        "    eye = subject[0u].position;\n    target = subject[uint(t) + 1u].position;",
    ));
    assert_eq!(checked.geometry_slot(), Some("subject"));
    assert!(!checked.reads_reduction());
}

#[test]
fn a_camera_reads_reductions_and_pays_for_them_per_element() {
    let checked = check_ok(&follower(
        "position",
        "    eye = subject.centroid + vec3(0.0, 0.0, 5.0);\n    \
         target = mix(subject.bounds_min, subject.bounds_max, vec3(0.5));",
    ));
    assert!(checked.reads_reduction());
    let cost = karakuri_ir::cost::estimate(&checked).expect("within budget");
    assert_eq!(
        cost.ops_per_element,
        karakuri_ir::cost::REDUCTION_OPS_PER_ELEMENT
    );
}

#[test]
fn an_element_index_is_a_uint() {
    let text = rendered(&follower(
        "position",
        "    eye = subject[0].position;\n    target = vec3(0.0);",
    ));
    assert!(text.contains("`uint`") && text.contains("0u"), "{text}");
}

#[test]
fn an_element_attribute_must_be_consumed() {
    let text = rendered(&follower(
        "position",
        "    eye = subject[0u].position + subject[0u].tint;\n    target = vec3(0.0);",
    ));
    assert!(text.contains("tint") && text.contains("consumes"), "{text}");
}

#[test]
fn a_reduction_reads_position_and_so_needs_it_consumed() {
    let text = rendered(&follower(
        "tint",
        "    eye = subject.centroid + subject[0u].tint;\n    target = vec3(0.0);",
    ));
    assert!(text.contains("add `position` to `consumes`"), "{text}");
}

#[test]
fn a_camera_has_no_paired_element_and_is_told_to_index() {
    let text = rendered(&follower(
        "position",
        "    eye = subject.position;\n    target = vec3(0.0);",
    ));
    assert!(text.contains("subject[0u].position"), "{text}");
}

#[test]
fn an_index_is_refused_where_it_does_not_address_an_l3_element() {
    let src = r#"
proc morph {
  kind L2
  uses far : Geometry
  consumes position
  deform {
    position = far[0u].position;
  }
}
"#;
    let text = rendered(src);
    assert!(text.contains("`camera` block"), "{text}");

    let text = rendered(&follower(
        "position",
        "    let v = vec3(1.0);\n    eye = vec3(v[0u]);\n    target = vec3(0.0);",
    ));
    assert!(text.contains("no arrays"), "{text}");
}

#[test]
fn a_camera_reads_one_geometry() {
    let src = r#"
proc follow {
  kind L3
  uses subject : Geometry
  uses other : Geometry
  consumes position
  camera {
    eye = subject[0u].position;
    target = vec3(0.0);
  }
}
"#;
    let text = rendered(src);
    assert!(text.contains("a camera reads one geometry"), "{text}");
}
