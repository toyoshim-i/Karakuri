//! Check-pass tests: the three complete `.kir` examples from `docs/ir-spec.md`
//! must check clean and produce the resolved shape we expect, and a battery of
//! invalid fixtures exercises every failure mode the check pass is
//! responsible for.

use karakuri_ir::ast::{Attr, BinOp, BlockKind, Kind, Output, SlotTy, Topology, Ty};
use karakuri_ir::check::check;
use karakuri_ir::parse::parse;
use karakuri_ir::typed::{Checked, Slot, TExprKind, TStmt, Target};

/// Parse then check, panicking with rendered diagnostics if either stage
/// unexpectedly fails. Used for fixtures this test expects to be valid.
fn check_ok(src: &str) -> Checked {
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "expected source to parse:\n{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    check(&proc).unwrap_or_else(|errs| {
        panic!(
            "expected source to check clean:\n{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
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
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    check(&proc).expect_err("expected the check pass to reject this source")
}

/// Every refusal a source draws, whichever stage made it.
///
/// **For the fixtures where which stage refuses is the uninteresting half.**
/// `uses far : Points` is a parse-time refusal and `far.position = …` is a
/// grammar one, where everything else in this section is a contract check —
/// and a test that had to know which would be asserting the shape of the
/// compiler rather than that the file is refused.
fn refusals(src: &str) -> Vec<karakuri_ir::error::IrError> {
    match parse(src) {
        Err(errs) => errs,
        Ok(proc) => check(&proc).expect_err("expected this source to be rejected"),
    }
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
    assert_eq!(
        checked.emit,
        vec![Attr::Position, Attr::Velocity, Attr::Age]
    );
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
        TStmt::If {
            cond, then, els, ..
        } => {
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
    // Was `is_none()` while `topology` was an L1 field with no L4 counterpart.
    // An L4's is now inferred from whether `vertex` writes a second endpoint,
    // and this one does not — see `an_l4_that_assigns_clip_b_is_inferred_to_draw_lines`.
    assert_eq!(checked.topology, Some(Topology::Points));
    assert!(checked.capacity.is_none());
    assert_eq!(
        checked.consumes,
        vec![Attr::Position, Attr::Velocity, Attr::Age]
    );
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
            assert_eq!(*target, Target::Output(Output::PointRate));
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
                    other => panic!(
                        "expected compound assignment to desugar to a binary op, got {other:?}"
                    ),
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
/// exists to refuse — see
/// `docs/principles/0089-a-check-you-have-not-watched-fail-is-guessing.md`.
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

// ---------------------------------------------------------------------------
// Closed form versus accumulating.
//
// The property the engine's governor uses to decide that a Set needs no
// priming. Nothing is rejected either way, so there is no diagnostic to assert
// on and no way for a mistake here to be loud: these are the only thing
// standing between a wrong answer and a slot going on air showing an unwarmed
// image while claiming it needed no warming. Every case below is written so
// that the *permissive* direction is what fails it.
// ---------------------------------------------------------------------------

/// A procedure whose position is a function of `seed` and `t` alone is closed
/// form: any `t` can be evaluated directly, so it needs no priming.
#[test]
fn a_procedure_that_never_reads_what_it_emits_is_closed_form() {
    let src = r#"
proc pure_shell {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  param radius : float [0.1, 8.0] = 2.0

  emit position

  element {
    let u = hash1(seed);
    let v = hash1(seed + 1000u);
    position = sphere_point(u, v) * radius * (1.0 + sin(t));
  }
}
"#;
    assert!(
        check_ok(src).closed_form,
        "a pure function of seed, t and params was classified as accumulating"
    );
}

/// The one the name is about: `age = age + dt` reads what it emits, so the
/// state at `t` is the sum of every step taken to get there.
#[test]
fn a_procedure_that_reads_what_it_emits_is_accumulating() {
    let src = r#"
proc accumulator {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  emit position, age

  element {
    position = vec3(hash1(seed), 0.0, 0.0);
    age      = age + dt;
  }
}
"#;
    assert!(
        !check_ok(src).closed_form,
        "`age = age + dt` reads `age`, which is emitted — this is the definition \
         of accumulating and it was classified closed form"
    );
}

/// **The read is found wherever it is.** Buried in the condition of an `if`
/// inside a `for`, bound to a local, and used two statements later — a
/// classifier that only looked at the right-hand side of an attribute
/// assignment, or that only walked the top level of a block, would miss every
/// one of these and call this procedure seekable.
#[test]
fn a_read_inside_a_nested_if_in_a_for_still_counts() {
    let src = r#"
proc buried_read {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  emit position, age

  element {
    var acc = 0.0;
    for i in 0..4 {
      if age > float(i) * 0.25 {
        acc = acc + 0.1;
      }
    }
    position = vec3(acc, 0.0, 0.0);
    age      = float(1.0);
  }
}
"#;
    assert!(
        !check_ok(src).closed_form,
        "the only read of `age` is in an `if` condition inside a `for`, and it was \
         not found — a classifier that misses it calls this seekable"
    );
}

/// The same read through a local. `let prev = position;` is a read of
/// `position`, and the fact that what is assigned back is spelled `prev` does
/// not make it one.
#[test]
fn a_read_that_reaches_the_value_through_a_local_still_counts() {
    let src = r#"
proc laundered_read {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  emit position

  element {
    let prev = position;
    let drift = vec3(0.0, 0.01, 0.0);
    position = prev + drift;
  }
}
"#;
    assert!(
        !check_ok(src).closed_form,
        "the read of `position` was laundered through a `let` and got past the \
         classifier"
    );
}

/// **Spawning and closed form cannot coexist**, and not because of attributes:
/// this procedure's `element` block is a pure function of `seed` and `t`. What
/// is accumulated is the *population* — the spawn accumulator and the live
/// range are engine state built up over every frame since the Set started — and
/// jumping to `t` does not conjure the elements that would have been born
/// getting there.
#[test]
fn a_spawn_block_disqualifies_even_a_pure_element_block() {
    let src = r#"
proc pure_fountain {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  param spawn_rate : float [0.0, 40000.0] = 100.0

  emit position

  spawn {
    position = vec3(0.0, 0.0, 0.0);
  }

  element {
    position = sphere_point(hash1(seed), hash1(seed + 7u)) * t;
  }
}
"#;
    let checked = check_ok(src);
    assert!(
        !checked.closed_form,
        "a procedure that spawns was called seekable; the elements that would have \
         been born on the way to `t` do not exist when `t` is jumped to"
    );
}

/// `kill()` disqualifies for the mirror-image reason: a killed element stays
/// killed, so which elements are alive at `t` is a function of every step taken
/// to get there rather than of `t`.
#[test]
fn a_kill_disqualifies_even_a_pure_element_block() {
    let src = r#"
proc pure_cull {
  kind     L1
  topology points
  capacity [1, 1024] = 256

  emit position

  element {
    position = sphere_point(hash1(seed), hash1(seed + 7u)) * 2.0;
    if t > 5.0 && t < 5.1 {
      kill();
    }
  }
}
"#;
    assert!(
        !check_ok(src).closed_form,
        "a procedure that can `kill()` was called seekable; arriving at t = 6 in one \
         step removes nothing this would have removed at t = 5.05"
    );
}

/// An L4 procedure holds no per-element state at all, so the property is
/// vacuously true of it — and a Set is closed form when both of its procedures
/// are, which in practice means when its L1 is.
#[test]
fn an_l4_procedure_is_vacuously_closed_form() {
    let src = include_str!("fixtures/soft_points.kir");
    assert!(
        check_ok(src).closed_form,
        "L4 is stateless and emits nothing, so there is nothing for it to warm"
    );
}

/// The canonical examples, as a check that the classifier's answers are the
/// ones the specification's own material deserves: `drift_shell` integrates a
/// velocity and spawns, and is accumulating on both counts.
#[test]
fn drift_shell_is_accumulating() {
    let checked = check_ok(include_str!("fixtures/drift_shell.kir"));
    assert!(!checked.closed_form);
}

/// **L4 is vacuously closed form, and nothing an L4 file can say changes it.**
///
/// The property is about per-element state and an L4 has none: it reads what
/// reached it and throws the result at a target. Classifying by "reads an
/// attribute it emits" would make a renderer that consumes what it draws look
/// accumulating, and since a Set is closed form only when every procedure in it
/// is, one renderer would make a whole seekable Set look like it needed
/// priming.
///
/// `emit` on an L4 is refused outright now — see
/// [`emit_is_refused_on_a_renderer`] — which is the same fact said once instead
/// of compensated for here. This stays because the short-circuit is what makes
/// the answer independent of what the file says.
#[test]
fn an_l4_is_vacuously_closed_form() {
    let src = r#"
proc odd_l4 {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.kind, karakuri_ir::Kind::L4);
    assert!(
        checked.closed_form,
        "an L4 holds no per-element state at all, so the flag is supposed to be \
         vacuously true — a renderer reading what it draws made the whole Set \
         look like it needed priming"
    );
}

// ---------------------------------------------------------------------------
// `topology lines`: what an L4 draws is inferred from `clip_b`, not declared.
// ---------------------------------------------------------------------------

/// An L4 whose `vertex` block assigns a second endpoint is drawing segments,
/// and the check pass is where that is decided — nothing downstream re-derives
/// it, so a wrong answer here silently picks the wrong quad expansion.
#[test]
fn an_l4_that_assigns_clip_b_is_inferred_to_draw_lines() {
    let src = r#"
proc streaks {
  kind  L4
  blend additive

  consumes position, velocity

  vertex {
    clip       = camera * vec4(position, 1.0);
    clip_b     = camera * vec4(position - velocity, 1.0);
    point_rate = 0.008;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(
        checked.topology,
        Some(Topology::Lines),
        "a second endpoint is the only thing that could make a procedure draw segments"
    );
}

/// The control for the test above, and not a redundant one: an inference that
/// answered `Lines` unconditionally would satisfy it, and this is what says
/// the answer depends on the source.
#[test]
fn an_l4_without_clip_b_is_inferred_to_draw_points() {
    let src = r#"
proc sprites {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.008;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.topology, Some(Topology::Points));
}

/// A procedure either draws segments or it does not. Assigning the far end
/// under a condition would leave it uninitialised on the other path, which is
/// a stroke laid along whatever the register happened to hold.
#[test]
fn clip_b_assigned_on_only_one_path_is_rejected() {
    let src = r#"
proc sometimes {
  kind  L4
  blend additive

  consumes position, velocity

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.008;
    if length(velocity) > 0.5 {
      clip_b = camera * vec4(position - velocity, 1.0);
    }
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("clip_b") && e.message.contains("every path")),
        "expected a coverage diagnostic naming `clip_b`, got: {errs:?}"
    );
}

/// `clip_b` is a vertex output, and the fragment stage has no second endpoint
/// to place. Reusing the existing output-legality rule rather than a new one is
/// the point — a new output that quietly escaped it would be assignable in a
/// block that cannot lower it.
#[test]
fn clip_b_in_a_fragment_block_is_rejected() {
    let src = r#"
proc misplaced {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.008;
  }

  fragment {
    clip_b = vec4(1.0, 1.0, 1.0, 1.0);
    color  = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("clip_b")),
        "expected a diagnostic naming `clip_b`, got: {errs:?}"
    );
}

/// An L4 still may not declare `topology`. The inference is not a second way
/// of saying it — it is the only way, and a header field would be a place for
/// the file to contradict its own `vertex` block.
#[test]
fn an_l4_that_declares_topology_is_still_rejected() {
    let src = r#"
proc declared {
  kind     L4
  topology lines
  blend    additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    clip_b     = camera * vec4(position, 1.0);
    point_rate = 0.008;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("topology")
            && e.hint.as_deref().unwrap_or_default().contains("clip_b")),
        "expected the diagnostic to point at `clip_b` as the way to say it, got: {errs:?}"
    );
}

/// An L1 may declare `topology lines`, and it survives checking as itself.
/// Nothing lowers from it — see `Set::build` — but a declaration that silently
/// became `points` would make the geometry's own statement of what it is a lie.
#[test]
fn an_l1_may_declare_topology_lines() {
    let src = r#"
proc strands {
  kind     L1
  topology lines
  capacity [1, 64] = 8

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.topology, Some(Topology::Lines));
}

// ---------------------------------------------------------------------------
// `fullscreen`: an L4 with no `vertex` block draws the frame.
// ---------------------------------------------------------------------------

/// The declaration is the absence. A procedure with nothing to place per
/// element has nothing for a vertex block to do, so not having one is how it
/// says it covers the frame.
#[test]
fn an_l4_with_no_vertex_block_is_inferred_to_draw_the_whole_frame() {
    let src = r#"
proc marcher {
  kind  L4
  blend additive

  fragment {
    let d = length(ray) + length(eye) + point_coord.x;
    color = vec4(d, d, d, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.topology, Some(Topology::Fullscreen));
}

/// **The rule that makes skipping the paired L1's simulation provable.** With
/// no vertex block there is nowhere to read an element from, so a `consumes`
/// here is a claim the procedure cannot honour.
#[test]
fn a_fullscreen_l4_that_consumes_attributes_is_rejected() {
    let src = r#"
proc marcher {
  kind  L4
  blend additive

  consumes position

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter().any(|e| e.message.contains("whole frame")
            && e.hint.as_deref().unwrap_or_default().contains("vertex")),
        "expected a diagnostic pairing the two, got: {errs:?}"
    );
}

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

// ---------------------------------------------------------------------------
// `kind Field`: a spatial function, and the only kind with no pass of its own.
// ---------------------------------------------------------------------------

const BLOB: &str = r#"
proc blob {
  kind Field

  param ball : float [0.1, 2.0] = 0.8

  field {
    let sph = sd_sphere(point - vec3(0.9, 0.0, 0.0), ball);
    let bx  = sd_box(point + vec3(0.9, 0.0, 0.0), vec3(0.7, 0.7, 0.7));
    distance = op_smooth_union(sph, bx, 0.55);
  }
}
"#;

/// A field is a procedure like any other — a `kind`, a block, `param`s an
/// operator rides — and declares nothing about geometry, because it is not
/// geometry.
#[test]
fn a_field_checks_clean_and_carries_no_geometry() {
    let checked = check_ok(BLOB);
    assert_eq!(checked.kind, Kind::Field);
    assert_eq!(
        checked.topology, None,
        "a field is a function, not geometry"
    );
    assert!(checked.emit.is_empty() && checked.consumes.is_empty());
    assert_eq!(checked.params.len(), 1);
    assert!(checked.block(BlockKind::Field).is_some());
}

/// **`point` is a field's only input, and no other block has one.** Every other
/// block is handed an element or a fragment; this one is handed a position.
#[test]
fn point_is_readable_in_a_field_block_and_nowhere_else() {
    check_ok(BLOB);

    let errs = check_err(
        r#"
proc reads_point {
  kind     L1
  topology points
  capacity [1, 1] = 1

  emit position

  element {
    position = point;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("point")),
        "expected `point` to be refused outside a field, got: {errs:?}"
    );
}

/// `distance` is the whole of what a field produces, so a field that assigns
/// nothing is a function with no return value.
#[test]
fn a_field_must_assign_distance() {
    let errs = check_err(
        r#"
proc silent {
  kind Field

  field {
    let d = sd_sphere(point, 1.0);
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("distance")),
        "expected `distance` to be required, got: {errs:?}"
    );
}

/// **A field has no element**, so nothing an element carries is readable in
/// one. The diagnostic says why rather than telling the author to declare it,
/// because declaring it is not available and would not help.
#[test]
fn attributes_are_refused_in_a_field_block() {
    let errs = check_err(
        r#"
proc peeks {
  kind Field

  field {
    distance = length(position) - 1.0;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("position")
            && e.hint
                .as_deref()
                .unwrap_or_default()
                .contains("function of space")),
        "expected a diagnostic explaining a field has no element, got: {errs:?}"
    );
}

/// **A renderer emits nothing**, and it was the one layer that did not say so.
///
/// An L3 and a field both refuse `emit` by name. An L4's was accepted, given no
/// buffer, and then read back by `is_closed_form` — which states "vacuously
/// true for L4" partly to stop a stray `emit` making a Set look accumulating
/// and dragging it into needing to be primed. That is a compensation for a
/// declaration that should not have parsed.
#[test]
fn emit_is_refused_on_a_renderer() {
    let errs = check_err(
        r#"
proc draws {
  kind L4
  blend additive

  emit position

  fragment {
    color = vec4(1.0);
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("emit")
            && e.hint.as_deref().unwrap_or_default().contains("consumes")),
        "expected `emit` to be refused on an L4, and `consumes` offered, got: {errs:?}"
    );
}

/// **`seed` is per element, and a field has none** — the same sentence
/// `attributes_are_refused_in_a_field_block` states, for the one per-element
/// value that is not an attribute.
///
/// It needed its own test for the reason it needed its own rule: `seed` is
/// available in every block *an element reaches*, and that sentence is
/// falsified twice — by a fullscreen L4 and by a field. The first was found by
/// running it; this one checked clean and reached `FieldResolver::read_seed`,
/// which is an `unreachable!`, so a `.kir` nobody could see anything wrong with
/// panicked the thread that compiled it.
#[test]
fn seed_is_refused_in_a_field_block() {
    let errs = check_err(
        r#"
proc jitter {
  kind Field

  field {
    distance = length(point) - 1.0 + hash1(seed) * 0.01;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("seed")
            && e.hint
                .as_deref()
                .unwrap_or_default()
                .contains("function of space")),
        "expected `seed` to be refused with a field's own reason, got: {errs:?}"
    );
}

/// Every geometry declaration is refused where it is written, on the same terms
/// an L3 refuses them: a field counts nothing, draws nothing and carries
/// nothing.
#[test]
fn a_field_refuses_every_geometry_declaration() {
    for (decl, word) in [
        ("capacity [1, 8] = 4", "capacity"),
        ("topology points", "topology"),
        ("blend additive", "blend"),
        ("amplify 4", "amplify"),
    ] {
        let src = format!(
            r#"
proc wrong {{
  kind Field
  {decl}

  field {{
    distance = length(point) - 1.0;
  }}
}}
"#
        );
        let errs = check_err(&src);
        assert!(
            errs.iter().any(|e| e.message.contains(word)),
            "expected `{word}` to be refused on a field, got: {errs:?}"
        );
    }

    let errs = check_err(
        r#"
proc wrong {
  kind Field

  emit position

  field {
    distance = length(point) - 1.0;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("emit")),
        "expected `emit` to be refused on a field, got: {errs:?}"
    );
}

/// **A geometry slot is refused on a field too**, which is the one geometry
/// declaration this arm used to let past.
///
/// A field is a function of space: it is handed `point` and returns a
/// distance, and there is no element for a far one to be paired with. Nothing
/// downstream could have honoured it either — a Set's edges resolve against
/// what an L2 declares, so the slot was invisible where it would have been
/// bound and the `edge` naming it came back as an unknown slot, a page away
/// from the line that caused it.
#[test]
fn a_field_refuses_a_geometry_slot() {
    let errs = check_err(
        r#"
proc wrong {
  kind Field

  uses far : Geometry

  field {
    distance = length(point) - 1.0;
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("`uses` is L2 only")),
        "expected `uses` to be refused on a field, got: {errs:?}"
    );
}

/// **`is_static` asks whether anything ever moves an element between slots**,
/// which is two questions and used to be one.
///
/// A `spawn` block allocates; `kill()` makes the next step's scan compact the
/// survivors down. Either one and `seed` stops being the slot index — which is
/// the property a caller wants it for, and which the `kill`-only case
/// falsified while the answer stayed true.
#[test]
fn a_procedure_that_kills_is_not_static() {
    let lattice = r#"
proc lattice {
  kind     L1
  topology points
  capacity [64, 64] = 64

  emit position

  element {
    position = vec3(float(seed % 8u), float(seed / 8u), 0.0);
  }
}
"#;
    assert!(
        check_ok(lattice).is_static(),
        "nothing spawns and nothing dies"
    );

    // The same procedure, killing. Every element is live at frame zero and not
    // after it, so the old sentence was true and the answer was wrong.
    let culled = lattice.replace(
        "    position = vec3(float(seed % 8u), float(seed / 8u), 0.0);",
        "    position = vec3(float(seed % 8u), float(seed / 8u), 0.0);\n    if seed == 3u { kill(); }",
    );
    assert!(
        !check_ok(&culled).is_static(),
        "a `kill()` compacts, and compaction moves every element after the gap"
    );

    let spawning = r#"
proc fountain {
  kind     L1
  topology points
  capacity [64, 64] = 64

  param spawn_rate : float [0.0, 100.0] = 10.0

  emit position

  spawn   { position = vec3(0.0, 0.0, 0.0); }
  element { position = position; }
}
"#;
    assert!(
        !check_ok(spawning).is_static(),
        "and a `spawn` block allocates"
    );
}

// ---------------------------------------------------------------------------
// `uses`: an L2 that declares a named geometry input.
// ---------------------------------------------------------------------------

const MORPH: &str = r#"
proc morph {
  kind L2
  uses far : Geometry

  param k : float [0.0, 1.0] = 0.5

  consumes position

  deform {
    position = mix(position, far.position, vec3(k, k, k));
  }
}
"#;

/// The declaration lands on `Checked` **as a name and a type**, and
/// `far.<attr>` resolves to a read of the far element.
///
/// The name is what the whole change is: it is the procedure's own, so nothing
/// couples to a Set, and it is what an `edge` is written against. The type is
/// what says which sort of thing may fill it — one today, and asked for by
/// type rather than by position.
#[test]
fn a_used_geometry_checks_clean_and_carries_its_name() {
    let checked = check_ok(MORPH);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "far".to_string(),
            ty: SlotTy::Geometry,
        }]
    );
    assert_eq!(checked.geometry_slot(), Some("far"));
    assert_eq!(checked.kind, Kind::L2);

    let deform = checked.block(BlockKind::Deform).expect("a deform");
    let reads_far = |stmts: &[TStmt]| {
        stmts.iter().any(|s| match s {
            TStmt::Assign { value, .. } => format!("{value:?}").contains("Far"),
            _ => false,
        })
    };
    assert!(reads_far(&deform.stmts), "`far.position` is a far read");
}

/// **The base name is the procedure's own**, so a read through a name it did
/// not declare resolves to nothing rather than to the second geometry.
///
/// This is the whole difference from the reserved `other` this replaced: a
/// spelling means the far side because the header said so, not because the
/// language reserved a word — which is what let there be only ever one.
#[test]
fn a_read_through_an_undeclared_name_is_refused() {
    let errs = check_err(&MORPH.replace("  uses far : Geometry\n", ""));
    assert!(
        errs.iter().any(|e| e.message.contains("`far`")),
        "expected the name to be reported as resolving to nothing, got: {errs:?}"
    );

    // And a procedure that declares one slot does not get a second by writing
    // a different name.
    let errs = check_err(&MORPH.replace("far.position", "near.position"));
    assert!(
        errs.iter().any(|e| e.message.contains("`near`")),
        "expected `near` to resolve to nothing, got: {errs:?}"
    );
}

/// **One `consumes` covers both sides.** The far geometry is an input edge, and
/// a node reads the same attribute from each — so an attribute this node does
/// not take is not readable on either side.
#[test]
fn a_far_read_takes_only_what_the_node_consumes() {
    let errs = check_err(&MORPH.replace("far.position", "far.tint"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("tint") && e.message.contains("not consumed")),
        "expected `far.tint` to need `tint` in `consumes`, got: {errs:?}"
    );
}

/// **A geometry is not a value.** `far` alone is the whole second source, which
/// this language has no type for and no way to pass — so the one thing that can
/// be said about it is what one of its elements holds.
///
/// The complement of the read above: the new name appears in expression
/// position, and every position it can appear in either produces a value or is
/// refused with its own reason.
#[test]
fn a_used_geometry_is_not_a_value_on_its_own() {
    let errs = check_err(&MORPH.replace("far.position", "far"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`far` is a geometry, not a value")),
        "expected a used geometry to be unreadable as a value, got: {errs:?}"
    );
}

/// **And it is not assignable.** The far side is an input edge: this node reads
/// it and writes its own output, so a `.kir` that assigned to it would be
/// asking to write another source's buffer.
///
/// Refused by the grammar rather than by the checker — an assignment target is
/// a bare name — which is a refusal all the same, and the one that matters is
/// that it never reaches the generator.
#[test]
fn a_used_geometry_cannot_be_assigned_to() {
    let errs = refusals(&MORPH.replace(
        "position = mix(position, far.position, vec3(k, k, k));",
        "far.position = position;",
    ));
    assert!(
        !errs.is_empty(),
        "writing to the far side has to be refused"
    );
}

/// `uses` is L2's, on the same terms `amplify` is: an L1 makes geometry rather
/// than taking any, an L3 makes a viewpoint, and an L4 draws what reaches it.
#[test]
fn uses_is_refused_outside_an_l2() {
    let l1 = r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 8] = 4
  uses far : Geometry

  emit position

  element { position = vec3(0.0, 0.0, 0.0); }
}
"#;
    let l4 = r#"
proc dots {
  kind  L4
  blend additive
  uses far : Geometry

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment { color = vec4(1.0, 1.0, 1.0, 1.0); }
}
"#;
    for src in [l1, l4] {
        let errs = check_err(src);
        assert!(
            errs.iter().any(|e| e.message.contains("`uses` is L2 only")),
            "expected `uses` to be refused, got: {errs:?}"
        );
    }
}

/// **A node takes one second geometry**, and a second `uses` is refused with a
/// sentence rather than quietly overwriting the first.
///
/// One name would win and the other's reads would resolve against the wrong
/// geometry — which is the shape this notation exists to end, arriving through
/// the notation itself.
#[test]
fn a_second_used_geometry_is_refused() {
    let errs = check_err(&MORPH.replace(
        "  uses far : Geometry\n",
        "  uses far : Geometry\n  uses other : Geometry\n",
    ));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("one second geometry")),
        "expected a second slot to be refused, got: {errs:?}"
    );
}

/// **`Geometry` is what a slot can be**, and anything else is refused by name
/// rather than accepted and ignored.
///
/// The type is written even though there is one of them, so that the slot that
/// takes a camera or a field — which is what this notation is for next — does
/// not have to grow a type incompatibly.
#[test]
fn an_unknown_slot_type_is_refused() {
    let errs = refusals(&MORPH.replace("uses far : Geometry", "uses far : Points"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("unknown input type `Points`")),
        "expected the type to be refused, got: {errs:?}"
    );
}

/// **A slot shares one scope with everything else nameable here.** It is read
/// the way a local is — `far.position` — so a slot called `position` would make
/// one spelling mean two things depending on whether a dot follows it.
#[test]
fn a_slot_name_cannot_shadow_or_be_shadowed() {
    // An attribute.
    let errs = check_err(&MORPH.replace("uses far : Geometry", "uses position : Geometry"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("shadows an attribute name")),
        "expected a slot named after an attribute to be refused, got: {errs:?}"
    );

    // A param declared by this same procedure.
    let errs = check_err(&MORPH.replace("param k :", "param far :"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`far` is already a param")),
        "expected a slot and a param of one name to collide, got: {errs:?}"
    );

    // And a local, from the other direction.
    let errs = check_err(&MORPH.replace(
        "    position = mix",
        "    let far = 1.0;\n    position = mix",
    ));
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`far` is the geometry this procedure uses")),
        "expected a local to collide with the slot, got: {errs:?}"
    );
}

/// **`other` is an ordinary name again.** It was reserved everywhere because it
/// was the one spelling a paired read could have; a slot is named by the
/// procedure now, so reserving a word would be reserving one nothing means.
#[test]
fn other_is_no_longer_a_reserved_name() {
    let checked = check_ok(
        r#"
proc shadow {
  kind L2

  consumes position

  deform {
    let other = length(position);
    position = position * other;
  }
}
"#,
    );
    assert!(checked.uses.is_empty(), "and it declares no geometry slot");
}

// ---------------------------------------------------------------------------
// A field, reached through a slot
// ---------------------------------------------------------------------------

/// A marcher that contains no shape: it declares the field it takes and calls
/// it by that name.
const LENS: &str = r#"
proc lens {
  kind  L4
  blend additive

  uses shape : Field

  fragment {
    var p = eye;
    var hit = 0.0;
    for i in 0..8 {
      let d = shape(p);
      if d < 0.005 {
        hit = 1.0;
      }
      p = p + ray * max(d, 0.005);
    }
    color = vec4(hit, hit, hit, 1.0);
  }
}
"#;

/// **A declared Field slot is callable, and the call carries the slot's name.**
///
/// The name is the whole change. `field(p)` named the one field by being the
/// one spelling there was; this resolves against what the header declared, so
/// what reaches the lowering is a call that says *which* field — and a lowering
/// that has the name can address a second one without any of this moving.
#[test]
fn a_declared_field_slot_is_called_and_carries_its_name() {
    let checked = check_ok(LENS);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "shape".to_string(),
            ty: SlotTy::Field,
        }]
    );
    assert_eq!(checked.field_slots(), vec!["shape"]);
    assert_eq!(
        checked.geometry_slot(),
        None,
        "and it is not a geometry slot: the two answer different questions"
    );

    let fragment = checked.block(BlockKind::Fragment).expect("a fragment");
    let calls = format!("{:?}", fragment.stmts);
    assert!(
        calls.contains("Field") && calls.contains("shape"),
        "the call resolves to a field read naming its slot: {calls}"
    );

    // And its type is the field block's: one `vec3` in, a `float` out.
    let errs = check_err(&LENS.replace("shape(p)", "shape(p, 1.0)"));
    assert!(
        errs.iter().any(|e| e.message.contains("takes 1 argument")),
        "expected the arity to be the field's, got: {errs:?}"
    );
    let errs = check_err(&LENS.replace("shape(p)", "shape(1.0)"));
    assert!(
        errs.iter().any(|e| e.message.contains("expects `vec3`")),
        "expected a position to be required, got: {errs:?}"
    );
}

/// **Several Field slots on one procedure are legal**, which is the rule a
/// geometry slot does not follow.
///
/// A marcher wanting a shape and a cutter is the ordinary case, and it costs
/// nothing: a field has no node and no buffer, so a second slot is one more
/// body spliced under one more name. The count is a rule about *geometry* —
/// a second bound element buffer per node is what is not built — which is why
/// splitting the arity rule by type is what this notation needed.
#[test]
fn two_field_slots_on_one_procedure_are_accepted() {
    let checked = check_ok(
        r#"
proc carve {
  kind  L4
  blend additive

  uses shape  : Field
  uses cutter : Field

  fragment {
    let d = max(shape(eye), -cutter(eye + ray));
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    assert_eq!(checked.field_slots(), vec!["shape", "cutter"]);

    // **Counted apart**, which is what lets a Set multiply each by the field
    // bound to it rather than by whichever one it found first.
    let cost = karakuri_ir::cost::estimate(&checked).expect("a marcher of two fields costs");
    assert_eq!(
        cost.field_calls.slot("shape").map(|c| c.total),
        Some(1),
        "{:?}",
        cost.field_calls
    );
    assert_eq!(
        cost.field_calls.slot("cutter").map(|c| c.total),
        Some(1),
        "{:?}",
        cost.field_calls
    );

    // Two of one *name* is still refused: an edge names a slot, so two would be
    // one address for two inputs.
    let errs = check_err(
        r#"
proc twice {
  kind  L4
  blend additive

  uses shape : Field
  uses shape : Field

  fragment {
    let d = shape(eye);
    color = vec4(d, d, d, 1.0);
  }
}
"#,
    );
    assert!(
        errs.iter().any(|e| e.message.contains("already a slot")),
        "expected two slots of one name to be refused, got: {errs:?}"
    );
}

/// **A Field slot is legal on L1, L2, L3 and L4** — the four kinds that can
/// evaluate one — and a geometry slot is still L2's alone.
///
/// The two are told apart by the type on the declaration, which is what the
/// type was written down for: every refusal about `uses` is a sentence about
/// one of them, and this is the commit where each of them grew the arm beside
/// the one it had.
#[test]
fn a_field_slot_is_legal_on_every_kind_that_can_evaluate_one() {
    for src in [
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses shape : Field

  emit position

  element { position = vec3(shape(position), 0.0, 0.0); }
}
"#,
        r#"
proc warp {
  kind L2

  uses shape : Field

  consumes position

  deform { position = position * (1.0 + shape(position) * 0.01); }
}
"#,
        r#"
proc look {
  kind L3

  uses shape : Field

  camera {
    eye    = vec3(0.0, 0.0, 4.0 + shape(vec3(0.0, 0.0, 0.0)));
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#,
        LENS,
    ] {
        let checked = check_ok(src);
        assert_eq!(
            checked.field_slots(),
            vec!["shape"],
            "in `{}`",
            checked.name
        );
    }
}

/// **A field may not take a field**, and the reason is the one the recursion
/// refusal it replaced carried: a field bound to itself is a function calling
/// itself, which WGSL forbids outright.
///
/// Two fields naming each other is the same failure at one remove, and telling
/// that apart from a legal chain of shapes is a walk over every edge in the
/// Set. That is a graph question, answerable where the Set is built and nowhere
/// in one file — so the whole thing is refused rather than half-checked here.
#[test]
fn a_field_refuses_a_field_slot() {
    let errs = check_err(
        r#"
proc wrong {
  kind Field

  uses other : Field

  field {
    distance = other(point);
  }
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.message.contains("a field cannot take a field")),
        "expected a field taking a field to be refused, got: {errs:?}"
    );
}

/// **`field(p)` no longer resolves**, and the sentence says what to write
/// instead.
///
/// It was a reserved word, which is exactly what capped a procedure at one
/// field: a second would have had nothing to be called. Keeping it as an alias
/// for "the one field, if there is exactly one" would be that rule reinstated
/// under a new spelling, so it is gone — and this is a breaking change to the
/// language, which is why the refusal is written for the person migrating a
/// file rather than for the compiler.
#[test]
fn field_is_no_longer_a_reserved_word() {
    let errs = check_err(&LENS.replace("shape(p)", "field(p)"));
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`field` is not a builtin function or a type constructor")),
        "expected `field(p)` to resolve to nothing, got: {errs:?}"
    );
    assert!(
        errs.iter().any(|e| e
            .hint
            .as_deref()
            .is_some_and(|h| h.contains("uses shape : Field"))),
        "and the hint has to say what replaced it, got: {errs:?}"
    );

    // It is an ordinary name again, in both directions: a slot may be called
    // `field`, and then `field(p)` means that slot.
    let checked = check_ok(&LENS.replace("shape", "field"));
    assert_eq!(checked.field_slots(), vec!["field"]);
}

/// **A Field slot's name is called, so it may not be a builtin's.**
///
/// `check_reserved` asks about names that are *read* — an attribute, an
/// ambient, a stage output — and a builtin is none of those: `sin` was a
/// perfectly good slot name while a slot was only ever read with a dot after
/// it. A call resolves against the header first, so `sin(x)` would stop meaning
/// the sine, and either resolution order is a spelling that quietly means
/// something other than it says.
#[test]
fn a_field_slot_may_not_be_named_after_a_builtin_or_a_type() {
    for (name, what) in [
        ("sin", "a builtin function"),
        ("vec3", "a type constructor"),
    ] {
        let errs = check_err(&LENS.replace("shape", name));
        assert!(
            errs.iter().any(|e| e.message.contains(what)),
            "expected `{name}` to be refused as a Field slot, got: {errs:?}"
        );
    }

    // A *geometry* slot is not called, so it keeps the name: the refusal is
    // about the type, not about `uses`.
    let checked = check_ok(
        &MORPH
            .replace("uses far : Geometry", "uses sin : Geometry")
            .replace("far.position", "sin.position"),
    );
    assert_eq!(checked.geometry_slot(), Some("sin"));
}

// ---------------------------------------------------------------------------
// A camera, reached through a slot
// ---------------------------------------------------------------------------

/// A renderer that says which camera it draws from, and reads the projection
/// through it.
const THROUGH: &str = r#"
proc through {
  kind  L4
  blend additive

  uses view : Camera

  consumes position

  vertex {
    clip       = view.clip * vec4(position, 1.0);
    point_rate = 0.012;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

/// A marcher that says which camera it marches from.
const MARCH_THROUGH: &str = r#"
proc march_through {
  kind  L4
  blend additive

  uses view : Camera

  fragment {
    let p = view.eye + view.ray;
    color = vec4(p.x, p.y, p.z, 1.0);
  }
}
"#;

/// **A declared Camera slot is read as a member, and the members are the
/// ambients under another name.**
///
/// That is the whole of what this notation is: an L4 could already read the
/// Set's camera as `camera`, `eye` and `ray`, and what it could not say was
/// *which* camera. So the read resolves to the same three values — a new
/// spelling for an old capability, which is why nothing below the checker had
/// to move.
#[test]
fn a_declared_camera_slot_is_read_as_a_member() {
    let checked = check_ok(THROUGH);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "view".to_string(),
            ty: SlotTy::Camera,
        }]
    );
    assert_eq!(checked.camera_slot(), Some("view"));
    assert_eq!(
        checked.geometry_slot(),
        None,
        "and it is not a geometry slot: the two answer different questions"
    );
    assert!(checked.field_slots().is_empty());

    // `view.clip` is the `camera` ambient, so what reaches the lowering is
    // exactly what a renderer that named no slot produces.
    let vertex = checked.block(BlockKind::Vertex).expect("a vertex block");
    let read = format!("{:?}", vertex.stmts);
    assert!(
        read.contains("Ambient(Camera)"),
        "`view.clip` has to resolve to the camera ambient: {read}"
    );
    assert!(
        !read.contains("Far("),
        "and not to an attribute of a far element: {read}"
    );

    // The other two, in the stage that has them.
    let marching = format!(
        "{:?}",
        check_ok(MARCH_THROUGH)
            .block(BlockKind::Fragment)
            .expect("a fragment block")
            .stmts
    );
    assert!(
        marching.contains("Ambient(Eye)") && marching.contains("Ambient(Ray)"),
        "`view.eye` and `view.ray` have to resolve to the marcher's two: {marching}"
    );
}

/// **A Camera slot is L4's alone**, and each of the other four refuses it with
/// a sentence about itself rather than about `uses`.
///
/// An L3 *is* a camera — every member is a derivation of the six numbers it
/// writes — an L1 and an L2 work in world space and do not project, and a field
/// is a function of space whose answer cannot depend on where it is watched
/// from.
#[test]
fn a_camera_slot_is_l4s_alone() {
    for src in [
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses view : Camera

  emit position

  element { position = vec3(0.0, 0.0, 0.0); }
}
"#,
        r#"
proc warp {
  kind L2

  uses view : Camera

  consumes position

  deform { position = position * 1.5; }
}
"#,
        r#"
proc look {
  kind L3

  uses view : Camera

  camera {
    eye    = vec3(0.0, 0.0, 4.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#,
        r#"
proc blob {
  kind Field

  uses view : Camera

  field { distance = length(point) - 1.0; }
}
"#,
    ] {
        let errs = check_err(src);
        assert!(
            errs.iter()
                .any(|e| e.message.contains("`uses … : Camera` is L4 only")),
            "expected a Camera slot to be refused here, got: {errs:?}"
        );
    }

    // And an L4 takes one, so none of the above is a refusal of the type as
    // such.
    assert_eq!(check_ok(THROUGH).camera_slot(), Some("view"));
}

/// **A renderer draws from one camera.** Two slots would need two bind groups
/// in one pipeline and two edges per Set, and neither is what anybody asked
/// for: a frame drawn from two viewpoints is two renderers, which is exactly
/// what an edge per renderer makes possible.
///
/// Refused rather than resolved by position, on the terms every other collision
/// in a header is.
#[test]
fn two_camera_slots_on_one_renderer_are_refused() {
    let errs = check_err(
        r#"
proc twice {
  kind  L4
  blend additive

  uses left  : Camera
  uses right : Camera

  consumes position

  vertex {
    clip       = left.clip * vec4(position, 1.0);
    point_rate = 0.012;
  }

  fragment { color = vec4(1.0, 1.0, 1.0, 1.0); }
}
"#,
    );
    assert!(
        errs.iter()
            .any(|e| e.message.contains("a renderer draws from one camera")),
        "expected a second Camera slot to be refused, got: {errs:?}"
    );

    // Two of one *name* is refused as well, and by the check every slot type
    // goes through: an edge names a slot, so two would be one address for two
    // inputs.
    let errs = check_err(&THROUGH.replace(
        "uses view : Camera",
        "uses view : Camera\n  uses view : Camera",
    ));
    assert!(
        errs.iter().any(|e| e.message.contains("already a slot")),
        "expected two slots of one name to be refused, got: {errs:?}"
    );
}

/// **A camera's members are resolved against its type**, so a name that is not
/// one of the three is refused with the three named — and with the five values
/// an L4 still cannot read, which is unbuilt by decision rather than by
/// oversight:
/// `docs/adr/0153-a-renderer-reads-three-camera-members-and-the-l3s-five-stay-unreadable.md`.
#[test]
fn a_member_a_camera_has_not_got_is_refused() {
    let errs = check_err(&THROUGH.replace("view.clip", "view.target"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`view.target` is not part of a camera")),
        "expected an unknown member to be refused, got: {errs:?}"
    );
    assert!(
        errs.iter()
            .any(|e| e.hint.as_deref().is_some_and(|h| h.contains("view.clip"))),
        "and the hint has to name the members there are, got: {errs:?}"
    );

    // A camera is not a value on its own either: what can be had from it is one
    // of its parts.
    let errs = check_err(&THROUGH.replace("view.clip *", "view *"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`view` is a camera, not a value")),
        "expected the bare slot name to be refused, got: {errs:?}"
    );
}

/// **The members keep the stage rules the ambients have**, asked rather than
/// restated: `.eye` and `.ray` are built by the ray prologue a fullscreen
/// fragment stage opens with, and a per-element renderer has no such prologue.
///
/// A `.kir` that read them anyway used to check clean and lower to a bare
/// identifier nothing declared — WGSL naga refuses it, and wgpu's uncaptured
/// error handler takes the thread down. Reaching them through a slot must not
/// be a way back to that.
#[test]
fn the_members_keep_the_ambients_stage_rules() {
    // **Per element, and in the `fragment` stage**, which is where the ambient
    // itself is legal: what refuses this is the procedure having a `vertex`
    // block at all, so reading it anywhere else would be refused by the block
    // rule instead and prove nothing about this one.
    let errs = check_err(&THROUGH.replace(
        "color = vec4(1.0, 1.0, 1.0, 1.0)",
        "color = vec4(view.ray, 1.0)",
    ));
    assert!(
        errs.iter().any(|e| e
            .message
            .contains("`view.ray` is only available to a procedure that draws the whole frame")),
        "expected a per-element `.ray` to be refused, got: {errs:?}"
    );

    // Fullscreen: the same read is exactly what a marcher is for.
    assert_eq!(check_ok(MARCH_THROUGH).camera_slot(), Some("view"));

    // And `.clip` is readable in a vertex stage, which is where a projection is
    // used — so the rule is the ambient's own and not a blanket one.
    assert_eq!(check_ok(THROUGH).camera_slot(), Some("view"));
}

/// **A Camera slot's name goes through the same reserved-word check every slot
/// name does**, and it is *not* checked against the builtins.
///
/// The builtin rule is Field-only by design — a slot that is *called* is the
/// one with that collision, and `uses sin : Camera` is read `sin.clip`, which
/// cannot be confused with the sine. The same reasoning a geometry slot
/// already followed.
#[test]
fn a_camera_slot_name_shares_the_scope_every_slot_name_does() {
    let errs = check_err(&THROUGH.replace("view", "position"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("shadows an attribute name")),
        "expected a slot named after an attribute to be refused, got: {errs:?}"
    );
    let errs = check_err(&THROUGH.replace("view", "beats"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("shadows an ambient value")),
        "expected a slot named after an ambient to be refused, got: {errs:?}"
    );

    // A builtin's name is still a name here, because a camera slot is read and
    // not called.
    let checked = check_ok(&THROUGH.replace("view", "sin"));
    assert_eq!(checked.camera_slot(), Some("sin"));
}

// ---------------------------------------------------------------------------
// `source`, and the slot a mask compares it against
// ---------------------------------------------------------------------------

/// A generator that varies with which geometry of the Set it is making.
const SOURCE_L1: &str = r#"
proc grain {
  kind     L1
  topology points
  capacity [1, 64] = 8

  emit position, tint

  element {
    position = vec3(hash1(seed), hash1(seed + 1u), 0.0);
    tint     = vec3(float(source % 7u) * 0.1, 0.0, 0.0);
  }
}
"#;

/// A deformation that reads it, in the block a mask lives in.
const SOURCE_L2: &str = r#"
proc shrink {
  kind L2

  consumes position, size

  mask   { strength = float(source % 2u); }
  deform { size = size * 0.5; }
}
"#;

/// A renderer that reads it, in both its stages.
const SOURCE_L4: &str = r#"
proc lit
{
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = float(source % 3u) + 1.0;
  }

  fragment {
    color = vec4(float(source % 5u) * 0.2, 1.0, 1.0, 1.0);
  }
}
"#;

/// **`source` is readable on the three kinds a Set instantiates per geometry**,
/// and it is a `uint`.
///
/// It is a per-source *uniform*, not the fourth implicit attribute the spec's
/// table once called it: a chain is instantiated per source, so an instance
/// knows statically which geometry it runs over and the value is the same for
/// every element it touches. Which is why nothing had to be carried to make it
/// readable — `Set::prepare` has been writing the salt into every one of these
/// uniform blocks all along.
#[test]
fn source_is_readable_in_a_generator_a_deformation_and_a_renderer() {
    for (src, block) in [
        (SOURCE_L1, BlockKind::Element),
        (SOURCE_L2, BlockKind::Mask),
        (SOURCE_L4, BlockKind::Fragment),
    ] {
        let checked = check_ok(src);
        let read = format!(
            "{:?}",
            checked.block(block).expect("the block under test").stmts
        );
        assert!(
            read.contains("Ambient(Source)"),
            "`source` has to resolve to the ambient in {block:?}: {read}"
        );
        assert!(
            !read.contains("Attr(Source"),
            "and not to a carried attribute — the value is a uniform: {read}"
        );
    }

    // A vertex stage reads it as well as a fragment one: it is per instance,
    // not per fragment.
    let vertex = format!(
        "{:?}",
        check_ok(SOURCE_L4)
            .block(BlockKind::Vertex)
            .expect("a vertex block")
            .stmts
    );
    assert!(vertex.contains("Ambient(Source)"), "{vertex}");
}

/// **A `uint`, and the checker types it as one.** `float(source)` is the
/// spelling a colour wants and `source == only` is the one a mask wants;
/// neither works if the read comes back the wrong width.
#[test]
fn source_is_a_uint() {
    assert_eq!(karakuri_ir::ast::Ambient::Source.ty(), Ty::Uint);
    assert_eq!(
        karakuri_ir::ast::Ambient::from_name("source"),
        Some(karakuri_ir::ast::Ambient::Source)
    );

    // Read into a `let`, so the type shows up on the binding rather than
    // inside an arithmetic that could have coerced it.
    let src = SOURCE_L2.replace(
        "strength = float(source % 2u);",
        "let s = source; strength = float(s % 2u);",
    );
    let checked = check_ok(&src);
    let stmts = &checked.block(BlockKind::Mask).expect("a mask").stmts;
    match &stmts[0] {
        TStmt::Let { value, .. } => assert_eq!(value.ty, Ty::Uint),
        other => panic!("expected the `let` first: {other:?}"),
    }
}

/// **Refused on the two kinds that run over no geometry**, on exactly the
/// grounds `seed` is refused there: an L3 runs once a frame over nothing and
/// the salt is a property of a Set's geometry, which a camera is not; a field
/// is a function of space, spliced into callers that may have no geometry at
/// all.
#[test]
fn source_is_refused_in_a_camera_and_in_a_field() {
    let camera = r#"
proc drift {
  kind L3

  camera {
    eye    = vec3(0.0, 0.0, float(source % 4u) + 6.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let errs = check_err(camera);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`source` is per geometry")
                && e.message.contains("a camera")),
        "an L3 has to refuse `source` with a sentence about itself: {errs:?}"
    );

    let field = r#"
proc blob {
  kind Field

  field {
    distance = sd_sphere(point, 1.0) + float(source % 2u);
  }
}
"#;
    let errs = check_err(field);
    assert!(
        errs.iter().any(
            |e| e.message.contains("`source` is per geometry") && e.message.contains("a field")
        ),
        "a field has to refuse `source` with a sentence about itself: {errs:?}"
    );
}

/// **Refused in a procedure that declares a geometry slot**, because there the
/// reading would be silently one of two.
///
/// A pairing Set is one source made of two simulations: the far one feeds the
/// slot and shares the near one's uniform, so there is one salt for two
/// geometries and `source` would answer for the near one without saying so.
/// The refusal names the slot, which is the half that helps — a hint saying
/// *which* reading was ambiguous beats a rule the author has to infer.
#[test]
fn source_is_refused_beside_a_geometry_slot() {
    let src = r#"
proc morph {
  kind L2

  uses far : Geometry

  consumes position

  mask   { strength = float(source % 2u); }
  deform { position = mix(position, far.position, vec3(0.5, 0.5, 0.5)); }
}
"#;
    let errs = check_err(src);
    let hit = errs
        .iter()
        .find(|e| e.message.contains("`source` is ambiguous"))
        .unwrap_or_else(|| panic!("expected `source` to be refused here: {errs:?}"));
    assert!(
        hit.message.contains("far"),
        "the refusal names the slot that made it ambiguous: {}",
        hit.message
    );
    assert!(
        hit.hint
            .as_ref()
            .is_some_and(|h| h.contains("uses <name> : Source")),
        "and points at the spelling that is not ambiguous: {:?}",
        hit.hint
    );

    // The same procedure without the geometry slot is fine, which is what
    // makes this a rule about the pair rather than about `mask`.
    let alone = src.replace("  uses far : Geometry\n\n", "").replace(
        "mix(position, far.position, vec3(0.5, 0.5, 0.5))",
        "position",
    );
    assert_eq!(check_ok(&alone).source_slots(), Vec::<&str>::new());
}

/// A mask that dissolves whichever geometry its `only` slot was bound to.
const DISSOLVE: &str = r#"
proc dissolve {
  kind L2

  uses only : Source

  consumes position, size

  mask {
    strength = 0.0;
    if source == only { strength = 1.0; }
  }

  deform { size = size * 0.2; }
}
"#;

/// **A Source slot is read as a value, alone**, which no other slot type is.
///
/// A geometry is not a value because the language has no type for a whole
/// source; a field is not one because it is a function; a camera is not one
/// because it is six numbers. This is a `uint`, and the language has one of
/// those — so `only` on its own is the bound geometry's identity and needs no
/// dot, no call and no member.
#[test]
fn a_declared_source_slot_is_read_as_a_value() {
    let checked = check_ok(DISSOLVE);
    assert_eq!(
        checked.uses,
        vec![Slot {
            name: "only".to_string(),
            ty: SlotTy::Source,
        }]
    );
    assert_eq!(checked.source_slots(), vec!["only"]);
    assert_eq!(
        checked.geometry_slot(),
        None,
        "and it is not a geometry slot: a mask reads no elements of it"
    );
    assert!(checked.field_slots().is_empty());
    assert_eq!(checked.camera_slot(), None);

    let mask = checked.block(BlockKind::Mask).expect("a mask block");
    let read = format!("{:?}", mask.stmts);
    assert!(
        read.contains(r#"Source { slot: "only" }"#),
        "`only` has to resolve to its own slot read, carrying the name: {read}"
    );
    assert!(
        read.contains("Ambient(Source)"),
        "and `source` beside it to the ambient: {read}"
    );
}

/// **Several are legal**, which is the difference from `Geometry` and `Camera`
/// and the reason the type is worth its own variant.
///
/// A `Geometry` slot binds an element buffer and is capped at one because a
/// second bound buffer is not built. This binds a `u32` in a uniform the
/// module already has, so `source == a || source == b` costs two of them and
/// nothing else.
#[test]
fn two_source_slots_on_one_node_are_accepted() {
    let src = r#"
proc pair {
  kind L2

  uses a : Source
  uses b : Source

  consumes position, size

  mask {
    strength = 0.0;
    if source == a || source == b { strength = 1.0; }
  }

  deform { size = size * 0.2; }
}
"#;
    let checked = check_ok(src);
    assert_eq!(checked.source_slots(), vec!["a", "b"]);
    let read = format!(
        "{:?}",
        checked.block(BlockKind::Mask).expect("a mask").stmts
    );
    assert!(
        read.contains(r#"Source { slot: "a" }"#) && read.contains(r#"Source { slot: "b" }"#),
        "each read carries its own slot, or one edge would answer for both: {read}"
    );

    // Two of one *name* is still refused, on the terms every other collision
    // here is: an edge names a slot, so two of one name is one address for two
    // inputs.
    let errs = check_err(&src.replace("uses b : Source", "uses a : Source"));
    assert!(
        errs.iter().any(|e| e.message.contains("is already a slot")),
        "expected the duplicate name to be refused: {errs:?}"
    );
}

/// **Legal on L1, L2 and L4 and refused on L3 and Field**, which is the
/// ambient's rule stated one level up — a slot exists to be compared against
/// `source`, so it belongs exactly where `source` does.
#[test]
fn a_source_slot_follows_the_ambients_kinds() {
    // The three that may.
    for src in [
        r#"
proc gen {
  kind     L1
  topology points
  capacity [1, 64] = 8

  uses only : Source

  emit position

  element {
    var k = 0.0;
    if source == only { k = 1.0; }
    position = vec3(k, 0.0, 0.0);
  }
}
"#,
        DISSOLVE,
        r#"
proc lit {
  kind  L4
  blend additive

  uses only : Source

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.016;
  }

  fragment {
    var k = 0.0;
    if source == only { k = 1.0; }
    color = vec4(k, 0.0, 0.0, 1.0);
  }
}
"#,
    ] {
        assert_eq!(check_ok(src).source_slots(), vec!["only"]);
    }

    // And the two that may not, each with a sentence about itself.
    for (src, about) in [
        (
            r#"
proc look {
  kind L3

  uses only : Source

  camera {
    eye    = vec3(0.0, 0.0, 6.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#,
            "an L3 masks nothing",
        ),
        (
            r#"
proc blob {
  kind Field

  uses only : Source

  field { distance = sd_sphere(point, 1.0); }
}
"#,
            "a field masks nothing",
        ),
    ] {
        let errs = check_err(src);
        assert!(
            errs.iter().any(|e| e.message.contains(about)),
            "expected a refusal saying `{about}`: {errs:?}"
        );
    }
}

/// **A fullscreen renderer reads it too**, and that is what separates this from
/// `seed`.
///
/// `seed` is per element and a fullscreen L4 has none, so reading it there is
/// refused. `source` is per *instance*: the chain is instantiated per geometry,
/// so a procedure with no element still knows whose chain it is running in, and
/// its uniform holds the salt like every other module's.
#[test]
fn source_is_readable_in_a_fullscreen_renderer_where_seed_is_not() {
    let src = r#"
proc march {
  kind  L4
  blend additive

  fragment {
    color = vec4(float(source % 3u) * 0.3, length(ray) * 0.0 + 0.5, 0.5, 1.0);
  }
}
"#;
    let checked = check_ok(src);
    let read = format!(
        "{:?}",
        checked
            .block(BlockKind::Fragment)
            .expect("a fragment")
            .stmts
    );
    assert!(read.contains("Ambient(Source)"), "{read}");

    // The mirror, unchanged: `seed` there is still refused.
    let errs = check_err(&src.replace("float(source % 3u)", "float(seed % 3u)"));
    assert!(
        errs.iter()
            .any(|e| e.message.contains("`seed` is per element")),
        "expected `seed` to stay refused in a fullscreen procedure: {errs:?}"
    );
}

/// A `.kir` still writing `point_size` is refused by name, and the refusal says
/// what to write instead.
///
/// The rename is the whole reason this test exists. `point_size` is not a
/// declared name and not an output any more, so without a case of its own it
/// would fall out of `resolve_target` as "assigning to `point_size`, which was
/// never declared" — a sentence that is true and useless. Every `.kir` written
/// before the rename hits this line, and each of them needs the same two facts:
/// the new spelling, and that the number is no longer a pixel count.
#[test]
fn a_file_still_writing_point_size_is_refused_with_the_new_spelling() {
    let src = r#"
proc stale {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    let hit = errs
        .iter()
        .find(|e| e.message.contains("point_size"))
        .unwrap_or_else(|| panic!("no refusal named `point_size`: {errs:?}"));
    assert!(
        hit.message.contains("point_rate"),
        "the refusal does not name the new spelling: {}",
        hit.message
    );
    assert!(
        !hit.message.contains("never declared"),
        "the rename was reported as an undeclared name: {}",
        hit.message
    );
    let hint = hit
        .hint
        .as_deref()
        .unwrap_or_else(|| panic!("the refusal carries no hint: {hit:?}"));
    assert!(
        hint.contains("fraction") && hint.contains("height"),
        "the hint does not say what the unit became: {hint}"
    );
    // **No reference resolution in the hint.** The division is a fact about the
    // file being migrated, not about the language, and a number here would
    // become an anchor authors write against.
    assert!(
        !hint.contains("720"),
        "the hint anchors the unit to a resolution: {hint}"
    );
}

/// The same name met in an expression is refused the same way.
///
/// Reading a stage output was never allowed under either spelling, so the
/// interesting half is which sentence comes back: "does not resolve to a local,
/// a param, an attribute, or an ambient value" would send the author looking
/// for a typo in a name that is not a typo.
#[test]
fn reading_point_size_is_also_refused_with_the_new_spelling() {
    let src = r#"
proc stale_read {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(point_size, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("point_size") && e.message.contains("point_rate")),
        "expected the rename refusal on a read as well: {errs:?}"
    );
}

/// `point_rate` is the required vertex output, under that name.
///
/// The coverage rule did not change with the rename, and this is what says so:
/// a `vertex` block that writes only `clip` is refused, and the diagnostic
/// names `point_rate` rather than the spelling it replaced.
#[test]
fn a_vertex_block_without_point_rate_is_refused_and_the_refusal_names_point_rate() {
    let src = r#"
proc no_rate {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip = vec4(position, 1.0);
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = check_err(src);
    assert!(
        errs.iter()
            .any(|e| e.message.contains("point_rate") && e.message.contains("every path")),
        "expected a coverage diagnostic naming `point_rate`, got: {errs:?}"
    );
}
