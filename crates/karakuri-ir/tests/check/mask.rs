use super::common::*;

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
