use super::common::*;

// ---------------------------------------------------------------------------
// The three canonical examples must check clean with the expected shape.
// ---------------------------------------------------------------------------

#[test]
fn drift_shell_checks_clean_with_expected_shape() {
    let src = include_str!("../fixtures/drift_shell.kir");
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
    let src = include_str!("../fixtures/soft_points.kir");
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
    let src = include_str!("../fixtures/var_accum.kir");
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
