use super::*;

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

/// Verifies that the vocabulary documentation lists all topologies, blend modes, and stage outputs.
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

/// Verifies that nodes are indexed within their own layer and maintain declaration order.
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

/// Verifies that renderer index addressing refuses out-of-range indices with range diagnostics.
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
    let valid_src = include_str!("../../../karakuri-ir/tests/fixtures/drift_shell.kir");
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
