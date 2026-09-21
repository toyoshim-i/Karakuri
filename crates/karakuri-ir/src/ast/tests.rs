use super::*;

#[test]
fn attribute_names_round_trip() {
    for a in Attr::ALL {
        assert_eq!(Attr::from_name(a.name()), Some(a));
    }
}

#[test]
fn id_is_not_an_attribute_or_an_ambient() {
    // Identity is `seed`. A slot index is not an identity once compaction
    // moves elements, so `id` must not resolve to anything.
    assert_eq!(Attr::from_name("id"), None);
    assert_eq!(Ambient::from_name("id"), None);
}

#[test]
fn seed_is_readable_everywhere() {
    for (kind, block) in [
        (Kind::L1, BlockKind::Spawn),
        (Kind::L1, BlockKind::Element),
        (Kind::L4, BlockKind::Vertex),
        (Kind::L4, BlockKind::Fragment),
    ] {
        assert!(Ambient::Seed.available_in(kind, block));
    }
}

#[test]
fn point_coord_is_fragment_only() {
    assert!(Ambient::PointCoord.available_in(Kind::L4, BlockKind::Fragment));
    assert!(!Ambient::PointCoord.available_in(Kind::L4, BlockKind::Vertex));
}

#[test]
fn only_velocity_and_age_are_derivable() {
    let derivable: Vec<_> = Attr::ALL.into_iter().filter(|a| a.is_derivable()).collect();
    assert_eq!(derivable, vec![Attr::Velocity, Attr::Age]);
}

#[test]
fn multiplication_binds_tighter_than_comparison() {
    assert!(BinOp::Mul.precedence() > BinOp::Lt.precedence());
    assert!(BinOp::Add.precedence() > BinOp::Eq.precedence());
}

/// One L4 declaring whatever `params` says, checked, so every test below asks
/// the fold about a declaration the checker has already accepted.
fn declared(params: &str) -> Vec<Param> {
    let src = format!(
        r#"
proc folds {{
  kind  L4
  blend additive

{params}

  consumes position

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.004;
  }}

  fragment {{
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }}
}}
"#
    );
    let proc = crate::parse(&src).unwrap_or_else(|e| panic!("parse: {e:?}"));
    crate::check::check(&proc)
        .unwrap_or_else(|e| panic!("check: {e:?}"))
        .params
}

fn only(params: &str) -> Param {
    declared(params).into_iter().next().expect("one param")
}

/// The whole of what a vector default was missing. Before
/// [`Param::default_components`] the only fold was [`Param::default_scalar`],
/// which answers `None` for every `vec3`, so the three numbers a `.kir` writes
/// down were unreachable and the engine packed zeroes. `docs/ir-spec.md`'s own
/// `param` example is this declaration.
#[test]
fn a_vector_default_folds_to_one_number_per_component() {
    let glow = only("  param glow : vec3 [0.0, 4.0] = vec3(0.4, 0.7, 1.0)");
    assert_eq!(
        glow.default_components(),
        Some(vec![0.4, 0.7, 1.0]),
        "the three numbers the declaration states"
    );
    assert_eq!(
        glow.default_scalar(),
        None,
        "one number is still not what a `vec3` declares"
    );
    assert_eq!(
        glow.keys(),
        vec!["glow.x", "glow.y", "glow.z"],
        "the component order is a MIDI address and is `x`, `y`, `z`"
    );
}

/// The broadcast `docs/ir-spec.md` makes legal in "Types" — *"or a single
/// scalar to broadcast … `vec3(0.0)`"* — which is one argument for three
/// components, so an arity test alone would read it as a mismatch.
#[test]
fn a_broadcast_default_folds_to_that_number_in_every_component() {
    assert_eq!(
        only("  param wash : vec3 [0.0, 1.0] = vec3(0.25)").default_components(),
        Some(vec![0.25, 0.25, 0.25])
    );
    assert_eq!(
        only("  param pan : vec2 [-1.0, 1.0] = vec2(-0.5)").default_components(),
        Some(vec![-0.5, -0.5])
    );
}

/// A negative component is a value and not an absence, which is the defect
/// `default_scalar` was widened for once already — and it has to survive inside
/// a constructor, where the argument is `Unary { Neg, Lit }` for the same
/// reason the whole default was.
#[test]
fn a_negated_component_is_folded_inside_the_constructor() {
    assert_eq!(
        only("  param drift : vec2 [-1.0, 1.0] = vec2(-0.35, 0.25)").default_components(),
        Some(vec![-0.35, 0.25])
    );
}

/// A scalar keeps its one key and its one number, so nothing about a `float`
/// param moves.
#[test]
fn a_scalar_default_is_one_component_under_the_declared_name() {
    let radius = only("  param radius : float [0.1, 8.0] = 2.0");
    assert_eq!(radius.default_components(), Some(vec![2.0]));
    assert_eq!(radius.default_scalar(), Some(2.0));
    assert_eq!(radius.keys(), vec!["radius"]);
}

/// `None` is *this default is not a number I can state*. Both cases are legal
/// `.kir` that the checker accepts: an argument that is an expression rather
/// than a literal, and a nested constructor whose component count reaches the
/// width through an inner `vec2`. The fold says so rather than inventing a
/// number, and the engine leaves those keys out of its value map.
#[test]
fn a_default_this_cannot_state_folds_to_nothing() {
    assert_eq!(
        only("  param glow : vec3 [0.0, 4.0] = vec3(0.4, 0.7, 0.5 + 0.5)").default_components(),
        None,
        "an argument that is not a literal is not a number this states"
    );
    assert_eq!(
        only("  param glow : vec3 [0.0, 4.0] = vec3(vec2(0.1, 0.2), 0.3)").default_components(),
        None,
        "a nested constructor answers `None`, which is said in the doc"
    );
}
