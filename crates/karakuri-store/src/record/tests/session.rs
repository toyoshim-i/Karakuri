use super::*;

/// The wire line the spec documents, parsed and written back. Every other
/// record has one of these; without it a rename or a reordered field breaks
/// every recorded session and nothing says so.
#[test]
fn a_transition_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"transition","slot":0,"control":"opacity","to":0.0,"start":64.0,"beats":8.0,"curve":"smooth"}"#;
    let rec = round_trip(line);
    assert_eq!(
        rec,
        Record::Transition {
            slot: DeckSlot(0),
            control: "opacity".to_string(),
            to: 0.0,
            start: 64.0,
            beats: 8.0,
            curve: "smooth".to_string(),
        }
    );
    assert!(!rec.is_set_state());
}

/// Verifies that a selection record round-trips through JSON serialization verbatim.
#[test]
fn a_selection_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"select","slot":1,"renderer":2,"start":64.0}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::Select {
            slot: DeckSlot(1),
            renderer: 2,
            start: 64.0,
        }
    );
    assert!(!rec.is_set_state());
    assert!(!rec.is_metadata());
}

/// Verifies that a ride record round-trips addressed and as a wildcard.
#[test]
fn a_ride_round_trips_addressed_and_as_a_wildcard() {
    // Addressed, and at a node that is not 0 — the index is written only
    // where it says something, so an addressed write at node 0 and a
    // wildcard are told apart by `at` and never by `index`.
    let rec = round_trip_verbatim(
        r#"{"t":"ride","slot":2,"at":{"layer":"L4","index":1},"key":"glow.x","value":0.4}"#,
    );
    assert_eq!(
        rec,
        Record::Ride {
            slot: DeckSlot(2),
            at: Some(NodeAddress {
                layer: Layer::L4,
                index: 1
            }),
            key: "glow.x".to_string(),
            value: Value::Scalar(0.4),
        }
    );
    // The session's, and for `procedure`'s and `authority`'s reason rather
    // than `select`'s: it is state, and it is the deck's. A Set file that
    // carried one would restore a knob position wherever it was loaded.
    assert!(!rec.is_set_state());
    assert!(!rec.is_metadata());

    // Node 0 of a layer, addressed. The `index` is absent because it is
    // zero, and the `at` is present because a node was named — which is the
    // distinction `Record::Param` cannot draw at all.
    let rec = round_trip_verbatim(
        r#"{"t":"ride","slot":0,"at":{"layer":"L1"},"key":"radius","value":2.6}"#,
    );
    assert_eq!(
        rec,
        Record::Ride {
            slot: DeckSlot(0),
            at: Some(NodeAddress {
                layer: Layer::L1,
                index: 0
            }),
            key: "radius".to_string(),
            value: Value::Scalar(2.6),
        }
    );

    // The wildcard: no node, and so no layer either.
    let rec = round_trip_verbatim(r#"{"t":"ride","slot":1,"key":"exposure","value":2.0}"#);
    assert_eq!(
        rec,
        Record::Ride {
            slot: DeckSlot(1),
            at: None,
            key: "exposure".to_string(),
            value: Value::Scalar(2.0),
        }
    );

    // A wide value on one line, which is what a model writes and what a
    // reader with the Set in hand expands — `param`'s rule exactly
    // (ADR-0268).
    let rec = round_trip_verbatim(r#"{"t":"ride","slot":0,"key":"glow","value":[0.4,0.7,1.0]}"#);
    assert_eq!(
        rec,
        Record::Ride {
            slot: DeckSlot(0),
            at: None,
            key: "glow".to_string(),
            value: Value::Vec3([0.4, 0.7, 1.0]),
        }
    );
}

/// Verifies that a merge record round-trips with and without the optional `live` field.
#[test]
fn a_merge_round_trips_and_an_absent_live_stays_absent() {
    let rec = round_trip_verbatim(r#"{"t":"merge"}"#);
    assert_eq!(rec, Record::Merge { live: None });

    assert_eq!(
        round_trip_verbatim(r#"{"t":"merge","live":1}"#),
        Record::Merge { live: Some(1) }
    );

    // Renderer 0 is explicitly written when selected rather than omitted.
    assert_eq!(
        round_trip_verbatim(r#"{"t":"merge","live":0}"#),
        Record::Merge { live: Some(0) }
    );
}

/// Verifies that merge records are marked as Set state.
#[test]
fn a_merge_is_set_state_and_not_an_artifacts_declaration() {
    let rec = Record::Merge { live: Some(2) };
    assert!(rec.is_set_state());
    assert!(!rec.is_metadata());
}

/// The wire line the spec prints, parsed and written back.
#[test]
fn a_mask_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"mask","slot":1,"kind":"linear","angle":0.0,"position":0.5,"softness":0.1}"#;
    let rec = round_trip(line);
    assert_eq!(
        rec,
        Record::Mask {
            slot: DeckSlot(1),
            kind: "linear".to_string(),
            angle: 0.0,
            position: 0.5,
            softness: 0.1,
        }
    );
    assert!(!rec.is_set_state());
}

/// The wire line the spec prints, parsed and written back.
#[test]
fn a_canvas_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"canvas","width":1920,"height":1080}"#;
    let rec = round_trip(line);
    assert_eq!(
        rec,
        Record::Canvas {
            width: 1920,
            height: 1080,
        }
    );
    assert!(!rec.is_set_state());
}

/// Verifies that save records round-trip and are not marked as Set state.
#[test]
fn a_save_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"save","slot":1,"id":"20260816-143052-271"}"#;
    let rec = round_trip(line);
    assert_eq!(
        rec,
        Record::Save {
            slot: DeckSlot(1),
            id: "20260816-143052-271".to_string(),
        }
    );
    assert!(!rec.is_set_state());
}

/// Verifies that transport records round-trip verbatim and are not marked as Set state.
#[test]
fn a_transport_round_trips_through_the_line_the_spec_prints() {
    let line = r#"{"t":"transport","slot":0,"sync":"beat","anchor_bpm":126.0,"scrub_beats":-0.25}"#;
    let rec = round_trip_verbatim(line);
    assert_eq!(
        rec,
        Record::Transport {
            slot: DeckSlot(0),
            sync: "beat".to_string(),
            anchor_bpm: 126.0,
            scrub_beats: -0.25,
        }
    );
    assert!(!rec.is_set_state());
}

/// Verifies that legacy `offset_beats` produces a deserialization error naming `scrub_beats`.
#[test]
fn a_stream_with_the_old_offset_beats_fails_its_line_rather_than_reading_a_zero_scrub() {
    let old = r#"{"t":"transport","slot":0,"sync":"beat","anchor_bpm":126.0,"offset_beats":-0.25}"#;
    let err = serde_json::from_str::<Record>(old)
        .expect_err("the pre-rename spelling parsed — the rename left an alias behind");
    assert!(
        err.to_string().contains("scrub_beats"),
        "the error does not name the field that is missing: {err}"
    );
}

/// Verifies that legacy four-field master chain syntax is rejected with informative error.
#[test]
fn the_four_field_master_chain_is_refused_naming_the_shape() {
    let old = r#"{"t":"master_chain","feedback":0.5,"cut":"exit","bloom":0.6,"rgb_shift":0.4}"#;
    let err = serde_json::from_str::<Record>(old)
        .expect_err("the four-field form parsed — the list did not replace it");
    let said = err.to_string();
    assert!(
        said.contains("feedback"),
        "the refusal does not name the shape it found: {said}"
    );
    assert!(
        said.contains("slots"),
        "the refusal does not name the shape it wants: {said}"
    );
}

/// Verifies that slot list master chain records round-trip properly.
#[test]
fn a_master_chain_slot_list_round_trips() {
    let line = r#"{"t":"master_chain","slots":[{"proc":"sha256:a3","cut":"exit","params":{"amount":0.5}},{"proc":"sha256:77","params":{"amount":0.8}}]}"#;
    let Record::MasterChain(chain) = round_trip(line) else {
        panic!("not a master_chain");
    };
    assert_eq!(chain.slots.len(), 2);
    assert_eq!(chain.slots[0].procedure, "sha256:a3");
    assert_eq!(chain.slots[0].cut.as_deref(), Some("exit"));
    assert_eq!(chain.slots[0].params.get("amount"), Some(&0.5));
    assert_eq!(chain.slots[1].cut, None);
    assert_eq!(
        round_trip(r#"{"t":"master_chain","slots":[]}"#),
        Record::MasterChain(Chain::default())
    );
}

/// Verifies that chain slots with unrecognized keys are rejected.
#[test]
fn a_chain_slot_with_an_unknown_key_is_refused() {
    let line = r#"{"t":"master_chain","slots":[{"proc":"sha256:a3","amount":0.5}]}"#;
    let err = serde_json::from_str::<Record>(line)
        .expect_err("an unknown key on a chain slot was dropped");
    assert!(err.to_string().contains("amount"), "{err}");
}
