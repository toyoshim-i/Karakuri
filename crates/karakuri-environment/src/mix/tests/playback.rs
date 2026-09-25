use super::common::*;

/// Verifies that WriteParam operations decode into proper ParamWrites.
#[test]
fn a_ride_decodes_into_the_write_a_deck_takes() {
    let record = from_operation(karakuri_operation::Operation::WriteParam {
        deck: 2,
        param: karakuri_operation::ParamAt {
            node: Some(karakuri_operation::NodeAddress {
                layer: karakuri_operation::Layer::L4,
                index: 1,
            }),
            key: "heat".to_string(),
        },
        value: karakuri_operation::ParamValue::Scalar(2.5),
    });
    let line = serde_json::to_string(&record).expect("serialise");
    let decoded: Record = serde_json::from_str(&line).expect("parse");
    assert_eq!(
        change(&decoded, 4).expect("a record this build built"),
        Some(Change::Ride {
            slot: 2,
            writes: vec![karakuri_engine::ParamWrite::at(
                karakuri_ir::Kind::L4,
                1,
                "heat",
                2.5
            )],
        }),
        "through {line}"
    );

    // Wildcard target moves every node declaring the key.
    let record = from_operation(karakuri_operation::Operation::WriteParam {
        deck: 0,
        param: karakuri_operation::ParamAt {
            node: None,
            key: "exposure".to_string(),
        },
        value: karakuri_operation::ParamValue::Scalar(1.0),
    });
    assert_eq!(
        change(&record, 4).expect("built here"),
        Some(Change::Ride {
            slot: 0,
            writes: vec![karakuri_engine::ParamWrite::everywhere("exposure", 1.0)],
        })
    );
}

/// Verifies that signal attachments and take-backs decode into appropriate bindings.
#[test]
fn an_attachment_decodes_into_the_binding_a_deck_takes_and_a_take_back_into_none() {
    let record = from_operation(karakuri_operation::Operation::AttachSignal {
        deck: 2,
        param: karakuri_operation::BindAt {
            layer: karakuri_operation::Layer::L1,
            index: Some(1),
            key: "turbulence".to_string(),
        },
        signal: "energy".to_string(),
        curve: karakuri_operation::Curve::Pow2,
        range: [0.1, 2.4],
    });
    let line = serde_json::to_string(&record).expect("serialise");
    let decoded: Record = serde_json::from_str(&line).expect("parse");
    let Some(Change::Source {
        slot,
        layer,
        index,
        key,
        binding,
    }) = change(&decoded, 4).expect("a record this build built")
    else {
        panic!("not an attachment, through {line}");
    };
    assert_eq!(
        (slot, layer, index, key.as_str()),
        (2, karakuri_ir::Kind::L1, Some(1), "turbulence")
    );
    let binding = binding.expect("an attachment carries a binding");
    assert_eq!(
        (
            binding.layer,
            binding.index,
            binding.key.as_str(),
            binding.signal.as_str(),
            binding.curve,
            binding.range,
        ),
        (
            karakuri_ir::Kind::L1,
            Some(1),
            "turbulence",
            "energy",
            karakuri_engine::binding::Curve::Pow2,
            [0.1, 2.4],
        ),
        "the binding is not the attachment the operation named"
    );

    // **The take-back is the same record with nothing in it**, and what it
    // has to carry is the address — a decoder that lost the index would
    // take the layer's binding away instead of this node's.
    let record = from_operation(karakuri_operation::Operation::TakeParamBack {
        deck: 2,
        param: karakuri_operation::BindAt {
            layer: karakuri_operation::Layer::L1,
            index: Some(1),
            key: "turbulence".to_string(),
        },
    });
    assert_eq!(
        change(&record, 4).expect("built here"),
        Some(Change::Source {
            slot: 2,
            layer: karakuri_ir::Kind::L1,
            index: Some(1),
            key: "turbulence".to_string(),
            binding: None,
        }),
        "a take-back did not come back as an attachment that is absent"
    );

    // **One decoder, so the `bind` diagnostics are reached.** A tempo is
    // not a `[0, 1]` signal, and a binding to it would sit at the top of
    // its range for the whole run — which is the same sentence a Set file
    // and a `--bind` meet, in `setfile::binding_from_record`.
    let pinned = Record::Source {
        slot: DeckSlot(0),
        layer: karakuri_store::record::Layer::L1,
        index: None,
        key: "radius".to_string(),
        source: Some(karakuri_store::record::Source {
            signal: "bpm".to_string(),
            curve: "lin".to_string(),
            range: [0.0, 1.0],
            noise: None,
        }),
    };
    let refused = change(&pinned, 4).expect_err("a `bpm` source is refused");
    assert!(
        refused.contains("bpm") && refused.contains("beat"),
        "the refusal does not name the source or what to use instead: {refused}"
    );
}

/// An authority decodes into the level the operation named, and a word this
/// build does not know reaches a diagnostic rather than a parser that refuses
/// the line — which is why `Record::Authority` carries a `String`.
#[test]
fn an_authority_decodes_into_the_level_it_names() {
    let record = from_operation(karakuri_operation::Operation::SetAuthority {
        deck: 3,
        node: karakuri_operation::NodeAddress {
            layer: karakuri_operation::Layer::L4,
            index: 1,
        },
        authority: karakuri_operation::Authority::Suggesting,
    });
    let line = serde_json::to_string(&record).expect("serialise");
    let decoded: Record = serde_json::from_str(&line).expect("parse");
    assert_eq!(
        change(&decoded, 4).expect("a record this build built"),
        Some(Change::Authority {
            slot: 3,
            layer: karakuri_ir::Kind::L4,
            index: 1,
            authority: Authority::Suggesting,
        }),
        "through {line}"
    );

    let unknown = Record::Authority {
        slot: DeckSlot(0),
        at: karakuri_store::record::NodeAddress {
            layer: karakuri_store::record::Layer::L1,
            index: 0,
        },
        authority: "supervising".to_string(),
    };
    let refused = change(&unknown, 4).expect_err("a level this build has not got");
    assert!(
        refused.contains("supervising") && refused.contains("manual"),
        "the refusal does not say what was asked for or what this build has: {refused}"
    );
}

/// Verifies that vector writes expand into writes per component key (`key.x`, `key.y`, etc.).
#[test]
fn a_wide_ride_becomes_one_write_per_component() {
    let record = from_operation(karakuri_operation::Operation::WriteParam {
        deck: 1,
        param: karakuri_operation::ParamAt {
            node: None,
            key: "glow".to_string(),
        },
        value: karakuri_operation::ParamValue::Vec3([0.4, 0.7, 1.0]),
    });
    let line = serde_json::to_string(&record).expect("serialise");
    let decoded: Record = serde_json::from_str(&line).expect("parse");
    assert_eq!(
        change(&decoded, 4).expect("a record this build built"),
        Some(Change::Ride {
            slot: 1,
            writes: vec![
                karakuri_engine::ParamWrite::everywhere(karakuri_ir::component_key("glow", 0), 0.4),
                karakuri_engine::ParamWrite::everywhere(karakuri_ir::component_key("glow", 1), 0.7),
                karakuri_engine::ParamWrite::everywhere(karakuri_ir::component_key("glow", 2), 1.0),
            ],
        }),
        "through {line}"
    );
}

/// A record this build cannot obey is reported, not dropped. Silently ignoring
/// it would leave a session replaying at the wrong gain with nothing said,
/// which is worse than refusing the line.
#[test]
fn a_record_this_build_cannot_obey_says_so_rather_than_vanishing() {
    let unknown_level = Record::Residency {
        slot: DeckSlot(0),
        level: "cooling".to_string(),
    };
    let message = change(&unknown_level, 4).expect_err("`cooling` is not a level here");
    assert!(message.contains("cooling"), "{message}");
    assert!(message.contains("live"), "{message}");

    let unknown_op = Record::Look {
        op: "filmic".to_string(),
        exposure: 1.0,
        white_point: 4.0,
    };
    let message = change(&unknown_op, 4).expect_err("`filmic` is not an operator here");
    assert!(message.contains("filmic"), "{message}");
    assert!(message.contains("aces"), "{message}");

    // **And a feedback cut this build has not got**, refused on the tone
    // map operator's terms and for the sharper reason: a default would not
    // report a wrong level, it would silently play the other picture — one
    // echo where the session had a trail.
    let unknown_cut = Record::MasterChain(karakuri_store::record::Chain {
        slots: vec![karakuri_store::record::ChainSlot {
            procedure: "sha256:feedback".into(),
            cut: Some("previous".to_string()),
            params: Default::default(),
        }],
    });
    let message = change(&unknown_cut, 4).expect_err("`previous` is not a cut here");
    assert!(message.contains("previous"), "{message}");
    for cut in Cut::ALL {
        assert!(message.contains(cut.name()), "{message}");
    }

    for (start, beats, to, wanted) in [
        (f64::NAN, 4.0, 1.0, "not a position"),
        (0.0, -4.0, 1.0, "expected a number of beats"),
        (0.0, f64::NAN, 1.0, "expected a number of beats"),
        (0.0, 4.0, f32::NAN, "not a value"),
    ] {
        let record = Record::Transition {
            slot: DeckSlot(0),
            control: "gain".to_string(),
            to,
            start,
            beats,
            curve: "lin".to_string(),
        };
        let message = change(&record, 4).expect_err("a number no move can use");
        assert!(message.contains(wanted), "{start}/{beats}/{to}: {message}");
    }

    // A selection that never lands, for the reason a transition's does not:
    // nothing clears a queued move whose instant cannot arrive.
    for start in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let record = Record::Select {
            slot: DeckSlot(0),
            renderer: 0,
            start,
        };
        let message = change(&record, 4).expect_err("a beat no selection can land on");
        assert!(message.contains("not a position"), "{start}: {message}");
    }

    let unknown_shape = Record::Mask {
        slot: DeckSlot(0),
        kind: "diagonal".to_string(),
        angle: 0.0,
        position: 1.0,
        softness: 0.0,
    };
    let message = change(&unknown_shape, 4).expect_err("`diagonal` is not a shape");
    assert!(message.contains("diagonal"), "{message}");
    assert!(message.contains("radial"), "{message}");

    let unknown_control = Record::Transition {
        slot: DeckSlot(0),
        control: "residency".to_string(),
        to: 1.0,
        start: 0.0,
        beats: 4.0,
        curve: "lin".to_string(),
    };
    let message = change(&unknown_control, 4).expect_err("`residency` is not a control");
    assert!(message.contains("residency"), "{message}");
    assert!(message.contains("opacity"), "{message}");

    let unknown_curve = Record::Transition {
        slot: DeckSlot(0),
        control: "gain".to_string(),
        to: 1.0,
        start: 0.0,
        beats: 4.0,
        curve: "bezier".to_string(),
    };
    let message = change(&unknown_curve, 4).expect_err("`bezier` is not a curve");
    assert!(message.contains("bezier"), "{message}");
    assert!(message.contains("smooth"), "{message}");

    let unknown_mode = Record::Blend {
        slot: DeckSlot(0),
        mode: "screen".to_string(),
    };
    let message = change(&unknown_mode, 4).expect_err("`screen` is not a mode here");
    assert!(message.contains("screen"), "{message}");
    assert!(message.contains("over"), "{message}");
}

/// Verifies that referencing an out-of-range slot is refused with no_such_slot.
#[test]
fn a_slot_past_the_deck_is_refused_with_the_range_it_missed() {
    let message = change(
        &Record::Gain {
            slot: DeckSlot(4),
            value: 1.0,
        },
        4,
    )
    .expect_err("slot 4 of a deck of 4");
    assert_eq!(message, crate::no_such_slot(4, 4));
    // And the boundary either side of it, which is where the off-by-one
    // this shares with the digit keys would live.
    assert!(change(
        &Record::Gain {
            slot: DeckSlot(3),
            value: 1.0
        },
        4
    )
    .is_ok());
    assert!(change(
        &Record::Gain {
            slot: DeckSlot(0),
            value: 1.0
        },
        1
    )
    .is_ok());
    assert!(change(
        &Record::Gain {
            slot: DeckSlot(1),
            value: 1.0
        },
        1
    )
    .is_err());
}

/// A record that is not the mix's is not an error. `karakuri-environment`'s
/// `audio.rs` decodes those, and both decoders see every record a session
/// carries.
#[test]
fn a_record_that_is_not_the_mixs_is_left_alone() {
    assert_eq!(change(&Record::Tick { steps: 1 }, 4), Ok(None));
}

/// Verifies that shipped presets resolve without requiring a store.
#[test]
fn a_shipped_procedure_resolves_without_a_store() {
    for (name, source) in shipped::ALL {
        let address = shipped::address(source);
        assert!(
            address.starts_with("sha256:"),
            "{name} is addressed as `{address}`"
        );
        assert_eq!(
            resolve_procedure(None, &address).as_deref(),
            Some(source),
            "{name} did not resolve to its own source"
        );
    }
}

/// Verifies that stored procedures resolve through the run's Store.
#[test]
fn a_stored_procedure_resolves_through_the_runs_store() {
    const OWN: &str = "proc dimmer {\n  kind L5\n\n  frame {\n    color = texel(src);\n  }\n}\n";
    let root = tempfile::tempdir().expect("a temporary store root");
    let store = karakuri_store::store::Store::open(root.path()).expect("a store opens");
    let hash = store
        .put_artifact(OWN.as_bytes())
        .expect("an artifact is stored");
    let address = hash.to_string();

    assert_eq!(
        resolve_procedure(None, &address),
        None,
        "an address nobody shipped resolved with no store behind it"
    );
    assert_eq!(
        resolve_procedure(Some(&store), &address).as_deref(),
        Some(OWN),
        "the store did not answer for an address it holds"
    );
    // And the shipped three still answer through a store.
    assert_eq!(
        resolve_procedure(Some(&store), &shipped::address(shipped::BLOOM)).as_deref(),
        Some(shipped::BLOOM)
    );
}

/// Verifies that resolving a chain with an unknown address refuses with the slot and address.
#[test]
fn a_chain_naming_an_address_nothing_holds_is_refused_with_the_address() {
    let root = tempfile::tempdir().expect("a temporary store root");
    let store = karakuri_store::store::Store::open(root.path()).expect("a store opens");
    let resolve = |address: &str| resolve_procedure(Some(&store), address);

    let missing = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let slots = vec![
        SlotSpec {
            procedure: shipped::address(shipped::RGB_SHIFT),
            cut: None,
            params: Default::default(),
        },
        SlotSpec {
            procedure: missing.to_string(),
            cut: None,
            params: Default::default(),
        },
    ];

    let refusal = resolve_chain(&slots, &resolve).expect_err("an address nothing holds is refused");
    assert!(
        refusal.contains(missing),
        "the refusal does not name the address: {refusal}"
    );
    assert!(
        refusal.contains('1'),
        "the refusal does not name the slot: {refusal}"
    );

    let held = resolve_chain(&slots[..1], &resolve).expect("a chain of presets resolves whole");
    assert_eq!(held, vec![shipped::RGB_SHIFT.to_string()]);
    assert_eq!(
        resolve_chain(&[], &resolve),
        Ok(Vec::new()),
        "an empty chain is not a refusal"
    );
}
