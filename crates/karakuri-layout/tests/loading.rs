//! What a saved arrangement has to *be* to load, beyond parsing: the arena's
//! structural invariants, one refusal each.
//!
//! Every test here takes the console's arrangement, saves it, breaks one thing
//! about the file the way a truncation, a hand edit or an older writer would,
//! and expects a refusal that says what is wrong with it. Three of the
//! mutations are failures the loader used to pass on to the caller — an index
//! out of bounds inside the first solve, a stack overflow inside it, and a walk
//! up the parent pointers that never returns.
//!
//! **The negative control is [`saved`] itself**, which asserts that the
//! unmodified file loads and solves to exactly the rectangles it was saved
//! from. Every test below goes through it, so a loader that refused everything
//! would fail all of them before it ever reached the mutation.
//!
//! **None of these tests can hang.** The refusal happens at load, so a file
//! whose parent pointers loop is an `Err` before anything walks them.

mod common;

use common::{assert_invariants, named_console, rects};
use karakuri_layout::{Layout, Rect};
use serde_json::{json, Value};

/// The console's arrangement solved at 1280x720, and the file it saves to —
/// having first checked that the file comes back as the same arrangement.
fn saved() -> Value {
    let mut l = named_console();
    l.set_viewport(Rect::new(0.0, 0.0, 1280.0, 720.0));
    l.solve();
    let file = serde_json::to_value(&l).unwrap();

    let back: Layout = serde_json::from_value(file.clone())
        .expect("the unmodified arrangement must still load: this is the negative control");
    assert_eq!(
        rects(&back),
        rects(&l),
        "the round trip changed a rectangle"
    );
    assert_invariants(&back);
    file
}

/// Load a broken file and hand back the sentence it was refused with.
fn refused(file: Value) -> String {
    match serde_json::from_value::<Layout>(file) {
        Ok(_) => panic!("a structurally broken arrangement loaded rather than being refused"),
        Err(e) => e.to_string(),
    }
}

/// The index of the node named `name`. It is the number the file writes and the
/// number a refusal names, which is the whole point of naming indices.
fn node(file: &Value, name: &str) -> usize {
    let named =
        |n: &Value| n["kind"]["View"]["name"] == *name || n["kind"]["Split"]["name"] == *name;
    file["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(named)
        .unwrap_or_else(|| panic!("no node named {name} in the saved arrangement"))
}

fn len(file: &Value) -> usize {
    file["nodes"].as_array().unwrap().len()
}

fn root(file: &Value) -> usize {
    file["root"].as_u64().unwrap() as usize
}

/// Replace child `index` of the split named `name`.
fn set_child(file: &mut Value, name: &str, index: usize, child: usize) {
    let split = node(file, name);
    file["nodes"][split]["kind"]["Split"]["children"][index] = json!(child);
}

#[test]
fn a_saved_arrangement_with_no_nodes_at_all_fails_to_load_rather_than_panicking() {
    // A file truncated to nothing still parses: it is an arena with no arena
    // in it, and every index into it — starting with the root's — is past the
    // end.
    let mut file = saved();
    file["nodes"] = json!([]);

    let err = refused(file);
    assert!(
        err.contains("the arrangement has no nodes"),
        "loaded, or failed for the wrong reason: {err}"
    );
}

#[test]
fn a_saved_arrangement_whose_root_is_not_a_node_fails_to_load_rather_than_panicking() {
    // Without the check this is `index out of bounds` inside the first solve —
    // which the load itself performs, so the panic arrives from `from_str`.
    let mut file = saved();
    let past = len(&file);
    file["root"] = json!(past);

    let err = refused(file);
    assert!(
        err.contains(&format!(
            "the root is node {past}, and the arrangement has {past} nodes"
        )),
        "loaded, or failed for the wrong reason: {err}"
    );
}

#[test]
fn a_saved_arrangement_whose_root_records_a_parent_fails_to_load() {
    // A root repointed at an interior node: everything outside that node's
    // subtree is now unreachable, and `visible` walks *up* past the root into
    // nodes the arrangement does not contain — so folding one of them hides an
    // arrangement it is not part of.
    let mut file = saved();
    let pane = node(&file, "left-pane");
    let panes = node(&file, "panes");
    file["root"] = json!(pane);

    let err = refused(file);
    assert!(
        err.contains(&format!(
            "the root, node {pane}, records node {panes} as its parent"
        )),
        "loaded, or failed for the wrong reason: {err}"
    );
}

#[test]
fn a_saved_arrangement_with_a_child_that_is_not_a_node_fails_to_load_rather_than_panicking() {
    // The second of the three: `index out of bounds` again, this time from the
    // arena access the solve makes for every child of every split.
    let mut file = saved();
    let past = len(&file);
    set_child(&mut file, "left-pane", 1, past);
    let pane = node(&file, "left-pane");

    let err = refused(file);
    assert!(
        err.contains(&format!("node {pane} lists node {past} as its child 1")),
        "loaded, or failed for the wrong reason: {err}"
    );
}

#[test]
fn a_saved_arrangement_that_holds_one_node_in_two_places_fails_to_load() {
    // No cycle and no bad index: the left pane is simply listed twice, which
    // is an arena that is a graph rather than a tree. It would solve — twice,
    // to two different rectangles, the second overwriting the first — and one
    // of the two regions on screen would answer to nothing.
    let mut file = saved();
    let left = node(&file, "left-pane");
    set_child(&mut file, "panes", 1, left);
    let panes = node(&file, "panes");

    let err = refused(file);
    assert!(
        err.contains(&format!(
            "node {panes} lists node {left} as a child, and node {left} has already been reached"
        )),
        "loaded, or failed for the wrong reason: {err}"
    );
}

#[test]
fn a_saved_arrangement_whose_children_lead_back_up_fails_to_load_rather_than_overflowing_the_stack()
{
    // The third of the three, and the one that does not even leave a panic to
    // read: the solve recurses into children, so a children list that points
    // back at an ancestor aborts the process with a stack overflow — during
    // the load, since the load solves.
    let mut file = saved();
    let panes = node(&file, "panes");
    set_child(&mut file, "left-pane", 1, panes);
    let left = node(&file, "left-pane");

    let err = refused(file);
    assert!(
        err.contains(&format!("node {left} lists node {panes} as a child")),
        "loaded, or failed for the wrong reason: {err}"
    );
}

#[test]
fn a_saved_arrangement_whose_parents_loop_fails_to_load_rather_than_hanging() {
    // The worst of the three. `visible` and `is_ancestor` walk up the parent
    // pointers, so a pane that records its own child as its parent makes them
    // spin forever: no panic, no output, nothing to read afterwards — and the
    // suite would be stuck rather than red.
    //
    // Nothing here reaches that loop, because the file is refused before a
    // `Layout` exists to walk.
    let mut file = saved();
    let left = node(&file, "left-pane");
    let library = node(&file, "library");
    let panes = node(&file, "panes");
    file["nodes"][left]["parent"] = json!(library);

    let err = refused(file);
    assert!(
        err.contains(&format!(
            "node {left} is a child of node {panes} but records node {library} as its parent"
        )),
        "loaded, or failed for the wrong reason: {err}"
    );
}

#[test]
fn a_saved_arrangement_whose_node_records_no_parent_fails_to_load() {
    // The other half of the same disagreement, and the one an older writer
    // leaves: the children list holds the node, the node says it hangs from
    // nothing. `visible` then stops at it and reports a pane inside a folded
    // one as on screen.
    let mut file = saved();
    let library = node(&file, "library");
    let left = node(&file, "left-pane");
    file["nodes"][library]["parent"] = json!(null);

    let err = refused(file);
    assert!(
        err.contains(&format!(
            "node {library} is a child of node {left} but records no parent at all"
        )),
        "loaded, or failed for the wrong reason: {err}"
    );
}

#[test]
fn a_saved_arrangement_with_a_node_nothing_reaches_fails_to_load() {
    // Not a crash and not a hang — a decision. The status row is still in the
    // file, still named, still carrying its parent, and no children list
    // mentions it: it solves to nothing, draws nothing, and no fold or solo
    // above it can move it. A file that is a tree plus debris is refused
    // rather than loaded with the debris quietly along for the ride.
    let mut file = saved();
    let status = node(&file, "status");
    let root = root(&file);
    file["nodes"][root]["kind"]["Split"]["children"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c != &json!(status));

    let err = refused(file);
    assert!(
        err.contains(&format!("nothing reaches node {status} from the root")),
        "loaded, or failed for the wrong reason: {err}"
    );
}
