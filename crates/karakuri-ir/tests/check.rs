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
    // Was `is_none()` while `topology` was an L1 field with no L4 counterpart.
    // An L4's is now inferred from whether `vertex` writes a second endpoint,
    // and this one does not — see `an_l4_that_assigns_clip_b_is_inferred_to_draw_lines`.
    assert_eq!(checked.topology, Some(Topology::Points));
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
/// `emit` has no meaning on an L4 procedure — only L1 attributes get buffers —
/// and nothing rejects one, so a generated file can carry a stray `emit` that
/// duplicates its `consumes`. Classifying by "reads an attribute in `emit`"
/// alone then calls a stateless procedure accumulating, and since a Set is
/// closed form only when both of its procedures are, one meaningless line in an
/// L4 file makes a whole seekable Set look like it needs priming. The property
/// is about per-element state and L4 has none.
#[test]
fn an_l4_that_declares_emit_is_still_vacuously_closed_form() {
    let src = r#"
proc odd_l4 {
  kind  L4
  blend additive

  emit     position
  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
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
         vacuously true — a stray `emit` on a stateless procedure made the whole \
         Set look like it needed priming"
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
    point_size = 2.0;
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
    point_size = 2.0;
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
    point_size = 2.0;
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
    point_size = 2.0;
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
    point_size = 2.0;
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
    point_size = 4.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    for src in [l1, l3, l4] {
        let errs = check_err(src);
        assert!(
            errs.iter().any(|e| e.message.contains("`amplify` is L2 only")),
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
            errs.iter().any(|e| e.message.contains("at least 2") && e.message.contains(expected)),
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
    point_size = 4.0;
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
    point_size = 1.0;
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
    point_size = 2.0;
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("velocity"), "{rendered}");
    assert!(rendered.contains("consumes"), "the hint does not offer `consumes`: {rendered}");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
        let rendered = errs.iter().map(|e| e.render(&src)).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains(field), "`{field}` was not named: {rendered}");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
        let rendered = errs.iter().map(|e| e.render(&src)).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains(missing), "`{missing}` was not required: {rendered}");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("consume"), "{rendered}");
    assert!(rendered.contains("ir-spec"), "the hint does not point at where this is decided: {rendered}");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
        let rendered = errs.iter().map(|e| e.render(&src)).collect::<Vec<_>>().join("\n");
        assert!(rendered.contains(field), "`{field}` was not named: {rendered}");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("camera"), "{rendered}");
    assert!(rendered.contains("L3"), "the diagnostic does not say where it belongs: {rendered}");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
        let rendered = errs.iter().map(|e| e.render(&src)).collect::<Vec<_>>().join("\n");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
    point_size = 4.0;
  }}
  fragment {{
    color = vec4({name}, 1.0);
  }}
}}
"#
        );
        let errs = check_err(&src);
        let rendered = errs.iter().map(|e| e.render(&src)).collect::<Vec<_>>().join("\n");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("position"), "{rendered}");
    assert!(rendered.contains("deform"), "the hint does not offer the block that can: {rendered}");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
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
    let rendered = errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n");
    assert!(rendered.contains("mask"), "{rendered}");
    assert!(rendered.contains("L2"), "the diagnostic does not say where it belongs: {rendered}");
}
