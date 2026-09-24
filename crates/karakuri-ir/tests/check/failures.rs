use super::common::*;

// ---------------------------------------------------------------------------
// Failure modes
// ---------------------------------------------------------------------------

#[test]
fn undefined_name_is_rejected() {
    let src = r#"
proc bad {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit position

  element {
    position = position + nonexistent_thing;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("nonexistent_thing")),
        "expected a diagnostic naming the undefined identifier, got: {errs:?}"
    );
}

#[test]
fn mixing_int_and_float_literals_is_rejected() {
    let src = r#"
proc bad {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit position

  element {
    let x = 1 + 1.0;
    position = position;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("int") && e.message.contains("float")),
        "expected a diagnostic about mixing `int` and `float`, got: {errs:?}"
    );
}

/// A procedure may legally declare `param energy`, in which case a bare
/// `energy` resolves to the param and is perfectly ordinary to read.
#[test]
fn a_signal_name_declared_as_a_param_resolves() {
    let src = r#"
proc lawful {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  param energy : float [0.0, 1.0] = 0.5

  emit position

  element {
    position = position * energy;
  }
}
"#;
    check_ok(src);
}

/// The same bare name, undeclared, is an attempted signal-bus read: the
/// signal bus is not readable from IR, and the fix is to declare a `param`
/// and attach a `bind` record in the Set file.
#[test]
fn an_undeclared_signal_name_is_rejected_with_the_bind_hint() {
    let src = r#"
proc unlawful {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit position

  element {
    position = position * energy;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.hint.as_deref().unwrap_or_default().contains("bind")
                && e.hint.as_deref().unwrap_or_default().contains("param")),
        "expected a hint pointing at `param` + `bind`, got: {errs:?}"
    );
}

#[test]
fn attribute_assigned_in_only_one_if_arm_is_rejected() {
    let src = r#"
proc bad {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit position, age

  element {
    position = position;
    if age > 1.0 {
      age = 0.0;
    }
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("age") && e.message.contains("every path")),
        "expected a coverage diagnostic naming `age`, got: {errs:?}"
    );
}

#[test]
fn missing_required_output_is_rejected() {
    let src = r#"
proc bad {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    let unused = 1.0;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("color") && e.message.contains("every path")),
        "expected a coverage diagnostic naming `color`, got: {errs:?}"
    );
}

#[test]
fn kill_outside_l1_element_is_rejected() {
    let src = r#"
proc bad {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = vec4(position, 1.0);
    point_rate = 0.004;
    kill();
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("kill()")),
        "expected a diagnostic about `kill()`, got: {errs:?}"
    );
}

#[test]
fn swizzle_wider_than_the_source_is_rejected() {
    let src = r#"
proc bad {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit position

  element {
    let x = position.xyzw;
    position = position;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("out of range")),
        "expected a diagnostic about the out-of-range swizzle, got: {errs:?}"
    );
}

#[test]
fn non_literal_fbm_octave_count_is_rejected() {
    let src = r#"
proc bad {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  param n : float [1.0, 8.0] = 4.0

  emit position

  element {
    let x = fbm(position, int(n));
    position = position;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("literal")),
        "expected a diagnostic requiring a literal octave count, got: {errs:?}"
    );
}

#[test]
fn compound_assignment_on_an_attribute_is_rejected() {
    let src = r#"
proc bad {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit age

  element {
    age += dt;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("compound assignment") && e.message.contains("age")),
        "expected a diagnostic about compound assignment on `age`, got: {errs:?}"
    );
}

/// Verifies that consuming `velocity` is satisfied when `position` is emitted.
#[test]
fn a_consumed_velocity_is_satisfied_by_the_derivation_when_position_is_emitted() {
    check_ok(
        r#"
proc weird {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  param spawn_rate : float [0.0, 4000.0] = 100.0

  emit     position, age
  consumes position, velocity, age

  spawn {
    position = vec3(0.0);
    age      = 0.0;
  }

  element {
    position = position + velocity * dt;
    age      = age + dt;
  }
}
"#,
    );

    // And the half that is still a refusal, with the source missing.
    let errs = check_err(
        r#"
proc no_source {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit     age
  consumes velocity, age

  element {
    age = age + dt;
  }
}
"#,
    );
    assert!(
        errs.iter().any(
            |e| e.message.contains("velocity") && e.message.contains("derived from `position`")
        ),
        "expected the rule's source to be named, got: {errs:?}"
    );
}

/// `normal` has no derivation rule in the spec at all, unlike `velocity` and
/// `age`, so its rejection carries the plain "not emitted" message rather
/// than a mention of an unimplemented rule.
#[test]
fn consumes_with_no_derivation_rule_is_rejected() {
    let src = r#"
proc bad {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit     size
  consumes size, normal

  element {
    size = size;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("normal") && e.message.contains("not emitted")),
        "expected a diagnostic about `normal` not being emitted, got: {errs:?}"
    );
    assert!(
        !errs
            .iter()
            .any(|e| e.message.contains("normal") && e.message.contains("not implemented")),
        "`normal` has no derivation rule in the spec, so it should not get the \"not \
         implemented\" wording — got: {errs:?}"
    );
}

/// Verifies that L4 procedures consuming non-derivable attributes check clean.
#[test]
fn l4_consuming_an_unemitted_non_derivable_attribute_still_checks_clean() {
    let src = r#"
proc bad {
  kind  L4
  blend additive

  consumes normal

  vertex {
    clip       = vec4(normal, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.consumes, vec![Attr::Normal]);
    assert!(checked.emit.is_empty());
}

/// Verifies that consumes diagnostics point at the attribute span and preserve declaration order.
#[test]
fn a_consumes_diagnostic_points_at_the_attribute_and_keeps_declaration_order() {
    let src = r#"
proc probe {
  kind     L1
  topology points
  capacity [64, 1024] = 256

  emit position
  consumes position, normal, uv

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let errs = check_err(src);
    let named: Vec<&str> = errs
        .iter()
        .filter(|e| e.message.contains("consumed but not emitted"))
        .map(|e| {
            if e.message.contains("normal") {
                "normal"
            } else if e.message.contains("uv") {
                "uv"
            } else {
                "?"
            }
        })
        .collect();
    assert_eq!(named, vec!["normal", "uv"], "{errs:?}");

    // The `consumes` line, not the `proc` line: the span covers exactly the
    // word, so a repair prompt can point at it.
    let normal = errs
        .iter()
        .find(|e| e.message.contains("normal"))
        .expect("a diagnostic about `normal`");
    let text = &src[normal.span.start as usize..normal.span.end as usize];
    assert_eq!(text, "normal", "span covers `{text}`");
}

/// Verifies that declaring a `spawn` block requires a `spawn_rate` parameter.
#[test]
fn a_spawn_block_without_a_spawn_rate_param_is_rejected() {
    let src = r#"
proc no_rate {
  kind     L1
  topology points
  capacity [64, 4096] = 256

  emit position

  spawn   { position = vec3(0.0, 0.0, 0.0); }
  element { position = position; }
}
"#;
    let errs = check_err(src);
    let e = errs
        .iter()
        .find(|e| e.message.contains("spawn_rate"))
        .unwrap_or_else(|| panic!("expected a diagnostic about `spawn_rate`, got: {errs:?}"));
    assert!(
        e.hint.as_deref().unwrap_or("").contains("param spawn_rate"),
        "the hint has to spell the declaration a regenerating model should write: {e:?}"
    );
    // The span is the `spawn` block, which is what has to change or go.
    let text = &src[e.span.start as usize..e.span.end as usize];
    assert!(text.starts_with("spawn"), "span covers `{text}`");
}

/// Verifies that non-float `spawn_rate` parameters are rejected.
#[test]
fn a_spawn_rate_that_is_not_a_float_is_rejected() {
    let src = r#"
proc wrong_rate {
  kind     L1
  topology points
  capacity [64, 4096] = 256

  param spawn_rate : vec3 [0.0, 100.0] = vec3(1.0, 1.0, 1.0)

  emit position

  spawn   { position = vec3(0.0, 0.0, 0.0); }
  element { position = position; }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`spawn_rate` must be a `float`")),
        "expected a type diagnostic for `spawn_rate`, got: {errs:?}"
    );
}

/// The negative control for both: a `spawn` block *with* a float
/// `spawn_rate` must still check clean. Without this, the two tests above
/// would pass just as well against a rule that rejected every `spawn` block.
#[test]
fn a_spawn_block_with_a_float_spawn_rate_checks_clean() {
    let src = r#"
proc with_rate {
  kind     L1
  topology points
  capacity [64, 4096] = 256

  param spawn_rate : float [0.0, 40000.0] = 8000.0

  emit position

  spawn   { position = vec3(0.0, 0.0, 0.0); }
  element { position = position; }
}
"#;
    let checked = check_ok(src);
    assert!(checked.block(BlockKind::Spawn).is_some());
}

/// Verifies that capacity ranges starting at zero are rejected.
#[test]
fn a_capacity_range_starting_at_zero_is_rejected() {
    let src = r#"
proc empty_ok {
  kind     L1
  topology points
  capacity [0, 1024] = 256

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let errs = check_err(src);
    let hit = errs
        .iter()
        .find(|e| e.message.contains("capacity") && e.message.contains("at least 1"))
        .unwrap_or_else(|| panic!("expected a capacity-minimum diagnostic, got: {errs:?}"));
    let text = &src[hit.span.start as usize..hit.span.end as usize];
    assert!(
        text.contains("capacity"),
        "the span points at `{text}` rather than the declaration"
    );
}

/// The negative control: a minimum of exactly 1 is fine, so the check above
/// cannot be passing by rejecting every `capacity` it sees.
#[test]
fn a_capacity_range_starting_at_one_is_accepted() {
    let src = r#"
proc smallest {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.capacity.expect("a capacity range").min, 1);
}
