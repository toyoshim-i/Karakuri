//! Check-pass tests: the three complete `.kir` examples from `docs/ir-spec.md`
//! must check clean and produce the resolved shape we expect, and a battery of
//! invalid fixtures exercises every failure mode the check pass is
//! responsible for.

use karakuri_ir::ast::{Attr, BinOp, BlockKind, Kind, Output, Topology, Ty};
use karakuri_ir::check::check;
use karakuri_ir::parse::parse;
use karakuri_ir::typed::{Checked, Target, TExprKind, TStmt};

/// Parse then check, panicking with rendered diagnostics if either stage
/// unexpectedly fails. Used for fixtures this test expects to be valid.
fn check_ok(src: &str) -> Checked {
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "expected source to parse:\n{}",
            errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
        )
    });
    check(&proc).unwrap_or_else(|errs| {
        panic!(
            "expected source to check clean:\n{}",
            errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
        )
    })
}

/// Parse then check, panicking if parsing fails (these fixtures are meant to
/// be syntactically valid and semantically wrong) and returning the check
/// errors otherwise.
fn check_err(src: &str) -> Vec<karakuri_ir::error::IrError> {
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "expected source to parse (it should fail *checking*, not parsing):\n{}",
            errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
        )
    });
    check(&proc).expect_err("expected the check pass to reject this source")
}

// ---------------------------------------------------------------------------
// The three canonical examples must check clean with the expected shape.
// ---------------------------------------------------------------------------

#[test]
fn drift_shell_checks_clean_with_expected_shape() {
    let src = include_str!("fixtures/drift_shell.kir");
    let checked = check_ok(src);

    assert_eq!(checked.name, "drift_shell");
    assert_eq!(checked.kind, Kind::L1);
    assert_eq!(checked.topology, Some(Topology::Points));
    assert_eq!(checked.capacity.map(|c| c.default), Some(262144));
    assert_eq!(checked.emit, vec![Attr::Position, Attr::Velocity, Attr::Age]);
    assert!(checked.consumes.is_empty());
    assert!(checked.cost.is_none(), "cost estimation has not run yet");

    let spawn = checked.block(BlockKind::Spawn).expect("spawn block");
    assert_eq!(spawn.stmts.len(), 5);

    let element = checked.block(BlockKind::Element).expect("element block");
    assert_eq!(element.stmts.len(), 6);

    // `let flow = curl(...) * turbulence;` resolves to a `vec3`.
    match &element.stmts[0] {
        TStmt::Let { name, value, .. } => {
            assert_eq!(name, "flow");
            assert_eq!(value.ty, Ty::Vec3);
        }
        other => panic!("expected a `let`, got {other:?}"),
    }

    // `position = position + v * dt;` targets the attribute, not a local.
    match &element.stmts[3] {
        TStmt::Assign { target, value, .. } => {
            assert_eq!(*target, Target::Attr(Attr::Position));
            assert_eq!(value.ty, Ty::Vec3);
        }
        other => panic!("expected an assignment, got {other:?}"),
    }

    // `if age > lifetime { kill(); }` — condition is `bool`, `then` is a
    // single `kill()`, `els` is empty.
    match &element.stmts[5] {
        TStmt::If { cond, then, els, .. } => {
            assert_eq!(cond.ty, Ty::Bool);
            assert!(matches!(then.as_slice(), [TStmt::Kill { .. }]));
            assert!(els.is_empty());
        }
        other => panic!("expected an `if`, got {other:?}"),
    }
}

#[test]
fn soft_points_checks_clean_with_expected_shape() {
    let src = include_str!("fixtures/soft_points.kir");
    let checked = check_ok(src);

    assert_eq!(checked.name, "soft_points");
    assert_eq!(checked.kind, Kind::L4);
    assert!(checked.topology.is_none());
    assert!(checked.capacity.is_none());
    assert_eq!(checked.consumes, vec![Attr::Position, Attr::Velocity, Attr::Age]);
    assert!(checked.emit.is_empty());
    // The L4 side of `consumes ⊆ emit` is a cross-proc (Set-composition)
    // question this pass cannot answer alone — see the module docs on
    // `check.rs`.

    let vertex = checked.block(BlockKind::Vertex).expect("vertex block");
    match &vertex.stmts[0] {
        TStmt::Assign { target, value, .. } => {
            assert_eq!(*target, Target::Output(Output::Clip));
            assert_eq!(value.ty, Ty::Vec4);
        }
        other => panic!("expected an assignment, got {other:?}"),
    }
    match &vertex.stmts[1] {
        TStmt::Assign { target, value, .. } => {
            assert_eq!(*target, Target::Output(Output::PointSize));
            assert_eq!(value.ty, Ty::Float);
        }
        other => panic!("expected an assignment, got {other:?}"),
    }

    let fragment = checked.block(BlockKind::Fragment).expect("fragment block");
    match fragment.stmts.last() {
        Some(TStmt::Assign { target, value, .. }) => {
            assert_eq!(*target, Target::Output(Output::Color));
            assert_eq!(value.ty, Ty::Vec4);
        }
        other => panic!("expected a final assignment, got {other:?}"),
    }
}

#[test]
fn var_accum_checks_clean_with_expected_shape() {
    let src = include_str!("fixtures/var_accum.kir");
    let checked = check_ok(src);

    assert_eq!(checked.kind, Kind::L1);
    assert_eq!(checked.emit, vec![Attr::Position, Attr::Velocity]);

    let element = checked.block(BlockKind::Element).expect("element block");
    assert_eq!(element.stmts.len(), 5);

    match &element.stmts[0] {
        TStmt::Var { name, value, .. } => {
            assert_eq!(name, "flow");
            assert_eq!(value.ty, Ty::Vec3);
        }
        other => panic!("expected a `var`, got {other:?}"),
    }

    // `flow += curl(...) / f;` desugars to a simple assignment carrying a
    // `Binary { op: Add, .. }` value.
    match &element.stmts[1] {
        TStmt::For { body, .. } => match &body[1] {
            TStmt::Assign { target, value, .. } => {
                assert_eq!(*target, Target::Local("flow".to_string()));
                assert_eq!(value.ty, Ty::Vec3);
                match &value.kind {
                    TExprKind::Binary { op, .. } => assert_eq!(*op, BinOp::Add),
                    other => panic!("expected compound assignment to desugar to a binary op, got {other:?}"),
                }
            }
            other => panic!("expected an assignment, got {other:?}"),
        },
        other => panic!("expected a `for`, got {other:?}"),
    }
}

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
        errs.iter().any(|e| e.message.contains("int") && e.message.contains("float")),
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
        errs.iter().any(|e| e.hint.as_deref().unwrap_or_default().contains("bind")
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
    point_size = 1.0;
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
    point_size = 1.0;
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

/// `velocity` is consumed but not emitted. `position` being emitted used to
/// be enough for the check pass to derive `velocity` from it and let this
/// through — but nothing downstream ever implemented that derivation: there
/// is no WGSL emitter for it and no second frame of `position` history to
/// compute it from. A procedure in this shape checked clean and then read
/// zeros for `velocity` at runtime, which is exactly the failure "one
/// severity" cannot allow — a rejection is what gives a regenerating model
/// something to act on. Derivation is now design only, recorded below the
/// "specified, not implemented" line in `docs/ir-spec.md`, and this must be
/// a plain rejection.
#[test]
fn consumes_not_covered_by_emit_is_rejected_even_with_a_derivation_rule() {
    let src = r#"
proc weird {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit     position, age
  consumes position, velocity, age

  spawn {
    position = vec3(0.0);
    age      = 0.0;
  }

  element {
    position = position;
    age      = age + dt;
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("velocity") && e.message.contains("not implemented")),
        "expected a diagnostic naming `velocity` and saying its derivation is not implemented, \
         got: {errs:?}"
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
        errs.iter().any(|e| e.message.contains("normal") && e.message.contains("not emitted")),
        "expected a diagnostic about `normal` not being emitted, got: {errs:?}"
    );
    assert!(
        !errs.iter().any(|e| e.message.contains("normal") && e.message.contains("not implemented")),
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
    point_size = 1.0;
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
  consumes position, velocity, normal

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
            if e.message.contains("velocity") {
                "velocity"
            } else if e.message.contains("normal") {
                "normal"
            } else {
                "?"
            }
        })
        .collect();
    assert_eq!(named, vec!["velocity", "normal"], "{errs:?}");

    // The `consumes` line, not the `proc` line: column 22 is where `velocity`
    // starts, and the span covers exactly that word.
    let velocity = errs
        .iter()
        .find(|e| e.message.contains("velocity"))
        .expect("a diagnostic about `velocity`");
    let text = &src[velocity.span.start as usize..velocity.span.end as usize];
    assert_eq!(text, "velocity", "span covers `{text}`");
}

/// "`spawn` requires a spawn rate. Declare it as a parameter named
/// `spawn_rate`" — ir-spec, "Blocks". Unenforced until now, and harmless
/// while `spawn` was wired to nothing: with the lifecycle live, a `spawn`
/// block without a rate compiles clean, builds a Set, and then creates zero
/// elements every step forever, which reads as a procedure that draws
/// nothing rather than as a mistake. That is exactly the shape the README
/// says this pass exists to refuse.
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
        errs.iter().any(|e| e.message.contains("`spawn_rate` must be a `float`")),
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
