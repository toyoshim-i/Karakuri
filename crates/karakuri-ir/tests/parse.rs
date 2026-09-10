//! Parser tests: the three complete `.kir` examples from `docs/ir-spec.md`
//! parsed to their expected shape, plus a battery of invalid fixtures
//! exercising error recovery.

use karakuri_ir::{
    ast::{Attr, BinOp, BlockKind, Expr, Kind, Stmt, Topology, Ty},
    parse::parse,
};

/// A short tag for a statement's kind, so a whole statement list can be
/// asserted against a `Vec<&str>` shape in one line.
fn stmt_tag(s: &Stmt) -> &'static str {
    match s {
        Stmt::Let { .. } => "let",
        Stmt::Var { .. } => "var",
        Stmt::Assign { .. } => "assign",
        Stmt::If { .. } => "if",
        Stmt::For { .. } => "for",
        Stmt::Kill { .. } => "kill",
    }
}

fn stmt_tags(stmts: &[Stmt]) -> Vec<&'static str> {
    stmts.iter().map(stmt_tag).collect()
}

// ---------------------------------------------------------------------------
// drift_shell (docs/ir-spec.md, L1 example)
// ---------------------------------------------------------------------------

#[test]
fn drift_shell_parses_to_expected_shape() {
    let src = include_str!("fixtures/drift_shell.kir");
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "expected drift_shell.kir to parse, got errors:\n{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });

    assert_eq!(proc.name, "drift_shell");
    assert_eq!(proc.kind, Kind::L1);
    assert_eq!(proc.topology, Some(Topology::Points));

    let cap = proc.capacity.expect("capacity declared");
    assert_eq!(cap.min, 65536);
    assert_eq!(cap.max, 1048576);
    assert_eq!(cap.default, 262144);

    assert_eq!(proc.blend, None);

    let names: Vec<&str> = proc.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["spawn_rate", "radius", "turbulence", "lifetime"]
    );
    for p in &proc.params {
        assert_eq!(p.ty, Ty::Float);
    }
    let radius = proc.param("radius").unwrap();
    assert_eq!(radius.min, 0.1);
    assert_eq!(radius.max, 8.0);
    assert!(matches!(radius.default, Expr::Lit { .. }));

    let emit: Vec<Attr> = proc.emit.iter().map(|(a, _)| *a).collect();
    assert_eq!(emit, vec![Attr::Position, Attr::Velocity, Attr::Age]);
    assert!(proc.consumes.is_empty());

    assert_eq!(proc.blocks.len(), 2);

    let spawn = proc.block(BlockKind::Spawn).expect("spawn block");
    assert_eq!(
        stmt_tags(&spawn.stmts),
        vec!["let", "let", "assign", "assign", "assign"]
    );

    let element = proc.block(BlockKind::Element).expect("element block");
    assert_eq!(
        stmt_tags(&element.stmts),
        vec!["let", "let", "assign", "assign", "assign", "if"]
    );
    match &element.stmts[5] {
        Stmt::If {
            cond, then, els, ..
        } => {
            assert!(matches!(cond, Expr::Binary { op: BinOp::Gt, .. }));
            assert_eq!(stmt_tags(then), vec!["kill"]);
            assert!(els.is_empty());
        }
        other => panic!("expected an `if`, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// soft_points (docs/ir-spec.md, L4 example)
// ---------------------------------------------------------------------------

#[test]
fn soft_points_parses_to_expected_shape() {
    let src = include_str!("fixtures/soft_points.kir");
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "expected soft_points.kir to parse, got errors:\n{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });

    assert_eq!(proc.name, "soft_points");
    assert_eq!(proc.kind, Kind::L4);
    assert_eq!(proc.topology, None);
    assert!(proc.capacity.is_none());
    assert!(matches!(
        proc.blend,
        Some(karakuri_ir::ast::Blend::Additive)
    ));

    let consumes: Vec<Attr> = proc.consumes.iter().map(|(a, _)| *a).collect();
    assert_eq!(consumes, vec![Attr::Position, Attr::Velocity, Attr::Age]);
    assert!(proc.emit.is_empty());

    let names: Vec<&str> = proc.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["point_scale", "hue", "exposure", "falloff"]);

    assert_eq!(proc.blocks.len(), 2);

    let vertex = proc.block(BlockKind::Vertex).expect("vertex block");
    assert_eq!(stmt_tags(&vertex.stmts), vec!["assign", "assign"]);
    match &vertex.stmts[0] {
        Stmt::Assign { target, op, .. } => {
            assert_eq!(target, "clip");
            assert_eq!(*op, None);
        }
        other => panic!("expected an assignment, got {other:?}"),
    }

    let fragment = proc.block(BlockKind::Fragment).expect("fragment block");
    assert_eq!(
        stmt_tags(&fragment.stmts),
        vec!["let", "let", "let", "assign"]
    );
    match &fragment.stmts[3] {
        Stmt::Assign { target, .. } => assert_eq!(target, "color"),
        other => panic!("expected an assignment, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// var / for accumulation example (docs/ir-spec.md, "Statements and expressions")
// ---------------------------------------------------------------------------

#[test]
fn var_accum_parses_to_expected_shape() {
    let src = include_str!("fixtures/var_accum.kir");
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "expected var_accum.kir to parse, got errors:\n{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });

    assert_eq!(proc.kind, Kind::L1);
    assert_eq!(proc.blocks.len(), 1);

    let element = proc.block(BlockKind::Element).expect("element block");
    assert_eq!(
        stmt_tags(&element.stmts),
        vec!["var", "for", "let", "assign", "assign"]
    );

    match &element.stmts[0] {
        Stmt::Var { name, value, .. } => {
            assert_eq!(name, "flow");
            assert!(matches!(value, Expr::Call { name, .. } if name == "vec3"));
        }
        other => panic!("expected a `var`, got {other:?}"),
    }

    match &element.stmts[1] {
        Stmt::For {
            var,
            start,
            end,
            body,
            ..
        } => {
            assert_eq!(var, "i");
            assert_eq!(*start, 0);
            assert_eq!(*end, 4);
            assert_eq!(stmt_tags(body), vec!["let", "assign"]);
            match &body[1] {
                Stmt::Assign { target, op, .. } => {
                    assert_eq!(target, "flow");
                    assert_eq!(*op, Some(BinOp::Add));
                }
                other => panic!("expected a compound assignment, got {other:?}"),
            }
        }
        other => panic!("expected a `for`, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Invalid fixtures
// ---------------------------------------------------------------------------

#[test]
fn param_without_range_is_rejected() {
    let src = include_str!("fixtures/invalid_param_no_range.kir");
    let errs = parse(src).expect_err("a `param` with no range must not parse");
    assert!(
        errs.iter().any(|e| e.message.contains("range")),
        "expected a diagnostic mentioning the missing range, got: {errs:?}"
    );
}

#[test]
fn while_is_rejected() {
    let src = include_str!("fixtures/invalid_while.kir");
    let errs = parse(src).expect_err("`while` does not exist in the language");
    assert!(!errs.is_empty());
}

#[test]
fn non_constant_for_bound_is_rejected() {
    let src = include_str!("fixtures/invalid_for_bound.kir");
    let errs = parse(src).expect_err("a `for` bound that isn't a literal integer must be rejected");
    assert!(
        errs.iter().any(|e| e.message.contains("constant")),
        "expected a diagnostic about non-constant bounds, got: {errs:?}"
    );
}

#[test]
fn unterminated_block_is_rejected() {
    let src = include_str!("fixtures/invalid_unterminated_block.kir");
    let errs = parse(src).expect_err("a block missing its closing `}` must be rejected");
    assert!(
        errs.iter().any(|e| e.message.contains("unterminated")),
        "expected a diagnostic about the unterminated block, got: {errs:?}"
    );
}

#[test]
fn malformed_swizzle_is_rejected() {
    let src = include_str!("fixtures/invalid_swizzle.kir");
    let errs = parse(src).expect_err("`position.` with no components must be rejected");
    assert!(
        errs.iter().any(|e| e.message.contains("swizzle")),
        "expected a diagnostic about the malformed swizzle, got: {errs:?}"
    );
}

/// One file, four independent mistakes: an unknown `kind`, a `param` with no
/// range, `id` declared in `emit`, and a malformed swizzle. A repair prompt
/// needs all four in one pass, so the parser must not stop at the first.
#[test]
fn multiple_independent_errors_are_all_reported() {
    let src = include_str!("fixtures/invalid_multi.kir");
    let errs = parse(src).expect_err("this file has several syntax errors");
    assert_eq!(
        errs.len(),
        4,
        "expected exactly 4 diagnostics, got {}:\n{}",
        errs.len(),
        errs.iter()
            .map(|e| e.render(src))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let lines: Vec<u32> = errs
        .iter()
        .map(|e| karakuri_ir::span::line_col(src, e.span.start).line)
        .collect();
    assert_eq!(
        lines,
        vec![2, 6, 8, 11],
        "errors landed on unexpected lines"
    );
}

/// A signal-bus name is not a reserved word. Nothing stops a procedure from
/// declaring `param energy`, and once it has, reading `energy` is ordinary and
/// legal — whether a bare name resolves is a question about declarations, which
/// only name resolution can answer. The parser must not guess.
#[test]
fn a_param_may_be_named_after_a_signal() {
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
    parse(src).expect("a param named after a signal is legal");
}

/// `id` is different: it is reserved rather than merely undeclared, so the
/// parser is entitled to reject it, and doing so here puts the correction in
/// front of a repair prompt a stage earlier than name resolution would.
#[test]
fn reading_id_is_rejected_with_a_correction() {
    let src = r#"
proc uses_id {
  kind     L1
  topology points
  capacity [1024, 4096] = 2048

  emit position

  element {
    position = position * hash1(id);
  }
}
"#;
    let errs = parse(src).expect_err("`id` does not exist");
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0].hint.as_deref().unwrap_or_default().contains("seed"),
        "the diagnostic must name the replacement, not just the absence"
    );
}

/// `topology lines` is a distinct value and not a spelling of `points`.
///
/// Paired with the `points` assertion in
/// `drift_shell_parses_with_expected_shape`: a parser that mapped every
/// topology name to one variant would satisfy either test alone.
#[test]
fn topology_lines_parses_as_lines() {
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
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    assert_eq!(proc.topology, Some(Topology::Lines));
}

/// An unknown topology is still refused, and the hint says what the language
/// does have. This is the diagnostic a model reads when it guesses a name —
/// `strips`, `triangles` — so what it lists is the vocabulary it will try next.
#[test]
fn an_unknown_topology_is_refused_and_names_the_ones_that_exist() {
    let src = r#"
proc guessed {
  kind     L1
  topology triangles
  capacity [1, 64] = 8

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;
    let errs = parse(src).expect_err("`triangles` is not a topology");
    let hints: String = errs.iter().filter_map(|e| e.hint.clone()).collect();
    assert!(
        hints.contains("points") && hints.contains("lines"),
        "hint was: {hints}"
    );
}

/// `blend weighted` is a distinct value and not a spelling of `additive`.
///
/// The mirror of `topology_lines_parses_as_lines`, and it exists for the same
/// reason: a parser that mapped every blend name to one variant would satisfy
/// `soft_points_parses_to_expected_shape` alone.
#[test]
fn blend_weighted_parses_as_weighted() {
    let src = r#"
proc glassy {
  kind  L4
  blend weighted

  consumes position

  vertex {
    clip = camera * vec4(position, 1.0);
    point_rate = 0.03;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 0.5);
  }
}
"#;
    let proc = parse(src).unwrap_or_else(|errs| {
        panic!(
            "{}",
            errs.iter()
                .map(|e| e.render(src))
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    assert_eq!(proc.blend, Some(karakuri_ir::ast::Blend::Weighted));
}

/// An unknown blend mode is refused, and the hint names both of the ones that
/// exist *and* what separates them.
///
/// Same argument as `an_unknown_topology_is_refused_and_names_the_ones_that_exist`:
/// this is the diagnostic a model reads after guessing `over` or `screen` from
/// the L5 vocabulary, where those names are real. What the hint lists is what it
/// tries next, so listing the names without the distinction would send it back
/// with a coin flip.
#[test]
fn an_unknown_blend_is_refused_and_names_the_ones_that_exist() {
    let src = r#"
proc guessed {
  kind  L4
  blend screen

  consumes position

  vertex {
    clip = camera * vec4(position, 1.0);
    point_rate = 0.03;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
    let errs = parse(src).expect_err("`screen` is not an L4 blend mode");
    let hints: String = errs.iter().filter_map(|e| e.hint.clone()).collect();
    assert!(
        hints.contains("additive") && hints.contains("weighted") && hints.contains("opacity"),
        "hint was: {hints}"
    );
}

// ---------------------------------------------------------------------------
// kind L5 — the grammar, which is two words and a block
// ---------------------------------------------------------------------------

/// **The whole of what the sixth kind adds to the grammar**: `L5` as a `kind`
/// value, `retains` as a bare header word, `frame` as a block, and `Texture` as
/// a `uses` type. Nothing else in the language moves.
#[test]
fn an_l5_header_parses_to_its_four_new_words() {
    let src = r#"
proc over {
  kind L5
  retains

  param amount : float [0.0, 1.0] = 0.25

  uses under : Texture

  frame {
    let s = texel(src);
    color = vec4(s.xyz, s.w);
  }
}
"#;
    let proc = parse(src).expect("this is the grammar");
    assert_eq!(proc.kind, Kind::L5);
    assert!(proc.retains.is_some(), "`retains` takes no operand");
    assert_eq!(proc.uses.len(), 1);
    assert_eq!(proc.uses[0].name, "under");
    assert_eq!(proc.uses[0].ty, karakuri_ir::ast::SlotTy::Texture);
    assert!(proc.block(BlockKind::Frame).is_some());
    // Declared nowhere and readable in the block: `src` is an ordinary
    // identifier as far as the parser is concerned, and what it *means* is the
    // check pass's.
    assert_eq!(
        stmt_tags(&proc.block(BlockKind::Frame).unwrap().stmts),
        vec!["let", "assign"]
    );
}

/// **`retains` carries no operand**, so a file that writes one is a file with a
/// stray word after the declaration — refused where it is written rather than
/// read as something.
///
/// The cut is the slot's answer, and putting it here would be two procedures
/// where there is one.
#[test]
fn retains_with_an_operand_is_refused() {
    let src = r#"
proc echo {
  kind L5
  retains mix

  frame {
    color = texel(src);
  }
}
"#;
    let errs = parse(src).expect_err("`retains mix` is not the grammar");
    assert!(
        errs.iter()
            .any(|e| e.message.contains("expected a header declaration or block")),
        "{errs:?}"
    );
}

/// An unknown kind names every one that exists, and the sixth is in the list.
#[test]
fn an_unknown_kind_names_l5_among_the_ones_that_exist() {
    let errs = parse("proc x {\n  kind L9\n}\n").expect_err("`L9` is not a kind");
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("`L5` (a frame effect"),
        "the hint has to name the sixth kind: {hint}"
    );
}

/// An unknown `uses` type names every one that exists, and `Texture` is in the
/// list.
#[test]
fn an_unknown_slot_type_names_texture_among_the_ones_that_exist() {
    let errs = parse("proc x {\n  kind L5\n  uses back : Picture\n}\n")
        .expect_err("`Picture` is not a slot type");
    let hint = errs.iter().find_map(|e| e.hint.clone()).unwrap_or_default();
    assert!(
        hint.contains("`Texture`, a picture an L5 folds in"),
        "the hint has to name the fifth slot type: {hint}"
    );
}
