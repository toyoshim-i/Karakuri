#![allow(unused_imports)]

use super::wire_common::*;

/// A write also reaches every slot sharing that file, and says so. The
/// manual's own example gives one `soft_points.kir` to three slots.
#[test]
fn a_write_names_the_other_slots_it_reached() {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = dir.path().join("l1.kir");
    let l4 = dir.path().join("l4.kir");
    std::fs::write(&l1, PROBE_L1).expect("l1");
    std::fs::write(&l4, PROBE_L4).expect("l4");
    let shared = Slots::of(vec![(l1.clone(), vec![l4.clone()]), (l1, vec![l4])]);
    let reporter = serve(0, shared, store_root(&dir), true, closed(), policies()).expect("serve");
    let port = reporter.port();
    std::mem::forget(reporter);

    let (_, source) = call(port, "read_procedure", json!({"slot":0,"layer":"L4"}));
    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":source}),
    );
    assert!(!failed, "{said}");
    assert!(
        said.contains("also slot 1"),
        "the other slot was not named: {said}"
    );
    // **Where the version it replaced went.** This said "there is no
    // backup" while `--watch` was snapshotting every version that compiled
    // into the edit history — and it is the text a model reads, so the one
    // surface that could have told it the file was recoverable said the
    // opposite.
    assert!(
        said.contains("history"),
        "where the old version went: {said}"
    );
}

/// **A file shared as anything but a renderer is shared exactly as much.**
///
/// The scan behind that sentence walked an L1 and a list of renderers,
/// which is the shape a slot had before it could hold a deformation chain
/// — so one `swirl_warp.kir` given to two slots was a write that changed
/// both and named one, and the count a model is handed is only worth
/// having if it is the whole count.
#[test]
fn a_write_names_the_other_slots_it_reached_on_any_layer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let l1 = write("l1.kir", PROBE_L1);
    let warp = write("warp.kir", PROBE_L2);
    let l4 = write("l4.kir", PROBE_L4);
    let shared = Slots::of(vec![
        (l1.clone(), vec![warp.clone(), l4.clone()]),
        (l1, vec![warp, l4]),
    ]);
    let reporter = serve(0, shared, store_root(&dir), true, closed(), policies()).expect("serve");
    let port = reporter.port();
    std::mem::forget(reporter);

    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L2","source":PROBE_L2}),
    );
    assert!(!failed, "{said}");
    assert!(
        said.contains("also slot 1"),
        "the slot sharing this deformation was not named: {said}"
    );
}

/// The tools and resources a client is offered are the ones that answer.
#[test]
fn everything_advertised_can_be_called() {
    let server = start(true);
    let (_, listed) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
    );
    let listed: Value = serde_json::from_str(&listed).expect("json");
    let names: Vec<String> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("name").to_string())
        .collect();
    assert!(!names.is_empty(), "no tools were advertised");
    for name in &names {
        let (_, said) = call(server.port, name, json!({"slot":0,"layer":"L4"}));
        assert!(
            !said.is_empty(),
            "`{name}` is advertised and answers nothing"
        );
    }

    let (_, listed) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}).to_string(),
    );
    let listed: Value = serde_json::from_str(&listed).expect("json");
    let uris: Vec<String> = listed["result"]["resources"]
        .as_array()
        .expect("resources")
        .iter()
        .map(|r| r["uri"].as_str().expect("uri").to_string())
        .collect();
    assert!(!uris.is_empty(), "no resources were advertised");
    for uri in &uris {
        let (_, body) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":uri}})
                .to_string(),
        );
        let reply: Value = serde_json::from_str(&body).expect("json");
        let text = reply["result"]["contents"][0]["text"]
            .as_str()
            .unwrap_or("");
        assert!(
            text.len() > 100,
            "`{uri}` is advertised and reads as nothing"
        );
    }
}

/// **The save tool is offered, and a call reaches the loop with what it was
/// given and comes back with what the loop said.**
///
/// The whole of the new channel in one pass: advertised, sent, answered.
/// The stand-in asserts the arguments it was handed, because a request that
/// arrived with the wrong slot or no id would still have produced an answer.
#[test]
fn the_save_tool_is_offered_and_a_call_reaches_the_loop() {
    // **Two slots**, so that the slot the loop is handed is a fact about
    // the call rather than the only slot there is: a request that carried a
    // constant would pass against a one-slot deck.
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = dir.path().join("l1.kir");
    let l4 = dir.path().join("l4.kir");
    std::fs::write(&l1, PROBE_L1).expect("l1");
    std::fs::write(&l4, PROBE_L4).expect("l4");
    let pair = (l1, vec![l4]);
    let reporter = serve(
        0,
        Slots::of(vec![pair.clone(), pair]),
        store_root(&dir),
        true,
        closed(),
        policies(),
    )
    .expect("serve");
    let server = Server {
        port: reporter.port(),
        dir,
    };
    stand_in(reporter, |request| {
        let SaveRequest { slot, id, reply } = request;
        assert_eq!(slot, 1, "the request reached the loop naming another slot");
        let id = id.expect("the id the client named did not reach the loop");
        reply.accepted(&format!(
            "slot {slot}: saving 2 nodes as set `{id}` in <store>"
        ));
        reply.settled(Ok(format!(
            "slot {slot}: saved as set `{id}` — load it with `--load-set {id}`"
        )));
    });

    let (_, listed) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
    );
    let listed: Value = serde_json::from_str(&listed).expect("json");
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        names.contains(&"save_set"),
        "a model cannot call what it is not offered: {names:?}"
    );

    let (failed, said) = call(server.port, "save_set", json!({"slot":1,"id":"keeper"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("--load-set keeper"),
        "the call came back without the id it was saved under: {said}"
    );
}

/// **A slot that cannot be saved comes back with the loop's own refusal**,
/// unchanged.
///
/// The sentence comes from [`crate::nothing_to_save`] rather than being
/// written out here: a copy would go on passing after the real refusal was
/// corrected, which is exactly what happened to this wording once already —
/// it named `--watch` alone for as long as `--mcp` also made a slot savable.
#[test]
fn a_slot_that_cannot_be_saved_comes_back_with_the_loops_own_words() {
    let (server, reporter) = started(true);
    stand_in(reporter, |request| {
        let SaveRequest { slot, reply, .. } = request;
        reply.settled(Err(karakuri_environment::nothing_to_save(
            slot,
            Some("night01"),
            true,
        )));
    });
    let (failed, said) = call(server.port, "save_set", json!({"slot":0}));
    assert!(failed, "a refusal came back as a success: {said}");
    assert_eq!(
        said,
        karakuri_environment::nothing_to_save(0, Some("night01"), true),
        "the refusal was rewritten on its way to the client"
    );
}

/// **A slot this deck does not hold meets the refusal `read_procedure`
/// gives it — which is the refusal a key press meets**, without troubling
/// the render loop at all.
///
/// `contains("no slot 7")` was all this asked, and it passed under all four
/// spellings this program had of one sentence: the keys' `no slot 7: this
/// deck holds slots 0-6`, this module's `holds 0-6`, MIDI's `no slot 7 —
/// …`, and a record's `slot 7: …`. It is `assert_eq!` against
/// [`crate::no_such_slot`] now, on both surfaces of this module, because
/// `save_set` is the control a model and a hand both reach and the wording
/// they get for one mistake has to be one wording. See that function.
#[test]
fn a_save_for_a_slot_that_does_not_exist_is_refused_here() {
    let (server, reporter) = started(true);
    stand_in(reporter, |request| {
        let SaveRequest { slot, reply, .. } = request;
        reply.settled(Err(format!(
            "slot {slot}: the loop was asked about a slot this deck does not hold"
        )));
    });
    let (failed, said) = call(server.port, "save_set", json!({"slot":7}));
    assert!(failed, "{said}");
    assert_eq!(
        said,
        karakuri_environment::no_such_slot(7, 1),
        "a bad slot was not refused in the words every other surface refuses it in"
    );
    assert!(
        !said.contains("the loop"),
        "a slot this deck does not hold was sent to the render loop: {said}"
    );

    // The other door to the same refusal, which is where this module's own
    // spelling used to live.
    let (failed, said) = call(
        server.port,
        "read_procedure",
        json!({"slot":7,"layer":"L1"}),
    );
    assert!(failed, "{said}");
    assert_eq!(said, karakuri_environment::no_such_slot(7, 1));
}

/// **`"id": null` is a caller saying nothing about the id**, not a caller
/// getting its type wrong.
///
/// A client that builds its arguments from a record with an empty field
/// sends `null` for an argument it is not using, and this refused it with
/// "`id` is a string" — a refusal about a mistake the caller had not made,
/// and one it cannot act on, since what it wanted was the default. Every
/// other optional argument on this surface reads an absent one as its
/// default; `null` is absent's second spelling.
#[test]
fn a_null_id_is_an_absent_id_and_not_a_bad_one() {
    let (server, reporter) = started(true);
    stand_in(reporter, |request| {
        let SaveRequest { slot, id, reply } = request;
        // What the loop makes of `None` is a stamp; what this test is about
        // is that it was handed `None` rather than the call being refused
        // before it got there.
        reply.settled(Ok(match id {
            None => format!("slot {slot}: the loop was left to name it"),
            Some(id) => format!("slot {slot}: the loop was handed `{id}`"),
        }));
    });

    let (failed, said) = call(server.port, "save_set", json!({"slot":0,"id":null}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("left to name it"),
        "`null` was refused as a bad string rather than read as an absent id: {said}"
    );

    // And an argument that really is the wrong type still is one.
    let (failed, said) = call(server.port, "save_set", json!({"slot":0,"id":7}));
    assert!(failed, "{said}");
    assert!(said.contains("`id` is a string"), "{said}");
}

/// **A save in flight does not hold up another connection**, which is the
/// evidence rather than a comment saying the lock was dropped.
///
/// `handle` locks one mutex around `dispatch` and runs a thread per
/// connection, so a tool that waited for the render loop under that lock
/// would stop every other client for as long as the loop took. Here the
/// stand-in has taken a save and is holding it; a second connection asks for
/// a procedure and must be answered before the first is released.
///
/// The second call is not merely *started* while the first is in flight —
/// it is started only once the stand-in has the request in its hands, so
/// there is no ordering in which this passes by racing ahead of the wait.
#[test]
fn a_save_in_flight_does_not_block_another_connection() {
    let (server, reporter) = started(true);
    let (entered, arrived) = mpsc::channel::<()>();
    let (release, released) = mpsc::channel::<()>();
    let released = std::sync::Mutex::new(released);
    stand_in(reporter, move |request| {
        let SaveRequest { slot, id, reply } = request;
        let id = id.unwrap_or_else(|| "stamped".to_string());
        reply.accepted(&format!(
            "slot {slot}: saving 2 nodes as set `{id}` in <store>"
        ));
        entered.send(()).expect("the test is listening");
        released.lock().expect("release").recv().expect("released");
        reply.settled(Ok(format!(
            "slot {slot}: saved as set `{id}` — load it with `--load-set {id}`"
        )));
    });

    let port = server.port;
    let waiting = std::thread::spawn(move || call(port, "save_set", json!({"slot":0,"id":"slow"})));
    arrived
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the save never reached the stand-in loop");

    // The evidence. Under a held lock this blocks until the client's own
    // read timeout gives up on it.
    let (failed, source) = call(
        server.port,
        "read_procedure",
        json!({"slot":0,"layer":"L4"}),
    );
    assert!(!failed, "a second connection was refused: {source}");
    assert!(source.contains("proc probe_l4"), "{source}");

    release.send(()).expect("release the save");
    let (failed, said) = waiting.join().expect("the waiting call");
    assert!(!failed, "{said}");
    assert!(said.contains("--load-set slow"), "{said}");
}

/// `initialize` answers with what a client needs to proceed.
#[test]
fn initialize_answers_with_a_protocol_version_and_capabilities() {
    let server = start(true);
    let (status, body) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize"}).to_string(),
    );
    assert_eq!(status, 200);
    let reply: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(reply["result"]["protocolVersion"], json!(PROTOCOL));
    assert!(reply["result"]["capabilities"]["tools"].is_object());
    assert_eq!(reply["result"]["serverInfo"]["name"], json!("karakuri"));
}

/// Without `--watch` a write changes a file and nothing else, and the model
/// is told so — it has no other way to find out.
#[test]
fn a_write_without_watch_says_nothing_will_pick_it_up() {
    let server = start(false);
    let (_, source) = call(
        server.port,
        "read_procedure",
        json!({"slot":0,"layer":"L4"}),
    );
    let (failed, said) = call(
        server.port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":source}),
    );
    assert!(!failed, "{said}");
    assert!(said.contains("--watch"), "{said}");
}
