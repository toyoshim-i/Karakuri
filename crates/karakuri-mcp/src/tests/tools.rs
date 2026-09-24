use super::*;

// -- what the audit of this file against the engine turned up -----------

/// The description one published tool carries.
fn description(name: &str) -> String {
    tools()
        .as_array()
        .expect("tools() is an array")
        .iter()
        .find(|tool| tool["name"] == json!(name))
        .unwrap_or_else(|| panic!("`{name}` is not published"))["description"]
        .as_str()
        .expect("a tool has a description")
        .to_string()
}

/// Verifies that `write_procedure` documentation accurately reflects validation boundaries.
#[test]
fn write_procedure_says_what_a_clean_write_does_not_promise() {
    let described = description("write_procedure");
    assert!(
        described.contains("check is of this procedure alone"),
        "a clean write is not a slot that builds, and the description does not say so: \
         {described}"
    );
    assert!(
        described.contains("swap_outcome"),
        "nothing points at where the build failure will show up: {described}"
    );
    assert!(
        described.contains("`uses`") && described.contains("`edge`"),
        "a `uses` and the edge it needs are not mentioned: {described}"
    );
    assert!(
        described.contains("wire_input"),
        "the tool that writes the `edge` a `uses` needs is not named where a model \
         writing a `uses` will read it: {described}"
    );
}

/// Verifies that tool schemas publish input constraints like minimum slot indices.
#[test]
fn a_schema_states_the_constraints_its_tool_enforces() {
    for tool in tools().as_array().expect("tools() is an array") {
        let name = tool["name"].as_str().expect("a tool has a name");
        let properties = &tool["inputSchema"]["properties"];
        for counted in ["slot", "index"] {
            let Some(schema) = properties.get(counted) else {
                continue;
            };
            assert_eq!(
                schema.get("minimum"),
                Some(&json!(0)),
                "`{name}`'s `{counted}` counts from 0 and the schema does not say so"
            );
        }
        let Some(schema) = properties.get("id") else {
            continue;
        };
        assert_eq!(
            schema.get("pattern"),
            Some(&json!("^[A-Za-z0-9_-]+$")),
            "`{name}`'s `id` is put through `checked_id` and the schema does not \
             publish what it accepts"
        );
        assert_eq!(
            schema.get("maxLength"),
            Some(&json!(MAX_ID)),
            "`{name}`'s `id` becomes a file name and the schema does not publish the \
             length"
        );
    }

    // **And the published rule is the one that is enforced.** The pattern
    // above is a literal here on purpose: a `checked_id` that started
    // accepting a dot, or stopped accepting a dash, would leave the schema
    // describing a tool that no longer exists.
    assert!(checked_id("plain_id-9").is_ok());
    for refused in ["a.b", "a b", "../x", ""] {
        assert!(
            checked_id(refused).is_err(),
            "`{refused}` is outside the pattern the schema publishes and was accepted"
        );
    }
    assert!(checked_id(&"x".repeat(MAX_ID)).is_ok());
    assert!(checked_id(&"x".repeat(MAX_ID + 1)).is_err());
}

/// Verifies that `save_set` description mentions all elements it persists.
#[test]
fn save_set_names_everything_it_writes() {
    let described = description("save_set");
    for written in [
        "parameter",
        "capacit",
        "binding",
        "seed",
        "edge",
        "composite",
        "live",
        "camera",
    ] {
        assert!(
            described.contains(written),
            "`setfile::save` writes the {written} records and the description does not \
             mention them: {described}"
        );
    }
}

// -- the edge ----------------------------------------------------------

/// Verifies that edge endpoints are addressed by names rather than numeric node indices.
#[test]
fn an_edge_is_named_at_both_ends_and_never_addressed_by_a_position() {
    let properties = tools()
        .as_array()
        .expect("tools() is an array")
        .iter()
        .find(|tool| tool["name"] == json!("wire_input"))
        .expect("`wire_input` is published")["inputSchema"]["properties"]
        .clone();
    for named in ["node", "to"] {
        assert_eq!(
            properties[named]["type"],
            json!("string"),
            "`{named}` is one end of an edge and an end of an edge is a name"
        );
    }
    for positional in ["layer", "index"] {
        assert!(
            properties.get(positional).is_none(),
            "`wire_input` takes `{positional}` — an edge spelled as a position is the \
             one thing `Record::Edge` exists to prevent"
        );
    }
    // Verify that the deck's slot number and the declared input slot map correctly.
    let asked = asked(
        "wire_input",
        &json!({"slot":0,"node":"morph","input":"far","to":"sphere_shell"}),
        &slots(),
    )
    .expect("a published tool");
    let Asked::Named(Operation::WireInput {
        deck,
        node,
        slot,
        to,
    }) = asked
    else {
        panic!("`wire_input` no longer names `WireInput`");
    };
    assert_eq!(deck, 0, "`slot` on the wire is the deck slot");
    assert_eq!(node, "morph");
    assert_eq!(
        slot,
        "far".into(),
        "`input` on the wire is the operation's `slot`"
    );
    assert_eq!(to, "sphere_shell");
}

/// Verifies that `wire_input` description explains edge replacement semantics and limits.
#[test]
fn wire_input_says_what_it_replaces_and_what_it_cannot_take_back() {
    let described = description("wire_input");
    assert!(
        described.contains("replaced"),
        "an edge already binding this input is replaced, and a model reading this \
         would not know it: {described}"
    );
    assert!(
        described.contains("unbinds"),
        "nothing here takes an edge back, and the description does not say so — which \
         is the trap this tool otherwise leaves behind it: {described}"
    );
    assert!(
        described.contains("swap_outcome"),
        "the names are refused where the Set is built and nothing points at where \
         that refusal will show up: {described}"
    );
    assert!(
        described.contains("names, not") || described.contains("names and not"),
        "an edge is named at both ends and the description does not say why it is not \
         an address: {described}"
    );
}

/// Verifies that timeout handling for rewiring operations reports truthful error statuses.
#[test]
fn an_edge_the_loop_does_not_take_ends_and_says_something_true() {
    // Never taken. Nothing was rewired and saying so is safe.
    let (held, news) = mpsc::channel::<News>();
    let started = std::time::Instant::now();
    let said = applied(&news, std::time::Duration::from_millis(60))
        .expect_err("an edge nobody took came back as a success");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "the wait ran past its bound"
    );
    assert!(said.contains("Nothing was rewired"), "{said}");
    drop(held);

    // Taken, then silence. Neither a success nor a failure, and it does not
    // tell a model to write the edge again.
    let (tx, news) = mpsc::channel();
    tx.send(News::Accepted("slot 0: taking an edge".to_string()))
        .expect("accepted");
    let said = applied(&news, std::time::Duration::from_millis(60))
        .expect_err("an edge with no outcome came back as a success");
    assert!(said.contains("slot 0: taking an edge"), "{said}");
    assert!(
        said.contains("neither a success nor a failure"),
        "an edge with no outcome was reported as one or the other: {said}"
    );

    // The loop ended without answering: told at once rather than at the
    // deadline, which the long wait here is what proves.
    let (tx, news) = mpsc::channel::<News>();
    drop(tx);
    let started = std::time::Instant::now();
    let said = applied(&news, std::time::Duration::from_secs(60))
        .expect_err("a loop that ended came back as a success");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "a client waited out the deadline on a loop that was already gone"
    );
    assert!(
        said.contains("nothing was rewired"),
        "a loop that ended before taking the edge said something else: {said}"
    );

    // And the loop's own answer is what a client gets when there is one.
    let (tx, news) = mpsc::channel();
    Reply(tx).settled(Ok(
        "slot 0: `morph.far` is bound to `sphere_shell`".to_string()
    ));
    assert_eq!(
        applied(&news, std::time::Duration::from_secs(60)),
        Ok("slot 0: `morph.far` is bound to `sphere_shell`".to_string()),
        "what the loop said did not come back unchanged"
    );
}
