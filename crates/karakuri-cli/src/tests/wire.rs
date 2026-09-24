use super::*;

#[cfg(test)]
mod wire_tests {
    use super::*;
    use std::io::{BufRead, Write};

    fn edge(node: &str, slot: &str, to: &str) -> karakuri_engine::set::Edge {
        karakuri_engine::set::Edge {
            node: node.to_string(),
            slot: slot.into(),
            to: to.to_string(),
        }
    }

    /// Creates test Aim state holding only edges.
    fn aimed(edges: Vec<karakuri_engine::set::Edge>) -> watch::Aim {
        watch::Aim {
            head: Named::bare("head.kir"),
            rest: Vec::new(),
            layering: karakuri_engine::set::Layering::Overdraw,
            live: None,
            capacity: None,
            seed_salt: 7,
            salts: vec![7],
            camera: karakuri_engine::camera::Orbit::default(),
            overrides: Vec::new(),
            published: Vec::new(),
            bindings: Vec::new(),
            edges,
            authorities: Vec::new(),
            set: None,
        }
    }

    /// One watched slot, and the end a re-point arrives on.
    fn watcher(
        edges: Vec<karakuri_engine::set::Edge>,
    ) -> (Aiming, std::sync::mpsc::Receiver<watch::Aim>) {
        let (aim, aimed_at) = std::sync::mpsc::channel();
        (
            Aiming {
                aim,
                at: aimed(edges),
            },
            aimed_at,
        )
    }

    /// Verifies that rewiring an edge replaces existing binding for (node, slot) without affecting others.
    #[test]
    fn a_wire_replaces_the_edge_on_the_input_it_binds_and_leaves_every_other_alone() {
        let mut edges = vec![
            edge("morph", "far", "sphere_shell"),
            edge("morph", "near", "lattice"),
            edge("veil", "far", "torus"),
        ];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        let said = rewired(
            &[(0, edge("morph", "far", "drift_shell"))],
            &mut edges,
            &mut slots,
            1,
        );

        assert_eq!(said.len(), 1);
        assert!(said[0].is_ok(), "{:?}", said[0]);
        assert_eq!(
            edges.len(),
            3,
            "the run is wired with {} edges after replacing one of three: {edges:?}",
            edges.len()
        );
        assert!(
            edges.contains(&edge("morph", "far", "drift_shell")),
            "the edge asked for is not in the run's wiring: {edges:?}"
        );
        assert!(
            edges.contains(&edge("morph", "near", "lattice")),
            "rewiring `morph.far` took `morph.near` with it: {edges:?}"
        );
        assert!(
            edges.contains(&edge("veil", "far", "torus")),
            "rewiring `morph.far` took `veil.far` with it — the key is the node and the \
             input, not the input alone: {edges:?}"
        );
        // And what the watcher was handed is the same list, not the one it
        // started with: a rebuild restates its own edges, so a re-aim that
        // carried the old wiring would put the old edge back on the next build.
        let sent = aims.try_recv().expect("the slot was not re-aimed");
        assert_eq!(sent.edges, edges, "the re-aim carried a different wiring");
    }

    /// Verifies that sequential wire requests on an input replace existing edges rather than creating duplicates.
    #[test]
    fn a_second_wire_on_one_input_replaces_rather_than_appends() {
        let mut edges = vec![edge("morph", "far", "sphere_shell")];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        for to in ["drift_shell", "lattice_shell"] {
            let said = rewired(&[(0, edge("morph", "far", to))], &mut edges, &mut slots, 1);
            assert!(said[0].is_ok(), "{:?}", said[0]);
        }

        assert_eq!(
            edges,
            vec![edge("morph", "far", "lattice_shell")],
            "one input is bound {} times: a Set built from this is refused for \
             `SlotBoundTwice` and the second call is where a model loses its way back",
            edges.len()
        );
        let sent: Vec<watch::Aim> = aims.try_iter().collect();
        assert_eq!(
            sent.len(),
            2,
            "one frame's rewiring did not reach the watcher"
        );
        assert_eq!(sent[1].edges, vec![edge("morph", "far", "lattice_shell")]);
    }

    /// Verifies that multiple wire requests on the same slot in a single frame apply the latest edge.
    #[test]
    fn two_wires_on_one_input_in_one_frame_leave_the_run_wired_with_the_later_one() {
        let mut edges = vec![edge("morph", "far", "sphere_shell")];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        let said = rewired(
            &[
                (0, edge("morph", "far", "drift_shell")),
                (0, edge("morph", "far", "lattice_shell")),
            ],
            &mut edges,
            &mut slots,
            1,
        );

        assert_eq!(
            edges,
            vec![edge("morph", "far", "lattice_shell")],
            "the run is wired with something other than the last edge of the frame"
        );
        let first = said[0].as_ref().expect("the first was applied");
        assert!(
            first.contains("replaced it with `lattice_shell`"),
            "the overwritten request was told its edge stands: {first}"
        );
        assert!(
            said[1]
                .as_ref()
                .expect("the second was applied")
                .contains("recompiling"),
            "the edge the run kept was not reported as rebuilding: {:?}",
            said[1]
        );
        let sent: Vec<watch::Aim> = aims.try_iter().collect();
        assert_eq!(
            sent.len(),
            1,
            "two edges on one frame re-aimed the slot {} times: a frame rebuilds once, at \
             the wiring it ended with",
            sent.len()
        );
        assert_eq!(sent[0].edges, vec![edge("morph", "far", "lattice_shell")]);
    }

    /// Verifies that attempts to wire non-existent slots are rejected without modifying edge configuration.
    #[test]
    fn a_wire_naming_a_slot_this_deck_does_not_hold_is_refused_and_nothing_is_rewired() {
        let mut edges = vec![edge("morph", "far", "sphere_shell")];
        let (aiming, aims) = watcher(edges.clone());
        let mut slots = vec![Some(aiming)];

        // With one that *is* applied in front of it, on the same input: a
        // refusal writes nothing, so the request before it must not be told
        // that a later one replaced its edge.
        let said = rewired(
            &[
                (0, edge("morph", "far", "lattice_shell")),
                (4, edge("morph", "far", "drift_shell")),
            ],
            &mut edges,
            &mut slots,
            1,
        );

        let refusal = said[1]
            .as_ref()
            .expect_err("a slot 4 of a one-slot deck was accepted");
        assert!(refusal.contains("no slot 4"), "{refusal}");
        assert!(refusal.contains("nothing was rewired"), "{refusal}");
        assert_eq!(
            edges,
            vec![edge("morph", "far", "lattice_shell")],
            "a refused request edited the run's wiring anyway"
        );
        let kept = said[0].as_ref().expect("the slot 0 request was applied");
        assert!(
            !kept.contains("replaced it with"),
            "a request that was refused was reported as having replaced the edge in front \
             of it: {kept}"
        );
        let sent: Vec<watch::Aim> = aims.try_iter().collect();
        assert_eq!(
            sent.len(),
            1,
            "a refused request re-aimed a watcher, so a slot is rebuilding for a call that \
             was told nothing happened"
        );
        assert_eq!(sent[0].edges, vec![edge("morph", "far", "lattice_shell")]);
    }

    /// Verifies that rewiring a slot without a watcher updates session edges while noting absence of rebuilds.
    #[test]
    fn a_wire_on_a_slot_with_no_watcher_is_applied_and_says_nothing_rebuilds() {
        let mut edges = Vec::new();
        let mut slots: Vec<Option<Aiming>> = vec![None];

        let said = rewired(
            &[(0, edge("morph", "far", "drift_shell"))],
            &mut edges,
            &mut slots,
            1,
        );

        let line = said[0]
            .as_ref()
            .expect("a run without a watcher refused an edge");
        assert!(
            line.contains("no watcher") && line.contains("save_set"),
            "{line}"
        );
        assert_eq!(edges, vec![edge("morph", "far", "drift_shell")]);
    }

    /// Verifies statically that `Live::run_requests` processes both saves and wire updates.
    #[test]
    fn a_frame_takes_the_edges_a_client_asked_for_and_not_only_the_saves() {
        let source = include_str!("../live/mod.rs");
        let body = source
            .split_once("\n    fn run_requests(&mut self) {")
            .expect("`Live::run_requests` is no longer spelled that way")
            .1;
        let body = body
            .split("\n    }")
            .next()
            .expect("the end of `Live::run_requests`");
        let code: Vec<&str> = body
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        let code = code.join("\n");
        for call in ["mcp.wires()", "self.rewire(wires)"] {
            assert!(
                code.contains(call),
                "`Live::run_requests` does not `{call}`: a `wire_input` call on this run \
                 waits out its deadline and is told the loop never took the edge"
            );
        }
    }

    // -- over the socket -------------------------------------------------

    /// One tool call, over TCP exactly as a client makes it. The answer is the
    /// thing being tested, so nothing here shares a channel with the server — it is
    /// the wire.
    fn call(port: u16, name: &str, args: serde_json::Value) -> (bool, String) {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": name, "arguments": args},
        })
        .to_string();
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .expect("read timeout");
        stream
            .write_all(
                format!(
                    "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .expect("write");
        let mut reader = std::io::BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).expect("status line");
        let mut length = 0usize;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).expect("header");
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
        }
        let mut body = vec![0u8; length];
        std::io::Read::read_exact(&mut reader, &mut body).expect("body");
        let reply: serde_json::Value =
            serde_json::from_slice(&body).expect("the server answered something that is not JSON");
        let result = &reply["result"];
        (
            result["isError"].as_bool().unwrap_or(true),
            result["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        )
    }

    /// Verifies end-to-end that an MCP `wire_input` request is applied and answered at frame execution.
    #[test]
    fn a_wire_request_is_answered_at_the_frame_it_was_applied_on() {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        std::fs::write(&l1, "proc probe { kind L1 }").expect("fixture");
        let reporter = mcp::serve(
            0,
            mcp::Slots::of(vec![(l1, Vec::new())]),
            dir.path().join("store"),
            true,
            karakuri_environment::Opening::closed(),
            karakuri_environment::SlotPolicies::default(),
        )
        .expect("serve");
        let port = reporter.port();

        // What the run is wired with, shared so the test can read it back —
        // the render loop's own copy is `Live::edges` and nothing else holds
        // one.
        let edges = Arc::new(std::sync::Mutex::new(vec![edge(
            "morph",
            "far",
            "sphere_shell",
        )]));
        let (aiming, aims) = watcher(edges.lock().expect("fresh mutex").clone());
        let run = edges.clone();
        // The thread never ends, which is what keeps the reporter alive: a
        // dropped reporter is a run that has quit, and the tool has a
        // different true sentence for that.
        std::thread::spawn(move || {
            let mut slots = vec![Some(aiming)];
            loop {
                let asked: Vec<mcp::WireRequest> = reporter.wires().collect();
                if !asked.is_empty() {
                    let mut wires = Vec::new();
                    let mut replies = Vec::new();
                    for mcp::WireRequest { slot, edge, reply } in asked {
                        wires.push((slot, edge));
                        replies.push(reply);
                    }
                    let mut held = run.lock().expect("the run's wiring");
                    let said = rewired(&wires, &mut held, &mut slots, 1);
                    drop(held);
                    for (reply, said) in replies.into_iter().zip(said) {
                        reply.settled(said);
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        });

        let (failed, said) = call(
            port,
            "wire_input",
            serde_json::json!({"slot": 0, "node": "morph", "input": "far", "to": "drift_shell"}),
        );
        assert!(!failed, "the call came back as a failure: {said}");
        assert!(
            said.contains("wired `morph.far=drift_shell`"),
            "the client was told something other than what the loop did: {said}"
        );
        assert_eq!(
            *edges.lock().expect("the run's wiring"),
            vec![edge("morph", "far", "drift_shell")],
            "the client was answered and the run is not wired with the edge"
        );
        assert_eq!(
            aims.try_recv().expect("the slot was not re-aimed").edges,
            vec![edge("morph", "far", "drift_shell")],
            "the edge was written and nothing was asked to rebuild with it"
        );
    }

    /// Verifies that unsupported operations are rejected over MCP wire while supported operations succeed.
    #[test]
    fn an_operation_this_program_has_no_control_for_is_refused_over_the_wire() {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        std::fs::write(&l1, "proc probe { kind L1 }").expect("fixture");
        // Gate MixFaders open for testing post-audit tool dispatch (ADR-0301, ADR-0341).
        let opening = karakuri_environment::Opening::closed();
        opening.set(
            karakuri_operation::gate::Open::CLOSED
                .with(karakuri_operation::gate::Class::MixFaders, true),
        );
        let reporter = mcp::serve(
            0,
            mcp::Slots::of(vec![(l1, Vec::new())]),
            dir.path().join("store"),
            true,
            opening,
            karakuri_environment::SlotPolicies::default(),
        )
        .expect("serve");
        let port = reporter.port();

        // What the loop wrote, so that *nothing was performed* is asserted as
        // well as *the call failed*. The thread never ends, which is what keeps
        // the reporter alive: a dropped reporter is a run that has quit, and
        // the tool has a different true sentence for that.
        let written = Arc::new(std::sync::Mutex::new(Vec::new()));
        let taken = written.clone();
        std::thread::spawn(move || loop {
            for mcp::OperateRequest { operation, reply } in reporter.operations() {
                // `Current::default()` is every reading absent, which is what
                // this stand-in has: it holds no deck. Neither operation below
                // asks for one — a gain carries everything its record says.
                match answered(&operation, &Current::default()) {
                    Ok(records) => {
                        taken.lock().expect("what the loop wrote").extend(records);
                        reply.settled(Ok(performed_at_the_frame(operation.title())));
                    }
                    Err(said) => reply.settled(Err(said)),
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        });

        let (failed, said) = call(
            port,
            "operate",
            serde_json::json!({
                "operation": "Star a Set, or take the star off",
                "with": {"set": "a_set", "favourite": true},
            }),
        );
        assert!(
            failed,
            "a star nothing on this surface performs was answered as a success: {said}"
        );
        assert!(
            said.contains("`Star a Set, or take the star off` was not performed"),
            "the refusal does not name the operation a model asked for: {said}"
        );
        assert!(
            said.contains("has no control for this one"),
            "the refusal does not say this program cannot perform it: {said}"
        );
        assert!(
            written.lock().expect("what the loop wrote").is_empty(),
            "the call was refused and the loop wrote a record anyway"
        );

        let (failed, said) = call(
            port,
            "operate",
            serde_json::json!({"operation": "Gain", "with": {"deck": 0, "gain": 0.8}}),
        );
        assert!(
            !failed,
            "a fader this surface does perform was refused with it: {said}"
        );
        assert!(
            said.contains("was performed on the frame it arrived on"),
            "the client was told something other than what the loop did: {said}"
        );
        assert_eq!(
            *written.lock().expect("what the loop wrote"),
            vec![Record::Gain {
                slot: DeckSlot(0),
                value: 0.8
            }],
            "the client was answered `ok` and the loop wrote something else"
        );
    }
}
