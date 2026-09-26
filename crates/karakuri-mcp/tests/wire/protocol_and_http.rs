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

    // Prepend comment to ensure modified content is distinct without relying on specific fixture text.
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

/// Verifies that re-pointing a slot updates file resolution and targets new paths on subsequent calls.
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

/// Verifies that accessing a node removed by a loaded Set produces an accurate descriptive error.
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

/// Verifies that every node across all layers in a slot can be read at its layer address and index.
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

/// Verifies that L2, L3, and Field procedures can be edited and rewritten over the wire.
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

/// Schema exposes every layer type that can occupy a slot.
#[test]
fn the_advertised_layers_are_every_layer_a_slot_can_hold() {
    let server = start_chain();
    let (_, listed) = post(
        server.port,
        &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
    );
    let listed: Value = serde_json::from_str(&listed).expect("json");
    let tools = listed["result"]["tools"].as_array().expect("tools");
    // Five layers (excluding L5) in `LAYERS`.
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
/// Verifies checker feedback is returned in tool call results.
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

/// Payloads exceeding max request length are rejected prior to buffer allocation.
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

/// Verifies Origin header validation prevents cross-origin requests.
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

/// Verifies that unserved paths return 404 with JSON bodies (satisfying OAuth discovery gracefully).
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

/// Verifies that GET requests to the root endpoint return 405 Method Not Allowed per transport spec.
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
