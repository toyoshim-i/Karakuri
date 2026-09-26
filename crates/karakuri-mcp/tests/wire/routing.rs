#![allow(unused_imports)]

use super::wire_common::*;

/// Verifies that write_procedure targets the exact node matching the layer and index without disturbing neighbouring nodes.
#[test]
fn a_write_reaches_the_node_its_address_names_and_not_its_neighbour() {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let head = write("l1.kir", PROBE_L1);
    let first = write("l4_a.kir", PROBE_L4);
    let second = write("l4_b.kir", PROBE_L4);
    let reporter = serve(
        0,
        Slots::of(vec![(head, vec![first.clone(), second.clone()])]),
        store_root(&dir),
        true,
        closed(),
        policies(),
    )
    .expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","index":1,"source":PROBE_L4_B}),
    );
    assert!(!failed, "the second renderer could not be written: {said}");
    assert_eq!(
        std::fs::read_to_string(&second).expect("the second renderer"),
        PROBE_L4_B,
        "`L4:1` was addressed and the file behind it does not hold what was written"
    );
    assert_eq!(
        std::fs::read_to_string(&first).expect("the first renderer"),
        PROBE_L4,
        "`L4:1` was addressed and `L4:0` changed — the address did not survive the \
         call, and the answer said the write had landed"
    );
}

// -- what the audit of this file against the engine turned up -----------

/// Helper constructing a test slot where the head procedure declares a camera (L3).
fn headed_by_a_camera(dir: &tempfile::TempDir) -> (Slots, Vec<std::path::PathBuf>) {
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let camera = write("camera.kir", PROBE_L3);
    let l1 = write("l1.kir", PROBE_L1);
    let l4 = write("l4.kir", PROBE_L4);
    (
        Slots::of(vec![(camera.clone(), vec![l1.clone(), l4.clone()])]),
        vec![camera, l1, l4],
    )
}

/// Verifies that the head file is addressed under the layer it declares rather than unconditionally as L1:0.
#[test]
fn a_head_is_addressed_under_the_kind_it_declares() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (slots, paths) = headed_by_a_camera(&dir);
    let (camera, l1, l4) = (paths[0].clone(), paths[1].clone(), paths[2].clone());

    assert_eq!(
        slots.path(0, Kind::L3, 0).expect("the head is the camera"),
        camera,
        "the head was not filed under the `kind` it declares"
    );
    assert_eq!(
        slots
            .path(0, Kind::L1, 0)
            .expect("the geometry is reachable"),
        l1,
        "`L1:0` did not reach the file that declares `kind L1`"
    );
    assert_eq!(slots.path(0, Kind::L4, 0).expect("the renderer"), l4);

    // The head has taken its own layer's index 0, so a second camera in the
    // chain would be `L3:1` — the counting rule the head always had, now
    // applied on the layer it is actually on.
    let past = slots
        .path(0, Kind::L3, 1)
        .expect_err("this slot was given one camera");
    assert!(past.contains("one L3"), "{past}");
}

/// Verifies that files declaring no explicit kind line fall back to positional defaults (first is L1).
#[test]
fn a_head_that_declares_nothing_is_still_the_slots_l1() {
    let dir = tempfile::tempdir().expect("tempdir");
    let silent = dir.path().join("silent.kir");
    std::fs::write(&silent, "proc nothing_declared {\n}\n").expect("fixture");
    let mute = dir.path().join("mute.kir");
    std::fs::write(&mute, "proc also_nothing {\n}\n").expect("fixture");
    let slots = Slots::of(vec![(silent.clone(), vec![mute.clone()])]);

    assert_eq!(
        slots.path(0, Kind::L1, 0).expect("the head is the L1"),
        silent
    );
    assert_eq!(
        slots
            .path(0, Kind::L4, 0)
            .expect("a later one is a renderer"),
        mute
    );
}

/// Verifies that writing a procedure targets the exact file path matching its layer and index,
/// without overwriting slot heads or losing attached camera definitions.
#[test]
fn a_write_addressed_to_a_geometry_does_not_overwrite_the_head() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (slots, paths) = headed_by_a_camera(&dir);
    let (camera, l1) = (paths[0].clone(), paths[1].clone());
    let reporter = serve(0, slots, store_root(&dir), true, closed(), policies()).expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L1","source":PROBE_L1_B}),
    );
    assert!(!failed, "{said}");
    assert_eq!(
        std::fs::read_to_string(&camera).expect("the camera is still there"),
        PROBE_L3,
        "a write addressed to `L1:0` landed on the head, which is the camera"
    );
    assert_eq!(
        std::fs::read_to_string(&l1).expect("the geometry"),
        PROBE_L1_B,
        "the write did not reach the file that declares `kind L1`"
    );

    // And the camera is readable at the address it is on, which is the
    // other half of the same defect: the real L3 could be reached by
    // nobody.
    let (failed, read) = call(port, "read_procedure", json!({"slot":0,"layer":"L3"}));
    assert!(!failed, "{read}");
    assert!(read.contains("probe_camera"), "{read}");
}

/// Verifies that writing a procedure with unbound inputs succeeds per-procedure without immediately enforcing set validation.
#[test]
fn a_write_that_needs_an_edge_still_returns_cleanly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let l1 = write("l1.kir", PROBE_L1);
    let warp = write("warp.kir", PROBE_L2);
    let l4 = write("l4.kir", PROBE_L4);
    let reporter = serve(
        0,
        Slots::of(vec![(l1, vec![warp, l4])]),
        store_root(&dir),
        true,
        closed(),
        policies(),
    )
    .expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L2","source":PROBE_L2_USES}),
    );
    assert!(
        !failed,
        "the per-procedure check refused this, so the description's caveat is about \
         something that cannot happen any more: {said}"
    );
}

/// Verifies that writing without watch still properly informs where original versions are backed up.
#[test]
fn a_write_without_watch_still_says_where_the_old_version_went() {
    let (server, reporter) = started(false);
    stand_in(reporter, no_loop);
    let (failed, said) = call(
        server.port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":PROBE_L4}),
    );
    assert!(!failed, "{said}");
    assert!(
        said.contains("`<store>/history/`"),
        "the version it replaced is in the edit history and this does not say so: {said}"
    );
    assert!(
        said.contains("scratch"),
        "the file it replaced is the run's copy and this does not say so: {said}"
    );
    assert!(!said.contains("no backup"), "there is a backup: {said}");
    // The thing that *is* true of this run stays said: nothing will pick
    // the write up.
    assert!(said.contains("without `--watch`"), "{said}");
}

// -- the edge, which is the other half of a `uses` ---------------------

/// One of the fixture's files, checked, exactly as this surface checks a
/// written one.
fn checked(dir: &std::path::Path, name: &str) -> Checked {
    let path = dir.join(name);
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    karakuri_environment::compile::check(&source)
        .unwrap_or_else(|e| panic!("{} does not check: {e}", path.display()))
}

/// Checks whether the slot's files validate together under the given wiring edges.
fn assembles(
    dir: &std::path::Path,
    edges: &[karakuri_engine::set::Edge],
) -> Result<(), karakuri_engine::set::SetError> {
    let l1 = checked(dir, "l1.kir");
    let warp = checked(dir, "warp.kir");
    let l4 = checked(dir, "l4.kir");
    let blob = checked(dir, "blob.kir");
    karakuri_engine::Set::validate(
        &[(&l1, 8)],
        &[&warp],
        &[],
        &[&blob],
        &[&l4],
        karakuri_engine::set::Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring {
            edges,
            ..Default::default()
        },
    )
    .map(|_| ())
}

/// Verifies that an unbound procedure written over the wire can be bound via wire_input to assemble the slot.
#[test]
fn a_uses_written_here_can_be_bound_here_and_the_slot_builds() {
    let (server, seen) = wired(true);
    let dir = server.dir.path().to_path_buf();

    // Before anything: the fixture assembles, so a refusal below is about
    // what the test did and not about the fixture.
    assembles(&dir, &[]).expect("the fixture's own slot assembles");

    let (failed, said) = call(
        server.port,
        "write_procedure",
        json!({"slot":0,"layer":"L2","source":PROBE_L2_USES}),
    );
    assert!(!failed, "{said}");

    // Verify slot fails to assemble with unbound slot error before wiring.
    let refused = assembles(&dir, &[]).expect_err(
        "a `uses` nothing binds assembled — this test's middle is gone and the two \
         halves either side of it are about nothing",
    );
    assert!(
        matches!(refused, karakuri_engine::set::SetError::SlotUnbound { .. }),
        "the slot failed to assemble for something other than the unbound `uses` this \
         test is about: {refused}"
    );

    // Bind the dependency over the wire and verify slot now assembles cleanly.
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":0,"node":"probe_warp_uses","input":"shape","to":"probe_blob"}),
    );
    assert!(!failed, "{said}");

    let edges: Vec<karakuri_engine::set::Edge> = seen
        .lock()
        .expect("seen")
        .iter()
        .map(|(_, edge)| edge.clone())
        .collect();
    assert_eq!(
        edges.len(),
        1,
        "one call, one edge on the loop's channel — {edges:?}"
    );
    assembles(&dir, &edges).expect(
        "the edge this server sent the render loop does not bind the `uses` this \
         server wrote: a model can still put a slot in a state only the command line \
         gets it out of",
    );
}

/// Verifies that wire_input delivers the exact node, slot input, and target names to the render loop.
#[test]
fn an_edge_reaches_the_loop_with_the_deck_and_both_ends_as_they_were_typed() {
    let (server, seen) = wired(true);
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":0,"node":"declaring_node","input":"the_input","to":"far_end"}),
    );
    assert!(!failed, "{said}");
    let seen = seen.lock().expect("seen");
    let (slot, edge) = seen.first().expect("the loop was sent an edge");
    assert_eq!(*slot, 0, "the deck slot the call named");
    assert_eq!(edge.node, "declaring_node", "the node that declares it");
    assert_eq!(
        edge.slot,
        "the_input".into(),
        "`input` on the wire is the edge's `slot`, which is what the procedure calls \
         its declared input — the deck's slot is the request's own field"
    );
    assert_eq!(edge.to, "far_end", "the node it is bound to");
    // The loop's own words come back to the client unchanged, which is what
    // makes a refusal from the Set legible to a model.
    assert!(
        said.contains("declaring_node") && said.contains("far_end"),
        "what the loop said did not reach the client: {said}"
    );
}

/// Verifies that edge definitions reject missing or empty node, input, or target fields.
#[test]
fn every_part_of_an_edge_names_something() {
    let (server, seen) = wired(true);
    for (missing, args) in [
        ("node", json!({"slot":0,"input":"shape","to":"probe_blob"})),
        (
            "input",
            json!({"slot":0,"node":"probe_warp","to":"probe_blob"}),
        ),
        ("to", json!({"slot":0,"node":"probe_warp","input":"shape"})),
        (
            "node",
            json!({"slot":0,"node":"","input":"shape","to":"probe_blob"}),
        ),
        (
            "input",
            json!({"slot":0,"node":"probe_warp","input":"","to":"probe_blob"}),
        ),
        (
            "to",
            json!({"slot":0,"node":"probe_warp","input":"shape","to":""}),
        ),
    ] {
        let (failed, said) = call(server.port, "wire_input", args.clone());
        assert!(failed, "`{args}` was accepted as an edge: {said}");
        assert!(
            said.contains(missing),
            "an edge missing `{missing}` was refused without naming it: {said}"
        );
    }
    // A slot number is still a slot number, and the deck is checked after
    // the four arguments have been read.
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":9,"node":"probe_warp","input":"shape","to":"probe_blob"}),
    );
    assert!(failed, "slot 9 was accepted: {said}");
    assert!(
        said.contains("this deck holds"),
        "a slot this deck does not hold was refused in some other surface's words: \
         {said}"
    );
    assert!(
        seen.lock().expect("seen").is_empty(),
        "a refused edge reached the render loop"
    );
}

/// Verifies that wiring without watch indicates that rebuilding will not happen automatically.
#[test]
fn an_edge_written_without_watch_says_no_watcher_will_rebuild_the_slot() {
    let (server, seen) = wired(false);
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":0,"node":"probe_warp","input":"shape","to":"probe_blob"}),
    );
    assert!(!failed, "{said}");
    assert!(
        said.contains("without `--watch`"),
        "a run with no watcher did not say so: {said}"
    );
    assert_eq!(
        seen.lock().expect("seen").len(),
        1,
        "the edge was not sent at all, so a save of this slot would not record it"
    );

    // And with a watcher, the caveat is absent rather than always printed —
    // a warning that fires on healthy material teaches a reader to skip it.
    let (server, _seen) = wired(true);
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":0,"node":"probe_warp","input":"shape","to":"probe_blob"}),
    );
    assert!(!failed, "{said}");
    assert!(
        !said.contains("without `--watch`"),
        "a run that does rebuild was told it does not: {said}"
    );
}

/// Verifies that operations permitted by safety gate policy reach the render loop,
/// while refused operations are blocked before queuing (ADR-0235, Principle 0094).
#[test]
fn an_operate_the_audit_passes_reaches_the_loop_and_a_closed_one_does_not() {
    let (server, reporter) = started(true);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let taken = seen.clone();
    // The stand-in drains what `Keeping::operated` drains, answers where it
    // answers, and holds the reporter alive — see [`stand_in`], whose shape
    // this is for the third channel.
    std::thread::spawn(move || loop {
        for OperateRequest { operation, reply } in reporter.operations() {
            taken.lock().expect("lock").push(operation.clone());
            reply.settled(Ok(format!("`{}` was performed", operation.title())));
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    });

    let (failed, said) = call(
        server.port,
        "operate",
        json!({
            "operation": "Put a node's previous version back",
            "with": {"deck": 0, "revision": {"picked": "20260908-143052-271_slot0_L4"}},
        }),
    );
    assert!(!failed, "{said}");
    assert!(said.contains("was performed"), "{said}");
    assert_eq!(
        seen.lock().expect("lock").as_slice(),
        [Operation::RestoreProcedure {
            deck: 0,
            revision: karakuri_operation::Revision::Picked("20260908-143052-271_slot0_L4".into()),
        }],
        "the operation reached the loop as something other than what was spelled"
    );

    // And the mix is closed on this fixture, so this one is answered on the
    // connection thread and the loop never hears about it.
    let (failed, said) = call(
        server.port,
        "operate",
        json!({"operation": "Gain", "with": {"deck": 0, "gain": 0.25}}),
    );
    assert!(failed, "{said}");
    assert!(said.contains("the mix faders"), "{said}");
    assert!(said.contains("Mixer"), "{said}");
    assert_eq!(
        seen.lock().expect("lock").len(),
        1,
        "a refused operation reached the render loop"
    );
}
