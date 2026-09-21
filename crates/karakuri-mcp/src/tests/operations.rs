use super::*;

/// Every tool this server publishes names an operation the gate lets through,
/// which is ADR-0235's promise that nothing closes on the day it is recorded:
/// *"the seven tools that exist are unaffected. Nothing closes today and no
/// model loses a call it could make yesterday."*
///
/// The names come from [`tools`] rather than from a list written here, so an
/// eighth tool that named a closed operation would fail this rather than slip
/// past a fixture that had not heard of it. The arguments are the smallest each
/// tool accepts — what is asserted is that [`asked`] names an operation and
/// that [`audited`] lets it by, not what the tool then does with it.
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
        // **`operate` names thirty operations and twenty-nine of them are
        // closed**, so what is driven here is the one the audit lets
        // through. That is not this test going soft: the promise it holds
        // is *a tool this server publishes is callable today*, and for
        // `operate` that promise is about the tool rather than about every
        // name it takes. Which name stands where is
        // `every_operation_operate_takes_stands_where_the_page_says_it_does`.
        (
            "operate",
            json!({
                "operation": "Put a node's previous version back",
                "with": {"deck": 0, "revision": {"previous": {"layer": "L4"}}},
            }),
        ),
    ];
    // Sorted, because the order a tool is published in is `tools()`'s to
    // choose and is not what this is about.
    let mut published: Vec<String> = tools()
        .as_array()
        .expect("a list of tools")
        .iter()
        .map(|tool| tool["name"].as_str().expect("a name").to_string())
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
                "`{name}` names `{}`, and ADR-0235 puts all seven tools in the open set: \
                 {refused}",
                operation.title()
            )
        });
    }
}

/// Every operation of the vocabulary is spelled here, once, and this is the
/// test that stops a sixty-fifth arriving without an answer.
///
/// [`sayable`] already stops the build, so this catches the other half: a row
/// of [`SPELLED`] whose heading no longer exists, and a heading with two rows.
/// Both directions, because they are two different mistakes.
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
    // The order is the page's, which is what makes the table readable
    // beside the manual and what `operations()` publishes.
    assert_eq!(spelled, Operation::TITLES.to_vec());
}

/// The table and the classification agree: a row has a spelling exactly where
/// [`sayable`] says this surface can name the operation.
///
/// Two lists that could disagree are what this crate exists to abolish, and
/// these two genuinely can: `make` is written per row and [`sayable`] is
/// written per variant. So they are checked against each other rather than kept
/// in step by hand.
#[test]
fn the_table_and_the_classification_agree() {
    for row in SPELLED {
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

/// Every operation `operate` takes round-trips through the spelling.
///
/// A table test over [`SPELLED`], which is what makes the sample beside each
/// `make` one statement rather than two: the call written there has to come
/// back as the operation written beside it, for all thirty, or the shape a
/// client is told and the shape the server reads have come apart.
///
/// Watched to fail: with `deck_of` reading `"deck"` where
/// `Operation::Crossfade` says `from`, six rows come back with the wrong deck
/// and this names each of them.
#[test]
fn every_operation_operate_takes_round_trips_through_the_wire() {
    let slots = slots();
    let mut checked = 0;
    for row in SPELLED {
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

/// A name this vocabulary does not carry is refused naming the nearest, which
/// is
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
/// the mistake a model makes here is a paraphrase, and the next attempt needs
/// the heading rather than a list of sixty-four.
///
/// And a name it does carry but this surface will not take is refused with its
/// own reason, which is the other half: a model told only *no* about `Read one
/// node's source` would go looking for a tool that is sitting right there.
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

    // **The walk is a tool of its own since 2026-09-10**, and this case
    // was *its payload is undecided* until then: settling the payload did
    // not make `operate` take it, because what decides that is whether the
    // work is something only this server can do, and a walk reads the
    // store (`docs/adr/0342-…`).
    let refusal = operated(&json!({ "operation": "Walk the edit history" }), &slots)
        .expect_err("`walk_history` is that row's tool");
    assert!(refusal.contains("walk_history"), "{refusal}");

    // **The two rows the page marks `gap` for MCP**, each refused with its
    // own sentence rather than with one answer covering both: a divider is
    // a surface's own state and a watched file is an event a model does not
    // perform, and a model told the wrong one of those would go looking for
    // the wrong thing (`docs/adr/0342-…`).
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

    // **And the row that is waiting for nothing.** This case named a row
    // *waiting for a performer* until 2026-09-10 — *Record the session*,
    // then ADR-0338's two as each moved — and there is no such row left:
    // `Sayable::Unperformed` went with its last one (ADR-0341). The
    // `Undecided` answer beside it went the same way on the same day, when
    // the last two rows carrying it turned out to be `gap` for reasons of
    // their own — so a `plan` badge in the MCP column now means nothing at
    // all. What is left is this answer, and it reads differently on
    // purpose: a model told *not yet* about a send would go on asking,
    // where what it needs is the flag its operator has.
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

/// A closed operation is refused at the seam every tool crosses, in the one
/// sentence.
///
/// Driven through [`audited`] rather than over the wire because no tool names a
/// closed operation today — ADR-0235 puts all seven in the open set — so the
/// seam is the only place this is reachable until the floodgate opens. Asserted
/// by equality against `karakuri_operation::gate::refusal`, which is P-0090:
/// one refusal, one sentence, and no second spelling of it in this crate.
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
