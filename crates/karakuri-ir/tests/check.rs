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
