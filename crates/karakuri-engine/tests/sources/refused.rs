use super::common::*;
use karakuri_engine::set::{Edge, Layering, SetError, Wiring};
use karakuri_engine::Set;
use karakuri_ir::typed::Checked;

fn validate_wired(l1s: &[&str], l2: &str, edges: &[Edge]) -> Result<(), SetError> {
    let compiled: Vec<Checked> = l1s.iter().map(|s| compile(s)).collect();
    let sources: Vec<(&Checked, u32)> = compiled.iter().map(|c| (c, 64)).collect();
    let l2 = compile(l2);
    let l4 = compile(DOTS);
    Set::validate(
        &sources,
        &[&l2],
        &[],
        &[],
        &[&l4],
        Layering::Overdraw,
        7,
        &[],
        Wiring {
            edges,
            ..Default::default()
        },
    )
    .map(|_| ())
}

fn validate_paired(l1s: &[&str], l2: &str) -> Result<(), SetError> {
    let far = compile(l1s[l1s.len() - 1]).name;
    validate_wired(l1s, l2, &[edge("morph", "far", &far)])
}

/// A declared slot that nothing binds is refused.
#[test]
fn an_unbound_slot_is_refused_and_says_what_would_bind_it() {
    let near = lattice_at("near", -1.2);
    let far = lattice_at("far", 1.2);

    let err = validate_wired(&[&near, &far], MORPH, &[])
        .expect_err("nothing says which geometry `far` is");
    let text = err.to_string();
    assert!(
        text.contains("morph") && text.contains("far") && text.contains("--edge morph.far="),
        "the refusal names the node, the slot and how to bind it: {text}"
    );
    assert!(
        text.contains("near") && text.contains("far"),
        "and what there is to bind it to: {text}"
    );
}

/// Every part of an edge whose node is in this Set has to resolve.
#[test]
fn an_edge_that_does_not_resolve_is_refused_by_the_part_that_missed() {
    let near = lattice_at("near", -1.2);
    let far = lattice_at("far", 1.2);
    let both = [near.as_str(), far.as_str()];

    let refused = |what: &str, edges: &[Edge]| -> String {
        validate_wired(&both, MORPH, edges)
            .err()
            .unwrap_or_else(|| panic!("{what} has to be refused"))
            .to_string()
    };

    let text = refused(
        "a slot the node does not declare",
        &[edge("morph", "other", "far")],
    );
    assert!(
        text.contains("declares no slot called `other`") && text.contains("declares `far`"),
        "it names the slot that is missing and the one that is there: {text}"
    );

    let text = refused(
        "a far end that is no node",
        &[edge("morph", "far", "sphere")],
    );
    assert!(
        text.contains("`sphere`") && text.contains("not a node of this Set"),
        "it names what was asked for and what there is: {text}"
    );

    let text = refused(
        "a far end that is not geometry",
        &[edge("morph", "far", "morph")],
    );
    assert!(
        text.contains("`: Geometry` takes an L1"),
        "it says what a geometry slot takes: {text}"
    );

    let text = refused(
        "a slot bound twice",
        &[edge("morph", "far", "far"), edge("morph", "far", "near")],
    );
    assert!(
        text.contains("bound twice") && text.contains("`far`") && text.contains("`near`"),
        "it names both answers: {text}"
    );
}

/// A pairing L2 needs exactly two sources.
#[test]
fn a_pairing_l2_states_what_it_needs() {
    let near = lattice_at("near", -1.2);

    let err = validate_wired(&[&near], MORPH, &[edge("morph", "far", "near")])
        .expect_err("one source is not two");
    assert!(err.to_string().contains("this Set has 1"), "{err}");
}

/// Two written names that collide are refused.
#[test]
fn two_written_names_that_collide_are_refused() {
    let grid = compile(&lattice("grid", 0.0));
    let draw = compile(DOTS);

    let err = Set::validate(
        &[(&grid, 64)],
        &[],
        &[],
        &[],
        &[&draw],
        Layering::Overdraw,
        0,
        &[],
        Wiring {
            l1s: &[Some("shape".to_string())],
            l4s: &[Some("shape".to_string())],
            ..Default::default()
        },
    )
    .err()
    .expect("two nodes cannot share a name");
    assert!(format!("{err}").contains("both called `shape`"), "{err}");
}

/// An unbound Source slot is refused.
#[test]
fn an_unbound_source_slot_is_refused() {
    let left = lattice_at("left", -1.2);
    let right = lattice_at("right", 1.2);

    let err = validate_wired(&[&left, &right], DISSOLVE, &[])
        .expect_err("a declared slot that nothing binds is refused");
    let text = err.to_string();
    assert!(
        text.contains("`dissolve` declares `only : Source`"),
        "the refusal names the slot and the type it was declared with: {text}"
    );
    assert!(
        text.contains("--edge dissolve.only="),
        "and says how to bind it: {text}"
    );

    let err = validate_wired(&[&left], DISSOLVE, &[])
        .expect_err("one source is not an excuse to fill the slot in");
    assert!(err
        .to_string()
        .contains("`dissolve` declares `only : Source`"));
}

/// A Source slot bound to something that is not a geometry is refused.
#[test]
fn a_source_slot_bound_to_something_that_is_not_a_geometry_is_refused() {
    let left = lattice_at("left", -1.2);
    let right = lattice_at("right", 1.2);

    let err = validate_wired(
        &[&left, &right],
        DISSOLVE,
        &[edge("dissolve", "only", "dots")],
    )
    .expect_err("a renderer is not a source");
    let text = err.to_string();
    assert!(
        text.contains("which is an L4"),
        "the refusal says what was bound: {text}"
    );
    assert!(
        text.contains("a slot declared `: Source` takes an L1"),
        "and what a Source slot takes: {text}"
    );
    assert!(
        text.contains("left") && text.contains("right"),
        "and lists the sources there are: {text}"
    );

    let err = validate_wired(
        &[&left, &right],
        DISSOLVE,
        &[edge("dissolve", "only", "dissolve")],
    )
    .expect_err("an L2 is not a source either");
    assert!(err.to_string().contains("which is an L2"), "{err}");

    let err = validate_wired(
        &[&left, &right],
        DISSOLVE,
        &[edge("dissolve", "only", "nowhere")],
    )
    .expect_err("a name nothing answers to");
    assert!(
        err.to_string().contains("is not a node of this Set"),
        "{err}"
    );
}

/// Pairing is by slot index, so a source that compacts cannot be paired.
#[test]
fn a_source_that_compacts_cannot_be_paired() {
    let near = lattice_at("near", -1.2);
    let culled = lattice_at("culled", 1.2).replace(
        "    tint     =",
        "    if seed == 3u { kill(); }\n    tint     =",
    );

    let err = validate_paired(&[&near, &culled], MORPH)
        .expect_err("a killing source moves its elements between slots");
    let text = err.to_string();
    assert!(
        text.contains("culled") && text.contains("slot"),
        "the refusal names the source and why: {text}"
    );
}
