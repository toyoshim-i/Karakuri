use std::sync::mpsc;

use karakuri_ir::Kind;
use serde_json::{json, Value};

use super::server::*;
use super::*;

/// A state with the loop's half of both channels missing, for the tests that
/// are about what one method answers rather than about a render loop.
///
/// The save channel's receiver is dropped on the way out, which is exactly the
/// "the loop is gone" case: anything that tried to ask for a save here would be
/// told so rather than wait.
fn state(events: mpsc::Receiver<Event>) -> State {
    State {
        slots: slots(),
        // **A root, and nothing here opens it.** Only `read_set` does, on
        // the call, which is what lets every test in this module build a
        // state without a directory — see `serve`.
        store: "a/store".into(),
        watching: true,
        // **Closed, all four classes**, which is the state a run starts in
        // and the state every test in this module reasons under.
        opening: karakuri_environment::Opening::closed(),
        events,
        asked: mpsc::sync_channel(ASKED).0,
        wiring: mpsc::sync_channel(ASKED).0,
        operating: mpsc::sync_channel(ASKED).0,
        recent: Vec::new(),
        dropped: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
    }
}

/// Two slots of a head and one more file, and none of these paths exists.
///
/// That is deliberate rather than lazy. A layer is read off a file's own `kind`
/// line, and a file that cannot be read counts as a renderer — the fallback
/// [`Slots::nodes`] shares with `history::seed`, asserted in
/// [`an_unreadable_file_is_counted_as_a_renderer`] and relied on here, so these
/// two slots are the L1-and-one-renderer pair they read as.
fn slots() -> Slots {
    Slots::of(vec![
        ("a/l1.kir".into(), vec!["a/l4.kir".into()]),
        ("b/l1.kir".into(), vec!["b/l4.kir".into()]),
    ])
}

/// Writes `name` declaring `kind`, and nothing that would compile.
///
/// A layer is scanned out of the text rather than parsed, so that this surface
/// works on a file the checker would refuse — which is the file a model most
/// needs to be able to read. A fixture that compiled would not say so.
fn declaring(dir: &std::path::Path, name: &str, kind: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, format!("kind {kind}\nnot a procedure at all\n")).expect("fixture");
    path
}

/// The text under one `# ` heading of the rendered vocabulary.
///
/// The two halves of the test below ask opposite questions of one section each,
/// and mixing sections silently weakens both — the "nothing invented" half went
/// looking for `clip` in `Builtin::from_name` the moment stage outputs were
/// added to the page, which is the failure working rather than a nuisance.
fn section<'a>(rendered: &'a str, heading: &str) -> &'a str {
    let start = rendered
        .find(heading)
        .unwrap_or_else(|| panic!("the vocabulary has no `{heading}` section"));
    let rest = &rendered[start + heading.len()..];
    match rest.find("\n# ") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

/// The vocabulary is generated, so it cannot say a function exists that does
/// not. That is the whole reason it is served beside the prose spec:
/// `docs/ir-spec.md` describes this language in English and English goes stale,
/// where this list is the one the checker matches against.
#[test]
fn the_vocabulary_is_the_checkers_own_table() {
    let rendered = vocabulary();
    let builtins = section(&rendered, "# Built-in functions");
    for builtin in karakuri_ir::builtin::Builtin::ALL {
        assert!(
            builtins.contains(&format!("| `{}` |", builtin.name())),
            "`{}` is accepted by the checker and missing from the vocabulary",
            builtin.name()
        );
    }
    // And nothing invented: every row names something `from_name` knows.
    for line in builtins.lines().filter(|l| l.starts_with("| `")) {
        let name = line
            .trim_start_matches("| `")
            .split('`')
            .next()
            .expect("a name");
        assert!(
            karakuri_ir::builtin::Builtin::from_name(name).is_some(),
            "the vocabulary lists `{name}`, which the checker does not know"
        );
    }
}

/// The same claim about the two other closed vocabularies a procedure is
/// written against — the topologies and the stage outputs.
///
/// A model that is told the wrong set here writes a file the checker refuses,
/// which is the cheap failure; one that is told *too few* never discovers a
/// whole rendering mode, which is not cheap at all. `clip_b` is the case in
/// point: it is the only way to draw a segment, and a page that omitted it
/// would leave the language looking exactly as it did before lines existed.
#[test]
fn the_vocabulary_lists_every_topology_every_blend_and_every_stage_output() {
    let rendered = vocabulary();

    let topologies = section(&rendered, "# Topologies");
    for name in ["points", "lines"] {
        assert!(
            topologies.contains(&format!("`{name}`")),
            "the vocabulary does not mention the `{name}` topology"
        );
        assert!(
            karakuri_ir::parse(&format!(
                "proc p {{ kind L1 topology {name} capacity [1, 2] = 1 \
                 emit position element {{ position = vec3(0.0, 0.0, 0.0); }} }}"
            ))
            .is_ok(),
            "the vocabulary lists `{name}`, which the parser does not accept"
        );
    }

    // **Every blend mode, and each one round-tripped through the parser**,
    // for the same reason the topologies are: a page listing a mode the
    // language does not accept sends a model into a diagnostic, and one
    // omitting a mode hides a whole way of drawing. `weighted` is this
    // milestone's `clip_b` — the only way to make material occlude
    // anything, and invisible to anyone not told it exists.
    let blends = section(&rendered, "# Blend modes");
    for name in ["additive", "weighted"] {
        assert!(
            blends.contains(&format!("`{name}`")),
            "the vocabulary does not mention the `{name}` blend mode"
        );
        assert!(
            karakuri_ir::parse(&format!(
                "proc p {{ kind L4 blend {name} consumes position \
                 vertex {{ clip = vec4(position, 1.0); point_rate = 0.004; }} \
                 fragment {{ color = vec4(1.0, 1.0, 1.0, 1.0); }} }}"
            ))
            .is_ok(),
            "the vocabulary lists `{name}`, which the parser does not accept"
        );
    }

    let outputs = section(&rendered, "# Stage outputs");
    for output in karakuri_ir::Output::ALL {
        assert!(
            outputs.contains(&format!("| `{}` |", output.name())),
            "`{}` is assignable and missing from the vocabulary",
            output.name()
        );
    }
    for line in outputs.lines().filter(|l| l.starts_with("| `")) {
        let name = line
            .trim_start_matches("| `")
            .split('`')
            .next()
            .expect("a name");
        assert!(
            karakuri_ir::Output::from_name(name).is_some(),
            "the vocabulary lists an output `{name}` the checker does not know"
        );
    }
}

/// A slot is a number and a layer is one of five. No path crosses the protocol:
/// a client may be on another machine through an `ssh -L`, where a path means
/// nothing — and a tool that took one would invite a model to write anywhere on
/// the render machine's disk.
#[test]
fn a_slot_a_layer_and_a_renderer_resolve_and_anything_else_is_refused() {
    let slots = slots();
    assert_eq!(
        slots.path(1, Kind::L4, 0).expect("slot 1 L4"),
        std::path::PathBuf::from("b/l4.kir")
    );
    assert_eq!(
        slots.path(0, Kind::L1, 0).expect("slot 0 L1"),
        std::path::PathBuf::from("a/l1.kir")
    );

    let past_the_end = slots
        .path(2, Kind::L1, 0)
        .expect_err("slot 2 does not exist");
    assert_eq!(past_the_end, karakuri_environment::no_such_slot(2, 2));
    // And the cheap door to the same answer, which is what `save_set` asks
    // rather than reading every file of a slot to learn a length.
    assert_eq!(
        slots.holds(2).expect_err("slot 2 does not exist"),
        past_the_end
    );
    assert!(slots.holds(1).is_ok());

    // **A layer this slot does not use is a different answer from a layer
    // this surface cannot reach**, and it used to give the second: "no
    // layer `L2` here" was true of the surface and false of the language.
    // Every layer resolves now, so what is left to say is that this
    // particular slot has none — with what a slot holds one for, because a
    // model that reads that can decide whether to ask for a different slot.
    let none_held = slots.path(0, Kind::L2, 0).expect_err("this slot has no L2");
    assert!(none_held.contains("holds no L2"), "{none_held}");
    assert!(none_held.contains("optional"), "{none_held}");
}

/// The index counts within a layer, keeping file order — the rule
/// `history::seed` files a snapshot under, so an address that reaches the
/// second renderer here reaches the second renderer's versions there.
///
/// The fixture interleaves the layers on purpose. Counting a file's position in
/// the slot instead would hand back a real procedure at every address and the
/// wrong one at most of them, which is the failure that reads as the language
/// being confusing rather than as a resolver being wrong.
#[test]
fn a_node_is_indexed_within_its_own_layer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let at = dir.path();
    let head = declaring(at, "head.kir", "L1");
    let warp_a = declaring(at, "warp_a.kir", "L2");
    let sprites = declaring(at, "sprites.kir", "L4");
    let warp_b = declaring(at, "warp_b.kir", "L2");
    let strokes = declaring(at, "strokes.kir", "L4");
    let source_b = declaring(at, "source_b.kir", "L1");
    let camera = declaring(at, "camera.kir", "L3");
    let blob = declaring(at, "blob.kir", "Field");
    let slots = Slots::of(vec![(
        head.clone(),
        vec![
            warp_a.clone(),
            sprites.clone(),
            warp_b.clone(),
            strokes.clone(),
            source_b.clone(),
            camera.clone(),
            blob.clone(),
        ],
    )]);

    for (layer, index, expected) in [
        (Kind::L1, 0, head.clone()),
        (Kind::L2, 0, warp_a.clone()),
        (Kind::L4, 0, sprites.clone()),
        (Kind::L2, 1, warp_b.clone()),
        (Kind::L4, 1, strokes.clone()),
        // **The head keeps L1 0**, so a second source is 1 — a chain that
        // names another geometry is another source, not a fresh count.
        (Kind::L1, 1, source_b.clone()),
        (Kind::L3, 0, camera.clone()),
        (Kind::Field, 0, blob.clone()),
    ] {
        let name = layer_name(layer);
        assert_eq!(
            slots
                .path(0, layer, index)
                .unwrap_or_else(|e| panic!("{name}:{index}: {e}")),
            expected,
            "{name}:{index} resolved to the wrong file"
        );
    }

    let past = slots
        .path(0, Kind::L2, 2)
        .expect_err("there is no third L2");
    assert!(past.contains("0-1"), "the range is not named: {past}");
    // This slot was given one camera, so an index past it is worth saying
    // rather than folding onto the one there is — the same sentence a
    // second L2 gets, since nothing here caps a layer.
    let two_cameras = slots
        .path(0, Kind::L3, 1)
        .expect_err("this slot was given one camera");
    assert!(two_cameras.contains("one L3"), "{two_cameras}");
}

/// A file that cannot be read is counted as a renderer, which is what
/// `history::seed` makes of one and what the compile is about to refuse it as.
/// Guessing nothing at all would make a slot's whole chain unaddressable the
/// moment one file in it went missing.
#[test]
fn an_unreadable_file_is_counted_as_a_renderer() {
    let missing = Slots::of(vec![(
        "nowhere/l1.kir".into(),
        vec!["nowhere/gone.kir".into()],
    )]);
    assert_eq!(
        missing.path(0, Kind::L4, 0).expect("counted as a renderer"),
        std::path::PathBuf::from("nowhere/gone.kir")
    );
}

/// A layer is parsed once, and a name this language does not have comes back
/// with the ones it does. A model that is told which five there are can fix its
/// own call, which is the same reason the checker's diagnostics come back
/// through this surface at all.
#[test]
fn a_layer_this_language_does_not_have_is_refused_with_the_list() {
    let refused =
        slot_layer_index(&json!({ "slot": 0, "layer": "L9" })).expect_err("there is no L9");
    assert!(refused.contains("L1, L2, L3, L4, Field"), "{refused}");

    // Case does not matter: `l1` and `field` are what a model tends to
    // type, and refusing them teaches nobody anything.
    assert_eq!(
        slot_layer_index(&json!({ "slot": 0, "layer": "l1" })).expect("l1"),
        (0, Kind::L1, 0)
    );
    assert_eq!(
        slot_layer_index(&json!({ "slot": 3, "layer": "field", "index": 0 })).expect("field"),
        (3, Kind::Field, 0)
    );
}

/// A renderer is addressed by index, and an index past the stack is refused
/// rather than folded to the first.
///
/// This surface used to hand back renderer 0 for any `L4` and say so in a
/// comment, which was honest and useless: a model told to rewrite the streaks
/// of a slot that draws sprites *and* streaks would have rewritten the sprites.
/// The refusal names the range, because a model that can read the range can fix
/// its own call — the same reason the checker's diagnostics come back through
/// this surface rather than going to a terminal nobody is watching.
#[test]
fn a_renderer_is_addressed_by_index_and_a_bad_one_names_the_range() {
    let stacked = Slots::of(vec![(
        "a/l1.kir".into(),
        vec!["a/sprites.kir".into(), "a/strokes.kir".into()],
    )]);

    assert_eq!(
        stacked.path(0, Kind::L4, 1).expect("the second renderer"),
        std::path::PathBuf::from("a/strokes.kir")
    );
    // Omitting it is 0, which is what every call written before stacks
    // existed means and what a slot with one renderer always means.
    assert_eq!(
        stacked.path(0, Kind::L4, 0).expect("the first renderer"),
        std::path::PathBuf::from("a/sprites.kir")
    );

    let past = stacked
        .path(0, Kind::L4, 2)
        .expect_err("there is no third renderer");
    assert!(past.contains("0-1"), "the range is not named: {past}");

    // This slot holds one geometry, so an index on it is a mistake worth
    // saying — quietly ignoring it would let a model believe it had
    // addressed something. A slot *may* hold a second source; naming one
    // is what makes it addressable, and this fixture names none.
    let l1_indexed = stacked
        .path(0, Kind::L1, 1)
        .expect_err("this slot has one source");
    assert!(l1_indexed.contains("one L1"), "{l1_indexed}");
}

/// A tool failure comes back as a result, not as a protocol error. A model told
/// "your call was malformed" learns nothing; one handed the checker's
/// diagnostics can fix its own source, which is the entire loop this surface
/// exists for.
#[test]
fn a_refused_write_returns_the_diagnostics_as_content() {
    let (tx, rx) = mpsc::channel();
    drop(tx);
    let mut state = state(rx);
    let request = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": "write_procedure",
            "arguments": { "slot": 0, "layer": "L4", "source": "not a procedure" },
        },
    });
    let reply = dispatch(&request, &mut state)
        .settled()
        .expect("a call is answered");
    let result = reply.get("result").expect("a result, not an error");
    assert_eq!(result["isError"], json!(true));
    let text = result["content"][0]["text"].as_str().expect("text");
    // The checker's words, against the source, rather than a bare failure.
    assert!(text.contains("parse"), "{text}");
    assert!(
        reply.get("error").is_none(),
        "a bad procedure must not look like a bad request"
    );
    // Structured diagnostic report attached for AI agents
    let report: DiagnosticReport =
        serde_json::from_value(result["report"].clone()).expect("structured report");
    assert!(!report.success);
    assert!(!report.diagnostics.is_empty());
    assert_eq!(report.diagnostics[0].code, "KIR-E100-PARSE");
}

#[test]
fn check_procedure_returns_structured_diagnostics() {
    let (tx, rx) = mpsc::channel();
    drop(tx);
    let mut state = state(rx);

    // 1. Valid procedure
    let valid_src = include_str!("../../karakuri-ir/tests/fixtures/drift_shell.kir");
    let req_valid = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": "check_procedure",
            "arguments": { "source": valid_src },
        },
    });
    let reply_valid = dispatch(&req_valid, &mut state)
        .settled()
        .expect("answered");
    let res_valid = reply_valid.get("result").expect("result");
    assert_eq!(res_valid["isError"], json!(false));
    let report_valid: DiagnosticReport =
        serde_json::from_value(res_valid["report"].clone()).expect("report");
    assert!(report_valid.success);
    assert!(report_valid.diagnostics.is_empty());

    // 2. Broken procedure with unknown kind
    let broken_src = "proc broken {\n  kind L9\n}\n";
    let req_broken = json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {
            "name": "check_procedure",
            "arguments": { "source": broken_src },
        },
    });
    let reply_broken = dispatch(&req_broken, &mut state)
        .settled()
        .expect("answered");
    let res_broken = reply_broken.get("result").expect("result");
    assert_eq!(res_broken["isError"], json!(true));
    let report_broken: DiagnosticReport =
        serde_json::from_value(res_broken["report"].clone()).expect("report");
    assert!(!report_broken.success);
    assert_eq!(report_broken.diagnostics.len(), 1);
    assert_eq!(report_broken.diagnostics[0].code, "KIR-E102-UNKNOWN-KIND");
    assert_eq!(report_broken.diagnostics[0].line, Some(2));
    assert!(report_broken.diagnostics[0].remedy.is_some());
}

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
        gate::refusal(&operation, Standing::Closed(Class::MixFaders)).expect("a refusal"),
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

/// A save the render loop does not answer ends, and says something true.
///
/// Three ways a model can be left waiting, and none of them may end in a call
/// that never returns or in a claim nobody can support. Over the channel rather
/// than over a socket for the reason `drained_saves` is tested that way: the
/// bound is the whole point and a render loop is not needed to see it.
#[test]
fn a_save_the_loop_does_not_answer_ends_and_says_something_true() {
    // Never taken. Nothing was saved and saying so is safe.
    let (kept, news) = mpsc::channel::<News>();
    let started = std::time::Instant::now();
    let said = awaited(&news, std::time::Duration::from_millis(60))
        .expect_err("a save nobody took came back as a success");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "the wait ran past its bound"
    );
    assert!(said.contains("Nothing was saved"), "{said}");
    drop(kept);

    // Taken and named, then silence. **The id is in the answer** — that is
    // what the first message is for — and the answer claims neither success
    // nor failure.
    let (tx, news) = mpsc::channel();
    tx.send(News::Accepted(
        "slot 0: saving 2 nodes as set `keeper` in <store>".to_string(),
    ))
    .expect("accepted");
    let said = awaited(&news, std::time::Duration::from_millis(60))
        .expect_err("a save with no outcome came back as a success");
    assert!(
        said.contains("`keeper`"),
        "a timed-out save did not name the id it was accepted under: {said}"
    );
    assert!(
        said.contains("neither a success nor a failure"),
        "a timed-out save was reported as one or the other: {said}"
    );

    // The loop ended without answering: told at once rather than at the
    // deadline, which the long wait here is what proves.
    let (tx, news) = mpsc::channel();
    tx.send(News::Accepted(
        "slot 0: saving 2 nodes as set `keeper` in <store>".to_string(),
    ))
    .expect("accepted");
    drop(tx);
    let started = std::time::Instant::now();
    let said = awaited(&news, std::time::Duration::from_secs(60))
        .expect_err("a loop that ended came back as a success");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "a client waited out the deadline on a loop that was already gone"
    );
    assert!(said.contains("`keeper`"), "{said}");
}

/// The render loop's own accept reaches a client that times out, and names the
/// id it will find the set under.
///
/// The two halves of a truthful timeout meeting for the first time:
/// [`crate::accepted_save`] is what the loop says at the frame it takes a save,
/// and [`awaited`] is what this server does with it. Every other test that
/// reaches a save drives a stand-in loop which sends `accepted` *itself* — so
/// deleting `reply.accepted` from the real loop left the whole suite green,
/// while a client whose deadline passed was told "Nothing was saved, and asking
/// again is safe" about a save that was running and would land. That is a false
/// claim to a model about a disk, and preventing exactly it is why [`Reply`]
/// carries two messages rather than one.
///
/// The reply is deliberately still alive at the deadline: this is a slow save,
/// not a dead loop, and the two have different answers.
#[test]
fn a_save_the_loop_has_taken_names_its_id_to_a_client_that_times_out() {
    let (tx, news) = mpsc::channel();
    let reply = Reply(tx);
    // One node, because the sentence counts them and a fixture that agreed
    // with a hardcoded plural would be checking the fixture.
    let sources =
        karakuri_environment::setfile::Sources(vec![karakuri_environment::setfile::SavedNode {
            layer: "L1",
            index: 0,
            hash: karakuri_store::hash::Hash::of(b"kind L1"),
            name: None,
            source: None,
            meta: None,
        }]);
    let id = karakuri_environment::accepted_save(
        1,
        // **A `Reply` exists only because a model asked**, so this is the
        // arm this test has always been about — see
        // `docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`.
        karakuri_environment::Asked::Model,
        Some("keeper".to_string()),
        &sources,
        std::path::Path::new("/nowhere/store"),
        Some(&reply),
    );
    assert!(
        id.ends_with("_keeper") && id != "keeper",
        "a client's own name rides behind the stamp so a snapshot cannot be \
         written over: {id}"
    );

    let said = awaited(&news, std::time::Duration::from_millis(60))
        .expect_err("a save with no outcome yet came back as a success");
    assert!(
        said.starts_with(&format!(
            "slot 1: saving 1 node as set `{id}` in /nowhere/store/sandbox"
        )),
        "the loop's acceptance did not reach the client, so a timeout has no id \
         to offer, and no directory to look in: {said}"
    );
    assert!(
        said.contains("neither a success nor a failure"),
        "a save with no outcome was reported as one or the other: {said}"
    );
    assert!(
        !said.contains("Nothing was saved"),
        "a save that had been taken and is being written was reported to a model \
         as one that never happened: {said}"
    );
    drop(reply);
}

/// An id from a client is one path component, which is what a Set id is
/// everywhere else in this program.
#[test]
fn a_set_id_from_a_client_is_one_path_component() {
    assert_eq!(checked_id("keeper-01"), Ok("keeper-01".to_string()));
    assert_eq!(checked_id("a_B_9"), Ok("a_B_9".to_string()));
    // What a save with no id is called, so a client can name one the same
    // way the run would have.
    let stamp = karakuri_environment::history::stamped_id();
    assert_eq!(checked_id(&stamp), Ok(stamp.clone()), "{stamp}");

    for bad in [
        "../../../etc/passwd",
        "sets/../../elsewhere",
        "a/b",
        "",
        "a b",
        "night.01",
        "~/mine",
    ] {
        assert!(
            checked_id(bad).is_err(),
            "`{bad}` was accepted as the name of a file in the store"
        );
    }
    assert!(checked_id(&"x".repeat(MAX_ID + 1)).is_err());

    // **The over-length refusal counts what it measures.** `str::len` is
    // bytes and the message said "characters", which agree for everything
    // that would get past the charset check and disagree for exactly the
    // caller this message exists for. Thirty-three two-byte characters is
    // sixty-six bytes, so the two readings cannot both be right here.
    let multibyte = "é".repeat(33);
    let refusal = checked_id(&multibyte).expect_err("66 bytes is past the cap");
    assert!(
        refusal.contains("is 66 bytes"),
        "an id was refused for a length its caller cannot count to: {refusal}"
    );
}

// -- the surface against the vocabulary and the page -------------------

/// The specification, relative to the workspace root — the same page
/// `karakuri-operation`'s `the_manual_and_the_vocabulary_agree.rs` and
/// `karakuri-console`'s `tests/vocabulary.rs` read, and this is that check for
/// the third surface.
const PAGE: &str = "docs/manual/operations.html";

/// What marks an operation on that page. Every row opens with this div and
/// nothing else on the page uses it; sections are `<h2>` and the legend is
/// neither. The same marker both other tests match, for their reason.
const ROW: &str = r#"<div class="op-head">"#;

fn page() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(PAGE);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} is the specification and could not be read: {e}",
            path.display()
        )
    })
}

/// Every row's title and its MCP badge, in page order: the badge's class —
/// `has`, `plan` or `gap` — and the text it names the route with.
///
/// Read verbatim and never decoded, exactly as the vocabulary's own test reads
/// a heading: a `gap` badge says `&mdash;`, and a tool name that needed
/// decoding to match would be a tool nobody could type.
fn mcp_routes() -> Vec<(String, String, String)> {
    let html = page();
    let mut found = Vec::new();
    for row in html.split(ROW).skip(1) {
        let Some(open) = row.find("<h3>") else {
            continue;
        };
        let rest = &row[open + "<h3>".len()..];
        let Some(close) = rest.find("</h3>") else {
            continue;
        };
        let title = rest[..close].to_string();
        // The row ends where the next section does; a badge found past that
        // would belong to another row.
        let body = &rest[close..];
        let body = &body[..body.find("</section>").unwrap_or(body.len())];
        let Some(at) = body.find(r#"<span class="rt "#) else {
            continue;
        };
        let mut badge = None;
        for span in body[at..].split(r#"<span class="rt "#).skip(1) {
            let Some(quote) = span.find('"') else {
                continue;
            };
            let class = span[..quote].to_string();
            let Some(text) = span[quote..].strip_prefix(r#"">MCP <b>"#) else {
                continue;
            };
            let Some(shut) = text.find("</b>") else {
                continue;
            };
            badge = Some((class, text[..shut].to_string()));
            break;
        }
        let Some((class, names)) = badge else {
            continue;
        };
        found.push((title, class, names));
    }
    found
}

/// Arguments each published tool accepts, and the only thing this file says
/// about a tool that the code does not.
///
/// Not a second list of titles: what a tool *is* comes back from [`asked`],
/// which is the path a real call takes. This is the smallest call that gets
/// past each schema, so that a tool cannot be surveyed by inventing what it
/// would have been named.
fn sample(name: &str) -> Value {
    match name {
        "read_procedure" => json!({ "slot": 0, "layer": "L4" }),
        "write_procedure" => json!({ "slot": 0, "layer": "L4", "source": "" }),
        "wire_input" => json!({ "slot": 0, "node": "warp", "input": "shape", "to": "blob" }),
        "swap_outcome" => json!({}),
        "save_set" => json!({ "slot": 0 }),
        "read_set" => json!({ "id": "a_set" }),
        "list_sets" => json!({}),
        "walk_history" => json!({ "set": "a_set" }),
        // **The one operation `operate` names that the audit lets through**,
        // which is what makes this survey mean the same thing for the eighth
        // tool as it does for the seven: the other twenty-nine are refused
        // by the gate by design, and a sample drawn from those would make
        // *every tool names an operation the gate lets through* false about
        // a tool that is working exactly as ADR-0235 says it should. The
        // rows that are closed are surveyed by
        // `every_operation_operate_takes_stands_where_the_page_says_it_does`
        // instead.
        "operate" => json!({
            "operation": "Put a node's previous version back",
            "with": { "deck": 0, "revision": { "previous": { "layer": "L4" } } },
        }),
        other => panic!(
            "`{other}` is published by `tools()` and this file has no arguments for it — \
             add the smallest call that gets past its schema, so the survey below reaches \
             it rather than passing over it"
        ),
    }
}

/// Every tool this server publishes, with the operation one call names.
fn published() -> Vec<(String, Operation)> {
    let slots = slots();
    tools()
        .as_array()
        .expect("tools() is an array")
        .iter()
        .map(|tool| {
            let name = tool["name"]
                .as_str()
                .expect("a tool has a name")
                .to_string();
            let asked = asked(&name, &sample(&name), &slots)
                .unwrap_or_else(|e| panic!("`{name}` is advertised and is not a tool: {e}"));
            match asked {
                Asked::Named(operation) => (name, operation),
                Asked::Refused(refusal) => panic!(
                    "`{name}` refused the sample call in this file: {refusal} — the \
                     arguments in `sample` no longer get past its schema"
                ),
            }
        })
        .collect()
}

/// A tool with no row is an operation nobody specified.
///
/// The page is the specification for which operations exist — that is what
/// `karakuri-operation`'s own manual test is built on — so a tool reaching
/// something the page does not name would be this surface inventing an
/// operation, with no prose and no other three routes.
///
/// The row is matched on the operation's title, which comes from [`asked`]
/// rather than from a table here, and on the badge's own text, which has to
/// name the tool: a row marked `has` that named a different tool would be a
/// route the page describes and nobody can call.
#[test]
fn every_tool_this_server_publishes_has_a_route_on_the_page() {
    let routes = mcp_routes();
    assert!(
        routes.len() >= 50,
        "only {} rows with an MCP badge found in {PAGE} — is a row still `{ROW}` \
         followed by an `<h3>` and four `rt` badges? A scan that matched nothing would \
         pass every assertion below",
        routes.len()
    );
    let published = published();
    assert!(
        published.len() >= 7,
        "only {} tools published — this server has fewer than the page's MCP column \
         claims",
        published.len()
    );
    for (name, operation) in &published {
        let title = operation.title();
        let row = routes
            .iter()
            .find(|(row, _, _)| row == title)
            .unwrap_or_else(|| {
                panic!(
                    "`{name}` names `{title}` and {PAGE} has no row with that heading — a \
                     tool reaching an operation nobody specified. The page is the \
                     specification, so add the row there first"
                )
            });
        assert_eq!(
            row.1, "has",
            "`{name}` names `{title}`, which {PAGE} marks `{}` for MCP — a tool that \
             exists and a page that says it does not",
            row.1
        );
        assert_eq!(
            row.2, *name,
            "`{title}` is marked as reached over MCP by `{}`, and the tool that names \
             that operation is `{name}` — the page names a call nobody can make",
            row.2
        );
    }
}

/// The other direction: a `has` badge with no tool is the page claiming a route
/// that does not exist.
///
/// It fails apart from the test above because it is a different failure: that
/// one says the surface reached past the specification, this one says the
/// specification promises a model something it cannot do.
#[test]
fn every_mcp_route_the_page_claims_is_a_tool_this_server_publishes() {
    let routes = mcp_routes();
    let claimed: Vec<&(String, String, String)> = routes
        .iter()
        .filter(|(_, class, _)| class == "has")
        .collect();
    assert!(
        claimed.len() >= 6,
        "only {} rows of {PAGE} claim an MCP route — the scan found less than the \
         column holds, which would pass this test by finding nothing",
        claimed.len()
    );
    let published = published();
    for (title, _, names) in claimed {
        // **`operate` is checked against the spelling rather than against
        // one operation**, which is the difference between the eighth tool
        // and the seven: a tool of its own names one row, and `operate`
        // names every row the spelling takes. So the page claiming
        // `operate` on a row is checked by asking [`SPELLED`] whether it
        // takes that row's operation — the same question a call asks.
        if names == "operate" {
            let row = spelled_named(title).unwrap_or_else(|| {
                panic!(
                    "{PAGE} says `{title}` is reached over MCP by `operate`, and this \
                     vocabulary carries no operation with that heading"
                )
            });
            assert!(
                row.make.is_some(),
                "{PAGE} says `{title}` is reached over MCP by `operate`, and `operate` \
                 refuses that name — the page claims a route a model cannot take"
            );
            continue;
        }
        let tool = published
            .iter()
            .find(|(name, _)| name == names)
            .unwrap_or_else(|| {
                panic!(
                    "{PAGE} says `{title}` is reached over MCP by `{names}`, and this \
                     server publishes no such tool — the page claims a route a model \
                     cannot take. Either the tool went and the badge is now `gap`, or it \
                     was renamed on the wire"
                )
            });
        assert_eq!(
            tool.1.title(),
            title,
            "{PAGE} says `{title}` is reached by `{names}`, and `{names}` names \
             `{}` — one operation on the page and another in the server",
            tool.1.title()
        );
    }
}

/// Every operation the spelling takes has a row marked `has operate`, and every
/// row that is not marked so is one the spelling refuses.
///
/// The other direction of the test above, and the one that catches the silent
/// half: a row `operate` reaches whose badge still reads `plan` is a route a
/// model can take and the page does not describe, which nothing else here would
/// notice.
#[test]
fn every_operation_operate_takes_stands_where_the_page_says_it_does() {
    let routes = mcp_routes();
    assert!(
        routes.len() >= 50,
        "only {} rows with an MCP badge found in {PAGE}",
        routes.len()
    );
    for row in SPELLED {
        let title = row.title();
        let (badge, names) = routes
            .iter()
            .find(|(heading, _, _)| heading == title)
            .map(|(_, class, names)| (class.as_str(), names.as_str()))
            .unwrap_or_else(|| panic!("{PAGE} has no row headed `{title}`"));
        match row.make {
            Some(_) => assert_eq!(
                (badge, names),
                ("has", "operate"),
                "`operate` takes `{title}` and {PAGE} marks it `{badge}` naming \
                 `{names}` — a route a model can take that the page does not describe"
            ),
            None => assert!(
                names != "operate",
                "{PAGE} says `{title}` is reached by `operate`, and `operate` refuses \
                 it: {}",
                match sayable(&(row.sample)().0) {
                    Sayable::Tool(tool) => format!("`{tool}` is its tool"),
                    Sayable::Window => "a model has no window".to_string(),
                    Sayable::Never(why) => why.to_string(),
                    Sayable::Operable => "it does not".to_string(),
                }
            ),
        }
    }
}

/// Not one of the seven writes a record where it is asked, which is why none of
/// them routes through `Live::operate` and why this module performs its own.
///
/// And one of them writes no record at all, which is a hole this test pins
/// rather than blesses. `wire_input` answers `NoRecord` because `Record::Edge`
/// is a Set file's record with no `slot` to carry the deck `WireInput` names —
/// so a rewiring during a set is the one thing a model can do on this surface
/// that a replay does not reconstruct. It is asserted here so that the day
/// `Record::Edge` grows a `slot` and `written` answers with it, this fails and
/// names the tool whose answer has changed.
///
/// Asserted against `karakuri-operation-record` rather than against this file,
/// in the shape ADR-0198 gave the key handler's owed list: the day one of these
/// conversions changes — a `read_set` that logged, a `save_set` whose record
/// moved off the landing frame — the failure names the tool that is due to move
/// rather than leaving this surface performing something the record layer has
/// since taken over.
#[test]
fn no_tool_writes_a_record_where_it_is_asked() {
    use karakuri_operation_record::{Current, Silent, Written};
    for (name, operation) in published() {
        let written = karakuri_operation_record::written(&operation, &Current::default());
        let expected = match name.as_str() {
            // It asks rather than changes, and a question writes no record.
            //
            // **`walk_history` is here on the day it arrived**, which is
            // the whole of why it is a tool: what it does is a listing of
            // the store, and `written` says so rather than this file
            // asserting it (`docs/adr/0342-…`). Landing one of the rows it
            // returns is `RestoreProcedure`, which is `OnLanding` two arms
            // down and reaches the frame through `operate`.
            "read_procedure" | "read_set" | "list_sets" | "walk_history" | "swap_outcome" => {
                Silent::Question
            }
            // Its record is written where the work lands: `Record::Save` at
            // the frame the save landed, `Record::Procedure` when a swap
            // lands.
            "save_set" | "write_procedure" => Silent::OnLanding,
            // **Nothing carries it**, which is the hole named above and not
            // a question this tool asks or work it lands.
            "wire_input" => Silent::NoRecord,
            // **`operate` is the tool this claim is not about**, and saying
            // so is the point rather than an exception. The seven perform
            // themselves *because* they write nothing where they are asked;
            // `operate` performs nothing and hands the operation to the
            // frame the panel performs presses on, so whatever it writes is
            // written there, by the same `written` this asserts against, at
            // the same instant a press of it would write. The sample here is
            // `RestoreProcedure`, whose `Record::Procedure` lands at the
            // swap.
            "operate" => Silent::OnLanding,
            other => {
                panic!("`{other}` is published and this test does not know what it writes")
            }
        };
        assert_eq!(
            written,
            Written::Silent(expected),
            "`{name}` names `{}`, and what it writes is no longer `{expected:?}` — this \
             surface performs it here because there was no record to route into, and \
             that is what has changed",
            operation.title()
        );
    }
}

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

/// What a clean write does not promise, said where a model reads it.
///
/// The description said a clean return meant the material compiled and so a
/// non-compiling change never reached the screen. `compile::check` sees one
/// procedure; everything between nodes is `Set::validate`, and one of the
/// things it refuses — a `uses` slot nothing binds — is a state this surface
/// can create and has no tool to undo. See
/// [`super::wire_tests::a_write_that_needs_an_edge_still_returns_cleanly`] for
/// the write that proves it.
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
    // **And it names the tool that binds it.** The description used to say
    // there was no such tool and to offer *rewrite the procedure without
    // it* as the only way out — a warning sign on a trap. There is a tool
    // now, and a model reading this one is reading the sentence that has to
    // point at it.
    assert!(
        described.contains("wire_input"),
        "the tool that writes the `edge` a `uses` needs is not named where a model \
         writing a `uses` will read it: {described}"
    );
}

/// A schema states the constraints its tool enforces.
///
/// `slot: -1` was refused with "`slot` is required and is a number", which is a
/// sentence about the wrong mistake — the same class of refusal [`kept`] argues
/// about for `"id": null`, and one a client could have been told to avoid
/// before it called. The bounds a schema *can* carry belong in it; the deck's
/// upper bound cannot be one, because how many slots this run holds is not
/// known when `tools/list` is answered.
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

/// `save_set` names everything it writes.
///
/// It described the file as the hashes, the parameters, the capacities and the
/// salts, and `setfile::save` writes the edges, the merge, the camera and the
/// seeds as well — so a model was told a Set file carries less of its slot than
/// it does, and the operations page's own row for this operation had already
/// said otherwise.
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

/// An edge is named at both ends and never addressed by a position.
///
/// This is the one tool whose arguments are not `{slot, layer, index}`, and the
/// asymmetry is deliberate: `Record::Edge`'s reason is that *a position moves
/// when the list is reordered, and reordering silently changing which geometry
/// a morph blends towards is the exact failure this record exists to end*, and
/// `NodeAddress`'s own documentation says the two spellings are not
/// interchangeable. A tool given a `layer` and an `index` here because its
/// neighbours have them would be that failure with a schema in front of it, and
/// it is the kind of tidying that looks like consistency — so it is asserted
/// against rather than left to a comment.
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
    // **And the deck's slot is still the deck's.** `Operation::WireInput`
    // calls the declared input `slot` too, so the wire's `input` and the
    // wire's `slot` must land on different fields — the mistake that reads
    // as a working call and fails at the Set.
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

/// `wire_input` says what it replaces and what it cannot take back.
///
/// Two facts a model cannot find out by calling it, and each is a way to wedge
/// a slot. A second edge on a bound input would be `SetError::SlotBoundTwice`
/// if it were appended rather than replaced, so *changing your mind is one
/// call* has to be said or a model will not try; and an edge outlives the
/// `uses` that needed it, so a procedure rewritten without that `uses` leaves
/// an edge naming a slot nothing declares — a state this surface still cannot
/// get out of, and the reason the missing half is named in the description
/// rather than discovered.
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

/// An edge the render loop does not take ends, and says something true.
///
/// [`awaited`]'s three cases, for the wait that is not a save's — see
/// [`applied`]. The first of them is the one that matters most here and is not
/// hypothetical: a run whose loop never drains [`Reporter::wires`] reaches it
/// on every call, and *nothing was rewired* is what such a run has to answer
/// rather than a claim about a rebuild nobody started.
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
