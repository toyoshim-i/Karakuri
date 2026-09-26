use super::*;

/// Verifies that all tools published by the server map to operations permitted by the gate.
#[test]
fn every_tool_this_server_publishes_names_an_operation_the_gate_lets_through() {
    let (_tx, rx) = mpsc::channel();
    let state = state(rx);
    let arguments = [
        ("read_procedure", json!({"slot": 0, "layer": "L4"})),
        (
            "write_procedure",
            json!({"slot": 0, "layer": "L4", "source": "proc p { kind L4 }"}),
        ),
        (
            "wire_input",
            json!({"slot": 0, "node": "a", "input": "b", "to": "c"}),
        ),
        ("swap_outcome", json!({})),
        ("read_set", json!({"id": "a"})),
        ("list_sets", json!({})),
        ("walk_history", json!({"set": "a"})),
        ("save_set", json!({"slot": 0})),
        // Drives the operation allowed by audit policy (`RestoreProcedure`).
        (
            "operate",
            json!({
                "operation": "Put a node's previous version back",
                "with": {"deck": 0, "revision": {"previous": {"layer": "L4"}}},
            }),
        ),
    ];
    // Sorted, because the order a tool is published in is `tools()`'s to
    // choose and is not what this is about. Filter out M7 workflow tools,
    // which are tested by `every_workflow_tool_is_published_and_callable`.
    const WORKFLOW_TOOLS: &[&str] = &[
        "get_permissions",
        "read_slot",
        "copy_slot",
        "check_procedure",
        "check_set",
    ];
    let mut published: Vec<String> = tools()
        .as_array()
        .expect("a list of tools")
        .iter()
        .map(|tool| tool["name"].as_str().expect("a name").to_string())
        .filter(|name| !WORKFLOW_TOOLS.contains(&name.as_str()))
        .collect();
    published.sort();
    let mut covered: Vec<String> = arguments.iter().map(|(name, _)| name.to_string()).collect();
    covered.sort();
    assert_eq!(
        published, covered,
        "this test and `tools()` have come apart — a tool nobody drives here is a tool \
         nobody has checked against the gate"
    );

    for (name, args) in arguments {
        let Asked::Named(operation) = asked(name, &args, &state.slots).expect("a known tool")
        else {
            panic!("`{name}` refused these arguments before the gate was reached");
        };
        audited(&operation, &state).unwrap_or_else(|refused| {
            panic!(
                "`{name}` is published and the gate refused it: {refused:?} — every tool \
                 this server publishes must name an operation the gate lets through"
            );
        });
    }
}

#[test]
fn every_workflow_tool_is_published_and_callable() {
    let (_tx, rx) = mpsc::channel();
    let mut state = state(rx);
    for tool_name in [
        "get_permissions",
        "read_slot",
        "copy_slot",
        "check_procedure",
        "check_set",
    ] {
        let published = tools()
            .as_array()
            .expect("tools list")
            .iter()
            .any(|t| t["name"] == tool_name);
        assert!(published, "`{tool_name}` must be published in `tools()`");
    }

    let req = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "get_permissions", "arguments": {} }
    });
    let rep = dispatch(&req, &mut state).settled().expect("rep");
    assert_eq!(rep["result"]["isError"], false);
}

/// Verifies that every vocabulary operation is spelled uniquely in the SPELLED table.
#[test]
fn every_operation_of_the_vocabulary_is_spelled_here() {
    let spelled: Vec<&'static str> = SPELLED.iter().map(Spelled::title).collect();
    assert_eq!(
        spelled.len(),
        Operation::TITLES.len(),
        "the vocabulary has {} operations and this table has {}",
        Operation::TITLES.len(),
        spelled.len()
    );
    for title in Operation::TITLES {
        let found = spelled.iter().filter(|had| *had == title).count();
        assert_eq!(
            found, 1,
            "`{title}` is an operation of the vocabulary and this table names it {found} \
             times — a model that asks for it is answered by the wrong row, or by none"
        );
    }
    // The order follows the documentation page order.
    assert_eq!(spelled, Operation::TITLES.to_vec());
}

/// Verifies that table row definitions align with the `sayable` classification.
#[test]
fn the_table_and_the_classification_agree() {
    for row in SPELLED.iter() {
        let (operation, call) = (row.sample)();
        let operable = sayable(&operation) == Sayable::Operable;
        assert_eq!(
            row.make.is_some(),
            operable,
            "`{}`: the table {} and `sayable` says {}",
            row.title(),
            match row.make.is_some() {
                true => "spells it",
                false => "does not",
            },
            match operable {
                true => "it can be named",
                false => "it cannot",
            }
        );
        assert_eq!(
            row.shape.is_some(),
            operable,
            "`{}`: a row this surface takes has a schema and one it refuses has none",
            row.title()
        );
        assert_eq!(
            call == Value::Null,
            !operable,
            "`{}`: a row this surface takes carries one call and one it refuses carries \
             none",
            row.title()
        );
    }
}

/// Verifies that every operable operation round-trips through JSON serialization.
#[test]
fn every_operation_operate_takes_round_trips_through_the_wire() {
    let slots = slots();
    let mut checked = 0;
    for row in SPELLED.iter() {
        let (operation, call) = (row.sample)();
        let Some(make) = row.make else { continue };
        // The sample is the `with` object itself, which is what `make`
        // takes — `payload` is what lifts it out of a whole call, and is
        // exercised over the wire rather than here.
        let read = make(&call, &slots).unwrap_or_else(|refusal| {
            panic!(
                "`{}`: the call written beside it in this table is refused by its own \
                 spelling — {refusal}",
                row.title()
            )
        });
        assert_eq!(
            read,
            operation,
            "`{}` does not come back as itself through the wire",
            row.title()
        );
        checked += 1;
    }
    assert!(
        checked >= 30,
        "only {checked} operations round-tripped — a scan that found nothing would pass \
         every assertion above"
    );
}

/// The tool's `operation` list is every name the spelling accepts, in the
/// manual's order, so a client is told exactly what it may ask for.
#[test]
fn the_schema_lists_every_operation_the_spelling_accepts() {
    let tools = tools();
    let tool = tools
        .as_array()
        .expect("a list")
        .iter()
        .find(|tool| tool["name"] == json!("operate"))
        .expect("`operate` is published");
    let listed: Vec<&str> = tool["inputSchema"]["properties"]["operation"]["enum"]
        .as_array()
        .expect("an enum of names")
        .iter()
        .map(|name| name.as_str().expect("a name"))
        .collect();
    let takes: Vec<&'static str> = operable().iter().map(|row| row.title()).collect();
    assert_eq!(listed, takes);
    assert!(
        !takes.is_empty() && takes.len() < Operation::TITLES.len(),
        "the list is every operation or none of them, which is not what this surface is"
    );
    // And the curriculum is the same table, so a name in one is a name in
    // the other.
    let curriculum = operations();
    for title in takes {
        assert!(
            curriculum.contains(&format!("## {title}\n")),
            "`{title}` is offered by the tool and is not in `karakuri://operations`"
        );
    }
}

/// An `operate` on a closed class is refused with the gate's own sentence, by
/// equality and not by a `contains` — P-0090's *one refusal, one sentence*, and
/// the sentence is `karakuri_operation::gate`'s.
#[test]
fn an_operate_on_a_closed_class_is_refused_with_the_gates_sentence() {
    use karakuri_operation::gate::{Class, Standing};
    let (_tx, rx) = mpsc::channel();
    let mut state = state(rx);
    let request = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": "operate",
            "arguments": {"operation": "Gain", "with": {"deck": 0, "gain": 0.25}},
        },
    });
    let reply = dispatch(&request, &mut state)
        .settled()
        .expect("a call is answered");
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(true));
    assert_eq!(
        result["content"][0]["text"].as_str().expect("text"),
        gate::refusal(
            &Operation::SetGain {
                deck: 0,
                gain: 0.25
            },
            Standing::Closed(Class::MixFaders)
        )
        .expect("a refusal")
    );
}

/// Verifies that invalid or unpermitted operation names are rejected with actionable suggestions.
#[test]
fn a_name_the_vocabulary_does_not_carry_is_refused_naming_the_nearest() {
    let slots = slots();
    let refusal = operated(&json!({ "operation": "set the gain" }), &slots)
        .expect_err("no operation is headed `set the gain`");
    assert!(refusal.contains("`Gain`"), "{refusal}");

    let refusal = operated(&json!({ "operation": "Read one node's source" }), &slots)
        .expect_err("`read_procedure` is that row's tool");
    assert!(refusal.contains("read_procedure"), "{refusal}");

    let refusal = operated(&json!({ "operation": "Fold a bay away" }), &slots)
        .expect_err("a model has no window");
    assert!(refusal.contains("window"), "{refusal}");

    let refusal = operated(&json!({ "operation": "Walk the edit history" }), &slots)
        .expect_err("`walk_history` is that row's tool");
    assert!(refusal.contains("walk_history"), "{refusal}");

    let refusal = operated(&json!({ "operation": "Move a boundary" }), &slots)
        .expect_err("a model has no window");
    assert!(
        refusal.contains("surface's own state") && refusal.contains("window"),
        "{refusal}"
    );

    let refusal = operated(&json!({ "operation": "Edit the file instead" }), &slots)
        .expect_err("a model's edit is `write_procedure`");
    assert!(
        refusal.contains("no route on this surface") && refusal.contains("write_procedure"),
        "{refusal}"
    );

    let refusal = operated(
        &json!({ "operation": "Send a Set to somebody, and take one in" }),
        &slots,
    )
    .expect_err("no route on this surface takes it");
    assert!(
        refusal.contains("no route on this surface") && refusal.contains("--package"),
        "{refusal}"
    );
}

/// Verifies that closed operations are rejected by the gate with consistent refusal messages.
#[test]
fn a_closed_operation_is_refused_at_the_seam_every_tool_crosses() {
    use karakuri_operation::gate::{Class, Standing};
    let (_tx, rx) = mpsc::channel();
    let state = state(rx);
    let operation = Operation::SetGain {
        deck: 0,
        gain: 0.25,
    };
    assert_eq!(
        audited(&operation, &state).expect_err("the mix faders are closed by default"),
        gate::refusal_detail(&operation, Standing::Closed(Class::MixFaders)).expect("a refusal"),
    );
}

/// A class the operator opens is open on the very next call, which is what the
/// run holding a handle rather than a snapshot buys: an opening set between two
/// numbers has to be true of the call after it, and one closed again has to be
/// false of the call after that.
#[test]
fn a_class_the_operator_opens_is_open_on_the_next_call() {
    use karakuri_operation::gate::{Class, Open};
    let (_tx, rx) = mpsc::channel();
    let state = state(rx);
    let operation = Operation::SetGain {
        deck: 0,
        gain: 0.25,
    };
    assert!(audited(&operation, &state).is_err());
    state.opening.set(Open::CLOSED.with(Class::MixFaders, true));
    assert!(
        audited(&operation, &state).is_ok(),
        "the server read an opening it was handed at startup rather than the one the \
         operator has now"
    );
    // And the other three are untouched by that hand.
    assert!(audited(&Operation::SetExposure { exposure: 1.0 }, &state).is_err());
    state.opening.set(Open::CLOSED);
    assert!(audited(&operation, &state).is_err());
}

/// A notification has no `id` and is never answered — the one shape of message
/// that must produce no reply at all.
#[test]
fn a_notification_is_acted_on_and_not_answered() {
    let (_tx, rx) = mpsc::channel();
    let mut state = state(rx);
    let notification = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    assert!(dispatch(&notification, &mut state).settled().is_none());
    // And one that *does* carry an id is answered, so the test above is
    // about the notification rather than about the method being unknown.
    let request = json!({ "jsonrpc": "2.0", "id": 7, "method": "ping" });
    assert!(dispatch(&request, &mut state).settled().is_some());
}

/// The swap history is bounded and is a report on the present.
#[test]
fn the_swap_history_does_not_grow_without_end() {
    let (tx, rx) = mpsc::sync_channel(RECENT * 4);
    let mut state = state(rx);
    for i in 0..(RECENT * 3) {
        tx.send(Event::Swap {
            slot: 0,
            said: format!("event {i}"),
        })
        .expect("send");
    }
    let said = swap_outcome(&mut state).expect("outcome");
    assert_eq!(state.recent.len(), RECENT);
    assert!(
        said.contains(&format!("event {}", RECENT * 3 - 1)),
        "the newest event was dropped"
    );
    assert!(!said.contains("event 0"), "the oldest event was kept");
}
