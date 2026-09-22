#![allow(unused_imports)]

use super::wire_common::*;

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
    let raw = call_raw(port, "copy_slot", json!({"from_slot": 0, "to_slot": 1}));
    assert_eq!(raw["result"]["refusal"]["code"], "SLOT_IN_MIX");
    assert_eq!(raw["result"]["refusal"]["slot"], 1);
    assert_eq!(raw["result"]["refusal"]["policy"], "auto");
    assert_eq!(raw["result"]["refusal"]["in_mix"], true);

    // Also write_procedure to slot 1 is refused
    let (failed_write, text_write) = call(
        port,
        "write_procedure",
        json!({"slot": 1, "layer": "L1", "source": PROBE_L1}),
    );
    assert!(failed_write);
    assert!(text_write.contains("active in the mix"), "{text_write}");
    let raw_write = call_raw(
        port,
        "write_procedure",
        json!({"slot": 1, "layer": "L1", "source": PROBE_L1}),
    );
    assert_eq!(raw_write["result"]["refusal"]["code"], "SLOT_IN_MIX");

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
    let raw_rend = call_raw(
        port,
        "operate",
        json!({
            "operation": "Choose which renderer of a deck is live",
            "with": {"deck": 1, "renderer": 0}
        }),
    );
    assert_eq!(raw_rend["result"]["refusal"]["code"], "SLOT_IN_MIX");

    // Also SetCompositing on slot 1 is refused when in mix
    let (failed_comp, text_comp) = call(
        port,
        "operate",
        json!({
            "operation": "Composite a deck's renderers",
            "with": {"deck": 1, "compositing": true}
        }),
    );
    assert!(failed_comp);
    assert!(text_comp.contains("active in the mix"), "{text_comp}");
    let raw_comp = call_raw(
        port,
        "operate",
        json!({
            "operation": "Composite a deck's renderers",
            "with": {"deck": 1, "compositing": true}
        }),
    );
    assert_eq!(raw_comp["result"]["refusal"]["code"], "SLOT_IN_MIX");

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
    let raw_off = call_raw(port, "copy_slot", json!({"from_slot": 0, "to_slot": 1}));
    assert_eq!(raw_off["result"]["refusal"]["code"], "SLOT_POLICY_OFF");
    assert_eq!(raw_off["result"]["refusal"]["slot"], 1);
    assert_eq!(raw_off["result"]["refusal"]["policy"], "off");

    // 3b. Unallocated slot returns SLOT_UNALLOCATED refusal
    let raw_unalloc = call_raw(port, "copy_slot", json!({"from_slot": 0, "to_slot": 99}));
    assert_eq!(raw_unalloc["result"]["refusal"]["code"], "SLOT_UNALLOCATED");
    assert_eq!(raw_unalloc["result"]["refusal"]["slot"], 99);

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
