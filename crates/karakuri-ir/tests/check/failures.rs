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

/// **`velocity` consumed and not emitted is now satisfied, and this test says
/// the opposite of what it used to.**
///
/// It was a rejection test, and its reason was good while it held: the check
/// pass had once accepted this shape on the strength of a derivation nothing
/// implemented, so a procedure checked clean and then read zeros at runtime —
/// the one failure "one severity" cannot allow. What changed is the second
/// half. The engine writes a `velocity` slot from the step it already has, so
/// the value is there and the acceptance is not a promise any more.
///
/// The rejection it becomes is the one that is still real: **`velocity` derives
/// from `position`, and a procedure that emits neither cannot have it.** That
/// is a question one file can answer, which is why it stayed in the checker
/// while "does anybody emit this" moved to the Set.
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

/// `check_consumes_emitted` only runs `if kind != Kind::L1 { return; }` — the
/// module docs call this out as deliberate: an L4 procedure's `consumes` is a
/// cross-proc question left for Set-composition time, outside this crate.
/// `soft_points_checks_clean_with_expected_shape` already exercises this for
/// `velocity` and `age`, but those two also happen to be the only attributes
/// `is_derivable` recognizes, so that alone would not catch a regression that
/// swapped the `kind != Kind::L1` guard for `!attr.is_derivable()` and
/// otherwise left L4 behaving the same for those two names. `normal` has no
/// derivation rule at all, so an L4 procedure consuming it must still check
/// clean purely on the strength of the kind check.
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

/// A diagnostic points at the name that is wrong, and two of them come out in
/// declaration order.
///
/// Both properties come from walking `consumes` as a list rather than as a
/// set. The span matters because a caret under the whole procedure tells a
/// reader — human or model — nothing they did not already know. The order
/// matters because diagnostics are output, and the same source has to produce
/// the same output: a regeneration loop reacting to a list that reshuffles
/// between runs is reacting to noise. `HashSet` iteration order does exactly
/// that, and it is stable often enough to look fine in a quick test.
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

/// "`spawn` requires a spawn rate. Declare it as a parameter named
/// `spawn_rate`" — ir-spec, "Blocks". Unenforced until now, and harmless
/// while `spawn` was wired to nothing: with the lifecycle live, a `spawn`
/// block without a rate compiles clean, builds a Set, and then creates zero
/// elements every step forever, which reads as a procedure that draws
/// nothing rather than as a mistake. That is exactly the shape this pass
/// exists to refuse — see `docs/contributing.md` §3, *A test is watched to
/// fail before it is kept*.
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

/// The same rule's other half: the engine reads `spawn_rate` as elements per
/// second, so a `spawn_rate` of some other type is a rate the engine cannot
/// read rather than a param it can. It is also the only param name whose
/// type the engine depends on, which is why this is checked here and no
/// other param name is.
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

/// A `capacity` range starting at zero is rejected at the declaration.
///
/// Not a nicety. `Compaction::new` asserts a non-zero capacity, and
/// `Set::build` runs on the hot-swap worker thread — so a `.kir` declaring
/// `[0, …]` built at 0 was an assert firing on a background thread, which the
/// render thread sees as nothing at all. Catching it here means the diagnostic
/// points at the declaration that is wrong, which is also the only place a
/// regenerating model can fix it.
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
