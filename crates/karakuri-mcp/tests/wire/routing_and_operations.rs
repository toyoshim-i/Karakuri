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

/// **A write lands on the file its address names, and a camera-headed slot
/// does not lose its camera to a valid L1.**
///
/// This is what the address bug cost: `write_procedure(slot, "L1", 0, …)`
/// with a real geometry in it passed the kind guard — the source said L1
/// and the address said L1 — and overwrote the camera's file. The rebuild
/// then sorted by `kind`, so the slot quietly gained a second geometry and
/// lost the camera it was looking through.
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

/// **A stand-in loop that takes the edges as well, and keeps them where a
/// test can look at them.**
///
/// [`stand_in`] answers saves and nothing else, which is what every test
/// before this one needed. What a rewiring *is* belongs to the render loop
/// and needs a deck and a watcher; what these tests are about is that the
/// edge crosses with both its ends intact and that the edge which crosses is
/// one that makes the slot build.
fn wiring_loop(
    reporter: Reporter,
) -> std::sync::Arc<std::sync::Mutex<Vec<(usize, karakuri_engine::set::Edge)>>> {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let kept = seen.clone();
    std::thread::spawn(move || loop {
        for request in reporter.saves() {
            no_loop(request);
        }
        for request in reporter.wires() {
            let WireRequest { slot, edge, reply } = request;
            let said = format!(
                "slot {slot}: `{}.{}` is bound to `{}`, and the slot is rebuilding",
                edge.node, edge.slot, edge.to
            );
            kept.lock().expect("seen").push((slot, edge));
            reply.settled(Ok(said));
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    });
    seen
}

/// A slot of a geometry, a deformation, a renderer and a field — the
/// smallest chain in which a `uses shape : Field` has something to be bound
/// to.
#[allow(clippy::type_complexity)]
fn wired(
    watching: bool,
) -> (
    Server,
    std::sync::Arc<std::sync::Mutex<Vec<(usize, karakuri_engine::set::Edge)>>>,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let head = write("l1.kir", PROBE_L1);
    let rest = vec![
        write("warp.kir", PROBE_L2),
        write("l4.kir", PROBE_L4),
        write("blob.kir", PROBE_FIELD),
    ];
    let reporter = serve(
        0,
        Slots::of(vec![(head, rest)]),
        store_root(&dir),
        watching,
        closed(),
        policies(),
    )
    .expect("serve");
    let port = reporter.port();
    let seen = wiring_loop(reporter);
    (Server { port, dir }, seen)
}

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

/// **The seven tools still work with every class closed**, over the socket
/// a model actually reaches them on.
///
/// ADR-0235 puts all seven in the open set and promises *"no code in this
/// workspace changes on the day this is recorded"* of their behaviour. The
/// gate is new code on the path every one of them takes, so this is checked
/// rather than assumed — and checked here rather than only over
/// [`asked`], because the gate could have been wired into the wrong seam
/// and a unit test on the right one would never notice.
///
/// **An `operate` the audit passes reaches the render loop's drain and is
/// performed there; one it refuses never reaches it at all.**
///
/// This is the whole claim of the eighth tool, and both halves of it are
/// here because they are one mechanism: the operation is named on a
/// connection thread, audited on that thread, and *performed* on the frame
/// the panel performs a press on — so nothing that a closed class would have
/// stopped can be sitting in the queue when the operator looks
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// **No device, and a stand-in for the loop.** What a put-back *is* belongs
/// to `crates/karakuri`'s own performer and needs a window; what is asserted
/// here is that the operation crosses the channel with its payload as it was
/// spelled, and that what the loop says is what the client is handed.
///
/// **Watched to fail** with the audit moved after the send: the second call
/// then comes back as a success and the loop has two operations rather than
/// one, which is the failure this is really about.
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

/// **The five ADR-0334 left `plan` reach the loop now, and the one it
/// listed beside them never will.**
///
/// ADR-0334 named six rows that `operate` refused for want of a performer
/// on the frame the drain lands on, and ADR-0341 is the day each of them
/// got one — four by the drain calling the window's own press arm, one by
/// a reading `crates/karakuri` was not taking. **What is asserted here is
/// the half this crate owns**: the name is taken, the payload crosses the
/// channel as it was spelled, and the audit is what stands between them
/// rather than a refusal written in this file. What each of them then *does*
/// needs a window and is asserted where the window is.
///
/// **With every class open**, because four of the five are closed rows and
/// a fixture that left them shut would assert the gate a second time
/// instead of the route. The one that is open either way is the star: its
/// class is nobody's, the refusal it meets is the performer's, and
/// `gate.rs` is untouched (ADR-0301).
///
/// **No count in the name.** It was five when it was written and six by
/// the end of the day, because ADR-0338's two moved in another session
/// while this one was running — so the list below is what it is and the
/// name does not have to be edited when it grows again.
///
/// **Watched to fail** with any of them taken out of `sayable`'s operable
/// list: the call is refused on the connection thread, `failed` is true,
/// and the loop sees nothing.
#[test]
fn the_rows_that_grew_a_performer_reach_the_loop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = dir.path().join("l1.kir");
    let l4 = dir.path().join("l4.kir");
    std::fs::write(&l1, PROBE_L1).expect("l1");
    std::fs::write(&l4, PROBE_L4).expect("l4");
    let opening = karakuri_environment::Opening::closed();
    opening.set(
        karakuri_operation::gate::Class::ALL
            .iter()
            .fold(karakuri_operation::gate::Open::CLOSED, |open, class| {
                open.with(*class, true)
            }),
    );
    let reporter = serve(
        0,
        Slots::of(vec![(l1, vec![l4])]),
        store_root(&dir),
        true,
        opening,
        policies(),
    )
    .expect("serve");
    let port = reporter.port();

    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let taken = seen.clone();
    std::thread::spawn(move || loop {
        for OperateRequest { operation, reply } in reporter.operations() {
            taken.lock().expect("lock").push(operation.clone());
            reply.settled(Ok(format!("`{}` was performed", operation.title())));
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    });

    // The five, each with the call written beside its row in [`SPELLED`],
    // so this test and the schema a client reads cannot come apart.
    let asked = [
        (
            "Set a deck's mask position",
            json!({"deck": 0, "position": 0.5}),
            Operation::SetMaskPosition {
                deck: 0,
                position: 0.5,
            },
        ),
        (
            "Narrow the published interface",
            json!({"deck": 0, "controls": []}),
            Operation::Publish {
                deck: 0,
                controls: Vec::new(),
            },
        ),
        (
            "Star a Set, or take the star off",
            json!({"set": "a_set", "favourite": true}),
            Operation::SetFavourite {
                id: "a_set".to_string(),
                favourite: true,
            },
        ),
        (
            "Choose where the frame goes",
            json!({"output": "projector", "index": 0, "on": true}),
            Operation::RouteFrame {
                output: karakuri_operation::Output::Projector(0),
                on: true,
            },
        ),
        (
            "Record the session",
            json!({"recording": "start"}),
            Operation::RecordSession {
                recording: karakuri_operation::Recording::Start { id: None },
            },
        ),
    ];
    for (title, with, expected) in &asked {
        let (failed, said) = call(port, "operate", json!({"operation": title, "with": with}));
        assert!(
            !failed,
            "`{title}` is refused over the wire with every class open: {said}"
        );
        assert!(
            said.contains("was performed"),
            "`{title}` was answered by something other than the loop: {said}"
        );
        assert!(
            seen.lock().expect("lock").contains(expected),
            "`{title}` reached the loop as something other than what was spelled — {:?}",
            seen.lock().expect("lock")
        );
    }

    // **And ADR-0338's load, which is taken here and answered by the
    // audit rather than by this file.** Its class is a predicate over the
    // deck it names and this server has read no residency, so it is
    // refused with the reading nobody took — `LoadSet`'s own answer beside
    // it (ADR-0334), and a built route rather than a missing one. What is
    // asserted is *which* refusal: a spelling that did not take the name
    // would refuse it before the gate ever saw it.
    let (failed, said) = call(
        port,
        "operate",
        json!({
            "operation": "Load a procedure over a layer",
            "with": {"deck": 0, "procedure": "orbit_wide"},
        }),
    );
    assert!(
        failed,
        "the load was accepted with no residency read: {said}"
    );
    assert!(
        !said.contains("no operation `") && !said.contains("no route on this surface"),
        "the load was refused by this file's spelling rather than by the audit: {said}"
    );

    // **And the send is refused, in a sentence naming the flag that
    // sends.** It is the row that is `gap` rather than `plan`: a send names
    // no destination and a model cannot answer the dialog the panel puts
    // one in, and a take names a file, which never crosses this protocol.
    let (failed, said) = call(
        port,
        "operate",
        json!({"operation": "Send a Set to somebody, and take one in", "with": {"set": "a_set"}}),
    );
    assert!(failed, "the send was accepted: {said}");
    assert!(
        said.contains("--package") && said.contains("--take-in"),
        "the send's refusal does not say where a model's operator sends one from: {said}"
    );
    assert_eq!(
        seen.lock().expect("lock").len(),
        asked.len(),
        "a refused operation reached the render loop"
    );
}

/// **Two assertions, and the weaker one covers more.** The four this
/// fixture can carry to a real answer must succeed outright. All seven must
/// come back saying something other than the refusal — a tool that fails
/// because this fixture has no store, no saved set and no render loop is
/// this fixture failing it, and a tool the audit stopped says so in the one
/// sentence, which is what makes the two distinguishable at all.
#[test]
fn the_seven_tools_still_work_with_every_class_closed() {
    let server = start(true);
    let asked = [
        ("read_procedure", json!({"slot": 0, "layer": "L4"}), true),
        (
            "write_procedure",
            json!({"slot": 0, "layer": "L4", "source": PROBE_L4}),
            true,
        ),
        ("swap_outcome", json!({}), true),
        ("list_sets", json!({}), true),
        // Answered by this fixture's refusals rather than by the gate: no
        // set was ever saved, and `no_loop` is a render loop that says so.
        ("read_set", json!({"id": "never_saved"}), false),
        ("save_set", json!({"slot": 0}), false),
    ];
    for (name, args, must_succeed) in asked {
        let (failed, said) = call(server.port, name, args);
        assert!(
            !said.contains("closed by default"),
            "`{name}` was stopped by the audit, and ADR-0235 puts all seven tools in the \
             open set: {said}"
        );
        if must_succeed {
            assert!(!failed, "`{name}`: {said}");
        }
    }

    // **The seventh needs the other half of the surface**, so it gets the
    // fixture that has one: `no_loop` never applies an edge, and a client
    // waiting out `WIRE_REPLY` for it would be this test hanging rather
    // than this test failing.
    let (wiring, _seen) = wired(true);
    let (failed, said) = call(
        wiring.port,
        "wire_input",
        json!({"slot": 0, "node": "probe_warp_uses", "input": "shape", "to": "probe_blob"}),
    );
    assert!(!said.contains("closed by default"), "{said}");
    assert!(!failed, "`wire_input`: {said}");
}

#[test]
fn get_permissions_returns_bay_and_slot_status() {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = dir.path().join("l1.kir");
    let l4 = dir.path().join("l4.kir");
    std::fs::write(&l1, PROBE_L1).expect("l1");
    std::fs::write(&l4, PROBE_L4).expect("l4");
    let slot_policies = karakuri_environment::SlotPolicies::new();
    slot_policies.set_policy(0, SlotPolicy::Auto);
    slot_policies.set_in_mix(0, true);
    slot_policies.set_policy(1, SlotPolicy::Off);
    let slots = Slots::of(vec![(l1.clone(), vec![l4.clone()]), (l1, vec![l4])]);
    let reporter = serve(0, slots, store_root(&dir), true, closed(), slot_policies).expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, text) = call(port, "get_permissions", json!({}));
    assert!(!failed, "{text}");
    let v: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(v["bays"]["program"], "off");
    assert_eq!(v["bays"]["mixer"], "off");
    assert_eq!(v["slots"][0]["slot"], 0);
    assert_eq!(v["slots"][0]["name"], "Deck A");
    assert_eq!(v["slots"][0]["policy"], "auto");
    assert_eq!(v["slots"][0]["in_mix"], true);
    assert_eq!(v["slots"][0]["writable"], false);
    assert_eq!(v["slots"][1]["slot"], 1);
    assert_eq!(v["slots"][1]["name"], "Deck B");
    assert_eq!(v["slots"][1]["policy"], "off");
    assert_eq!(v["slots"][1]["writable"], false);
}

#[test]
fn read_slot_returns_all_procedures_of_a_slot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = dir.path().join("l1.kir");
    let l4 = dir.path().join("l4.kir");
    std::fs::write(&l1, PROBE_L1).expect("l1");
    std::fs::write(&l4, PROBE_L4).expect("l4");
    let slots = Slots::of(vec![(l1, vec![l4])]);
    let reporter = serve(0, slots, store_root(&dir), true, closed(), policies()).expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, text) = call(port, "read_slot", json!({"slot": 0}));
    assert!(!failed, "{text}");
    let v: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(v["slot"], 0);
    let nodes = v["nodes"].as_array().expect("nodes array");
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0]["layer"], "L1");
    assert_eq!(nodes[0]["index"], 0);
    assert!(nodes[0]["source"]
        .as_str()
        .unwrap()
        .contains("proc probe_l1"));
    assert_eq!(nodes[1]["layer"], "L4");
    assert_eq!(nodes[1]["index"], 0);
    assert!(nodes[1]["source"]
        .as_str()
        .unwrap()
        .contains("proc probe_l4"));
}

#[test]
fn copy_slot_copies_and_respects_slot_policies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1_0 = dir.path().join("l1_0.kir");
    let l4_0 = dir.path().join("l4_0.kir");
    let l1_1 = dir.path().join("l1_1.kir");
    let l4_1 = dir.path().join("l4_1.kir");
    std::fs::write(&l1_0, PROBE_L1).expect("l1_0");
    std::fs::write(&l4_0, PROBE_L4).expect("l4_0");
    std::fs::write(&l1_1, PROBE_L1_B).expect("l1_1");
    std::fs::write(&l4_1, PROBE_L4).expect("l4_1");
    let slot_policies = karakuri_environment::SlotPolicies::new();
    let slots = Slots::of(vec![
        (l1_0.clone(), vec![l4_0.clone()]),
        (l1_1.clone(), vec![l4_1.clone()]),
    ]);
    let opening = karakuri_environment::Opening::closed();
    let mut open = karakuri_operation::gate::Open::CLOSED;
    for &class in karakuri_operation::gate::Class::ALL {
        open = open.with(class, true);
    }
    opening.set(open);
    let reporter = serve(
        0,
        slots,
        store_root(&dir),
        true,
        opening,
        slot_policies.clone(),
    )
    .expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    // 1. Slot 1 is in_mix under Auto -> copy to it is refused
    slot_policies.set_policy(1, SlotPolicy::Auto);
    slot_policies.set_in_mix(1, true);
    let (failed, text) = call(port, "copy_slot", json!({"from_slot": 0, "to_slot": 1}));
    assert!(failed, "expected refusal when target is active in mix");
    assert!(text.contains("active in the mix"), "{text}");

    // Also write_procedure to slot 1 is refused
    let (failed_write, text_write) = call(
        port,
        "write_procedure",
        json!({"slot": 1, "layer": "L1", "source": PROBE_L1}),
    );
    assert!(failed_write);
    assert!(text_write.contains("active in the mix"), "{text_write}");

    // Also SelectRenderer on slot 1 is refused
    let (failed_rend, text_rend) = call(
        port,
        "operate",
        json!({
            "operation": "Choose which renderer of a deck is live",
            "with": {"deck": 1, "renderer": 0}
        }),
    );
    assert!(failed_rend);
    assert!(text_rend.contains("active in the mix"), "{text_rend}");

    // 2. Slot 1 is off-air (in_mix = false) -> copy succeeds
    slot_policies.set_in_mix(1, false);
    let (failed, text) = call(port, "copy_slot", json!({"from_slot": 0, "to_slot": 1}));
    assert!(!failed, "{text}");
    assert_eq!(std::fs::read_to_string(&l1_1).unwrap(), PROBE_L1);

    // 3. Slot 1 is policy Off -> refused even when off-air
    slot_policies.set_policy(1, SlotPolicy::Off);
    let (failed_off, text_off) = call(port, "copy_slot", json!({"from_slot": 0, "to_slot": 1}));
    assert!(failed_off);
    assert!(text_off.contains("locked against MCP"), "{text_off}");

    // 4. Slot 1 is policy On -> allowed even when in_mix
    slot_policies.set_policy(1, SlotPolicy::On);
    slot_policies.set_in_mix(1, true);
    let (failed_on, text_on) = call(port, "copy_slot", json!({"from_slot": 0, "to_slot": 1}));
    assert!(!failed_on, "{text_on}");

    // 5. Layer filter copies only the requested layer and leaves others untouched
    std::fs::write(&l1_1, PROBE_L1_B).expect("reset l1_1");
    std::fs::write(&l4_1, "initial l4").expect("reset l4_1");
    let (failed_layer, text_layer) = call(
        port,
        "copy_slot",
        json!({"from_slot": 0, "to_slot": 1, "layer": "L4"}),
    );
    assert!(!failed_layer, "{text_layer}");
    assert_eq!(
        std::fs::read_to_string(&l1_1).unwrap(),
        PROBE_L1_B,
        "L1 untouched"
    );
    assert_eq!(
        std::fs::read_to_string(&l4_1).unwrap(),
        PROBE_L4,
        "L4 copied"
    );
    assert!(!dir.path().join("l4_1.kir.tmp").exists(), "no tmp left");
}
