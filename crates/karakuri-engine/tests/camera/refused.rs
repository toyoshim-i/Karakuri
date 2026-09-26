use super::common::compile;
use super::fixtures::{through, MARK};
use karakuri_engine::set::{Edge, Layering, SetError, Wiring};
use karakuri_engine::Set;
use karakuri_ir::typed::Checked;

/// The Set `wired` above describes — one `MARK` at capacity 1, these
/// cameras and these renderers — minus the device and everything
/// downstream of it.
fn validate_wired(
    l3s: &[String],
    l4s: &[&str],
    edges: &[(&str, &str, &str)],
) -> Result<(), SetError> {
    let l3s: Vec<Checked> = l3s.iter().map(|s| compile(s)).collect();
    let l4s: Vec<Checked> = l4s.iter().map(|s| compile(s)).collect();
    let edges: Vec<Edge> = edges
        .iter()
        .map(|(node, slot, to)| Edge {
            node: node.to_string(),
            slot: (*slot).into(),
            to: to.to_string(),
        })
        .collect();
    Set::validate(
        &[(&compile(MARK), 1)],
        &[],
        &l3s.iter().collect::<Vec<_>>(),
        &[],
        &l4s.iter().collect::<Vec<_>>(),
        Layering::Overdraw,
        7,
        &[],
        Wiring {
            edges: &edges,
            ..Default::default()
        },
    )
    .map(|_| ())
}

/// Verifies that declared Camera slots must be explicitly bound.
#[test]
fn an_unbound_camera_slot_is_refused() {
    let named = through("named", [1.0, 0.0, 0.0]);
    let err = validate_wired(&[], &[&named], &[])
        .expect_err("an unbound slot is not filled in from the Set's only camera");
    let text = format!("{err}");
    assert!(
        text.contains("`named` declares `view : Camera`"),
        "the refusal has to name the slot and the type it takes: {text}"
    );
}

/// Edge bindings must target camera nodes rather than arbitrary nodes.
/// name. The sentence says what the node it found actually is, because that is
/// the half an operator cannot see from the edge.
#[test]
fn a_camera_slot_bound_to_something_that_is_not_a_camera_is_refused() {
    let named = through("named", [1.0, 0.0, 0.0]);
    let err = validate_wired(&[], &[&named], &[("named", "view", "mark")])
        .expect_err("a geometry is not a camera");
    let text = format!("{err}");
    assert!(
        text.contains("bound to `mark`") && text.contains("an L1"),
        "the refusal has to say what was bound and what it is: {text}"
    );
    assert!(
        text.contains(karakuri_engine::set::BUILTIN_CAMERA),
        "and which cameras this Set holds: {text}"
    );
}
