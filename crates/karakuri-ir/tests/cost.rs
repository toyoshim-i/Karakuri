//! Cost estimation tests (stage 4).
//!
//! `check.rs` (stages 2-3) is still a stub as of this writing, so there is no
//! `parse` + `check` pipeline yet to hand this module real `Checked` values.
//! Every fixture below is therefore built by hand from the public types in
//! `typed.rs`. Once the check pass lands, these are good candidates to
//! rebuild on top of `parse(src)` + `check(&proc)` instead — real `.kir`
//! source is easier to read than a tree of `TExpr::new` calls — but the
//! assertions (the numbers themselves) should carry over unchanged, since
//! they are hand-computed against `cost.rs`'s documented weights below.

use karakuri_ir::ast::{Attr, BinOp, BlockKind, Kind, Topology, Ty};
use karakuri_ir::builtin::Builtin;
use karakuri_ir::cost::{estimate, MAX_OPS_PER_ELEMENT};
use karakuri_ir::span::Span;
use karakuri_ir::typed::{Checked, TBlock, TExpr, TExprKind, TStmt, Target};
use karakuri_ir::Lit;

// ---------------------------------------------------------------------------
// Builders: small, deliberately unstructured helpers for constructing typed
// trees without going through the parser or the (unimplemented) check pass.
// ---------------------------------------------------------------------------

fn attr_expr(a: Attr) -> TExpr {
    TExpr::new(a.ty(), Span::EMPTY, TExprKind::Attr(a))
}

fn ambient_dt() -> TExpr {
    TExpr::new(
        Ty::Float,
        Span::EMPTY,
        TExprKind::Ambient(karakuri_ir::Ambient::Dt),
    )
}

fn lit_int(v: i32) -> TExpr {
    TExpr::new(Ty::Int, Span::EMPTY, TExprKind::Lit(Lit::Int(v)))
}

fn binary(op: BinOp, ty: Ty, lhs: TExpr, rhs: TExpr) -> TExpr {
    TExpr::new(
        ty,
        Span::EMPTY,
        TExprKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
    )
}

fn call(func: Builtin, ty: Ty, args: Vec<TExpr>) -> TExpr {
    TExpr::new(ty, Span::EMPTY, TExprKind::Builtin { func, args })
}

fn swizzle_x(value: TExpr) -> TExpr {
    TExpr::new(
        Ty::Float,
        Span::EMPTY,
        TExprKind::Swizzle {
            value: Box::new(value),
            components: vec![0],
        },
    )
}

fn let_stmt(name: &str, value: TExpr) -> TStmt {
    TStmt::Let {
        name: name.to_string(),
        value,
        span: Span::EMPTY,
    }
}

fn assign_attr(a: Attr, value: TExpr) -> TStmt {
    TStmt::Assign {
        target: Target::Attr(a),
        value,
        span: Span::EMPTY,
    }
}

fn for_stmt(start: i32, end: i32, body: Vec<TStmt>) -> TStmt {
    TStmt::For {
        var: "i".to_string(),
        start,
        end,
        body,
        span: Span::EMPTY,
    }
}

fn block(kind: BlockKind, stmts: Vec<TStmt>) -> TBlock {
    TBlock {
        kind,
        stmts,
        span: Span::EMPTY,
    }
}

/// A minimal but otherwise-plausible `Checked`. Tests override `emit` and
/// `blocks`; everything else is filler that cost estimation does not look at.
fn checked(emit: Vec<Attr>, blocks: Vec<TBlock>) -> Checked {
    Checked {
        name: "fixture".to_string(),
        kind: Kind::L1,
        topology: Some(Topology::Points),
        capacity: None,
        amplify: None,
        uses: Vec::new(),
        blend: None,
        params: vec![],
        emit,
        consumes: vec![],
        blocks,
        cost: None,
        // Hand-built fixtures: `check` is what decides this, and these never
        // run it. `false` is the conservative side and nothing here reads it.
        closed_form: false,
        reads_beats: false,
        span: Span::EMPTY,
    }
}

// ---------------------------------------------------------------------------
// A trivial procedure costs little.
// ---------------------------------------------------------------------------

#[test]
fn trivial_procedure_costs_little() {
    // element { position = position + velocity * dt; }
    //
    // Every leaf read and every operator/statement is one unit in this
    // module's accounting:
    //   velocity * dt         = 1 (op) + 1 (velocity) + 1 (dt)        = 3
    //   position + (v * dt)   = 1 (op) + 1 (position) + 3             = 5
    //   assign                = 1 (statement) + 5                     = 6
    let value = binary(
        BinOp::Add,
        Ty::Vec3,
        attr_expr(Attr::Position),
        binary(
            BinOp::Mul,
            Ty::Vec3,
            attr_expr(Attr::Velocity),
            ambient_dt(),
        ),
    );
    let c = checked(
        vec![Attr::Position],
        vec![block(
            BlockKind::Element,
            vec![assign_attr(Attr::Position, value)],
        )],
    );

    let cost = estimate(&c).expect("well under the ceiling");
    assert_eq!(cost.ops_per_element, 6);
    assert!(cost.ops_per_element < MAX_OPS_PER_ELEMENT / 100);
}

// ---------------------------------------------------------------------------
// Nested loops multiply rather than add.
// ---------------------------------------------------------------------------

#[test]
fn nested_loops_multiply_the_body_cost() {
    // element {
    //   for i in 0..2 {
    //     for i in 0..3 {
    //       let x = abs(velocity.x);
    //     }
    //   }
    // }
    //
    // Body: velocity.x = 1 (swizzle) + 1 (velocity)        = 2
    //       abs(velocity.x) = 1 (weight) + 2                = 3
    //       let x = 1 + 3                                   = 4
    // Inner loop (3 iterations): 3 * (4 + 1)                = 15
    // Outer loop (2 iterations): 2 * (15 + 1)                = 32
    //
    // A model that *added* iteration counts instead of nesting them would
    // land nowhere near 32 (e.g. flattening to a single 6-iteration loop
    // gives 6 * (4 + 1) = 30, a different number, for a different program) —
    // the exact value pins down that nesting multiplies.
    let body = vec![let_stmt(
        "x",
        call(
            Builtin::Abs,
            Ty::Float,
            vec![swizzle_x(attr_expr(Attr::Velocity))],
        ),
    )];
    let inner = for_stmt(0, 3, body);
    let outer = for_stmt(0, 2, vec![inner]);
    let c = checked(vec![], vec![block(BlockKind::Element, vec![outer])]);

    let cost = estimate(&c).expect("well under the ceiling");
    assert_eq!(cost.ops_per_element, 32);
}

// ---------------------------------------------------------------------------
// An expensive builtin outweighs a cheap one.
// ---------------------------------------------------------------------------

#[test]
fn expensive_builtin_outweighs_cheap_one() {
    // let a = curl(position);   vs.   let a = abs(position.x);
    let curl_proc = checked(
        vec![],
        vec![block(
            BlockKind::Element,
            vec![let_stmt(
                "a",
                call(Builtin::Curl, Ty::Vec3, vec![attr_expr(Attr::Position)]),
            )],
        )],
    );
    let abs_proc = checked(
        vec![],
        vec![block(
            BlockKind::Element,
            vec![let_stmt(
                "a",
                call(
                    Builtin::Abs,
                    Ty::Float,
                    vec![swizzle_x(attr_expr(Attr::Position))],
                ),
            )],
        )],
    );

    let curl_cost = estimate(&curl_proc).expect("under the ceiling");
    let abs_cost = estimate(&abs_proc).expect("under the ceiling");

    // curl is weighted as six `perlin` calls (several noise evaluations, per
    // the task brief); `abs` is the cheapest tier. The gap should be large,
    // not incidental.
    assert!(
        curl_cost.ops_per_element > abs_cost.ops_per_element * 10,
        "curl ({}) should dwarf abs ({})",
        curl_cost.ops_per_element,
        abs_cost.ops_per_element
    );
}

// ---------------------------------------------------------------------------
// `fbm` scales with its octave count.
// ---------------------------------------------------------------------------

#[test]
fn fbm_scales_with_octave_count() {
    // let a = fbm(position, N);
    fn fbm_proc(octaves: i32) -> Checked {
        checked(
            vec![],
            vec![block(
                BlockKind::Element,
                vec![let_stmt(
                    "a",
                    call(
                        Builtin::Fbm,
                        Ty::Float,
                        vec![attr_expr(Attr::Position), lit_int(octaves)],
                    ),
                )],
            )],
        )
    }

    let two = estimate(&fbm_proc(2)).expect("under the ceiling");
    let eight = estimate(&fbm_proc(8)).expect("under the ceiling");

    assert!(
        eight.ops_per_element > two.ops_per_element,
        "more octaves must cost more: 2 octaves = {}, 8 octaves = {}",
        two.ops_per_element,
        eight.ops_per_element
    );
    // fbm is weighted as (perlin weight + 1) per octave, so going from 2 to 8
    // octaves (6 more) adds a fixed amount regardless of the fixed overhead
    // around it (the position/octave-literal reads and the `let`).
    let per_octave = (eight.ops_per_element - two.ops_per_element) / 6;
    assert!(
        per_octave >= 10,
        "fbm should cost noticeably more per octave, got {per_octave}"
    );
}

// ---------------------------------------------------------------------------
// A rejection's message carries both the estimate and the ceiling as numbers.
// ---------------------------------------------------------------------------

#[test]
fn rejection_message_carries_the_estimate_and_the_ceiling() {
    // element {
    //   for i in 0..8 {
    //     for i in 0..8 {
    //       let x = curl(position);
    //     }
    //   }
    // }
    //
    // Body: curl(position) = 96 (weight) + 1 (position) = 97
    //       let x           = 1 + 97                      = 98
    // Inner loop (8): 8 * (98 + 1)                         = 792
    // Outer loop (8): 8 * (792 + 1)                         = 6344
    //
    // 6344 > MAX_OPS_PER_ELEMENT (4096), so this must be rejected, and the
    // message must say both 6344 and 4096 — "over budget" alone would tell a
    // repair prompt nothing about how much to cut.
    let body = vec![let_stmt(
        "x",
        call(Builtin::Curl, Ty::Vec3, vec![attr_expr(Attr::Position)]),
    )];
    let inner = for_stmt(0, 8, body);
    let outer = for_stmt(0, 8, vec![inner]);
    let c = checked(vec![], vec![block(BlockKind::Element, vec![outer])]);

    let errs = estimate(&c).expect_err("6344 ops/element must exceed the 4096 ceiling");
    assert_eq!(errs.len(), 1);
    let message = &errs[0].message;

    assert!(
        message.contains("6344"),
        "message should state the estimate: {message}"
    );
    assert!(
        message.contains(&MAX_OPS_PER_ELEMENT.to_string()),
        "message should state the ceiling: {message}"
    );
    assert!(
        message.contains("curl"),
        "message should name what dominated: {message}"
    );

    // The hint is where the actionable part belongs, per the pipeline's
    // rejection rules — the message states numbers, the hint states what to
    // do about them.
    let hint = errs[0]
        .hint
        .as_ref()
        .expect("a cost rejection should carry a hint");
    assert!(
        hint.contains("curl"),
        "hint should point at the dominant call: {hint}"
    );
}

#[test]
fn rejection_without_a_dominant_builtin_still_names_a_block() {
    // A pathologically deep loop of plain arithmetic, with no builtin call to
    // point at. The message should still carry both numbers and fall back to
    // naming the block that dominated.
    let body = vec![assign_attr(
        Attr::Position,
        binary(
            BinOp::Add,
            Ty::Vec3,
            attr_expr(Attr::Position),
            attr_expr(Attr::Velocity),
        ),
    )];
    let mut loop_stmt = for_stmt(0, 2000, body);
    for _ in 0..3 {
        loop_stmt = for_stmt(0, 2, vec![loop_stmt]);
    }
    let c = checked(vec![], vec![block(BlockKind::Element, vec![loop_stmt])]);

    let errs = estimate(&c).expect_err("deeply nested arithmetic should exceed the ceiling");
    let message = &errs[0].message;
    assert!(
        message.contains(&MAX_OPS_PER_ELEMENT.to_string()),
        "message should state the ceiling: {message}"
    );
    assert!(
        message.contains("element"),
        "message should name the dominant block: {message}"
    );
}

// ---------------------------------------------------------------------------
// The three op counts scale with different things and must not be summed.
// ---------------------------------------------------------------------------

/// A procedure that seeds an expensive initial state and then coasts is a shape
/// worth allowing: `spawn` runs once in an element's life, `element` runs every
/// frame. Charging the two together would reject it for work it does not repeat.
#[test]
fn spawn_cost_is_not_charged_against_the_per_frame_figure() {
    let expensive = call(Builtin::Curl, Ty::Vec3, vec![attr_expr(Attr::Position)]);
    let c = checked(
        vec![Attr::Position],
        vec![
            block(
                BlockKind::Spawn,
                vec![assign_attr(Attr::Position, expensive)],
            ),
            block(
                BlockKind::Element,
                vec![assign_attr(Attr::Position, attr_expr(Attr::Position))],
            ),
        ],
    );

    let cost = estimate(&c).expect("neither figure is over its own ceiling");
    assert!(
        cost.ops_per_spawn > cost.ops_per_element * 10,
        "the expensive block is spawn: element {} spawn {}",
        cost.ops_per_element,
        cost.ops_per_spawn
    );
}

/// Fragment work scales with covered pixels, not with elements, so it is
/// reported and bounded on its own axis.
#[test]
fn fragment_cost_is_reported_separately_from_vertex_cost() {
    let c = checked(
        vec![],
        vec![
            block(
                BlockKind::Vertex,
                vec![let_stmt("a", lit_int(1)), let_stmt("b", lit_int(2))],
            ),
            block(BlockKind::Fragment, vec![let_stmt("c", lit_int(3))]),
        ],
    );

    let cost = estimate(&c).expect("both are trivial");
    assert!(cost.ops_per_element > 0, "vertex work is per element");
    assert!(cost.ops_per_fragment > 0, "fragment work is per fragment");
    assert_ne!(
        cost.ops_per_element, cost.ops_per_fragment,
        "the two blocks differ in size, so their figures must too"
    );
}

/// **An amplifying stage's per-element figure is a product, and this is where
/// the multiplication is charged.**
///
/// A `deform` in a node of factor `n` runs `n` times for each element that
/// reaches it, so its cost is `n` times its block cost — otherwise a stage of
/// factor 64 running an expensive body sails through the ceiling at a
/// sixty-fourth of what it really costs, which is the one thing this ceiling
/// exists to prevent.
///
/// Measured against the same procedure with no declaration rather than against
/// a constant, so the assertion stays true when the op weights change.
#[test]
fn an_amplifying_l2s_per_element_cost_is_multiplied_by_its_factor() {
    let body = vec![block(
        BlockKind::Deform,
        vec![let_stmt("a", lit_int(1)), let_stmt("b", lit_int(2))],
    )];
    let mut plain = checked(vec![], body.clone());
    plain.kind = Kind::L2;
    plain.topology = None;

    let mut amplified = plain.clone();
    amplified.amplify = Some(8);

    let plain_cost = estimate(&plain).expect("a trivial deform");
    let amplified_cost = estimate(&amplified).expect("a trivial deform, eight times");

    assert!(
        plain_cost.ops_per_element > 0,
        "the deform block is per element"
    );
    assert_eq!(
        amplified_cost.ops_per_element,
        plain_cost.ops_per_element * 8,
        "eight copies of a body is eight times the body"
    );
    assert_eq!(
        amplified_cost.ops_per_spawn, plain_cost.ops_per_spawn,
        "an L2 does not spawn, and amplification does not give it a way to"
    );
}

/// **A field's cost is on its own axis, and the three beside it stay zero.**
///
/// What a field scales with is *how often its caller calls it* — once per
/// element in a `vertex`, forty-eight times in a march loop — which is a
/// property of the caller. Charging it to `ops_per_element` would put a rate
/// against a quantity a field does not have, and would give it a ceiling that
/// says nothing about what evaluating it costs anybody.
#[test]
fn a_fields_cost_is_per_evaluation_and_not_per_element() {
    let mut c = checked(
        vec![],
        vec![block(
            BlockKind::Field,
            vec![let_stmt("a", lit_int(1)), let_stmt("b", lit_int(2))],
        )],
    );
    c.kind = Kind::Field;
    c.topology = None;

    let cost = estimate(&c).expect("a trivial field");
    assert!(
        cost.ops_per_evaluation > 0,
        "the field block is per evaluation"
    );
    assert_eq!(cost.ops_per_element, 0, "a field has no elements");
    assert_eq!(cost.ops_per_spawn, 0, "and nothing to spawn");
    assert_eq!(cost.ops_per_fragment, 0, "and covers no pixels");
}
