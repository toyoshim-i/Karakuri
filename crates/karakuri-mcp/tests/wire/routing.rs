#![allow(unused_imports)]

use super::wire_common::*;

/// **A write lands on the node its address names, and its neighbour is left
/// alone** — asserted on the files rather than on what the call said.
///
/// This is the one property the routing through
/// [`karakuri_operation::Operation`] could quietly lose: the wire's `index`
/// becomes [`NodeAddress::index`] and comes back out again to resolve a file, so
/// an address that arrived correct and was carried wrong would still return
/// *compiled and written* and change the wrong procedure. A slot with two
/// renderers is what makes that visible: with one, every wrong index is the
/// right one.
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

/// A slot whose head is anything but a geometry, for the tests below.
///
/// **`--set` takes whatever the operator typed first**, and nothing checks
/// that it is an L1 — the sort that assembles the slot reads every file's
/// own `kind` line, the head included, so a chain headed by a camera is an
/// ordinary slot with an ordinary camera in it.
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

/// **The head is addressed under the `kind` it declares**, like every other
/// file of the slot.
///
/// It was filed as `L1:0` whatever it said, which no other reader of these
/// files agrees with: `compile::sort_compiled` matches on the head's own
/// `kind` and `history::seed` reads the head's `kind` line, taking a
/// position only where a file declares nothing. So a slot headed by a
/// camera had its camera at `L1:0`, its geometry unreachable, and its real
/// `L3` addressable by nobody.
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

/// **A file with no `kind` line at all keeps `history::seed`'s positional
/// answer**: the first path is an L1 and every later one a renderer.
///
/// That fallback is the whole reason reading the head's `kind` is safe. A
/// slot whose head cannot be read — or that is a `.kir` the scan finds no
/// `kind` in — must still be addressable at `L1:0`, because that is where
/// its snapshots are filed.
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

/// **An L2 that declares `uses shape : Field` is written cleanly**, which
/// is the promise `write_procedure`'s description had to stop making.
///
/// `compile::check` checks one procedure in isolation; everything between
/// nodes is `Set::validate`, which is where an unbound slot is refused. So
/// a clean write is not a slot that rebuilds, and this is still true after
/// `wire_input` exists: the binding is a **second** call, and the window
/// between the two is a slot that does not build. What changed is that
/// there is now a second call to make — see
/// [`a_uses_written_here_can_be_bound_here_and_the_slot_builds`], which
/// takes this write the rest of the way.
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

/// **A run with `--mcp` and no `--watch` has a scratch and an edit history
/// like any other**, and the answer used to tell it the opposite.
///
/// `main.rs`'s `editable` is `watch || mcp.is_some()`: the deck runs from
/// copies, so the files the operator named are never written to, and
/// `history::seed` has filed the version the run started with. "It replaced
/// the file on disk and there is no backup" was wrong in both halves, and
/// it was wrong on the one surface whose reader cannot look at the
/// terminal.
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

/// **Whether the fixture's slot assembles**, with the wiring it is given.
///
/// `Set::validate` is `Set::build_many`'s whole check pass and needs no
/// device, which is the only reason this can be asserted here at all: it is
/// the same rule the render loop's rebuild would meet, run against the files
/// that are actually on disk after a write.
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

/// **The test that would have caught it**: a `uses` written through this
/// server, an edge written through this server, and the slot assembling.
///
/// This is the whole trap and the whole fix in one run. `write_procedure`
/// accepted a procedure declaring `uses shape : Field` — it compiles, and
/// one procedure is all `compile::check` ever sees — and the slot then
/// failed to build with `SetError::SlotUnbound`, which nothing on this
/// surface could answer. The middle assertion is that failure, asserted
/// rather than described, so that this test is about a trap that was real;
/// the last is that the edge **this server sent to the loop** is the one
/// that closes it.
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

    // **The trap, on the frame it goes wrong.** The file on disk is the one
    // the model wrote, it checks, and the slot it is in does not assemble.
    let refused = assembles(&dir, &[]).expect_err(
        "a `uses` nothing binds assembled — this test's middle is gone and the two \
         halves either side of it are about nothing",
    );
    assert!(
        matches!(refused, karakuri_engine::set::SetError::SlotUnbound { .. }),
        "the slot failed to assemble for something other than the unbound `uses` this \
         test is about: {refused}"
    );

    // **The way out, over the wire.** Both ends by name: the node is what
    // the procedure calls itself, because nothing named it.
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

/// **Both ends reach the loop as the names that were typed**, and the input
/// is not one of them.
///
/// Three strings on one request is three chances to hand the loop the wrong
/// one, and every mistake of that kind reads as a working call: the edge is
/// written, the slot refuses to build, and the refusal is about a node
/// nobody named. Every one of these three names is a different word, on
/// purpose.
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

/// **Every part of an edge names something**, which is `parse_edge`'s rule
/// on the command line and the same sentence here.
///
/// An absent part and an empty one are the two shapes, and neither may reach
/// the render loop: an edge with a hole in it is a statement about a node,
/// and there is no such statement.
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

/// **A run without `--watch` has no watcher, and the answer says so** — the
/// same fact `write_procedure` states about the other half of one edit.
///
/// The edge is still sent: it is the run's wiring from then on, and a
/// `save_set` of that slot records it. What does not happen is the rebuild,
/// and a model told "the slot is rebuilding" by a run that has nothing to
/// rebuild it with would go looking for a change on screen that is never
/// coming.
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
