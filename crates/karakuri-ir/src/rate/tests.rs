use super::*;

/// One L4 around `body`, checked, so every case below asks the analysis about a
/// procedure the checker has already accepted — a `point_rate` expression that
/// does not type is not a case this module owes an answer for.
fn renderer(params: &str, body: &str) -> Checked {
    let src = source(params, body);
    let proc = crate::parse(&src).expect("parse");
    crate::check::check(&proc).expect("check")
}

fn source(params: &str, body: &str) -> String {
    format!(
        r#"
proc bounded {{
  kind  L4
  blend additive

  consumes position, size

{params}

  vertex {{
    let cp = camera * vec4(position, 1.0);
    clip = cp;
{body}
  }}

  fragment {{
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }}
}}
"#
    )
}

fn bound(params: &str, body: &str) -> Bound {
    point_rate_bound(&renderer(params, body)).bound
}

fn at_least(params: &str, body: &str) -> f32 {
    match bound(params, body) {
        Bound::AtLeast { rate, .. } => rate,
        other => panic!("expected a bound, got {other:?}"),
    }
}

fn over(params: &str, body: &str) -> Vec<String> {
    match bound(params, body) {
        Bound::AtLeast { over, .. } => over,
        other => panic!("expected a bound, got {other:?}"),
    }
}

/// A rate that is a literal is its own bound, exactly.
#[test]
fn a_constant_rate_bounds_to_itself() {
    assert_eq!(at_least("", "point_rate = 0.004;"), 0.004);
}

/// The declared minimum, not the default. A param is a fader's range, so the
/// value the file opens at says nothing about where it will be a minute later —
/// and a floor read off the default would be wrong the first time anybody
/// turned the knob down.
#[test]
fn a_param_bounds_at_its_declaration_rather_than_its_default() {
    let rate = at_least(
        "  param point_scale : float [0.00069, 0.0556] = 0.00556",
        "point_rate = point_scale;",
    );
    assert_eq!(rate, 0.00069);
}

/// A `clamp` with constant ends bounds whatever it is given, which is how
/// `sheet_shade` states a floor over a division by a clip-space `w` nothing
/// here can bound.
#[test]
fn a_clamp_with_constant_ends_bounds_an_unbounded_expression() {
    let rate = at_least(
        "  param point_scale : float [0.00069, 0.0556] = 0.00417",
        "point_rate = clamp(point_scale * 4.0 / max(cp.w, 0.5), 0.00139, 0.0333);",
    );
    assert_eq!(rate, 0.00139);
}

/// `max` recovers a bound from an unbounded operand, which is what
/// `star_flares` does with an attribute: `max(size, 1.0)` is at least one
/// however the simulation left `size`.
#[test]
fn max_against_a_constant_recovers_a_bound_from_an_attribute() {
    let rate = at_least(
        "  param point_scale : float [0.0014, 0.0417] = 0.0076",
        "point_rate = point_scale * max(size, 1.0);",
    );
    assert_eq!(rate, 0.0014);
}

/// An attribute has no declared range, and the answer is a refusal. `hard_dots`
/// ships exactly this — `dot_scale * size` — so this is the shipped case rather
/// than a constructed one.
#[test]
fn an_attribute_with_nothing_guarding_it_is_refused() {
    match bound(
        "  param dot_scale : float [0.00069, 0.0833] = 0.0125",
        "point_rate = dot_scale * size;",
    ) {
        Bound::Unbounded { lower, .. } => {
            assert_eq!(lower, f32::NEG_INFINITY, "nothing bounded it below");
        }
        other => panic!("an attribute is not bounded, got {other:?}"),
    }
}

/// A rate that reaches zero is refused, and the reason is distinguishable from
/// the one above. There is no height at which a zero-rate primitive is a pixel
/// across, so this is not a floor that is merely large.
#[test]
fn a_rate_that_reaches_zero_is_refused_and_says_it_proved_zero() {
    match bound(
        "  param point_scale : float [0.00069, 0.0556] = 0.0111\n  \
             param width_var   : float [0.0, 1.0]  = 0.45",
        "point_rate = point_scale * (1.0 - width_var + width_var * hash1(seed) * 2.0);",
    ) {
        Bound::Unbounded { lower, .. } => assert_eq!(lower, 0.0),
        other => panic!("this rate reaches zero, got {other:?}"),
    }
}

/// The refusal points at the assignment that defeated it, not at the first one
/// or at the block. A vertex block writing three rates and failing on the third
/// has to say which, or the reader is left grepping.
#[test]
fn the_refusal_names_the_assignment_that_defeated_it() {
    let params = "  param width_var : float [0.0, 1.0] = 0.45";
    let body = "if seed < 100u {\n      point_rate = 0.01;\n    } else {\n      \
                    point_rate = 0.02 * (1.0 - width_var);\n    }";
    let checked = renderer(params, body);
    // The source `renderer` built, so the span can be read back against it.
    let src = source(params, body);
    match point_rate_bound(&checked).bound {
        Bound::Unbounded { at, lower } => {
            assert_eq!(lower, 0.0);
            // The parser's span for a parenthesised operand stops short of
            // the closing bracket, which is not this module's to fix.
            assert_eq!(at.text(&src), "0.02 * (1.0 - width_var");
        }
        other => panic!("the second arm reaches zero, got {other:?}"),
    }
}

/// Every path counts and the least of them wins. Which arm an element takes is
/// per-element and undecidable here, so the rate the Set can emit is either —
/// and the floor has to hold for both.
#[test]
fn both_arms_of_a_branch_are_bounded_and_the_smaller_holds() {
    let rate = at_least(
        "",
        "if seed < 100u {\n      point_rate = 0.01;\n    } else {\n      \
             point_rate = 0.002;\n    }",
    );
    assert_eq!(rate, 0.002);
}

/// A bound says which declarations it rests on, and names no others. Nothing
/// clamps a write to a declared range, so a caller holding the values needs to
/// know which ones the floor depends on — `P-0095` at the remove a static bound
/// sits at.
#[test]
fn a_bound_names_the_declarations_it_rests_on() {
    let names = over(
        "  param point_scale : float [0.0014, 0.0417] = 0.0076\n  \
             param exposure    : float [0.0, 8.0] = 1.0",
        "point_rate = point_scale * max(size, 1.0);",
    );
    assert_eq!(names, vec!["point_scale".to_string()]);
    assert!(over("", "point_rate = 0.004;").is_empty());
}

/// A procedure with no `vertex` block emits no rate, which is not a bound of
/// zero: there is no primitive that can fall under a pixel. It is the same
/// absence `check` infers `Topology::Fullscreen` from.
#[test]
fn a_fullscreen_procedure_has_no_primitive_rather_than_no_bound() {
    let src = r#"
proc wash {
  kind  L4
  blend additive

  param level : float [0.0, 1.0] = 0.2

  fragment {
    color = vec4(level, level, level, 1.0);
  }
}
"#;
    let proc = crate::parse(src).expect("parse");
    let checked = crate::check::check(&proc).expect("check");
    assert_eq!(point_rate_bound(&checked).bound, Bound::NoPrimitive);
    assert_eq!(checked.topology, Some(crate::ast::Topology::Fullscreen));
}

/// A `uint` is bounded by being a `uint`, and that is the whole of why this
/// expression has a floor: `source` is an identity nothing declares a range
/// for, `% 3u` puts it in `0..=2`, and the `+ 1.0` lifts it clear of zero.
///
/// The expression is the corpus's, verbatim from `SOURCE_L4` in
/// `crates/karakuri-ir/tests/check.rs` — the one computed `point_rate` in that
/// file, every other being a literal.
#[test]
fn an_integer_remainder_bounds_a_value_nothing_declared() {
    let src = r#"
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
    let proc = crate::parse(src).expect("parse");
    let checked = crate::check::check(&proc).expect("check");
    match point_rate_bound(&checked).bound {
        Bound::AtLeast { rate, over } => {
            assert_eq!(rate, 1.0);
            assert!(over.is_empty(), "no param is involved");
        }
        other => panic!("expected a bound, got {other:?}"),
    }
}

/// A local a loop assigns to is unknown for the whole loop. This pass walks a
/// body once, and the value a local holds at the top of an iteration is the one
/// the iteration before left — so the only sound reading is that it could be
/// anything, and a rate built from it is refused rather than bounded to what
/// one pass happened to produce.
#[test]
fn a_local_a_loop_mutates_is_not_bounded_by_one_pass() {
    match bound(
        "",
        "var w = 0.01;\n    for i in 0..4 {\n      w = w * 0.5;\n    }\n    \
             point_rate = w;",
    ) {
        Bound::Unbounded { lower, .. } => assert_eq!(lower, f32::NEG_INFINITY),
        other => panic!("one pass over a loop proves nothing, got {other:?}"),
    }
}

/// A local a loop only reads keeps its bound, so the widening above is about
/// mutation rather than about loops.
#[test]
fn a_loop_that_mutates_nothing_leaves_the_bound_alone() {
    let rate = at_least(
        "  param point_scale : float [0.002, 0.05] = 0.01",
        "var w = point_scale;\n    for i in 0..4 {\n      let unused = float(i);\n    }\n    \
             point_rate = w;",
    );
    assert_eq!(rate, 0.002);
}

/// `%` on a float takes the sign of its divisor, which the generated WGSL
/// spells `a - b * floor(a / b)` — so a positive divisor makes the result
/// non-negative whatever the dividend was, and `+ 0.001` is then a floor over
/// an ambient nothing declares.
#[test]
fn a_float_remainder_by_a_positive_divisor_is_non_negative() {
    let rate = at_least("", "point_rate = (t % 0.5) + 0.001;");
    assert_eq!(rate, 0.001);
}
