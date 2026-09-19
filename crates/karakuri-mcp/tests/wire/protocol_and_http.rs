#![allow(unused_imports)]

use super::wire_common::*;

/// The loop the whole surface exists for: read, write, and the file changes.
#[test]
fn a_procedure_can_be_read_and_rewritten_over_the_wire() {
    let server = start(true);
    let (failed, source) = call(
        server.port,
        "read_procedure",
        json!({"slot":0,"layer":"L4"}),
    );
    assert!(!failed, "{source}");
    assert!(
        source.contains("proc probe_l4"),
        "{}",
        &source[..80.min(source.len())]
    );

    // **Prepended rather than substituted.** This asserted a phrase out of
    // the example's own comment header once, and broke the day somebody
    // rewrote the example — the substitution found nothing, the "edit" was
    // identical to the source, and the failure read as "the write did not
    // reach the file". A test of *writing* must not depend on what the
    // fixture happens to say.
    let edited = format!("// Edited over the wire.\n{source}");
    let (failed, said) = call(
        server.port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":edited}),
    );
    assert!(!failed, "{said}");
    let on_disk = std::fs::read_to_string(server.dir.path().join("l4.kir")).expect("read back");
    assert!(
        on_disk.contains("Edited over the wire."),
        "the write did not reach the file"
    );
}

/// **A re-point moves what the server resolves, on the very next call.**
///
/// This is the whole of why [`Slots`] is a handle. The server was handed
/// the launch working copies and kept them for the run, so after the panel
/// loaded a Set onto a deck — which writes new scratch files and points
/// that slot's watcher at them — every address this surface resolved was
/// the layout the deck had stopped running. A `read_procedure` handed back
/// the material the operator had just replaced and a `write_procedure`
/// wrote a file no watcher was polling, **both of them answering
/// successfully**: the wrong answer arrives as a sentence saying it worked
/// (`docs/principles/0094-…`).
///
/// So the assertion is over the wire, on both tools, before and after one
/// re-point — and the write is read back off the **new** file, because a
/// write that went to the old one would still have said *compiled and
/// written*.
///
/// Watched to fail against the launch copy: a `Slots` that answers out of
/// what it was constructed with reads back `probe_l4` after the re-point
/// and leaves `after.kir` untouched on disk.
#[test]
fn a_re_point_moves_what_an_address_resolves_to() {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let launch_l1 = write("l1.kir", PROBE_L1);
    let launch_l4 = write("l4.kir", PROBE_L4);
    let slots = Slots::of(vec![(launch_l1, vec![launch_l4])]);
    let reporter = serve(
        0,
        slots.clone(),
        store_root(&dir),
        true,
        closed(),
        policies(),
    )
    .expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, source) = call(port, "read_procedure", json!({"slot":0,"layer":"L4"}));
    assert!(!failed, "{source}");
    assert!(
        source.contains("proc probe_l4"),
        "the launch layout is not what the server started on"
    );

    // Where a load leaves a slot: new files, and one aim naming them. The
    // handle is written from the aim rather than from these paths, because
    // that walk is `Slots::re_point`'s and there is one of it.
    let after_l1 = write("after_l1.kir", PROBE_L1_B);
    let after_l4 = write("after.kir", PROBE_L4_B);
    slots.re_point(
        0,
        &karakuri_environment::watch::Aim {
            head: karakuri_environment::compile::Named::bare(&after_l1),
            rest: vec![karakuri_environment::compile::Named::bare(&after_l4)],
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 0,
            salts: Vec::new(),
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            set: Some("night02".to_owned()),
        },
    );

    let (failed, source) = call(port, "read_procedure", json!({"slot":0,"layer":"L4"}));
    assert!(!failed, "{source}");
    assert!(
        source.contains("proc probe_l4_b"),
        "the read resolved against the layout the deck stopped running, which said: {}",
        source.lines().find(|l| l.starts_with("proc")).unwrap_or("")
    );

    let edited = format!("// Edited after the load.\n{source}");
    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":edited}),
    );
    assert!(!failed, "{said}");
    assert!(
        std::fs::read_to_string(dir.path().join("after.kir"))
            .expect("read back")
            .contains("Edited after the load."),
        "the write said it landed and went to the file the slot no longer runs"
    );
}

/// **A node the slot stopped holding is refused naming what it holds
/// now.**
///
/// The other half of the same defect, and the one that is refused rather
/// than answered — which makes it the *milder* half and still a wrong
/// sentence: a model told `slot 0 holds 2 L4 nodes, so index is 0-1` after
/// a load that left the slot one renderer will keep addressing a node that
/// is not there. The refusal is derived from the current nodes because
/// [`Slots::path`] walks them on every call
/// (`docs/principles/0083-…`).
///
/// Watched to fail against the launch copy: `L4:1` resolves and the write
/// lands on a file the deck is not running.
#[test]
fn a_node_a_load_took_away_is_refused_naming_what_the_slot_holds_now() {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let slots = Slots::of(vec![(
        write("l1.kir", PROBE_L1),
        vec![
            write("sprites.kir", PROBE_L4),
            write("strokes.kir", PROBE_L4_B),
        ],
    )]);
    // Two renderers at launch, so `L4:1` is a real address before the load
    // and the assertion below is a change rather than a constant.
    assert_eq!(
        slots.file(0, "L4", 1).expect("two renderers at launch"),
        dir.path().join("strokes.kir")
    );

    slots.re_point(
        0,
        &karakuri_environment::watch::Aim {
            head: karakuri_environment::compile::Named::bare(dir.path().join("l1.kir")),
            rest: vec![karakuri_environment::compile::Named::bare(
                dir.path().join("sprites.kir"),
            )],
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 0,
            salts: Vec::new(),
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges: Vec::new(),
            authorities: Vec::new(),
            set: Some("night02".to_owned()),
        },
    );

    let refused = slots
        .file(0, "L4", 1)
        .expect_err("the loaded Set holds one renderer");
    assert_eq!(refused, "slot 0 holds one L4 and `index` is 1");
    let refused = slots
        .file(0, "L2", 0)
        .expect_err("the loaded Set holds no deformation");
    assert!(
        refused.starts_with("slot 0 holds no L2:"),
        "the refusal is not about what the slot holds now: {refused}"
    );
}

/// **Every node a slot holds is reachable, at the address the rest of this
/// program already spells it by.**
///
/// This surface reached the L1 and the renderers and nothing else, so the
/// material a model could neither see nor edit was exactly the material
/// this language is most interesting about: the deformation between the
/// two, the camera, the field the renderers evaluate, and a second
/// simulation source. Each address is checked against the *name* the file
/// declares rather than against its position, because a resolver that had
/// them one place out would still hand back a procedure.
#[test]
fn every_node_of_a_slot_can_be_read_at_its_own_address() {
    let server = start_chain();
    for (layer, index, expected) in [
        ("L1", 0, "proc probe_l1"),
        ("L2", 0, "proc probe_warp"),
        ("L3", 0, "proc probe_camera"),
        ("Field", 0, "proc probe_blob"),
        ("L4", 0, "proc probe_l4"),
        // The second source, which is an L1 in the chain rather than the
        // head — so its index is 1 and the head keeps 0.
        ("L1", 1, "proc probe_source_b"),
    ] {
        let (failed, source) = call(
            server.port,
            "read_procedure",
            json!({"slot":0,"layer":layer,"index":index}),
        );
        assert!(!failed, "{layer}:{index} could not be read: {source}");
        assert!(
            source.contains(expected),
            "{layer}:{index} read back the wrong file, which said: {}",
            source.lines().find(|l| l.starts_with("proc")).unwrap_or("")
        );
    }
}

/// **A deformation, a camera and a field are written as themselves.**
///
/// The layer a write was checked against was `L1` or, for everything else,
/// `L4` — so a `kind L2` sent to a slot's L2 was refused for not being a
/// renderer, which is a refusal about a mistake nobody made. Both halves
/// are asserted here: the writes that must land, and the one that must not.
#[test]
fn a_deformation_a_camera_and_a_field_are_written_as_themselves() {
    let server = start_chain();
    for (layer, file, source) in [
        ("L2", "warp.kir", PROBE_L2),
        ("L3", "camera.kir", PROBE_L3),
        ("Field", "blob.kir", PROBE_FIELD),
    ] {
        let edited = format!("// Edited over the wire.\n{source}");
        let (failed, said) = call(
            server.port,
            "write_procedure",
            json!({"slot":0,"layer":layer,"source":edited}),
        );
        assert!(!failed, "a {layer} could not be written: {said}");
        let on_disk = std::fs::read_to_string(server.dir.path().join(file)).expect("read back");
        assert!(
            on_disk.contains("Edited over the wire."),
            "the {layer} write did not reach {file}"
        );
    }

    // And the refusal is still honest: the source has to declare the layer
    // it was addressed to, whichever layer that is.
    let (failed, said) = call(
        server.port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":PROBE_L2}),
    );
    assert!(failed, "a deformation was written over a renderer");
    assert!(said.contains("not interchangeable"), "{said}");
}

/// **The schema offers every layer a slot can hold**, because a layer a
/// client is not told about is one it will not ask for — the enum said
/// `L1` and `L4` for as long as a slot could hold five kinds of node.
#[test]
fn the_advertised_layers_are_every_layer_a_slot_can_hold() {
    let server = start_chain();
    let (_, listed) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
    );
    let listed: Value = serde_json::from_str(&listed).expect("json");
    let tools = listed["result"]["tools"].as_array().expect("tools");
    // **Five, where `Kind` is six** — see `LAYERS`. `kind L5` compiles and
    // lowers and is not a layer a *slot* holds, so this list is the one a
    // client can address rather than the one the language has.
    let expected = json!(["L1", "L2", "L3", "L4", "Field"]);
    for name in ["read_procedure", "write_procedure"] {
        let tool = tools
            .iter()
            .find(|t| t["name"] == json!(name))
            .unwrap_or_else(|| panic!("`{name}` is not advertised"));
        assert_eq!(
            tool["inputSchema"]["properties"]["layer"]["enum"], expected,
            "`{name}` offers a client the wrong layers"
        );
    }

    // And what is advertised is what answers: every advertised layer
    // resolves to something on a slot that holds one of each.
    for layer in expected.as_array().expect("layers") {
        let (failed, said) = call(
            server.port,
            "read_procedure",
            json!({"slot":0,"layer":layer}),
        );
        assert!(
            !failed,
            "`{layer}` is advertised and does not resolve: {said}"
        );
    }
}

/// A procedure that does not compile never reaches the disk, and what comes
/// back is the checker's words — **the return value is the point**.
#[test]
fn a_procedure_that_does_not_compile_is_refused_with_diagnostics() {
    let server = start(true);
    let before = std::fs::read_to_string(server.dir.path().join("l4.kir")).expect("before");
    let (failed, said) = call(
        server.port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":"proc broken {\n  out color = nope(1.0)\n}\n"}),
    );
    assert!(failed, "a broken procedure was accepted: {said}");
    assert!(said.contains("parse"), "{said}");
    assert_eq!(
        std::fs::read_to_string(server.dir.path().join("l4.kir")).expect("after"),
        before,
        "a procedure that does not compile reached the disk"
    );
}

/// An L1 procedure is not an L4 one, and the slot says which it wanted.
#[test]
fn a_procedure_for_the_other_layer_is_refused() {
    let server = start(true);
    let (_, l1) = call(
        server.port,
        "read_procedure",
        json!({"slot":0,"layer":"L1"}),
    );
    let (failed, said) = call(
        server.port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":l1}),
    );
    assert!(failed, "an L1 procedure was written into L4");
    assert!(said.contains("not interchangeable"), "{said}");
}

/// **A body larger than this server will read is refused before it is
/// allocated.** `vec![0u8; length]` on an attacker's number aborts the
/// process — not a panic, not catchable, and not confined to this thread.
/// A fifty-six byte request line used to take the render process down.
#[test]
fn an_enormous_content_length_is_refused_and_not_allocated() {
    let server = start(true);
    let (status, _) = raw(
        server.port,
        "POST / HTTP/1.1\r\nContent-Length: 1152921504606846976\r\n\r\n",
    );
    assert_eq!(status, 413, "an exabyte body was not refused");
    // And the server is still there afterwards, which is the whole claim.
    let (status, _) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string(),
    );
    assert_eq!(status, 200, "the server did not survive");
}

/// **A page on any site can POST here.** It cannot read the reply, and
/// `write_procedure` does not need to be read to have happened. The first
/// version of this server treated loopback as a boundary; it is not one.
#[test]
fn a_cross_origin_request_is_refused() {
    let server = start(true);
    let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"write_procedure",
        "arguments":{"slot":0,"layer":"L4","source":"proc x {\n}\n"}}})
    .to_string();
    let (status, _) = raw(
        server.port,
        &format!(
            "POST / HTTP/1.1\r\nOrigin: https://evil.example\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    );
    assert_eq!(status, 403, "a cross-origin write was answered");
    // A local client sends no Origin at all and is still served.
    let (status, _) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string(),
    );
    assert_eq!(status, 200);
}

/// The body of a non-POST used to be left in the reader and become the next
/// request line, so a `GET` with a body ran a smuggled call.
/// **A path this server does not have is a 404, and that is what lets a
/// client connect at all.**
///
/// A client's first move is authorization discovery:
/// `GET /.well-known/oauth-protected-resource`. This server answered 405 to
/// every path, which says "that resource exists, just not by this verb" —
/// so the client went off to fetch protected-resource metadata, tried to
/// parse `this server only answers POST` as JSON, and reported the server
/// as unreachable. Nothing was unreachable; the handshake died on a path
/// that has never existed here.
///
/// Asserted over a socket rather than against a handler, because a status
/// code is a property of the wire — see this module's other wire tests for
/// why that distinction has already mattered here.
#[test]
fn a_path_this_server_does_not_serve_is_not_found_rather_than_not_allowed() {
    let server = start(true);
    for path in [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-authorization-server",
        "/mcp",
    ] {
        let (status, body) = raw(
            server.port,
            &format!("GET {path} HTTP/1.1\r\nContent-Length: 0\r\n\r\n"),
        );
        assert_eq!(status, 404, "GET {path} answered {status}");
        // And a body a JSON client can read, because the one that got here
        // was parsing JSON when it failed.
        serde_json::from_str::<Value>(&body)
            .unwrap_or_else(|e| panic!("the 404 body for {path} is not JSON: {e} — {body}"));
    }
}

/// The endpoint itself still answers 405 to a GET, which is the Streamable
/// HTTP transport's own rule for a server offering no SSE stream there.
///
/// The control for the test above: answering 404 everywhere would satisfy
/// it and break the transport.
#[test]
fn the_endpoint_itself_answers_405_to_a_get() {
    let server = start(true);
    let (status, body) = raw(server.port, "GET / HTTP/1.1\r\nContent-Length: 0\r\n\r\n");
    assert_eq!(status, 405);
    serde_json::from_str::<Value>(&body).expect("the 405 body is JSON too");
}

/// And a POST to a path that is not the endpoint is a 404 as well — the
/// path decides, not the verb.
#[test]
fn a_post_to_another_path_is_also_not_found() {
    let server = start(true);
    let body = json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string();
    let (status, _) = raw(
        server.port,
        &format!(
            "POST /somewhere HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    );
    assert_eq!(status, 404);
}

#[test]
fn a_non_post_cannot_smuggle_a_second_request() {
    let server = start(true);
    let smuggled = json!({"jsonrpc":"2.0","id":99,"method":"ping"}).to_string();
    let (status, body) = raw(
        server.port,
        &format!(
            "GET / HTTP/1.1\r\nContent-Length: {}\r\n\r\nPOST / HTTP/1.1\r\nContent-Length: {}\r\n\r\n{smuggled}",
            smuggled.len() + 60,
            smuggled.len()
        ),
    );
    assert_eq!(status, 405);
    assert!(
        !body.contains("\"id\":99"),
        "the smuggled request ran: {body}"
    );
}

/// Field names are case-insensitive, and a length that is not a number is
/// said rather than read as zero — which used to tell the client its JSON
/// was malformed when it was not.
#[test]
fn header_names_are_case_insensitive_and_a_bad_length_is_named() {
    let server = start(true);
    let body = json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string();
    let (status, reply) = raw(
        server.port,
        &format!(
            "POST / HTTP/1.1\r\nCONTENT-LENGTH: {}\r\n\r\n{body}",
            body.len()
        ),
    );
    assert_eq!(status, 200, "an uppercase header name was not understood");
    assert!(reply.contains("result"), "{reply}");

    let (status, said) = raw(
        server.port,
        "POST / HTTP/1.1\r\nContent-Length: 12x\r\n\r\n",
    );
    assert_eq!(status, 400);
    assert!(said.contains("not a number"), "{said}");
}

/// One connection that says nothing must not take the surface with it.
#[test]
fn a_silent_connection_does_not_wedge_the_server() {
    let server = start(true);
    let _silent = std::net::TcpStream::connect(("127.0.0.1", server.port)).expect("connect");
    // The first version served every connection on one thread, so this
    // second one waited for the first to hang up — which it never does.
    let (status, _) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string(),
    );
    assert_eq!(status, 200, "a silent socket wedged the server");
}
