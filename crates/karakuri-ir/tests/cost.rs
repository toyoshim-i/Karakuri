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
use karakuri_ir::cost::{
    estimate, MAX_OPS_PER_ELEMENT, MAX_OPS_PER_FRAGMENT, MAX_OPS_PER_FULLSCREEN_FRAGMENT,
};
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
        retains: false,
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

// ---------------------------------------------------------------------------
// L5 — one axis, and the fullscreen ceiling without a topology
// ---------------------------------------------------------------------------

fn check_l5(src: &str) -> Checked {
    let proc = karakuri_ir::parse::parse(src)
        .unwrap_or_else(|errs| panic!("expected this fixture to parse: {errs:?}"));
    karakuri_ir::check::check(&proc)
        .unwrap_or_else(|errs| panic!("expected this fixture to check clean: {errs:?}"))
}

/// **An L5 is priced on `ops_per_fragment` and is zero on the other two axes.**
///
/// It has no element and no spawn, so there is nothing for the other rates to
/// be rates *against* — the `Field`'s shape rather than a new one, and
/// [ADR-0013]'s three axes still never summed.
#[test]
fn an_l5_is_priced_on_one_axis_and_zero_on_the_others() {
    let checked = check_l5(
        r#"
proc dim {
  kind L5

  param amount : float [0.0, 1.0] = 0.5

  frame {
    let s = texel(src);
    color = vec4(s.xyz * amount, s.w);
  }
}
"#,
    );
    let cost = estimate(&checked).expect("well under the ceiling");
    assert!(cost.ops_per_fragment > 0, "the frame block is charged");
    assert_eq!(cost.ops_per_element, 0, "an L5 has no element");
    assert_eq!(cost.ops_per_spawn, 0, "an L5 spawns nothing");
    assert_eq!(cost.ops_per_evaluation, 0, "an L5 is not a field");
}

/// **The ceiling is the fullscreen one, and an L5 never declares a topology.**
///
/// `fragment_ceiling` used to switch on `topology == Fullscreen` alone, which
/// held every L5 to `MAX_OPS_PER_FRAGMENT` — the per-element stand-in for
/// `capacity` sprites times their area times whatever they overlap, applied to
/// a pass that covers the frame exactly once with nothing overdrawing.
///
/// The assertion is the gap between the two numbers: a body priced between 512
/// and 4096 is accepted, and the same body would have been refused under the
/// tighter one.
#[test]
fn an_l5_is_held_to_the_fullscreen_ceiling_without_declaring_one() {
    // Forty-eight taps: comfortably over `MAX_OPS_PER_FRAGMENT`'s 512 and
    // comfortably under `MAX_OPS_PER_FULLSCREEN_FRAGMENT`'s 4096.
    let checked = check_l5(
        r#"
proc smear {
  kind L5

  param amount : float [0.0, 1.0] = 0.5

  frame {
    let d = frame_step(0.004 * amount);
    var sum = vec4(0.0);
    for i in 0..48 {
      sum = sum + tap(src, point_coord + vec2(float(i) * d.x, 0.0));
    }
    color = sum * 0.0416;
  }
}
"#,
    );
    assert_eq!(checked.topology, None, "an L5 declares no topology");
    let cost = estimate(&checked).expect("under the fullscreen ceiling");
    assert!(
        cost.ops_per_fragment > MAX_OPS_PER_FRAGMENT,
        "this fixture has to be over the per-element ceiling to mean anything: {} <= {}",
        cost.ops_per_fragment,
        MAX_OPS_PER_FRAGMENT
    );
    assert!(
        cost.ops_per_fragment <= MAX_OPS_PER_FULLSCREEN_FRAGMENT,
        "and under the fullscreen one: {} > {}",
        cost.ops_per_fragment,
        MAX_OPS_PER_FULLSCREEN_FRAGMENT
    );
}

/// **A refusal carries the estimate, the ceiling and the dominant term** —
/// [ADR-0012](../../../docs/adr/0012-one-severity-and-a-rejection-carries-numbers.md),
/// and "over budget" tells a repair prompt nothing about how much to cut.
#[test]
fn an_l5_over_the_ceiling_is_refused_with_numbers() {
    // The 9x9 kernel of `examples/bloom.kir` widened to 13x13 — 169 taps where
    // 81 fit. Nothing about it is subtle; what the test is about is what the
    // refusal says.
    let checked = check_l5(
        r#"
proc too_much {
  kind L5

  param amount : float [0.0, 4.0] = 1.0

  frame {
    let d    = frame_step(0.003);
    let knee = vec3(1.0);
    let zero = vec3(0.0);

    var sum = vec3(0.0);
    for i in -6..7 {
      let fi = float(i);
      let wi = exp(fi * fi * -0.125);
      let ox = fi * d.x;
      for j in -6..7 {
        let fj = float(j);
        let w  = wi * exp(fj * fj * -0.125);
        let s  = tap(src, point_coord + vec2(ox, fj * d.y));
        sum = sum + w * max(s.xyz - knee, zero);
      }
    }

    let c = texel(src);
    color = vec4(c.xyz + amount * sum, c.w);
  }
}
"#,
    );
    let errs = estimate(&checked).expect_err("169 taps is over the fullscreen ceiling");
    let e = &errs[0];
    assert!(
        e.message.contains("ops/fragment"),
        "the unit has to be the axis it was measured on: {}",
        e.message
    );
    assert!(
        e.message.contains(&format!(
            "{MAX_OPS_PER_FULLSCREEN_FRAGMENT} ops/fragment ceiling"
        )),
        "the ceiling has to be the fullscreen one, as a number: {}",
        e.message
    );
    // The estimate itself, as a number rather than as a verdict.
    let estimate_figure: u64 = e
        .message
        .split_whitespace()
        .next()
        .and_then(|w| w.parse().ok())
        .unwrap_or_else(|| panic!("the message has to open with the estimate: {}", e.message));
    assert!(
        estimate_figure > MAX_OPS_PER_FULLSCREEN_FRAGMENT,
        "the estimate has to be the figure that failed: {}",
        e.message
    );
    // And what dominated, so a regeneration has something to aim at.
    assert!(
        e.message.contains("dominated by") && e.message.contains("`frame`"),
        "the refusal has to name the block and the term: {}",
        e.message
    );
    assert!(
        e.hint.as_deref().unwrap_or_default().contains("loop"),
        "the hint has to point at what to cut: {:?}",
        e.hint
    );
}

/// **The three shipped procedures fit**, and the figures are recorded here
/// rather than left to be rediscovered.
///
/// `bloom` is the one worth watching: 81 filtered taps against a ceiling
/// calibrated for a raymarcher. `docs/adr/0340-…` owes it a measurement, and
/// this assertion is a band rather than an equality so that a weight changing
/// by one does not fail a test about whether a shipped part is affordable.
#[test]
fn the_three_shipped_procedures_are_within_the_fullscreen_ceiling() {
    for (name, floor, ceiling) in [
        ("feedback", 0u64, 64u64),
        ("rgb_shift", 0, 128),
        // Deliberately narrow on the high side: this is the figure the ADR
        // wants measured, and a silent drift towards 4096 is what the band is
        // watching for.
        ("bloom", 3000, 4000),
    ] {
        let src = std::fs::read_to_string(format!("../../examples/{name}.kir"))
            .unwrap_or_else(|e| panic!("examples/{name}.kir: {e}"));
        let checked = check_l5(&src);
        assert_eq!(checked.kind, Kind::L5, "{name}");
        let cost = estimate(&checked)
            .unwrap_or_else(|errs| panic!("{name} is over the ceiling: {errs:?}"));
        assert!(
            cost.ops_per_fragment > floor && cost.ops_per_fragment < ceiling,
            "{name} is {} ops/fragment, expected {floor}..{ceiling} \
             (against a ceiling of {MAX_OPS_PER_FULLSCREEN_FRAGMENT})",
            cost.ops_per_fragment
        );
    }
}
